// Android owns the UI looper. These JNI calls never create an event loop or
// initialize a cartridge while loading a library.
#include <jni.h>
#include <android/native_window_jni.h>
#include <dlfcn.h>
#include <math.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/syscall.h>
#include "../common/pax-desktop.h"

typedef struct {
    void *library, *android;
    PaxDesktopEngine *engine;
    uint32_t (*abi)(void);
    void *(*init)(float,float,const uint8_t*,size_t,const uint8_t*,size_t);
    PaxDesktopEngine *(*unwrap)(void*);
    bool (*attach)(void*,uint64_t,void*,uint32_t,uint32_t);
    void (*detach)(void*,uint64_t);
    bool (*retired)(void*,uint64_t);
    bool (*finish)(void*);
    void (*free_buffer)(PaxDesktopBuffer);
    PaxDesktopBuffer (*tick)(PaxDesktopEngine*,float,float,float);
    PaxDesktopBuffer (*text)(const uint8_t*,size_t);
    PaxDesktopBuffer (*plan)(PaxDesktopEngine*,uint32_t,float);
    PaxDesktopBuffer (*surfaces)(PaxDesktopEngine*,const uint8_t*,size_t);
    PaxDesktopBuffer (*render)(PaxDesktopEngine*);
    PaxDesktopBuffer (*poll)(PaxDesktopEngine*);
    PaxDesktopBuffer (*interrupt)(PaxDesktopEngine*,const uint8_t*,size_t);
    PaxDesktopBuffer (*image)(PaxDesktopEngine*,const uint8_t*,size_t,const uint8_t*,size_t,uint32_t,uint32_t);
    PaxDesktopBuffer (*screenshot)(PaxDesktopEngine*,uint32_t,const uint8_t*,size_t,uint32_t,uint32_t);
    void (*activate)(PaxDesktopEngine*);
    void (*close)(PaxDesktopEngine*);
    uint64_t (*pump)(void);
    bool closing;
} Session;
static void fail(JNIEnv *env, const char *message) {
    if (!(*env)->ExceptionCheck(env)) (*env)->ThrowNew(env, (*env)->FindClass(env,"java/lang/IllegalStateException"),message);
}
static Session *checked(JNIEnv *env, jlong handle) {
    if (syscall(SYS_gettid) != getpid()) { fail(env,"Pax Android requires the UI thread"); return NULL; }
    if (!handle) { fail(env,"Pax Android session is closed"); return NULL; }
    return (Session*)(intptr_t)handle;
}
static jbyteArray response(JNIEnv *env, Session *s, PaxDesktopBuffer buffer) {
    jbyteArray value = NULL;
    if (buffer.data && buffer.len <= 16*1024*1024) {
        value=(*env)->NewByteArray(env,(jsize)buffer.len);
        if (value) (*env)->SetByteArrayRegion(env,value,0,(jsize)buffer.len,(const jbyte*)buffer.data);
    } else fail(env,"Invalid cartridge response");
    s->free_buffer(buffer);
    return value;
}
// Program exports identify the selected Cargo library; dependency catalogs have
// their own names and cannot shadow the selected initializer.
static void *program_symbol(JNIEnv *env, void *library, jstring symbol) {
    if (!symbol) { fail(env,"Missing Pax program export metadata"); return NULL; }
    const char *name=(*env)->GetStringUTFChars(env,symbol,NULL);
    if (!name) return NULL;
    void *address=dlsym(library,name);
    (*env)->ReleaseStringUTFChars(env,symbol,name);
    if (!address) fail(env,"Missing selected Pax program export");
    return address;
}
JNIEXPORT jlong JNICALL Java_dev_pax_PaxActivity_nativeInit(JNIEnv *env,jclass cls,jstring path,jstring abi_export,jstring init_export,jbyteArray entry,jbyteArray startup,jstring design_server,jfloat width,jfloat height) {
    (void)cls;
    if (syscall(SYS_gettid)!=getpid()) { fail(env,"Pax initialization requires the UI thread"); return 0; }
    if (!isfinite(width) || !isfinite(height) || width<=0 || height<=0) { fail(env,"Invalid initial Android viewport"); return 0; }
    // Only explicit Debug launches may connect to the adb-forwarded server.
    unsetenv("PAX_DESIGN_SERVER_ADDR");
#if PAX_DEBUG_HOST
    if (design_server) {
        const char *address=(*env)->GetStringUTFChars(env,design_server,NULL);
        if (!address) return 0;
        char *end=NULL;
        long port=strncmp(address,"ws://127.0.0.1:",15)==0 ? strtol(address+15,&end,10) : 0;
        bool valid=port>0 && port<=65535 && end && *end=='\0';
        if (valid) setenv("PAX_DESIGN_SERVER_ADDR",address,1);
        (*env)->ReleaseStringUTFChars(env,design_server,address);
        if (!valid) { fail(env,"Android development requires a loopback websocket URL"); return 0; }
    }
#else
    (void)design_server;
#endif
    Session *s=calloc(1,sizeof(*s));
    if (!s) { fail(env,"Cannot allocate Pax session"); return 0; }
    const char *name=(*env)->GetStringUTFChars(env,path,NULL);
    if (!name) { free(s); return 0; }
    s->library=dlopen(name,RTLD_NOW|RTLD_LOCAL);
    (*env)->ReleaseStringUTFChars(env,path,name);
    if (!s->library) { fail(env,dlerror()); free(s); return 0; }
    // Pin cartridge code: Rust TLS and asynchronous cleanup may outlive its UI.
#define LOAD(field,name) do { *(void**)(&s->field)=dlsym(s->library,name); if (!s->field) { fail(env,"Missing " name); free(s); return 0; } } while(0)
    *(void**)(&s->abi)=program_symbol(env,s->library,abi_export);
    if (!s->abi) { free(s); return 0; }
    *(void**)(&s->init)=program_symbol(env,s->library,init_export);
    if (!s->init) { free(s); return 0; }
    LOAD(unwrap,"pax_android_engine"); LOAD(attach,"pax_android_attach_surface");
    LOAD(detach,"pax_android_detach_surface"); LOAD(retired,"pax_android_surface_retired");
    LOAD(finish,"pax_android_finish_close"); LOAD(free_buffer,"pax_desktop_free_buffer");
    LOAD(tick,"pax_desktop_tick"); LOAD(plan,"pax_desktop_plan");
    LOAD(text,"pax_android_text");
    LOAD(surfaces,"pax_desktop_set_surfaces"); LOAD(render,"pax_desktop_render");
    LOAD(poll,"pax_desktop_poll"); LOAD(interrupt,"pax_desktop_interrupt");
    LOAD(image,"pax_desktop_load_image"); LOAD(activate,"pax_desktop_activate");
    LOAD(screenshot,"pax_desktop_screenshot");
    LOAD(close,"pax_desktop_begin_close"); LOAD(pump,"pax_desktop_pump_shutdowns");
#undef LOAD
    if (s->abi()!=5) { fail(env,"Android cartridge requires ABI 5"); free(s); return 0; }
    jsize en=(*env)->GetArrayLength(env,entry), sn=(*env)->GetArrayLength(env,startup);
    if (en>4096 || sn>65536) { fail(env,"Oversized Pax startup data"); free(s); return 0; }
    jbyte *eb=(*env)->GetByteArrayElements(env,entry,NULL), *sb=(*env)->GetByteArrayElements(env,startup,NULL);
    if (eb && sb) s->android=s->init(width,height,(uint8_t*)eb,en,(uint8_t*)sb,sn);
    if (eb) (*env)->ReleaseByteArrayElements(env,entry,eb,JNI_ABORT);
    if (sb) (*env)->ReleaseByteArrayElements(env,startup,sb,JNI_ABORT);
    if (!s->android) { fail(env,"Pax application initializer failed"); free(s); return 0; }
    s->engine=s->unwrap(s->android);
    return (jlong)(intptr_t)s;
}
JNIEXPORT jbyteArray JNICALL Java_dev_pax_PaxActivity_nativeCall(JNIEnv *env,jclass cls,jlong handle,jint op,jbyteArray data,jfloat x,jfloat y,jfloat scale) {
    (void)cls; Session *s=checked(env,handle); if (!s) return NULL;
    if (s->closing && op!=4) { fail(env,"Pax session is closing"); return NULL; }
    PaxDesktopBuffer result={0};
    if (op==0) result=s->tick(s->engine,x,y,scale);
    else if (op==1) result=s->plan(s->engine,(uint32_t)x,scale);
    else if (op==3) result=s->render(s->engine);
    else if (op==4) { result=s->poll(s->engine); s->pump(); }
    else if (op==2 || op==5 || op==6) {
        jsize n=(*env)->GetArrayLength(env,data);
        if (n>16*1024*1024) { fail(env,"Oversized Pax message"); return NULL; }
        jbyte *bytes=(*env)->GetByteArrayElements(env,data,NULL); if (!bytes) return NULL;
        result=op==6?s->text((uint8_t*)bytes,n):(op==2?s->surfaces:s->interrupt)(s->engine,(uint8_t*)bytes,n);
        (*env)->ReleaseByteArrayElements(env,data,bytes,JNI_ABORT);
    } else { fail(env,"Unknown Pax operation"); return NULL; }
    return response(env,s,result);
}
JNIEXPORT jboolean JNICALL Java_dev_pax_PaxActivity_nativeSurface(JNIEnv *env,jclass cls,jlong handle,jlong generation,jobject surface,jint width,jint height) {
    (void)cls; Session *s=checked(env,handle); if (!s) return false;
    if (!surface) { s->detach(s->android,generation); return s->retired(s->android,generation); }
    ANativeWindow *window=ANativeWindow_fromSurface(env,surface);
    if (!window) { fail(env,"Cannot acquire Android surface"); return false; }
    bool ok=s->attach(s->android,generation,window,width,height);
    ANativeWindow_release(window);
    if (!ok) fail(env,"Cannot attach Android surface generation");
    return ok;
}
JNIEXPORT void JNICALL Java_dev_pax_PaxActivity_nativeActivate(JNIEnv *env,jclass cls,jlong handle) {
    (void)cls; Session *s=checked(env,handle); if (s && !s->closing) s->activate(s->engine);
}
JNIEXPORT jboolean JNICALL Java_dev_pax_PaxActivity_nativeClose(JNIEnv *env,jclass cls,jlong handle) {
    (void)cls; Session *s=checked(env,handle); if (!s) return false;
    if (!s->closing) { s->closing=true; s->close(s->engine); }
    s->free_buffer(s->poll(s->engine)); s->pump();
    if (s->finish(s->android)) { free(s); return true; }
    return false;
}
JNIEXPORT jbyteArray JNICALL Java_dev_pax_PaxActivity_nativeImage(JNIEnv *env,jclass cls,jlong handle,jbyteArray path,jobject rgba,jint width,jint height) {
    (void)cls; Session *s=checked(env,handle); if (!s || s->closing) return NULL;
    jsize pn=(*env)->GetArrayLength(env,path);
    jlong n=(*env)->GetDirectBufferCapacity(env,rgba);
    if (width<=0 || height<=0 || width>8192 || height>8192 || (int64_t)width*height*4!=n || pn>4096) { fail(env,"Invalid image size"); return NULL; }
    jbyte *p=(*env)->GetByteArrayElements(env,path,NULL);
    const uint8_t *bytes=(*env)->GetDirectBufferAddress(env,rgba);
    PaxDesktopBuffer result={0};
    if (p && bytes) result=s->image(s->engine,(uint8_t*)p,pn,(uint8_t*)bytes,n,width,height);
    if(p) (*env)->ReleaseByteArrayElements(env,path,p,JNI_ABORT);
    if(!result.data) return NULL;
    return response(env,s,result);
}
JNIEXPORT jbyteArray JNICALL Java_dev_pax_PaxActivity_nativeScreenshot(JNIEnv *env,jclass cls,jlong handle,jint id,jobject rgba,jint width,jint height) {
    (void)cls; Session *s=checked(env,handle); if (!s || s->closing) return NULL;
    jlong length=rgba ? (*env)->GetDirectBufferCapacity(env,rgba) : 0;
    const uint8_t *bytes=rgba ? (*env)->GetDirectBufferAddress(env,rgba) : NULL;
    if (width<0 || height<0 || width>8192 || height>8192 || length<0 || length>64*1024*1024 ||
        (int64_t)width*height*4!=length || (length && !bytes)) { fail(env,"Invalid screenshot buffer"); return NULL; }
    return response(env,s,s->screenshot(s->engine,(uint32_t)id,bytes,(size_t)length,width,height));
}

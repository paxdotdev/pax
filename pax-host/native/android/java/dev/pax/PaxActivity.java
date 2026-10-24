package dev.pax;

import android.app.Activity;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.graphics.SurfaceTexture;
import android.view.Choreographer;
import android.view.WindowManager;
import android.util.Log;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import org.json.*;
import java.nio.charset.StandardCharsets;
import java.util.*;

/** Explicit Activity entry. Loading the host library never initializes user code. */
public class PaxActivity extends Activity implements Choreographer.FrameCallback {
    static native long nativeInit(String library, String abiExport, String initExport, byte[] entry, byte[] startup, String designServer, float width, float height);
    static native byte[] nativeCall(long handle, int operation, byte[] data, float x, float y, float scale);
    static native boolean nativeSurface(long handle, long generation, android.view.Surface surface, int width, int height);
    static native void nativeActivate(long handle);
    static native boolean nativeClose(long handle);
    static native byte[] nativeImage(long handle, byte[] path, java.nio.ByteBuffer pixels, int width, int height);
    static native byte[] nativeScreenshot(long handle, int id, java.nio.ByteBuffer pixels, int width, int height);
    static final byte[] EMPTY = new byte[0];
    static final Handler UI = new Handler(Looper.getMainLooper());
    static class Session {
        final long startedAt;
        Session(long startedAt) {this.startedAt=startedAt;}
        long handle, generation;
        int layers=1;
        final PendingUpdates pendingMessages=new PendingUpdates();
        final LinkedHashMap<String,JSONObject> pendingImages=new LinkedHashMap<>();
        String[] assetRoots=new String[0];
        PaxAssets assets;
        String baseUrl="";
        final ArrayList<String> routes=new ArrayList<>();
        boolean activated, closing;
        // Retain model data, never the old Activity, Views, or Java listeners.
        final LinkedHashMap<Integer,JSONObject> models = new LinkedHashMap<>();
        final HashMap<Integer,String> kinds = new HashMap<>();
        final HashMap<Long,SurfaceTexture> retired = new HashMap<>();
        static final String[] operations={"Pax.tick","Pax.surfacePlan","Pax.registry","Pax.render","Pax.poll","Pax.event","Pax.richText"};
        final FrameStats[] operationFrames={new FrameStats(),new FrameStats(),new FrameStats(),new FrameStats(),new FrameStats(),new FrameStats(),new FrameStats()};
        final int[] operationMaxReplyBytes=new int[operations.length];
        JSONObject operationReplySizes() {
            JSONObject sizes=new JSONObject();
            try {for(int i=0;i<operations.length;i++) sizes.put(operations[i],operationMaxReplyBytes[i]);}
            catch(JSONException error) {throw new IllegalStateException(error);}
            return sizes;
        }
        JSONObject operationTimings() {
            JSONObject timings=new JSONObject();
            try {for(int i=0;i<operations.length;i++) timings.put(operations[i],new JSONArray(operationFrames[i].summary()));}
            catch(JSONException error) {throw new IllegalStateException(error);}
            return timings;
        }
        JSONObject call(int op, Object data, float x, float y, float scale) {
            long started=System.nanoTime();
            android.os.Trace.beginSection(operations[op]);
            try {
                byte[] input=data==null?EMPTY:data.toString().getBytes(StandardCharsets.UTF_8);
                byte[] reply=nativeCall(handle,op,input,x,y,scale);
                operationMaxReplyBytes[op]=Math.max(operationMaxReplyBytes[op],reply.length);
                JSONObject result=new JSONObject(new String(reply,StandardCharsets.UTF_8));
                if(!result.optBoolean("ok")) throw new IllegalStateException(result.optString("error","Pax operation failed"));
                return result;
            } catch(JSONException error) { throw new IllegalStateException(error); }
            finally {operationFrames[op].add(System.nanoTime()-started);android.os.Trace.endSection();}
        }
        boolean event(String name, JSONObject data) {
            if(closing || handle==0) return false;
            return call(5, json(name,data),0,0,0).optBoolean("preventDefault");
        }
        void retire(long generation, SurfaceTexture texture) {
            if(handle==0) { texture.release(); return; }
            if(nativeSurface(handle,generation,null,0,0)) texture.release();
            else { retired.put(generation,texture); drainLater(); }
        }
        void drainLater() {
            UI.postDelayed(new Runnable() { public void run() {
                if(handle==0) return;
                try {
                    call(4,null,0,0,0);
                    retired.entrySet().removeIf(e -> {
                        if(!nativeSurface(handle,e.getKey(),null,0,0)) return false;
                        e.getValue().release(); return true;
                    });
                    if(!retired.isEmpty()) UI.postDelayed(this,16);
                } catch(RuntimeException error) { Log.e("Pax","Surface retirement failed",error); }
            }},16);
        }
        void close() {
            final long closing=handle;
            if(closing==0 || this.closing) return;
            this.closing=true;
            if(assets!=null) assets.close();
            UI.post(new Runnable() { public void run() {
                if(nativeClose(closing)) {
                    handle=0;
                    for(SurfaceTexture texture:retired.values()) texture.release();
                    retired.clear();
                    Log.i("Pax","session drained");
                } else UI.postDelayed(this,16);
            }});
        }
    }
    Session session;
    PaxScene scene;
    boolean resumed, failed;
    final FrameStats paxFrames=new FrameStats(),windowFrames=new FrameStats();
    final android.view.Window.OnFrameMetricsAvailableListener metricsListener=(window,metrics,dropped)->
        windowFrames.add(metrics.getMetric(android.view.FrameMetrics.TOTAL_DURATION));
    Runnable unregisterBack;
    static class BackApi {
        static Runnable register(PaxActivity activity) {
            android.window.OnBackInvokedDispatcher dispatcher=activity.getOnBackInvokedDispatcher();
            android.window.OnBackInvokedCallback callback=activity::back;
            dispatcher.registerOnBackInvokedCallback(android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT,callback);
            return ()->dispatcher.unregisterOnBackInvokedCallback(callback);
        }
    }
    @Override public void onCreate(Bundle state) {
        long startupStarted=android.os.SystemClock.elapsedRealtime();
        super.onCreate(state);
        getWindow().addOnFrameMetricsAvailableListener(metricsListener,UI);
        getWindow().setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE);
        if(Build.VERSION.SDK_INT>=30) getWindow().setDecorFitsSystemWindows(false);
        System.loadLibrary("pax_host");
        session=(Session)getLastNonConfigurationInstance();
        try {
            if(session==null) {
                JSONObject config;
                try(java.io.InputStream stream=getAssets().open("pax-app.json")) {
                    config=new JSONObject(new String(readBytes(stream),StandardCharsets.UTF_8));
                }
                session=new Session(startupStarted);
                JSONArray roots=config.getJSONArray("assetRoots");session.assetRoots=new String[roots.length()];
                for(int i=0;i<roots.length();i++)session.assetRoots[i]=roots.getString(i);
                session.assets=new PaxAssets(getApplicationContext(),session.assetRoots);
                session.baseUrl=config.optString("baseUrl","");
                String startup=getIntent().getStringExtra("pax.startup");
                String entry=getIntent().getStringExtra("pax.entry");
                android.util.DisplayMetrics metrics=new android.util.DisplayMetrics();
                getWindowManager().getDefaultDisplay().getMetrics(metrics);
                android.graphics.Rect bounds=Build.VERSION.SDK_INT>=30
                    ?getWindowManager().getCurrentWindowMetrics().getBounds()
                    :new android.graphics.Rect(0,0,metrics.widthPixels,metrics.heightPixels);
                JSONObject exports=config.getJSONObject("nativeExports");
                session.handle=nativeInit(getApplicationInfo().nativeLibraryDir+"/"+config.getString("library"),
                    exports.getString("abiVersion"),exports.getString("initEntry"),
                    (entry==null?"":entry).getBytes(StandardCharsets.UTF_8),
                    (startup==null?"":startup).getBytes(StandardCharsets.UTF_8),
                    getIntent().getStringExtra("pax.design_server"),bounds.width()/metrics.density,bounds.height()/metrics.density);
                Log.i("Pax","explicit initialization complete");
                session.routes.add(state==null?"/":state.getString("pax.route","/"));
                publishRoute(session.routes.get(0));
            } else Log.i("Pax","reattaching retained session");
            scene=new PaxScene(this,session);
            setContentView(scene);
            scene.requestApplyInsets();
            if(Build.VERSION.SDK_INT>=33) unregisterBack=BackApi.register(this);
        } catch(Exception error) { fatal(error); }
    }
    @Override public void onSaveInstanceState(Bundle state) {
        super.onSaveInstanceState(state);
        if(session!=null && !session.routes.isEmpty()) state.putString("pax.route",session.routes.get(session.routes.size()-1));
    }
    @Override public void onBackPressed() { back(); }
    void back() {
        if(session==null || session.closing || failed) { finish();return; }
        try {
            if(session.event("KeyDown",json("key","Escape","modifiers",new JSONArray(),"is_repeat",false))) return;
            if(session.routes.size()>1) {
                session.routes.remove(session.routes.size()-1);publishRoute(session.routes.get(session.routes.size()-1));
            } else finish();
        } catch(RuntimeException error) {fatal(error);}
    }
    void publishRoute(String location) {
        Uri uri=Uri.parse(location);JSONArray segments=new JSONArray();JSONObject query=new JSONObject();
        for(String part:uri.getPathSegments()) segments.put(part);
        try {
            for(String key:uri.getQueryParameterNames()) query.put(key,new JSONArray(uri.getQueryParameters(key)));
        } catch(JSONException error) {throw new IllegalArgumentException(error);}
        session.event("RouteChange",json("path_segments",segments,"query",query,"fragment",uri.getFragment()==null?JSONObject.NULL:uri.getFragment()));
    }
    void navigate(JSONObject request) {
        try {navigateChecked(request);}
        catch(IllegalArgumentException error) {Log.e("Pax","Invalid navigation URL",error);}
    }
    void navigateChecked(JSONObject request) {
        String value=request.optString("url","");Uri uri=Uri.parse(value);
        boolean external=uri.getScheme()!=null || uri.getAuthority()!=null || !"Current".equalsIgnoreCase(request.optString("target","Current"));
        if(!external) {
            // Relative locations are shared Router state. Web document-hosting
            // policy does not change a native application's route transitions.
            String current=session.routes.isEmpty()?"/":session.routes.get(session.routes.size()-1);
            String route=NativeRoutes.resolve(current,value);
            if(!route.equals(current)) {
                if(session.routes.size()==128) session.routes.remove(0);
                session.routes.add(route);
            }
            publishRoute(route);return;
        }
        if(uri.getScheme()==null && !session.baseUrl.isEmpty()) uri=Uri.parse(java.net.URI.create(session.baseUrl).resolve(value).toString());
        String scheme=uri.getScheme();
        if(scheme==null || !Arrays.asList("https","http","mailto","tel").contains(scheme.toLowerCase(Locale.ROOT))) {
            Log.e("Pax","Unsupported navigation URL: "+value);return;
        }
        try {startActivity(new Intent(Intent.ACTION_VIEW,uri));}
        catch(android.content.ActivityNotFoundException error) {Log.e("Pax","No application handles "+uri,error);}
    }
    @Override public Object onRetainNonConfigurationInstance() { return failed ? null : session; }
    @Override public void onResume() { super.onResume(); resumed=true; Choreographer.getInstance().postFrameCallback(this); }
    @Override public void onPause() { resumed=false; Choreographer.getInstance().removeFrameCallback(this); super.onPause(); }
    @Override public void onDestroy() {
        getWindow().removeOnFrameMetricsAvailableListener(metricsListener);
        if(unregisterBack!=null) unregisterBack.run();
        Choreographer.getInstance().removeFrameCallback(this);
        if(scene!=null) scene.dispose();
        if(session!=null && !isChangingConfigurations()) session.close();
        super.onDestroy();
    }
    @Override public boolean dispatchKeyEvent(android.view.KeyEvent event) {
        if(!failed && session!=null && session.handle!=0 && !session.closing) {
            String key=keyName(event);
            if(key!=null && (event.getAction()==android.view.KeyEvent.ACTION_DOWN || event.getAction()==android.view.KeyEvent.ACTION_UP)) {
                JSONArray modifiers=new JSONArray();
                if(event.isShiftPressed()) modifiers.put("Shift");
                if(event.isCtrlPressed()) modifiers.put("Control");
                if(event.isAltPressed()) modifiers.put("Alt");
                if(event.isMetaPressed()) modifiers.put("Command");
                try {
                    if(session.event(event.getAction()==android.view.KeyEvent.ACTION_DOWN?"KeyDown":"KeyUp",
                        json("key",key,"modifiers",modifiers,"is_repeat",event.getRepeatCount()>0))) return true;
                } catch(RuntimeException error) { fatal(error); return true; }
            }
        }
        return super.dispatchKeyEvent(event);
    }
    // IME composition is delivered by EditText's editable buffer, never synthesized
    // from key events. This route covers hardware keys and their native defaults.
    static String keyName(android.view.KeyEvent event) {
        switch(event.getKeyCode()) {
            case android.view.KeyEvent.KEYCODE_ENTER: return "Enter";
            case android.view.KeyEvent.KEYCODE_TAB: return "Tab";
            case android.view.KeyEvent.KEYCODE_ESCAPE: return "Escape";
            case android.view.KeyEvent.KEYCODE_DEL: return "Backspace";
            case android.view.KeyEvent.KEYCODE_FORWARD_DEL: return "Delete";
            case android.view.KeyEvent.KEYCODE_DPAD_LEFT: return "ArrowLeft";
            case android.view.KeyEvent.KEYCODE_DPAD_RIGHT: return "ArrowRight";
            case android.view.KeyEvent.KEYCODE_DPAD_UP: return "ArrowUp";
            case android.view.KeyEvent.KEYCODE_DPAD_DOWN: return "ArrowDown";
            case android.view.KeyEvent.KEYCODE_MOVE_HOME: return "Home";
            case android.view.KeyEvent.KEYCODE_MOVE_END: return "End";
            case android.view.KeyEvent.KEYCODE_PAGE_UP: return "PageUp";
            case android.view.KeyEvent.KEYCODE_PAGE_DOWN: return "PageDown";
            default:
                int character=event.getUnicodeChar();
                return character>0 && Character.isValidCodePoint(character)?new String(Character.toChars(character)):null;
        }
    }
    @Override public void doFrame(long timestamp) {
        if(!resumed || failed || scene==null || session.handle==0) return;
        long start=System.nanoTime();
        try {
            if(scene.getWidth()>0 && scene.getHeight()>0) scene.frame();
        } catch(Exception error) { fatal(error); return; }
        paxFrames.add(System.nanoTime()-start);
        Choreographer.getInstance().postFrameCallback(this);
    }
    void fatal(Exception error) {
        if(failed) return;
        failed=true; Log.e("Pax","Android host failure",error);
        Choreographer.getInstance().removeFrameCallback(this);
        if(scene!=null) { scene.dispose(); scene=null; }
        if(session!=null) session.close();
        android.widget.TextView message=new android.widget.TextView(this);
        message.setText("Pax could not start\n"+error.getMessage());
        message.setPadding(32,32,32,32); setContentView(message);
    }
    static JSONObject json(Object...pairs) {
        JSONObject object=new JSONObject();
        try { for(int i=0;i<pairs.length;i+=2) object.put((String)pairs[i],pairs[i+1]); }
        catch(JSONException error) { throw new IllegalArgumentException(error); }
        return object;
    }
    static byte[] readBytes(java.io.InputStream stream) throws java.io.IOException {
        java.io.ByteArrayOutputStream output=new java.io.ByteArrayOutputStream();
        byte[] buffer=new byte[16384]; int n;
        while((n=stream.read(buffer))!=-1) output.write(buffer,0,n);
        return output.toByteArray();
    }
}

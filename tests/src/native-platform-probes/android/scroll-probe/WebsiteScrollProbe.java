package dev.pax;
import android.app.Instrumentation;
import android.content.Intent;
import android.graphics.*;
import android.os.*;
import android.view.*;
import android.widget.TextView;
import org.json.*;
import java.io.*;
import java.util.*;

/** Capture asynchronous website flings for review of native/canvas composition. */
public final class WebsiteScrollProbe extends Instrumentation {
    PaxActivity activity; PaxScene.Node scroller;
    File output; PrintWriter trace; long started; volatile String phase="initial";
    volatile boolean sampling; volatile Throwable samplingError; int captures,maximumScroll;
    IdentityHashMap<PaxScene.Tile,Long> latched=new IdentityHashMap<>();
    ViewTreeObserver.OnPreDrawListener observer;
    public void onCreate(Bundle args) {super.onCreate(args);start();}
    public void onStart() {
        Bundle result=new Bundle();
        try {
            activity=(PaxActivity)startActivitySync(new Intent().setClassName(getTargetContext(),"dev.pax.PaxActivity")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK).putExtra("pax.diagnostics",true));
            output=new File(activity.getExternalFilesDir(null),"website-scroll");output.mkdirs();
            for(File f:output.listFiles()) f.delete();
            trace=new PrintWriter(new File(output,"trace.jsonl"));started=SystemClock.uptimeMillis();
            long deadline=started+45000; boolean[] ready={false};
            while(!ready[0] && SystemClock.uptimeMillis()<deadline) {
                runOnMainSync(()->ready[0]=activity.scene!=null && activity.session.activated && activity.scene.committed>10);
                SystemClock.sleep(100);
            }
            require(ready[0],"No website first frame");
            runOnMainSync(()-> {
                for(PaxScene.Node n:activity.scene.nodes.values()) if(n.vertical!=null &&
                    (scroller==null||n.content.getHeight()>scroller.content.getHeight())) scroller=n;
                require(scroller!=null,"No page scroller");
                observer=()-> {record();return true;}; activity.scene.getViewTreeObserver().addOnPreDrawListener(observer);
            });
            cancel();SystemClock.sleep(2000);
            sampling=true;
            Thread sampler=new Thread(()->{try{while(sampling && captures<320){capture();SystemClock.sleep(60);}}
                catch(Throwable e){samplingError=e;}},"website-pixels");sampler.start();
            try {
                phase="live-hero-reversal";
                for(int i=0;i<2;i++) reversal(.82f,.35f,9);
                phase="settling-before-freeze";
                runOnMainSync(()->Choreographer.getInstance().removeFrameCallback(activity));
                // Drain the publication already submitted before freezing Pax.
                boolean[] settled={false};long until=SystemClock.uptimeMillis()+10000;
                while(!settled[0] && SystemClock.uptimeMillis()<until) {
                    runOnMainSync(()->settled[0]=activity.scene.presentationNeeded==null && !activity.scene.presentationDrawPending);
                    SystemClock.sleep(20);
                }
                require(settled[0],"Publication did not settle before frozen scrolling");
                long[] frozen={0,0};runOnMainSync(()->{frozen[0]=activity.scene.ticks;frozen[1]=activity.scene.committed;});
                phase="frozen-hero-reversal";
                for(int i=0;i<2;i++) reversal(.80f,.48f,9);
                runOnMainSync(()-> {
                    require(activity.scene.ticks==frozen[0] && activity.scene.committed==frozen[1],"Pax published during frozen scrolling");
                    Choreographer.getInstance().postFrameCallback(activity);
                });
                phase="live-page-fling";
                for(int i=0;i<12;i++) {drag(.87f,.16f,7);SystemClock.sleep(500);}
                phase="live-page-reverse";
                for(int i=0;i<12;i++) {drag(.16f,.87f,7);SystemClock.sleep(500);}
                phase="edge-stretch";
                for(int i=0;i<2;i++) reversal(.30f,.90f,9);
            } finally {sampling=false;sampler.join();runOnMainSync(()->activity.scene.getViewTreeObserver().removeOnPreDrawListener(observer));trace.close();}
            if(samplingError!=null) throw new AssertionError(samplingError);
            require(captures>=20,"Insufficient composed captures");
            runOnMainSync(()->require(maximumScroll>scroller.content.getHeight()/2,"Did not traverse the main page"));
            result.putBoolean("requires_visual_review",true);result.putInt("maximum_scroll_pixels",maximumScroll);
            result.putInt("captures",captures);result.putString("evidence",output.getAbsolutePath());finish(-1,result);
        } catch(Throwable e) {cancel();if(trace!=null)trace.close();result.putString("error",e.toString());finish(1,result);}
    }
    void record() {
        try {
            maximumScroll=Math.max(maximumScroll,scroller.vertical.getScrollY());
            PaxScene scene=activity.scene;JSONArray tiles=new JSONArray(),nodes=new JSONArray();
            for(PaxScene.Tile t:scene.tiles.values()) {
                if(!latched.containsKey(t)) {latched.put(t,0L);TextureView.SurfaceTextureListener previous=t.getSurfaceTextureListener();
                    t.setSurfaceTextureListener(new TextureView.SurfaceTextureListener(){
                        public void onSurfaceTextureAvailable(SurfaceTexture s,int w,int h){previous.onSurfaceTextureAvailable(s,w,h);}
                        public boolean onSurfaceTextureDestroyed(SurfaceTexture s){return previous.onSurfaceTextureDestroyed(s);}
                        public void onSurfaceTextureSizeChanged(SurfaceTexture s,int w,int h){previous.onSurfaceTextureSizeChanged(s,w,h);}
                        public void onSurfaceTextureUpdated(SurfaceTexture s){latched.put(t,SystemClock.uptimeMillis()-started);previous.onSurfaceTextureUpdated(s);}
                    });
                }
                int[] p=new int[2];t.getLocationOnScreen(p);
                tiles.put(PaxActivity.json("key",t.key,"owner",t.owner,"x",t.getX(),"y",t.getY(),"screenY",p[1],"latched",latched.get(t),"origin",t.info.optDouble("top"),"height",t.pixelsH));
            }
            for(PaxScene.Node n:scene.nodes.values()) if(n.control instanceof TextView) {
                String text=((TextView)n.control).getText().toString();if(text.isEmpty())continue;
                Rect r=new Rect();if(!n.getGlobalVisibleRect(r))continue;
                nodes.put(PaxActivity.json("id",n.id,"text",text.substring(0,Math.min(60,text.length())),"parent",n.model.opt("parent_frame"),"rect",new JSONArray(new int[]{r.left,r.top,r.right,r.bottom}),"worldY",n.worldValues[5]));
            }
            trace.println(PaxActivity.json("ms",SystemClock.uptimeMillis()-started,"phase",phase,"ticks",scene.ticks,"committed",scene.committed,"awaitingBuffers",scene.awaitingBuffers.size(),"awaitingDraw",scene.presentationDrawPending,"scroll",scroller.vertical.getScrollY(),"contentHeight",scroller.content.getHeight(),"tiles",tiles,"nodes",nodes));
        }catch(Throwable e){throw new RuntimeException(e);}
    }
    void capture() throws Exception {
        long time=SystemClock.uptimeMillis()-started;String state=phase;
        Bitmap b=getUiAutomation().takeScreenshot();if(b==null)return;
        try(FileOutputStream stream=new FileOutputStream(new File(output,String.format("%04d-%06d-%s.png",captures++,time,state)))) {b.compress(Bitmap.CompressFormat.PNG,100,stream);}finally{b.recycle();}
    }
    void reversal(float from,float to,int steps){Rect b=bounds();float x=b.left+b.width()*.025f,start=b.top+b.height()*from,end=b.top+b.height()*to;long down=SystemClock.uptimeMillis();inject(down,0,x,start);
        for(int i=1;i<=steps;i++){inject(down,2,x,start+(end-start)*i/steps);SystemClock.sleep(12);}
        for(int i=1;i<=steps;i++){inject(down,2,x,end+(start-end)*i/steps);SystemClock.sleep(12);}
        inject(down,1,x,start);SystemClock.sleep(200);
    }
    void drag(float from,float to,int steps){Rect b=bounds();float x=b.left+b.width()*.025f,start=b.top+b.height()*from,end=b.top+b.height()*to;long down=SystemClock.uptimeMillis();inject(down,0,x,start);
        for(int i=1;i<=steps;i++){inject(down,2,x,start+(end-start)*i/steps);SystemClock.sleep(12);}inject(down,1,x,end);
    }
    Rect bounds(){Rect b=new Rect();runOnMainSync(()->scroller.vertical.getGlobalVisibleRect(b));return b;}
    void inject(long down,int action,float x,float y){MotionEvent e=MotionEvent.obtain(down,SystemClock.uptimeMillis(),action,x,y,0);e.setSource(InputDevice.SOURCE_TOUCHSCREEN);try{require(getUiAutomation().injectInputEvent(e,false),"Rejected touch");}finally{e.recycle();}}
    void cancel(){long t=SystemClock.uptimeMillis();MotionEvent e=MotionEvent.obtain(t,t,3,0,0,0);e.setSource(InputDevice.SOURCE_TOUCHSCREEN);try{getUiAutomation().injectInputEvent(e,true);}finally{e.recycle();}}
    static void require(boolean c,String m){if(!c)throw new AssertionError(m);}
}

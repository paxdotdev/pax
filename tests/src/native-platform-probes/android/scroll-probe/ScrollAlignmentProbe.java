package dev.pax;

import android.app.Instrumentation;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.*;
import java.io.File;
import java.io.FileOutputStream;
import java.util.ArrayList;

/** Compare actual native/GPU pixels while Android scrolls, including without Pax frames. */
public final class ScrollAlignmentProbe extends Instrumentation {
    PaxActivity activity;
    PaxScene.Node scroller;
    int captures, comparisons, maximumError;
    float maximumScroll;
    File output;
    String phase;
    volatile boolean sampling;
    volatile Throwable samplingError;
    boolean nested;
    @Override public void onCreate(Bundle args) {super.onCreate(args);nested="true".equals(args.getString("nested"));start();}
    @Override public void onStart() {
        Bundle result=new Bundle();
        try {
            Intent intent=new Intent().setClassName(getTargetContext(),"dev.pax.PaxActivity")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK).putExtra("pax.startup",nested?"scroll-alignment-nested":"scroll-alignment");
            activity=(PaxActivity)startActivitySync(intent);
            output=new File(activity.getExternalFilesDir(null),"scroll-alignment");output.mkdirs();
            for(File file:output.listFiles()) if(file.getName().endsWith(".png")) file.delete();
            long deadline=SystemClock.uptimeMillis()+45000;
            boolean[] ready={false};
            while(!ready[0] && SystemClock.uptimeMillis()<deadline) {
                runOnMainSync(()->ready[0]=!activity.failed && activity.session.activated && activity.scene.committed>5);
                SystemClock.sleep(100);
            }
            require(ready[0],"Fixture did not present");
            runOnMainSync(()-> {
                for(PaxScene.Node node:activity.scene.nodes.values()) if(node.vertical!=null &&
                    (scroller==null || node.content.getHeight()>scroller.content.getHeight())) scroller=node;
                require(scroller!=null,"Missing native scroller");
                int ownedTiles=0;
                for(PaxScene.Tile tile:activity.scene.tiles.values()) if(tile.owner==scroller.id) {
                    require(tile.getParent()==scroller.content,"Canvas is outside native scrolling content");
                    ownedTiles++;
                }
                require(ownedTiles>0,"No canvas tiles owned by the native scroller");
                Choreographer.getInstance().removeFrameCallback(activity);
            });
            long[] frozenTicks={0};runOnMainSync(()->frozenTicks[0]=activity.scene.ticks);
            // A terminated instrumentation process can leave its injected pointer
            // down in InputDispatcher. Cancel that virtual-device sequence first.
            cancelTouch();
            phase="frozen";
            SystemClock.sleep(200);capture();
            exercise(4,0.78f,0.47f,true);
            runOnMainSync(()-> {
                require(activity.scene.ticks==frozenTicks[0],"Pax rendered during frozen phase");
                require(maximumScroll>100,"Frozen phase did not scroll");
                Choreographer.getInstance().postFrameCallback(activity);
            });
            result.putInt("frozen_captures",captures);
            result.putFloat("frozen_maximum_scroll",maximumScroll);
            phase="live";
            exercise(20,0.90f,0.10f,false);
            require(maximumScroll>3000,"Did not cross recycled tile windows: "+maximumScroll);
            runOnMainSync(()->require(activity.scene.ticks>frozenTicks[0],"Pax did not resume rendering"));
            result.putInt("captures",captures);result.putInt("paired_edges",comparisons);
            result.putInt("maximum_error_pixels",maximumError);result.putFloat("maximum_scroll",maximumScroll);
            result.putString("evidence",output.getAbsolutePath());
            finish(-1,result);
        } catch(Throwable error) {cancelTouch();result.putString("error",error.toString());finish(1,result);}
    }
    void exercise(int gestures,float low,float high,boolean alternate) throws Exception {
        sampling=true;samplingError=null;
        Thread sampler=new Thread(()-> {
            try {while(sampling) {capture();SystemClock.sleep(16);}}
            catch(Throwable error) {samplingError=error;}
        },"scroll-pixel-sampler");
        sampler.start();
        try {
            for(int i=0;i<gestures && samplingError==null;i++) {
                boolean forward=alternate?i%2==0:i<gestures/2;
                drag(forward?low:high,forward?high:low,16);
            }
        } finally {sampling=false;sampler.join();}
        if(samplingError!=null) throw new AssertionError(samplingError);
        capture();
    }
    void drag(float from,float to,int steps) throws Exception {
        Rect bounds=new Rect();runOnMainSync(()->scroller.vertical.getGlobalVisibleRect(bounds));
        require(!bounds.isEmpty(),"Scroller is not visible");
        float x=bounds.left+bounds.width()*0.92f;
        float start=bounds.top+from*bounds.height(),end=bounds.top+to*bounds.height();
        long down=SystemClock.uptimeMillis();
        inject(down,MotionEvent.ACTION_DOWN,x,start);
        for(int i=1;i<=steps;i++) {
            inject(down,MotionEvent.ACTION_MOVE,x,start+(end-start)*i/steps);
            SystemClock.sleep(16);
        }
        SystemClock.sleep(100);
        inject(down,MotionEvent.ACTION_MOVE,x,end);
        inject(down,MotionEvent.ACTION_UP,x,end);
        SystemClock.sleep(60);
    }
    void inject(long down,int action,float x,float y) {
        MotionEvent event=MotionEvent.obtain(down,SystemClock.uptimeMillis(),action,x,y,0);
        event.setSource(InputDevice.SOURCE_TOUCHSCREEN);
        try {require(getUiAutomation().injectInputEvent(event,true),"Android rejected touch");}
        finally {event.recycle();}
    }
    void cancelTouch() {
        long now=SystemClock.uptimeMillis();
        MotionEvent event=MotionEvent.obtain(now,now,MotionEvent.ACTION_CANCEL,0,0,0);
        event.setSource(InputDevice.SOURCE_TOUCHSCREEN);
        try {getUiAutomation().injectInputEvent(event,true);}
        finally {event.recycle();}
    }
    void capture() throws Exception {
        Rect viewport=new Rect();
        runOnMainSync(()-> {
            maximumScroll=Math.max(maximumScroll,scroller.vertical.getScrollY()/activity.scene.density);
            scroller.vertical.getGlobalVisibleRect(viewport);
        });
        Bitmap bitmap=getUiAutomation().takeScreenshot();require(bitmap!=null,"No screenshot");
        try {
            int w=bitmap.getWidth(),h=bitmap.getHeight();int[] row=new int[w];
            ArrayList<Integer> nativeEdges=new ArrayList<>(),gpuEdges=new ArrayList<>();
            boolean nativePrevious=false,gpuPrevious=false;
            int top=3,bottom=h-3;
            for(int y=top;y<bottom;y++) {
                bitmap.getPixels(row,0,w,0,y,w,1);int nativeCount=0,gpuCount=0;
                for(int c:row) {
                    int r=(c>>16)&255,g=(c>>8)&255,b=c&255;
                    if(r<60 && g>160 && b<140) nativeCount++;
                    if(r>160 && g<80 && b>130) gpuCount++;
                }
                boolean n=nativeCount>30,g=gpuCount>30;
                // Ignore an edge cut by the screenshot boundary itself.
                if(y>top && n!=nativePrevious) nativeEdges.add(y);
                if(y>top && g!=gpuPrevious) gpuEdges.add(y);
                nativePrevious=n;gpuPrevious=g;
            }
            File file=new File(output,String.format("%s-%03d.png",phase,captures++));
            try(FileOutputStream stream=new FileOutputStream(file)) {bitmap.compress(Bitmap.CompressFormat.PNG,100,stream);}
            require(nativeEdges.size()>=4,"Too few native markers: "+file);
            compareEdges(nativeEdges,gpuEdges,viewport,file);
        } finally {bitmap.recycle();}
    }
    static boolean clipped(ArrayList<Integer> edges,int index,Rect viewport) {
        return edges.get(index)<=viewport.top+2 || edges.get(index+1)>=viewport.bottom-2;
    }
    void compareEdges(ArrayList<Integer> nativeEdges,ArrayList<Integer> gpuEdges,Rect viewport,File file) {
        int i=0,j=0,matched=0;
        while(i+1<nativeEdges.size() || j+1<gpuEdges.size()) {
            boolean n=i+1<nativeEdges.size(),g=j+1<gpuEdges.size();
            if(n && g) {
                int error=Math.max(Math.abs(nativeEdges.get(i)-gpuEdges.get(j)),
                    Math.abs(nativeEdges.get(i+1)-gpuEdges.get(j+1)));
                if(error<=2) {
                    // Pair before excluding clip-boundary bands: rounding can
                    // put only one peer within two pixels of the viewport edge.
                    if(!clipped(nativeEdges,i,viewport) && !clipped(gpuEdges,j,viewport)) {
                        maximumError=Math.max(maximumError,error);comparisons+=2;matched++;
                    }
                    i+=2;j+=2;continue;
                }
            }
            // One renderer may leave a one-pixel sliver where the other is
            // fully clipped. Never skip an unmatched interior marker.
            if(n && clipped(nativeEdges,i,viewport) && (!g || nativeEdges.get(i)<=gpuEdges.get(j)+2)) {i+=2;continue;}
            if(g && clipped(gpuEdges,j,viewport) && (!n || gpuEdges.get(j)<=nativeEdges.get(i)+2)) {j+=2;continue;}
            throw new AssertionError("Missing or displaced canvas markers: "+file+" native="+nativeEdges+" gpu="+gpuEdges);
        }
        require(matched>=2,"Too few complete paired markers: "+file);
    }
    static void require(boolean condition,String message) {if(!condition) throw new AssertionError(message);}
}

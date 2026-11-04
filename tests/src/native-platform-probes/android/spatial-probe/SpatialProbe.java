package dev.pax;

import android.app.Instrumentation;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.*;
import android.widget.EditText;
import android.widget.TextView;
import java.io.File;
import java.io.FileOutputStream;
import java.util.ArrayList;

/** Real native input and retained identity in the orthographic spatial fixture. */
public final class SpatialProbe extends Instrumentation {
    PaxActivity activity;
    boolean edgeOn;
    File output;
    final ArrayList<String> checks=new ArrayList<>();
    interface Condition {boolean test();}
    @Override public void onCreate(Bundle args) {
        super.onCreate(args);edgeOn=args!=null && "true".equals(args.getString("edgeOn"));start();
    }
    @Override public void onStart() {
        Bundle result=new Bundle();
        try {
            activity=(PaxActivity)startActivitySync(new Intent()
                .setClassName(getTargetContext(),"dev.pax.PaxActivity").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            output=new File(activity.getExternalFilesDir(null),"spatial-acceptance");output.mkdirs();
            // A previous failed run must not donate captures to a later pass.
            for(String name:new String[]{"before-input","orthographic","zoomed","clipped","edge-on","recovered","failure"})
                for(String extension:new String[]{".png",".json"}) {
                    File old=new File(output,name+extension);
                    if(old.exists() && !old.delete())throw new IllegalStateException("Cannot reset probe capture: "+old);
                }
            waitFor(()->!activity.failed && activity.session.activated && activity.scene.committed>2,"first GPU frames");
            waitFor(()->activity.scene.getWidth()/activity.scene.density>=1000,"wide fixture viewport");
            for(String label:new String[]{"Open tile / near-plane lab","Show solid volume"}) {
                PaxScene.Node compact=button(label);
                onMain(()->{
                    TextView text=(TextView)compact.control;
                    require(text.getLayout()!=null && text.getLayout().getHeight()<=
                        text.getHeight()-text.getCompoundPaddingTop()-text.getCompoundPaddingBottom(),
                        "authored compact button fits its label: "+label);
                });
            }
            click(button("Reset pose"));click(button("Hide amber"));
            waitFor(()->hasButton("Show amber"),"amber surface is hidden");
            PaxScene.Node editor=findEditor();final int editorId=editor.id;
            Rect original=bounds(editor.control);
            require(original.width()>200,"transformed editor is visible");
            capture("before-input");
            click(editor);
            waitFor(()->editor.control.hasFocus(),"transformed editor receives native focus");
            key(KeyEvent.KEYCODE_A,KeyEvent.META_CTRL_ON);
            type("Pax orthographic");
            waitFor(()->"Pax orthographic".contentEquals(((EditText)editor.control).getText()),"native text entry");
            boolean[] keyboard={false};
            onMain(()->keyboard[0]=android.os.Build.VERSION.SDK_INT>=30 &&
                activity.getWindow().getDecorView().getRootWindowInsets().isVisible(WindowInsets.Type.ime()));
            if(keyboard[0])key(KeyEvent.KEYCODE_BACK,0);
            click(button("Press / 0"));
            waitFor(()->hasButton("Press / 1"),"transformed button routes native input");
            capture("orthographic");
            click(button("Zoom out"));
            waitFor(()->boundsOnMain(editor.control).width()<original.width()*.9,"orthographic zoom changes native bounds");
            onMain(()->require(activity.scene.nodes.get(editorId)==editor &&
                "Pax orthographic".contentEquals(((EditText)editor.control).getText()),"zoom retains editor identity and text"));
            capture("zoomed");
            click(button("Zoom reset"));
            waitFor(()->Math.abs(boundsOnMain(editor.control).width()-original.width())<=2,"zoom resets placement");
            Rect before=bounds(editor.control);
            click(button("Switch to perspective"));
            waitFor(()->hasButton("Switch to orthographic"),"camera toggle reaches application state");
            SystemClock.sleep(400);
            Rect after=bounds(editor.control);
            require(Math.abs(before.left-after.left)<=2 && Math.abs(before.top-after.top)<=2 &&
                Math.abs(before.width()-after.width())<=2,"unsupported perspective retains orthographic placement");
            click(button("Switch to orthographic"));
            click(button("Aperture off"));
            waitFor(()->hasButton("Aperture on"),"native clipping toggles");
            capture("clipped");
            if(edgeOn) {
                PaxScene.Node[] tilt={null};onMain(()->{
                    for(PaxScene.Node n:activity.scene.nodes.values())if(n.kind.equals("Slider") &&
                        n.model.optDouble("min")==-180 && n.model.optDouble("max")==180)tilt[0]=n;
                });
                require(tilt[0]!=null,"native Y tilt slider");
                Rect slider=bounds(tilt[0].control);float[] target={0};
                onMain(()->target[0]=slider.left+tilt[0].control.getPaddingLeft()+
                    .75f*(tilt[0].control.getWidth()-tilt[0].control.getPaddingLeft()-tilt[0].control.getPaddingRight()));
                touch(target[0],slider.exactCenterY());
                waitFor(()->Math.abs(tilt[0].model.optDouble("value")-90)<.01,"exact edge-on native tilt");
                onMain(()->require(activity.scene.nodes.get(editorId)==editor &&
                    "Pax orthographic".contentEquals(((EditText)editor.control).getText()),"edge-on pose retains editor"));
                capture("edge-on");
                click(button("Reset pose"));click(button("Hide amber"));
                waitFor(()->hasButton("Show amber"),"reset hides amber");
                click(button("Aperture off"));waitFor(()->hasButton("Aperture on"),"restored clip pose");
            }
            click(button("Aperture on"));
            waitFor(()->hasButton("Aperture off"),"native clipping restores");
            click(button("Press / 1"));
            waitFor(()->hasButton("Press / 2"),"restored native input");
            onMain(()->require(activity.scene.nodes.get(editorId)==editor &&
                "Pax orthographic".contentEquals(((EditText)editor.control).getText()),"clipping retains editor identity and text"));
            capture("recovered");
            result.putBoolean("passed",true);result.putBoolean("edgeOnQualified",edgeOn);result.putStringArrayList("checks",checks);
            result.putString("evidence",output.getAbsolutePath());finish(-1,result);
        } catch(Throwable error) {
            if(output!=null)try {capture("failure");}catch(Throwable ignored) {}
            if(activity!=null)try {onMain(()->{
                for(PaxScene.Node node:activity.scene.nodes.values())if(node.control instanceof EditText)
                    result.putString("editor_state","focused="+node.control.hasFocus()+" text="+((EditText)node.control).getText());
            });}catch(Throwable ignored) {}
            result.putString("error",error.toString());result.putStringArrayList("checks",checks);finish(1,result);
        }
    }
    void onMain(Runnable action) {
        Throwable[] error={null};
        super.runOnMainSync(()->{try {action.run();}catch(Throwable caught){error[0]=caught;}});
        if(error[0]!=null)throw new AssertionError(error[0]);
    }
    void waitFor(Condition condition,String label) {
        long deadline=SystemClock.uptimeMillis()+45000;boolean[] ready={false};
        while(!ready[0] && SystemClock.uptimeMillis()<deadline) {
            onMain(()->ready[0]=condition.test());if(!ready[0])SystemClock.sleep(50);
        }
        require(ready[0],label);
    }
    boolean hasButton(String label) {
        for(PaxScene.Node n:activity.scene.nodes.values()) if(n.kind.equals("Button") &&
            label.contentEquals(((TextView)n.control).getText()))return true;
        return false;
    }
    PaxScene.Node button(String label) {
        PaxScene.Node[] found={null};
        waitFor(()->{for(PaxScene.Node n:activity.scene.nodes.values()) if(n.kind.equals("Button") &&
            label.contentEquals(((TextView)n.control).getText())){found[0]=n;return true;}return false;},"button: "+label);
        return found[0];
    }
    PaxScene.Node findEditor() {
        PaxScene.Node[] found={null};onMain(()->{
            for(PaxScene.Node n:activity.scene.nodes.values())if(n.control instanceof EditText) {
                if(found[0]!=null)throw new AssertionError("Multiple native editors");found[0]=n;
            }
        });require(found[0]!=null,"native editor exists");return found[0];
    }
    Rect boundsOnMain(View view) {Rect r=new Rect();view.getGlobalVisibleRect(r);return r;}
    Rect bounds(View view) {Rect[] r={null};onMain(()->r[0]=boundsOnMain(view));return r[0];}
    void click(PaxScene.Node node) {
        Rect r=bounds(node.control);require(!r.isEmpty(),"native target is visible");
        touch(r.exactCenterX(),r.exactCenterY());
    }
    void touch(float x,float y) {
        long down=SystemClock.uptimeMillis();
        for(int action:new int[]{MotionEvent.ACTION_DOWN,MotionEvent.ACTION_UP}) {
            MotionEvent event=MotionEvent.obtain(down,SystemClock.uptimeMillis(),action,x,y,0);
            event.setSource(InputDevice.SOURCE_TOUCHSCREEN);
            try {require(getUiAutomation().injectInputEvent(event,true),"Android accepts native touch");}
            finally {event.recycle();}
            SystemClock.sleep(50);
        }
    }
    void key(int code,int meta) {
        long now=SystemClock.uptimeMillis();
        for(int action:new int[]{KeyEvent.ACTION_DOWN,KeyEvent.ACTION_UP})require(getUiAutomation().injectInputEvent(
            new KeyEvent(now,SystemClock.uptimeMillis(),action,code,0,meta,KeyCharacterMap.VIRTUAL_KEYBOARD,0,0,InputDevice.SOURCE_KEYBOARD),true),"Android accepts native key");
    }
    void type(String text) {
        KeyEvent[] events=KeyCharacterMap.load(KeyCharacterMap.VIRTUAL_KEYBOARD).getEvents(text.toCharArray());
        require(events!=null,"keyboard maps fixture text");
        for(KeyEvent original:events) {
            long now=SystemClock.uptimeMillis();require(getUiAutomation().injectInputEvent(new KeyEvent(now,now,
                original.getAction(),original.getKeyCode(),original.getRepeatCount(),original.getMetaState(),
                KeyCharacterMap.VIRTUAL_KEYBOARD,original.getScanCode(),original.getFlags(),InputDevice.SOURCE_KEYBOARD),true),"Android accepts typed character");
        }
    }
    void capture(String name) throws Exception {
        long[] tick={0};onMain(()->tick[0]=activity.scene.ticks);
        waitFor(()->activity.scene.ticks>=tick[0]+3,"composed frames settle");
        Bitmap image=getUiAutomation().takeScreenshot();require(image!=null,"composed screenshot");
        try(FileOutputStream stream=new FileOutputStream(new File(output,name+".png"))) {
            image.compress(Bitmap.CompressFormat.PNG,100,stream);
        } finally {image.recycle();}
        org.json.JSONArray nodes=new org.json.JSONArray();
        onMain(()->{
            for(PaxScene.Node node:activity.scene.nodes.values())try {
                nodes.put(new org.json.JSONObject().put("id",node.id).put("kind",node.kind)
                    .put("model",new org.json.JSONObject(node.model.toString()))
                    .put("bounds",boundsOnMain(node.control).toShortString()));
            }catch(org.json.JSONException error){throw new RuntimeException(error);}
        });
        try(FileOutputStream stream=new FileOutputStream(new File(output,name+".json"))) {
            stream.write(nodes.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8));
        }
    }
    void require(boolean value,String message) {
        if(!value)throw new AssertionError(message);checks.add(message);
    }
}

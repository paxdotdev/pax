package dev.pax;

import android.app.Instrumentation;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Color;
import android.os.Bundle;
import android.os.SystemClock;
import android.widget.Button;
import android.widget.ImageView;
import java.io.File;
import java.io.FileOutputStream;
import java.util.ArrayList;
import java.util.Comparator;

/** Exercise the native image primitive through real Pax patches and Android drawing. */
public final class NativeImageProbe extends Instrumentation {
    PaxActivity activity;
    int checks;
    interface Check { void run() throws Exception; }
    void ui(Check check) throws Exception {
        Throwable[] failure={null};
        runOnMainSync(()-> {try {check.run();}catch(Throwable error){failure[0]=error;}});
        if(failure[0]!=null) throw new AssertionError(failure[0]);
    }
    void await(Check check) throws Exception {
        long deadline=SystemClock.uptimeMillis()+30000;Throwable last=null;
        do {try {ui(check);return;}catch(Throwable error){last=error;SystemClock.sleep(50);}}while(SystemClock.uptimeMillis()<deadline);
        throw new AssertionError("Native image condition timed out",last);
    }
    void require(boolean condition,String message) {if(!condition)throw new AssertionError(message);}
    ArrayList<PaxScene.Node> images() {
        ArrayList<PaxScene.Node> result=new ArrayList<>();
        for(PaxScene.Node node:activity.scene.nodes.values())if(node.control instanceof ImageView)result.add(node);
        return result;
    }
    void loaded(int width,int height) {
        require(!activity.failed,"host failed");ArrayList<PaxScene.Node> images=images();
        require(images.size()==4,"four actual NativeImage nodes");
        PaxAssets.NativeImage shared=images.get(0).nativeImage;
        for(PaxScene.Node image:images) {
            require(image.nativeImage==shared,"same URL shares its decode");
            require(image.displayedImage!=null,"bitmap delivered to ImageView");
            require(image.displayedImage.getWidth()==width && image.displayedImage.getHeight()==height,"new source dimensions");
        }
        require(shared.references==4,"four leases");
        require(activity.session.assets.nativeImages.size()==1,"only current source retained");
        require(activity.session.assets.nativeImageBytes==shared.bitmap.getAllocationByteCount(),"decoded ownership accounting");
    }
    void click(String label) throws Exception {
        ui(()-> {
            for(PaxScene.Node node:activity.scene.nodes.values())if(node.control instanceof Button && ((Button)node.control).getText().toString().equals(label)) {
                require(node.control.performClick(),"button activation");return;
            }
            throw new AssertionError("Missing button "+label);
        });
    }
    void fitPixels() {
        ArrayList<PaxScene.Node> rows=images();rows.removeIf(node->node.model.optDouble("size_x")!=100);
        rows.sort(Comparator.comparingDouble(node->node.worldValues[2]));
        require(rows.size()==3,"three fit modes");
        int[] first={Color.RED,Color.GREEN,Color.RED};
        for(int i=0;i<rows.size();i++) {
            ImageView view=(ImageView)rows.get(i).control;
            Bitmap pixels=Bitmap.createBitmap(view.getWidth(),view.getHeight(),Bitmap.Config.ARGB_8888);
            view.draw(new Canvas(pixels));
            require(pixels.getPixel(pixels.getWidth()/8,pixels.getHeight()/2)==first[i],"fit horizontal crop "+i);
            int top=pixels.getPixel(pixels.getWidth()/8,pixels.getHeight()/10);
            require(i==0?Color.alpha(top)==0:top==first[i],"contain letterbox / cover / stretch "+i);
            pixels.recycle();
        }
    }
    void capture(String name) throws Exception {
        Bitmap shot=getUiAutomation().takeScreenshot();require(shot!=null,"screenshot");
        File directory=new File(activity.getExternalFilesDir(null),"native-images");directory.mkdirs();
        try(FileOutputStream out=new FileOutputStream(new File(directory,name+".png"))){shot.compress(Bitmap.CompressFormat.PNG,100,out);}
        shot.recycle();
    }
    void composedPixels() throws Exception {
        long deadline=SystemClock.uptimeMillis()+10000;
        do {
            int[][] samples=new int[3][2];
            ui(()-> {
                ArrayList<PaxScene.Node> rows=images();rows.removeIf(node->node.model.optDouble("size_x")!=100);
                rows.sort(Comparator.comparingDouble(node->node.worldValues[2]));
                require(rows.size()==3,"three composed images");
                for(int i=0;i<3;i++) {
                    ImageView view=(ImageView)rows.get(i).control;view.getLocationOnScreen(samples[i]);
                    samples[i][0]+=view.getWidth()/8;samples[i][1]+=view.getHeight()/2;
                }
            });
            Bitmap shot=getUiAutomation().takeScreenshot();
            if(shot!=null) {
                boolean matches=true;int[] expected={Color.RED,Color.GREEN,Color.RED};
                for(int i=0;i<3;i++) matches&=shot.getPixel(samples[i][0],samples[i][1])==expected[i];
                shot.recycle();if(matches)return;
            }
            SystemClock.sleep(50);
        }while(SystemClock.uptimeMillis()<deadline);
        throw new AssertionError("Native image pixels did not reach the composed screen");
    }
    @Override public void onCreate(Bundle args) {super.onCreate(args);start();}
    @Override public void onStart() {
        Bundle result=new Bundle();
        try {
            activity=(PaxActivity)startActivitySync(new Intent().setClassName(getTargetContext(),"dev.pax.PaxActivity")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK).putExtra("pax.startup","native-images"));
            await(()->loaded(40,20));checks++;
            ui(this::fitPixels);composedPixels();capture("fit-modes");checks++;
            click("Swap images");await(()->loaded(20,40));capture("replacement");checks++;
            PaxAssets.NativeImage[] leases=new PaxAssets.NativeImage[2];
            ui(()-> {
                PaxAssets assets=activity.session.assets;
                require(assets.decodedSlot.tryAcquire(),"decode slot is idle");
                try {
                    leases[0]=assets.acquireNativeImage("assets/native-image-wide.png");
                    assets.releaseNativeImage(leases[0]);
                    leases[1]=assets.acquireNativeImage("assets/native-image-wide.png");
                    require(leases[0]!=leases[1],"remount gets a new request identity");
                } finally {assets.decodedSlot.release();}
            });
            await(()-> {
                require(leases[0].cancelled && leases[0].bitmap==null,"cancelled request never receives pixels");
                require(leases[1].bitmap!=null,"replacement request completes");
            });
            ui(()->activity.session.assets.releaseNativeImage(leases[1]));checks++;
            click("Toggle images");await(()-> {
                require(images().isEmpty(),"native nodes deleted");
                require(activity.session.assets.nativeImages.isEmpty(),"last lease releases source");
                require(activity.session.assets.nativeImageBytes==0,"no retained native bitmap bytes");
            });checks++;
            click("Toggle images");await(()->loaded(20,40));checks++;
            click("Swap images");await(()->loaded(40,20));ui(this::fitPixels);composedPixels();checks++;
            result.putString("passed","true");result.putInt("checks",checks);
        } catch(Throwable error) {result.putString("passed","false");result.putString("error",android.util.Log.getStackTraceString(error));}
        finish("true".equals(result.getString("passed"))?-1:0,result);
    }
}

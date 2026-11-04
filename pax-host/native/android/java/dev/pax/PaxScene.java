package dev.pax;

import android.content.Context;
import android.graphics.*;
import android.graphics.drawable.GradientDrawable;
import android.text.*;
import android.text.method.ArrowKeyMovementMethod;
import android.text.method.LinkMovementMethod;
import android.text.method.TextKeyListener;
import android.util.TypedValue;
import android.view.*;
import android.widget.*;
import org.json.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import static dev.pax.PaxActivity.json;

/** Native Views project the runtime's model; only vector layers use TextureView. */
final class PaxScene extends FrameLayout {
    final PaxActivity activity;
    final PaxActivity.Session session;
    final float density;
    final HashMap<Integer,Node> nodes=new HashMap<>();
    final HashMap<Integer,TextMeasure> textMeasures=new HashMap<>();
    final TextView measurementView,linkStyleView;
    final LinkedHashMap<String,Tile> tiles=new LinkedHashMap<>();
    final ArrayList<Tile> replacedTiles=new ArrayList<>();
    final HashMap<Integer,PointF> touches=new HashMap<>();
    PointF tapStart;
    boolean disposed;
    boolean hierarchyDirty;
    boolean transformsDirty=true;
    final Matrix relativeScratch=new Matrix(),inverseScratch=new Matrix();
    boolean captureInFlight;
    long ticks, committed;
    long planGeneration=-1;
    final ArrayList<JSONObject> plans=new ArrayList<>();
    HashSet<String> presentationNeeded;
    boolean presentationDrawPending;
    final HashSet<Long> awaitingBuffers=new HashSet<>();
    final ArrayList<Tile> presentationTiles=new ArrayList<>();
    int width=-1,height=-1,fontRevision=-1,nativeImageRevision=-1,imeHeight;
    PaxScene(PaxActivity activity,PaxActivity.Session session) throws JSONException {
        super(activity); this.activity=activity; this.session=session;
        measurementView=new TextView(activity);measurementView.setIncludeFontPadding(false);
        linkStyleView=new TextView(activity);
        measurementView.setLayoutParams(new FrameLayout.LayoutParams(-2,-2));
        density=getResources().getDisplayMetrics().density;
        setClipChildren(false); setClipToPadding(false);
        setOnApplyWindowInsetsListener((view,insets)-> {
            int top,right,bottom,left;
            if(android.os.Build.VERSION.SDK_INT>=30) {
                Insets bars=insets.getInsets(WindowInsets.Type.systemBars()|WindowInsets.Type.displayCutout());
                imeHeight=insets.getInsets(WindowInsets.Type.ime()).bottom;
                top=bars.top;right=bars.right;bottom=imeHeight>0?0:bars.bottom;left=bars.left;
            } else {
                // API 29's decor already fits system bars and resizes for IME.
                top=right=bottom=left=0;
            }
            session.event("SafeAreaInsets",json("top",top/density,"right",right/density,"bottom",bottom/density,"left",left/density));
            return insets;
        });
        for(Integer id:session.models.keySet()) {
            if("Text".equals(session.kinds.get(id))) {
                TextMeasure measure=new TextMeasure(session.models.get(id));
                textMeasures.put(id,measure);measure.settle();
            }
            Node node=new Node(id,session.kinds.get(id)); nodes.put(id,node); node.apply(session.models.get(id));
        }
        reconcile();
    }
    void frame() throws Exception {
        if(nativeImageRevision!=session.assets.nativeImageRevision) {
            nativeImageRevision=session.assets.nativeImageRevision;
            for(Node node:nodes.values()) node.refreshNativeImage();
        }
        // Keep one publication in flight so each buffer-available event belongs
        // to this exact native/geometry transaction, including offscreen tiles.
        if(presentationNeeded!=null || presentationDrawPending) {session.call(4,null,0,0,0);return;}
        if(fontRevision!=session.assets.fontRevision) {
            fontRevision=session.assets.fontRevision;
            for(Node node:nodes.values()) if(node.control instanceof TextView || node.control instanceof Spinner || node.control instanceof RadioGroup) node.apply(node.model);
        }
        if(width!=getWidth() || height!=getHeight()-imeHeight) {
            width=getWidth(); height=Math.max(1,getHeight()-imeHeight);
        }
        JSONObject tick=session.call(0,null,width/density,height/density,density);
        JSONArray messages=tick.getJSONArray("messages");
        for(int i=0;i<messages.length();i++) {
            JSONObject message=messages.getJSONObject(i);
            if(message.has("ImageLoad")) {
                JSONObject request=message.getJSONObject("ImageLoad");
                session.pendingImages.put(request.getString("path"),request);
            } else session.pendingMessages.add(message);
        }
        // A font needed by new text may be queued behind an image worker that
        // owns the one decoded slot. Keep draining uploads while text settles,
        // or waiting for that font would deadlock both queues.
        if(session.activated) uploadReadyImage();
        // Measurement is a layout prerequisite, not a visible native patch.
        // Detached TextViews settle autosized geometry before GPU replay while
        // the currently presented controls retain their committed appearance.
        if(measureTexts(messages)) {session.call(4,null,0,0,0);return;}
        session.layers=tick.getInt("layerCount");
        long generation=tick.getLong("planGeneration");
        if(generation!=planGeneration || plans.size()!=session.layers) {
            plans.clear();
            for(int layer=0;layer<session.layers;layer++) plans.add(session.call(1,null,layer,0,density));
            planGeneration=generation;
        }
        HashSet<String> needed=new HashSet<>();
        for(int layer=0;layer<session.layers;layer++) {
            JSONObject planned=plans.get(layer);
            JSONObject plan=planned.optJSONObject("plan"); if(plan==null || !plan.optBoolean("active")) continue;
            int owner=planned.isNull("owner")?-1:planned.getInt("owner");
            JSONArray surfaces=plan.getJSONArray("surfaces");
            for(int i=0;i<surfaces.length();i++) {
                JSONObject info=surfaces.getJSONObject(i);
                String key=layer+":"+info.getString("key"); needed.add(key);
                Tile tile=tiles.get(key);
                int w=(int)Math.ceil(info.getDouble("width")*density), h=(int)Math.ceil(info.getDouble("height")*density);
                if(w<=0 || h<=0) continue;
                if(tile!=null && (tile.pixelsW!=w || tile.pixelsH!=h)) { replacedTiles.add(tile); tiles.remove(key); tile=null; }
                if(tile==null) {
                    tile=new Tile(layer,key,info,w,h); tiles.put(key,tile);
                    hierarchyDirty=true;
                    // Attach provisionally while pixels are prepared. The
                    // accepted transaction places it inside its native owner.
                    tile.setAlpha(0);attach(tile,this);
                }
                if(tile.owner!=owner || tile.info.optDouble("left")!=info.optDouble("left") ||
                    tile.info.optDouble("top")!=info.optDouble("top")) hierarchyDirty=true;
                tile.owner=owner; tile.info=info;
            }
        }

        JSONArray registry=new JSONArray();
        boolean ready=true;
        for(Tile tile:tiles.values()) {
            if(!needed.contains(tile.key)) continue;
            if(tile.generation==0) { ready=false; break; }
            registry.put(json("layer",tile.layer,"key",tile.info.getString("key"),"hostSignature",Long.toString(tile.generation),
                "originX",tile.info.getDouble("left"),"originY",tile.info.getDouble("top"),
                "logicalWidth",tile.info.getDouble("width"),"logicalHeight",tile.info.getDouble("height"),
                "width",tile.pixelsW,"height",tile.pixelsH,"dpr",new JSONArray(new float[]{density,density}),
                "replayPriority",tile.info.optInt("replayPriority")));
        }
        if(ready) {
            session.call(2,registry,0,0,0);
            for(JSONObject request:session.pendingImages.values()) session.assets.requestImage(request.getString("path"));
            session.pendingImages.clear();
            if(!session.activated) uploadReadyImage();
            JSONObject rendered=session.call(3,null,0,0,0);
            if(!rendered.optBoolean("blocked")) {
                presentationNeeded=needed;
                JSONObject publication=rendered.optJSONObject("frame");
                if(publication!=null) {
                    JSONArray generations=publication.getJSONArray("generations");
                    for(int i=0;i<generations.length();i++) awaitingBuffers.add(generations.getLong(i));
                    for(Tile tile:tiles.values()) if(awaitingBuffers.contains(tile.generation)) presentationTiles.add(tile);
                    if(presentationTiles.size()!=awaitingBuffers.size()) throw new IllegalStateException("Unknown presentation surface");
                }
                if(awaitingBuffers.isEmpty()) commitPresentation();
            }
        } else session.call(4,null,0,0,0);
        if(++ticks%120==0 && activity.getIntent().getBooleanExtra("pax.diagnostics",false)) {
            JSONArray scrolls=new JSONArray();
            for(Node node:nodes.values()) if(node.vertical!=null &&
                node.content.getHeight()>node.vertical.getHeight()+1 && scrolls.length()<16)
                scrolls.put(json("id",node.id,"y",node.vertical.getScrollY()/density,
                    "height",node.vertical.getHeight()/density,"contentHeight",node.content.getHeight()/density));
            android.util.Log.i("Pax","presentation "+json("ticks",ticks,"committed",committed,
                "pendingNativeBytes",session.pendingMessages.bytes(),"tiles",tiles.size(),"retired",session.retired.size(),
                    "scrolls",scrolls,"decodedImageBytes",session.assets.decodedBytes,
                    "uploadedImageBytes",session.assets.uploadedBytes,"assetFailures",session.assets.failed.size(),
                    "nativeImageBytes",session.assets.nativeImageBytes,"nativeImageSources",session.assets.nativeImages.size(),
                    "loadedFonts",session.assets.fonts.size(),
                    "paxFrameMs",new JSONArray(activity.paxFrames.summary()),
                    "windowFrameMs",new JSONArray(activity.windowFrames.summary()),
                    "operationMs",session.operationTimings(),
                    "operationMaxReplyBytes",session.operationReplySizes(),
                    "gpu",session.call(4,null,0,0,0).optJSONObject("gpu")));
        }
    }
    void bufferAvailable(Tile tile) {
        if(disposed || session.closing || session.handle==0) return;
        try {
            if(!awaitingBuffers.remove(tile.generation)) throw new IllegalStateException("Unexpected presentation buffer");
            if(awaitingBuffers.isEmpty()) commitPresentation();
        } catch(Exception error) {activity.fatal(error);}
    }
    void commitPresentation() throws Exception {
        HashSet<String> needed=presentationNeeded;
        presentationNeeded=null;
        committed++;
        hierarchyDirty |= tiles.size()!=needed.size();
        apply(session.pendingMessages.messages());session.pendingMessages.clear();
        for(String key:new ArrayList<>(tiles.keySet())) if(!needed.contains(key)) remove(tiles.remove(key));
        for(Tile old:replacedTiles) remove(old);
        replacedTiles.clear();
        if(hierarchyDirty) {reconcile();hierarchyDirty=false;}
        else updateTransforms();
        for(Tile tile:tiles.values()) tile.setAlpha(1);
        // Release TextureView's update notifications in the same UI turn as
        // placement/native patches. Early buffers cannot appear at old origins.
        for(Tile tile:presentationTiles) tile.texture.publish();
        if(!presentationTiles.isEmpty()) {
            // Do not replace these buffers before Android submits the View
            // frame containing their placement and native counterparts.
            presentationDrawPending=true;
            getViewTreeObserver().registerFrameCommitCallback(()->PaxActivity.UI.post(()->presentationDrawPending=false));
            invalidate();
        }
        presentationTiles.clear();
        if(!session.activated) {
            PaxActivity.nativeActivate(session.handle); session.activated=true;
            android.util.Log.i("Pax","first frame activated "+json("elapsedMs",android.os.SystemClock.elapsedRealtime()-session.startedAt,
                "origin","activity-onCreate",
                "gpu",session.call(4,null,0,0,0).optJSONObject("gpu")));
        }
    }
    void uploadReadyImage() throws JSONException {
        PaxAssets.Image image=session.assets.takeImage();
        if(image==null) return;
        try {
            JSONObject result=new JSONObject(new String(PaxActivity.nativeImage(session.handle,
                image.path.getBytes(StandardCharsets.UTF_8),image.rgba,image.width,image.height),StandardCharsets.UTF_8));
            if(!result.optBoolean("ok")) throw new IllegalStateException(result.optString("error"));
        } finally {session.assets.uploaded(image);}
    }
    final class TextMeasure {
        final JSONObject model;
        final RichText richText=new RichText();
        long generation=-1;
        int pixelsWidth=1,pixelsHeight=1;
        TextMeasure(JSONObject initial) throws JSONException {
            model=new JSONObject(initial.toString());
        }
        boolean settle() throws JSONException {
            if(!model.has("measure_generation") || generation==model.getLong("measure_generation")) return false;
            TextView view=measurementView;
            if(!style(view,model.optJSONObject("style"))) return true;
            view.setText(richText.get(model,PaxScene.this));
            if(!richText.ready) return true;
            wrap(view,model.optBoolean("wrap",true));
            double w=model.optDouble("size_x",-1);
            int spec=w<0?MeasureSpec.makeMeasureSpec(0,MeasureSpec.UNSPECIFIED)
                :MeasureSpec.makeMeasureSpec(Math.max(0,(int)(w*density)),MeasureSpec.AT_MOST);
            view.measure(spec,MeasureSpec.makeMeasureSpec(0,MeasureSpec.UNSPECIFIED));
            pixelsWidth=view.getMeasuredWidth();pixelsHeight=view.getMeasuredHeight();
            generation=model.getLong("measure_generation");
            session.event("TextMeasurementResponse",json("id",model.getInt("id"),"generation",generation,
                "width",view.getMeasuredWidth()/density,"height",view.getMeasuredHeight()/density));
            return true; // The response enters the next runtime tick.
        }
    }
    boolean measureTexts(JSONArray messages) throws JSONException {
        for(int i=0;i<messages.length();i++) {
            JSONObject message=messages.getJSONObject(i);
            if(message.has("TextDelete")) {textMeasures.remove(message.getInt("TextDelete"));continue;}
            JSONObject patch=message.optJSONObject("TextCreate");
            if(patch!=null) {textMeasures.put(patch.getInt("id"),new TextMeasure(patch));continue;}
            patch=message.optJSONObject("TextUpdate");
            if(patch!=null) {
                TextMeasure text=textMeasures.get(patch.getInt("id"));
                if(text!=null) {
                    Iterator<String> keys=patch.keys();
                    while(keys.hasNext()) {String key=keys.next();if(!patch.isNull(key)) text.model.put(key,patch.get(key));}
                }
            }
        }
        boolean settling=false;
        for(TextMeasure text:textMeasures.values()) settling |= text.settle();
        return settling;
    }
    void apply(JSONArray messages) throws Exception {
        LinkedHashSet<Integer> changed=new LinkedHashSet<>();
        for(int i=0;i<messages.length();i++) {
            JSONObject message=messages.getJSONObject(i); String kind=message.keys().next(); Object value=message.get(kind);
            if(kind.equals("ImageLoad")) { JSONObject request=(JSONObject)value;session.pendingImages.put(request.optString("path"),request);continue; }
            // Culling is advisory; retaining views preserves selection and accessibility.
            if(kind.equals("ShrinkLayersTo") || kind.equals("SetCursor") || kind.equals("NativeCullUpdate")) continue;
            if(kind.equals("Navigate")) {activity.navigate((JSONObject)value);continue;}
            if(kind.equals("Screenshot")) {capture((JSONObject)value);continue;}
            if(kind.endsWith("Delete")) {
                int id=((Number)value).intValue(); Node node=nodes.remove(id);
                if(node!=null) {
                    preserveTilesIn(node.placement);
                    node.releaseNativeImage();remove(node.placement);
                }
                session.models.remove(id);session.kinds.remove(id);changed.remove(id);hierarchyDirty=true;continue;
            }
            if(!(value instanceof JSONObject)) continue;
            JSONObject patch=(JSONObject)value; int id=patch.optInt("id",-1);
            if(kind.endsWith("Create")) {
                String type=kind.substring(0,kind.length()-6);
                if(nodes.containsKey(id)) throw new IllegalStateException("Duplicate native node "+id);
                JSONObject model=new JSONObject(patch.toString());session.models.put(id,model);session.kinds.put(id,type);
                Node node=new Node(id,type);nodes.put(id,node);changed.add(id);hierarchyDirty=true;
            } else if(kind.equals("NativeMaskUpdate")) {
                Node node=nodes.get(id); if(node!=null) { session.models.get(id).put("mask",patch);changed.add(id); }
            } else if(kind.endsWith("Update")) {
                Node node=nodes.get(id); if(node!=null) {
                    JSONObject model=session.models.get(id);
                    for(String key:new String[]{"parent_frame","z_index"})
                        if(patch.has(key) && !Objects.equals(patch.opt(key),model.opt(key))) hierarchyDirty=true;
                    Iterator<String> keys=patch.keys();
                    while(keys.hasNext()) { String key=keys.next(); if(!patch.isNull(key) || key.equals("parent_frame")) model.put(key,patch.get(key)); }
                    changed.add(id);
                }
            }
        }
        // Several layout passes can precede one accepted GPU frame. Only the
        // final model belongs to that frame; intermediate clips may be unmeasured.
        for(Integer id:changed) nodes.get(id).apply(session.models.get(id));
    }
    void reconcile() {
        transformsDirty=true;
        ArrayList<Node> order=new ArrayList<>(nodes.values());
        order.sort(Comparator.comparingInt(n -> n.model.optInt("z_index")));
        // Settle native ancestry before placing tiles: moving an ancestor also
        // detaches its TextureViews, even when their direct parent is unchanged.
        for(Node node:order) {
            int parent=node.model.isNull("parent_frame")?-1:node.model.optInt("parent_frame",-1);
            attach(node.placement,container(parent));
        }
        for(Tile tile:tiles.values()) {
            attach(tile,container(tile.owner));
            tile.setX((float)tile.info.optDouble("left")*density); tile.setY((float)tile.info.optDouble("top")*density);
            tile.bringToFront();
        }
        for(Node node:order) node.placement.bringToFront();
        updateTransforms();
    }
    void updateTransforms() {
        if(!transformsDirty) return;
        transformsDirty=false;
        for(Node node:nodes.values()) {
            int parent=node.model.isNull("parent_frame")?-1:node.model.optInt("parent_frame",-1);
            Node p=nodes.get(parent);
            long parentRevision=p==null?0:p.worldRevision;
            if(node.lastParent==p && node.lastParentRevision==parentRevision && node.lastWorldRevision==node.worldRevision &&
                node.lastWidth==node.getLayoutParams().width && node.lastHeight==node.getLayoutParams().height) continue;
            relativeScratch.set(node.world);
            if(p!=null && p.world.invert(inverseScratch)) relativeScratch.setConcat(inverseScratch,node.world);
            node.transform(relativeScratch);
            node.lastParent=p;node.lastParentRevision=parentRevision;node.lastWorldRevision=node.worldRevision;
        }
    }
    FrameLayout container(int id) { Node parent=nodes.get(id);return parent==null?this:parent.content; }
    void preserveTilesIn(View subtree) {
        // Route replacement can delete a scroller while reusing its tile keys.
        // Evacuate those consumers before Android detaches the ancestor; the
        // accepted transaction reparents them before the next View draw.
        for(Tile tile:tiles.values()) {
            for(ViewParent parent=tile.getParent();parent!=null;parent=parent.getParent()) {
                if(parent==subtree) {attach(tile,this);break;}
            }
        }
    }
    void attach(View view,FrameLayout parent) {
        if(view.getParent()==parent) return;
        if(view.getParent()!=null && view instanceof Tile && ((Tile)view).getSurfaceTexture()!=null) {
            Tile tile=(Tile)view;SurfaceTexture texture=tile.getSurfaceTexture();
            // Reparenting changes the native clip/scroll owner, not the backing
            // generation. Transfer the retained consumer rather than destroying
            // a window that may still have a submitted Vulkan frame.
            tile.moving=true;
            try {remove(tile);tile.setSurfaceTexture(texture);parent.addView(tile);}
            finally {tile.moving=false;}
        } else {
            if(view.getParent()!=null && !(view instanceof Tile)) preserveTilesIn(view);
            remove(view);parent.addView(view);
        }
    }
    void remove(View view) { if(view.getParent() instanceof android.view.ViewGroup) ((android.view.ViewGroup)view.getParent()).removeView(view); }
    void capture(JSONObject request) {
        int id=request.optInt("id");double scale=request.optDouble("scale",1);
        int w=(int)Math.ceil(width/density*scale),h=(int)Math.ceil(height/density*scale);
        if(captureInFlight || !Double.isFinite(scale) || scale<=0 || w<=0 || h<=0 || w>8192 || h>8192 || (long)w*h>16*1024*1024) {
            PaxActivity.nativeScreenshot(session.handle,id,null,0,0);return;
        }
        captureInFlight=true;
        Bitmap bitmap=Bitmap.createBitmap(w,h,Bitmap.Config.ARGB_8888);
        int[] origin=new int[2];getLocationInWindow(origin);
        Rect source=new Rect(origin[0],origin[1],origin[0]+width,origin[1]+height);
        // Window PixelCopy includes both TextureViews and native controls.
        try { PixelCopy.request(activity.getWindow(),source,bitmap,result->{
            captureInFlight=false;
            try {
                if(disposed || session.closing || session.handle==0) return;
                java.nio.ByteBuffer rgba=null;
                if(result==PixelCopy.SUCCESS) {
                    rgba=java.nio.ByteBuffer.allocateDirect(w*h*4);int[] row=new int[w];
                    for(int y=0;y<h;y++) {
                        bitmap.getPixels(row,0,w,0,y,w,1);
                        for(int pixel:row) {rgba.put((byte)(pixel>>16));rgba.put((byte)(pixel>>8));rgba.put((byte)pixel);rgba.put((byte)(pixel>>>24));}
                    }
                    rgba.flip();
                }
                PaxActivity.nativeScreenshot(session.handle,id,rgba,rgba==null?0:w,rgba==null?0:h);
            } finally {bitmap.recycle();}
        },PaxActivity.UI); }
        catch(IllegalArgumentException error) {
            // The window can lose its backing surface during recreation.
            captureInFlight=false;bitmap.recycle();
            PaxActivity.nativeScreenshot(session.handle,id,null,0,0);
        }
    }
    void dispose() { disposed=true; for(Node node:nodes.values()) node.releaseNativeImage();for(Tile tile:tiles.values()) remove(tile); for(Tile tile:replacedTiles) remove(tile); removeAllViews(); }
    @Override public boolean dispatchTouchEvent(MotionEvent event) {
        if(!disposed && session.handle!=0 && !session.closing) {
            int action=event.getActionMasked();
            if(action==MotionEvent.ACTION_DOWN) {
                tapStart=new PointF(event.getX(),event.getY());
                session.event("Focus",json());
            } else if(action==MotionEvent.ACTION_POINTER_DOWN || action==MotionEvent.ACTION_CANCEL) tapStart=null;
            if(tapStart!=null && Math.hypot(event.getX()-tapStart.x,event.getY()-tapStart.y)>ViewConfiguration.get(getContext()).getScaledTouchSlop()) tapStart=null;
            String kind=action==MotionEvent.ACTION_DOWN || action==MotionEvent.ACTION_POINTER_DOWN?"TouchStart":action==MotionEvent.ACTION_UP || action==MotionEvent.ACTION_POINTER_UP?"TouchEnd":action==MotionEvent.ACTION_CANCEL?"TouchCancel":"TouchMove";
            JSONArray points=new JSONArray();
            for(int i=0;i<event.getPointerCount();i++) {
                if((action==MotionEvent.ACTION_UP || action==MotionEvent.ACTION_POINTER_UP) && i!=event.getActionIndex()) continue;
                int id=event.getPointerId(i);float x=event.getX(i)/density,y=event.getY(i)/density;
                PointF previous=touches.put(id,new PointF(x,y));
                points.put(json("identifier",id,"x",x,"y",y,"delta_x",previous==null?0:x-previous.x,"delta_y",previous==null?0:y-previous.y));
                if(action==MotionEvent.ACTION_UP || action==MotionEvent.ACTION_POINTER_UP || action==MotionEvent.ACTION_CANCEL) touches.remove(id);
            }
            session.event(kind,json("touches",points));
            if(action==MotionEvent.ACTION_UP) {
                if(tapStart!=null) session.event("Tap",json("x",event.getX()/density,"y",event.getY()/density));
                tapStart=null;
            }
        }
        super.dispatchTouchEvent(event);
        return true;
    }
    final class Tile extends TextureView implements TextureView.SurfaceTextureListener {
        final int layer,pixelsW,pixelsH; final String key;
        final PresentationTexture texture;
        long generation; int owner=-1; JSONObject info;boolean moving;
        Tile(int layer,String key,JSONObject info,int w,int h) {
            super(activity);this.layer=layer;this.key=key;this.info=info;pixelsW=w;pixelsH=h;
            setOpaque(false);setLayoutParams(new FrameLayout.LayoutParams(w,h));setSurfaceTextureListener(this);
            setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
            texture=new PresentationTexture(this);
            setSurfaceTexture(texture);
            // setSurfaceTexture deliberately does not call the available
            // listener. Register this detached consumer before the first plan.
            onSurfaceTextureAvailable(texture,w,h);
        }
        public void onSurfaceTextureAvailable(SurfaceTexture texture,int w,int h) {
            if(disposed || session.closing || session.handle==0) return;
            texture.setDefaultBufferSize(pixelsW,pixelsH);generation=++session.generation;
            Surface surface=new Surface(texture);
            try { PaxActivity.nativeSurface(session.handle,generation,surface,pixelsW,pixelsH); }
            catch(RuntimeException error) {
                android.util.Log.e("Pax","Surface attach failed: generation="+generation+" size="+pixelsW+"x"+pixelsH+" tiles="+tiles.size(),error);
                PaxActivity.UI.post(()->activity.fatal(error));
            }
            finally { surface.release(); }
        }
        public boolean onSurfaceTextureDestroyed(SurfaceTexture texture) {
            if(moving) return false;
            if(generation!=0) session.retire(generation,texture); else texture.release();
            generation=0;return false;
        }
        public void onSurfaceTextureSizeChanged(SurfaceTexture texture,int w,int h) {}
        public void onSurfaceTextureUpdated(SurfaceTexture texture) {}
    }
    final class PresentationTexture extends SurfaceTexture {
        OnFrameAvailableListener consumer;
        boolean released;
        PresentationTexture(Tile tile) {
            super(false);
            // TextureView replaces its listener on attachment/visibility changes.
            // Keep receipt independent of drawing: a warm offscreen tile still
            // participates in the publication transaction.
            super.setOnFrameAvailableListener(texture -> {
                if(!released) bufferAvailable(tile);
            },PaxActivity.UI);
        }
        @Override public void setOnFrameAvailableListener(OnFrameAvailableListener listener,android.os.Handler handler) {
            consumer=listener;
        }
        @Override public void setOnFrameAvailableListener(OnFrameAvailableListener listener) {
            consumer=listener;
        }
        void publish() {if(consumer!=null) consumer.onFrameAvailable(this);}
        @Override public void release() {released=true;consumer=null;super.release();}
    }
    final class Node extends FrameLayout {
        final int id;final String kind;final FrameLayout content;
        final FrameLayout placement;
        final View control;
        final Matrix world=new Matrix();
        float[] worldValues;
        long worldRevision,lastWorldRevision=-1,lastParentRevision=-1;
        Node lastParent;
        float[] lastTransform;
        int lastWidth=-1,lastHeight=-1;
        JSONObject model=new JSONObject();
        boolean updating;
        boolean mountFocusHandled;
        Boolean editableText;
        CharSequence renderedText;
        String textStyleKey;
        int textFontRevision=-1;
        PaxAssets.NativeImage nativeImage;
        Bitmap displayedImage;
        float gestureX,gestureY;
        boolean gestureAxisChosen,gestureOwnsAxis;
        final ArrayList<String> options=new ArrayList<>();
        ArrayAdapter<String> optionAdapter;
        ScrollView vertical;HorizontalScrollView horizontal;
        ArrayList<Mask> masks=new ArrayList<>();Path clip;
        Node(int id,String kind) {
            super(activity);this.id=id;this.kind=kind;
            placement=new FrameLayout(activity);
            placement.setClipChildren(false);placement.setClipToPadding(false);
            placement.setPivotX(0);placement.setPivotY(0);setPivotX(0);setPivotY(0);
            placement.addView(this);
            setClipChildren(false);setClipToPadding(false);
            content=new FrameLayout(activity);content.setClipChildren(false);content.setClipToPadding(false);
            switch(kind) {
                case "Text": control=new TextView(activity) {
                    // TalkBack derives the editor role from the native class,
                    // independently of the buffer's editable state.
                    @Override public CharSequence getAccessibilityClassName() {
                        return Boolean.TRUE.equals(editableText)?EditText.class.getName():TextView.class.getName();
                    }
                };break;
                case "Textbox": control=new EditText(activity);break;
                case "Button":
                    control=new Button(activity);((Button)control).setAllCaps(false);
                    // Android's default vertical padding assumes its larger
                    // platform button height and clips labels in authored compact boxes.
                    control.setPadding(0,0,0,0);break;
                case "Checkbox": control=new CheckBox(activity);break;
                case "Slider": control=new SeekBar(activity);break;
                case "Dropdown": control=new Spinner(activity);break;
                case "RadioList": control=new RadioGroup(activity);break;
                case "NativeImage":
                    ImageView image=new ImageView(activity);image.setCropToPadding(true);
                    control=image;break;
                case "Frame": case "EventBlocker": control=content;break;
                case "Scroller":
                    vertical=new ScrollView(activity);horizontal=new HorizontalScrollView(activity);
                    vertical.setFillViewport(true);horizontal.setFillViewport(true);
                    horizontal.addView(vertical,new FrameLayout.LayoutParams(-1,-1));vertical.addView(content);
                    horizontal.setOnScrollChangeListener((v,x,y,ox,oy)->scrolled());
                    vertical.setOnScrollChangeListener((v,x,y,ox,oy)->scrolled());
                    control=horizontal;break;
                default: throw new IllegalStateException("Android control is not implemented: "+kind);
            }
            addView(control,new FrameLayout.LayoutParams(-1,-1));
            if(control instanceof TextView) ((TextView)control).setIncludeFontPadding(false);
            if(kind.equals("Text")) ((TextView)control).addTextChangedListener(new TextWatcher() {
                public void beforeTextChanged(CharSequence text,int start,int count,int after) {}
                public void onTextChanged(CharSequence text,int start,int before,int count) {
                    if(!updating && Boolean.TRUE.equals(editableText)) {
                        observe(json("content",text.toString()));
                        session.event("TextInput",json("id",id,"text",text.toString()));
                    }
                }
                public void afterTextChanged(Editable text) {}
            });
            if(control instanceof Button && !(control instanceof CheckBox)) control.setOnClickListener(v -> session.event("FormButtonClick",json("id",id)));
            if(control instanceof CheckBox) ((CheckBox)control).setOnCheckedChangeListener((button,value)->{
                if(!updating) {observe(json("checked",value));session.event("FormCheckboxToggle",json("id",id,"state",value));}
            });
            if(control instanceof EditText) {
                ((EditText)control).addTextChangedListener(new TextWatcher() {
                    public void beforeTextChanged(CharSequence s,int start,int count,int after) {}
                    public void onTextChanged(CharSequence s,int start,int before,int count) {
                        if(!updating) {observe(json("text",s.toString()));session.event("FormTextboxInput",json("id",id,"text",s.toString()));}
                    }
                    public void afterTextChanged(Editable text) {}
                });
                control.setOnFocusChangeListener((v,focus)->{if(!focus && !updating) session.event("FormTextboxChange",json("id",id,"text",((EditText)control).getText().toString()));});
                ((EditText)control).setOnEditorActionListener((v,action,event)->{session.event("FormTextboxChange",json("id",id,"text",v.getText().toString()));return false;});
            }
            if(control instanceof SeekBar) ((SeekBar)control).setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener() {
                public void onStartTrackingTouch(SeekBar bar) {} public void onStopTrackingTouch(SeekBar bar) {}
                public void onProgressChanged(SeekBar bar,int progress,boolean user) {
                    if(user&&!updating) {
                        double value=model.optDouble("min",0)+progress*model.optDouble("step",1);
                        observe(json("value",value));session.event("FormSliderChange",json("id",id,"value",value));
                    }
                }
            });
            if(control instanceof Spinner) {
                optionAdapter=new ArrayAdapter<String>(activity,android.R.layout.simple_spinner_item,options) {
                    @Override public View getView(int position,View recycled,ViewGroup parent) {
                        return styled(super.getView(position,recycled,parent));
                    }
                    @Override public View getDropDownView(int position,View recycled,ViewGroup parent) {
                        return styled(super.getDropDownView(position,recycled,parent));
                    }
                    View styled(View view) {
                        try {style((TextView)view,model.optJSONObject("style"));}
                        catch(JSONException error) {throw new IllegalArgumentException(error);}
                        return view;
                    }
                };
                optionAdapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
                ((Spinner)control).setAdapter(optionAdapter);
                ((Spinner)control).setOnItemSelectedListener(new AdapterView.OnItemSelectedListener() {
                    public void onNothingSelected(AdapterView<?> parent) {}
                    public void onItemSelected(AdapterView<?> parent,View view,int selected,long rowId) {
                        // Spinner can dispatch programmatic selection after layout.
                        if(!updating && selected==parent.getSelectedItemPosition() && selected!=selection()) {
                            observe(json("selected_id",selected));
                            session.event("FormDropdownChange",json("id",id,"selected_id",selected));
                        }
                    }
                });
            }
            if(control instanceof RadioGroup) ((RadioGroup)control).setOnCheckedChangeListener((group,checked)-> {
                if(!updating && checked!=-1) {
                    int selected=group.indexOfChild(group.findViewById(checked));
                    if(selected>=0) {
                        observe(json("selected_id",selected));
                        session.event("FormRadioListChange",json("id",id,"selected_id",selected));
                    }
                }
            });
        }
        int selection() {return options.isEmpty()?-1:Math.max(0,Math.min(options.size()-1,model.optInt("selected_id",0)));}
        void releaseNativeImage() {
            if(!(control instanceof ImageView)) return;
            ((ImageView)control).setImageDrawable(null);displayedImage=null;
            session.assets.releaseNativeImage(nativeImage);nativeImage=null;
        }
        void refreshNativeImage() {
            if(!(control instanceof ImageView)) return;
            Bitmap bitmap=nativeImage==null?null:nativeImage.bitmap;
            if(bitmap!=displayedImage) {((ImageView)control).setImageBitmap(bitmap);displayedImage=bitmap;}
        }
        void choices(JSONObject patch) throws JSONException {
            JSONArray values=patch.optJSONArray("options");ArrayList<String> next=new ArrayList<>();
            if(values!=null) for(int i=0;i<values.length();i++) next.add(values.getString(i));
            if(!next.equals(options)) {
                options.clear();options.addAll(next);
                if(control instanceof RadioGroup) {
                    RadioGroup group=(RadioGroup)control;group.removeAllViews();
                    for(String option:options) {
                        RadioButton button=new RadioButton(activity);button.setId(View.generateViewId());button.setText(option);
                        group.addView(button,new RadioGroup.LayoutParams(-1,0,1));
                    }
                }
            }
            if(control instanceof Spinner) {
                optionAdapter.notifyDataSetChanged();((Spinner)control).setSelection(selection());
            } else {
                RadioGroup group=(RadioGroup)control;
                for(int i=0;i<group.getChildCount();i++) {
                    RadioButton button=(RadioButton)group.getChildAt(i);style(button,patch.optJSONObject("style"));
                    int checked=patch.optJSONObject("background_checked")==null?Color.BLUE:color(patch.getJSONObject("background_checked"));
                    int unchecked=patch.optJSONObject("outline_color")==null?Color.GRAY:color(patch.getJSONObject("outline_color"));
                    button.setButtonTintList(new android.content.res.ColorStateList(new int[][]{{android.R.attr.state_checked},{}},new int[]{checked,unchecked}));
                }
                group.check(selection()<0?-1:group.getChildAt(selection()).getId());
            }
        }
        void observe(JSONObject fields) {
            // Native edits happen between display transactions. Their values
            // supersede older, uncommitted patches for the same fields. Keep
            // geometry/style patches, and accept subsequent runtime updates
            // normally, including application transformations of this input.
            try {
                Iterator<String> keys=fields.keys();
                while(keys.hasNext()) {String key=keys.next();model.put(key,fields.get(key));}
                session.pendingMessages.observed(kind,id,fields);
            } catch(JSONException error) {throw new IllegalStateException(error);}
        }
        void scrolled() {
            if(updating) return;
            float x=horizontal.getScrollX()/density,y=vertical.getScrollY()/density;
            // Native scrolling is immediate; later style/mask patches must not
            // restore a stale position from the retained model.
            observe(json("scroll_x",x,"scroll_y",y,"presentation_scroll_x",x,"presentation_scroll_y",y));
            session.event("ScrollerPosition",json("id",id,"scroll_x",x,"scroll_y",y));
        }
        void apply(JSONObject patch) throws JSONException {
            updating=true;model=patch;
            try {
                float w=(float)patch.optDouble("size_x",1),h=(float)patch.optDouble("size_y",1);
                // The placement wrapper must know intrinsic dimensions before
                // computing transformed bounds. WRAP_CONTENT here would be
                // constrained by its initially empty wrapper and collapse text.
                TextMeasure measured=textMeasures.get(id);
                int pixelsW=w<0&&measured!=null?measured.pixelsWidth:(int)Math.ceil(w*density);
                int pixelsH=h<0&&measured!=null?measured.pixelsHeight:(int)Math.ceil(h*density);
                if(lastWidth!=Math.max(1,pixelsW) || lastHeight!=Math.max(1,pixelsH)) transformsDirty=true;
                size(this,Math.max(1,pixelsW),Math.max(1,pixelsH));
                if(placement.getLayoutParams()==null) size(placement,getLayoutParams().width,getLayoutParams().height);
                float[] nextWorld=matrix(patch.optJSONArray("transform"),density);
                if(!Arrays.equals(worldValues,nextWorld)) {worldValues=nextWorld;world.setValues(nextWorld);worldRevision++;transformsDirty=true;}
                setAlpha((float)patch.optDouble("opacity",1));
                boolean clipping=patch.optBoolean("clip_content",false);setClipChildren(clipping);
                clip=null;
                if(!patch.optString("clip_path","").isEmpty()) {
                    clip=path(patch.getString("clip_path")); Matrix inv=new Matrix();
                    if(world.invert(inv)) { Matrix scale=new Matrix();scale.setScale(density,density);inv.preConcat(scale);clip.transform(inv); }
                } else if(clipping) {clip=new Path();float radius=(float)patch.optDouble("corner_radius",0)*density;clip.addRoundRect(0,0,w*density,h*density,radius,radius,Path.Direction.CW);}
                masks.clear();JSONObject mask=patch.optJSONObject("mask");
                if(mask!=null) { JSONArray entries=mask.getJSONArray("entries");for(int i=0;i<entries.length();i++)masks.add(new Mask(entries.getJSONObject(i),density)); }
                if(control instanceof TextView) {
                    TextView text=(TextView)control;
                    boolean editingChanged=false;
                    CharSequence value=patch.optString(control instanceof EditText?"text":"content","");
                    if(kind.equals("Text") && measured!=null) {
                        boolean editable=patch.optBoolean("editable",false),selectable=patch.optBoolean("selectable",false);
                        if(editableText==null || editableText!=editable) {
                            editingChanged=true;
                            text.setTextIsSelectable(false);
                            text.setKeyListener(editable?TextKeyListener.getInstance():null);
                            text.setRawInputType(editable?InputType.TYPE_CLASS_TEXT|InputType.TYPE_TEXT_FLAG_MULTI_LINE:InputType.TYPE_NULL);
                            editableText=editable;
                        }
                        value=measured.richText.get(patch,PaxScene.this);
                        if(!editable) text.setTextIsSelectable(selectable);
                        text.setMovementMethod(editable || !measured.richText.hasLinks
                            ? (editable || selectable?ArrowKeyMovementMethod.getInstance():null)
                            :LinkMovementMethod.getInstance());
                        text.setFocusable(editable || selectable || measured.richText.hasLinks);
                        text.setFocusableInTouchMode(editable || selectable);
                        wrap(text,patch.optBoolean("wrap",true));
                    }
                    if(editingChanged || value!=renderedText && (value instanceof Spanned || !text.getText().toString().contentEquals(value))) {
                        int selection=text.getSelectionStart(),selectionEnd=text.getSelectionEnd();
                        // A key listener permits edits but does not declare the
                        // buffer editable to accessibility. Preserve that native
                        // contract before focus, without resetting active input
                        // when the runtime merely echoes the same text.
                        if(!editingChanged && text.getText() instanceof Editable) {
                            // setText replaces the Editable and restarts the IME
                            // connection. Application transformations must keep
                            // that buffer alive while more input is arriving.
                            Editable buffer=(Editable)text.getText();
                            buffer.replace(0,buffer.length(),value);
                        } else if(kind.equals("Text")) text.setText(value,Boolean.TRUE.equals(editableText)
                            ? TextView.BufferType.EDITABLE:TextView.BufferType.NORMAL);
                        else text.setText(value);
                        if(text.getText() instanceof Editable && selection>=0 && selectionEnd>=0) {
                            Editable buffer=(Editable)text.getText();
                            Selection.setSelection(buffer,Math.min(selection,buffer.length()),Math.min(selectionEnd,buffer.length()));
                        }
                    }
                    renderedText=value;
                    if(control instanceof EditText) {text.setHint(patch.optString("placeholder",""));text.setSingleLine(!patch.optBoolean("is_text_area",false));}
                    JSONObject textStyle=patch.optJSONObject("style");
                    String nextStyle=textStyle==null?"":textStyle.toString();
                    if(!nextStyle.equals(textStyleKey) || textFontRevision!=session.assets.fontRevision) {
                        style(text,textStyle);textStyleKey=nextStyle;textFontRevision=session.assets.fontRevision;
                    }
                    if(control instanceof EditText && patch.has("focus_on_mount") && !mountFocusHandled) {
                        mountFocusHandled=true;
                        if(patch.optBoolean("focus_on_mount")) control.post(()-> { if(!disposed) control.requestFocus(); });
                    }
                }
                if(control instanceof CheckBox) ((CheckBox)control).setChecked(patch.optBoolean("checked",false));
                if(control instanceof ImageView) {
                    String url=patch.optString("url","");
                    if(nativeImage==null || !nativeImage.path.equals(url)) {
                        releaseNativeImage();nativeImage=session.assets.acquireNativeImage(url);
                    }
                    String fit=patch.optString("fit","contain");
                    ((ImageView)control).setScaleType(fit.equals("cover")?ImageView.ScaleType.CENTER_CROP:
                        fit.equals("fill")?ImageView.ScaleType.FIT_XY:ImageView.ScaleType.FIT_CENTER);
                    refreshNativeImage();
                }
                if(control instanceof Spinner || control instanceof RadioGroup) choices(patch);
                if(control instanceof SeekBar) { double min=patch.optDouble("min",0),step=Math.max(0.000001,patch.optDouble("step",1));((SeekBar)control).setMax((int)Math.min(1000000,(patch.optDouble("max",100)-min)/step));((SeekBar)control).setProgress((int)((patch.optDouble("value",min)-min)/step));}
                if(vertical!=null) {
                    horizontal.setClipChildren(clipping);vertical.setClipChildren(clipping);
                    horizontal.setClipToPadding(clipping);vertical.setClipToPadding(clipping);
                    int innerW=Math.max(1,(int)Math.ceil(patch.optDouble("size_inner_pane_x",w)*density));
                    int innerH=Math.max(1,(int)Math.ceil(patch.optDouble("size_inner_pane_y",h)*density));
                    // ScrollView measures its child with an unspecified extent.
                    // Translated Pax children do not contribute their offsets
                    // to FrameLayout measurement, so preserve the authored pane.
                    content.setMinimumWidth(innerW);content.setMinimumHeight(innerH);
                    vertical.setMinimumWidth(innerW);
                    size(content,innerW,innerH);
                    horizontal.scrollTo((int)Math.round(patch.optDouble("presentation_scroll_x",patch.optDouble("scroll_x",0))*density),0);
                    vertical.scrollTo(0,(int)Math.round(patch.optDouble("presentation_scroll_y",patch.optDouble("scroll_y",0))*density));
                }
                JSONObject background=patch.optJSONObject("background");if(kind.equals("Button"))background=patch.optJSONObject("color");
                if(background!=null && !kind.equals("RadioList")) {
                    GradientDrawable drawable=new GradientDrawable();drawable.setColor(color(background));drawable.setCornerRadius((float)patch.optDouble("corner_radius",0)*density);
                    if(patch.optJSONObject("stroke_color")!=null)drawable.setStroke((int)(patch.optDouble("stroke_width",0)*density),color(patch.getJSONObject("stroke_color")));
                    control.setBackground(drawable);
                }
                invalidate();
            } finally {updating=false;}
        }
        void transform(Matrix affine) {
            float[] values=new float[9];affine.getValues(values);
            double w=getLayoutParams().width<0?getMeasuredWidth():getLayoutParams().width;
            double h=getLayoutParams().height<0?getMeasuredHeight():getLayoutParams().height;
            if(Arrays.equals(values,lastTransform) && lastWidth==(int)w && lastHeight==(int)h) return;
            lastTransform=values;lastWidth=(int)w;lastHeight=(int)h;
            double[] parts=Affine.decompose(values[0],values[3],values[1],values[4]);
            double inner=Math.toRadians(parts[3]),outer=Math.toRadians(parts[0]);
            double rx=w*Math.cos(inner),ry=w*Math.sin(inner),ux=-h*Math.sin(inner),uy=h*Math.cos(inner);
            double left=Math.min(0,rx)+Math.min(0,ux),top=Math.min(0,ry)+Math.min(0,uy);
            int boxW=Math.max(1,(int)Math.ceil(Math.abs(rx)+Math.abs(ux)));
            int boxH=Math.max(1,(int)Math.ceil(Math.abs(ry)+Math.abs(uy)));
            // Each intermediate View must enclose its transformed child. Android
            // clips hit testing and accessibility to ancestor layout bounds.
            if(placement.getLayoutParams().width!=boxW || placement.getLayoutParams().height!=boxH)
                placement.setLayoutParams(new FrameLayout.LayoutParams(boxW,boxH));
            setTranslationX((float)-left);setTranslationY((float)-top);
            placement.setTranslationX(values[2]+(float)(Math.cos(outer)*parts[1]*left-Math.sin(outer)*parts[2]*top));
            placement.setTranslationY(values[5]+(float)(Math.sin(outer)*parts[1]*left+Math.cos(outer)*parts[2]*top));
            placement.setRotation((float)parts[0]);placement.setScaleX((float)parts[1]);placement.setScaleY((float)parts[2]);
            setRotation((float)parts[3]);
        }
        @Override protected void onLayout(boolean changed,int left,int top,int right,int bottom) {
            boolean previous=updating;updating=true;
            try {
                super.onLayout(changed,left,top,right,bottom);
                if(vertical!=null) {
                    // A retained Activity model is applied before children have
                    // extents. Restore its position once native layout can clamp it.
                    horizontal.scrollTo((int)Math.round(model.optDouble("presentation_scroll_x",model.optDouble("scroll_x",0))*density),0);
                    vertical.scrollTo(0,(int)Math.round(model.optDouble("presentation_scroll_y",model.optDouble("scroll_y",0))*density));
                }
            } finally {updating=previous;}
            if(vertical!=null && !updating &&
                (Math.abs(vertical.getScrollY()-model.optDouble("presentation_scroll_y",0)*density)>=1 ||
                 Math.abs(horizontal.getScrollX()-model.optDouble("presentation_scroll_x",0)*density)>=1))
                scrolled();
        }
        @Override protected void dispatchDraw(Canvas canvas) {
            int save=canvas.save();if(clip!=null)canvas.clipPath(clip);
            if(!masks.isEmpty())canvas.saveLayer(0,0,getWidth(),getHeight(),null);
            super.dispatchDraw(canvas);
            for(Mask mask:masks) {int mark=canvas.save();for(Path p:mask.clips)canvas.clipPath(p);canvas.drawPath(mask.path,mask.paint);canvas.restoreToCount(mark);}
            canvas.restoreToCount(save);
        }
        @Override public boolean dispatchTouchEvent(MotionEvent event) {
            if(event.getActionMasked()==MotionEvent.ACTION_DOWN) {
                if(clip!=null && !contains(clip,event.getX(),event.getY(),getWidth(),getHeight()))return false;
                for(Mask mask:masks) if(mask.paint.getAlpha()==255 && contains(mask.path,event.getX(),event.getY(),getWidth(),getHeight())) {
                    boolean inside=true;for(Path p:mask.clips) inside &= contains(p,event.getX(),event.getY(),getWidth(),getHeight());if(inside)return false;
                }
            }
            if(vertical!=null) {
                int action=event.getActionMasked();
                if(action==MotionEvent.ACTION_DOWN) {
                    gestureX=event.getX();gestureY=event.getY();gestureAxisChosen=false;
                    getParent().requestDisallowInterceptTouchEvent(true);
                } else if(action==MotionEvent.ACTION_MOVE && !gestureAxisChosen) {
                    float dx=Math.abs(event.getX()-gestureX),dy=Math.abs(event.getY()-gestureY);
                    if(Math.max(dx,dy)>ViewConfiguration.get(activity).getScaledTouchSlop()) {
                        gestureAxisChosen=true;
                        // A horizontal film row must let its vertical parent
                        // handle a vertical drag that starts over a card.
                        gestureOwnsAxis=dx>dy
                            ? horizontal.canScrollHorizontally(-1)||horizontal.canScrollHorizontally(1)
                            : vertical.canScrollVertically(-1)||vertical.canScrollVertically(1);
                        getParent().requestDisallowInterceptTouchEvent(gestureOwnsAxis);
                    }
                } else if(action==MotionEvent.ACTION_UP || action==MotionEvent.ACTION_CANCEL) {
                    getParent().requestDisallowInterceptTouchEvent(false);
                }
            }
            boolean handled=super.dispatchTouchEvent(event);
            // ScrollView may disallow interception again inside its own move
            // handler even with zero scroll range. Restore the axis decision
            // after dispatch so an ancestor can intercept the next move and
            // Android sends the child its normal ACTION_CANCEL.
            if(vertical!=null && gestureAxisChosen && !gestureOwnsAxis && event.getActionMasked()==MotionEvent.ACTION_MOVE)
                getParent().requestDisallowInterceptTouchEvent(false);
            return handled;
        }
    }
    static void size(View view,int width,int height) {
        android.view.ViewGroup.LayoutParams current=view.getLayoutParams();
        if(current==null || current.width!=width || current.height!=height)
            view.setLayoutParams(new FrameLayout.LayoutParams(width,height));
    }
    static void wrap(TextView view,boolean wrap) {
        // setSingleLine(true) replaces authored newlines with spaces. No-wrap
        // means horizontal overflow while retaining paragraph/code line breaks.
        view.setSingleLine(false);
        view.setHorizontallyScrolling(!wrap);
    }
    boolean style(TextView text,JSONObject style) throws JSONException {
        if(style==null)return true;
        boolean ready=true;
        if(style.has("font_size"))text.setTextSize(TypedValue.COMPLEX_UNIT_PX,(float)style.getDouble("font_size")*density);
        if(style.optJSONObject("fill")!=null)text.setTextColor(color(style.getJSONObject("fill")));
        int h="Center".equals(style.optString("align_horizontal"))?Gravity.CENTER_HORIZONTAL:"Right".equals(style.optString("align_horizontal"))?Gravity.RIGHT:Gravity.LEFT;
        int v="Center".equals(style.optString("align_vertical"))?Gravity.CENTER_VERTICAL:"Bottom".equals(style.optString("align_vertical"))?Gravity.BOTTOM:Gravity.TOP;
        text.setGravity(h|v);
        JSONObject font=style.optJSONObject("font");
        if(font!=null && font.keys().hasNext()) {
            JSONObject f=font.getJSONObject(font.keys().next());String url=f.optString("url",f.optString("path",""));
            int weight=fontWeight(f.optString("weight"));boolean italic=!"Normal".equals(f.optString("style","Normal"));
            String family=f.optString("family","sans-serif");
            text.setTypeface(session.assets.font(url,family,weight,italic));
            ready=session.assets.fontReady(url,family,weight,italic);
        }
        text.setPaintFlags(style.optBoolean("underline",false)?text.getPaintFlags()|Paint.UNDERLINE_TEXT_FLAG:text.getPaintFlags()&~Paint.UNDERLINE_TEXT_FLAG);
        return ready;
    }
    static int fontWeight(String value) {
        switch(value) {
            case "Thin":return 100;case "ExtraLight":return 200;case "Light":return 300;
            case "Medium":return 500;case "SemiBold":return 600;case "Bold":return 700;
            case "ExtraBold":return 800;case "Black":return 900;default:return 400;
        }
    }
    static float[] matrix(JSONArray a,float density) {if(a==null)return new float[]{1,0,0,0,1,0,0,0,1};return new float[]{(float)a.optDouble(0,1),(float)a.optDouble(2,0),(float)a.optDouble(4,0)*density,(float)a.optDouble(1,0),(float)a.optDouble(3,1),(float)a.optDouble(5,0)*density,0,0,1};}
    static int color(JSONObject object) throws JSONException {JSONArray a=object.optJSONArray("Rgba");boolean alpha=a!=null;if(!alpha)a=object.getJSONArray("Rgb");return Color.argb(alpha?(int)Math.round(a.getDouble(3)*255):255,(int)Math.round(a.getDouble(0)*255),(int)Math.round(a.getDouble(1)*255),(int)Math.round(a.getDouble(2)*255));}
    static boolean contains(Path p,float x,float y,int w,int h) {Region r=new Region();r.setPath(p,new Region(0,0,w,h));return r.contains((int)x,(int)y);}
    static final class Mask {
        final Path path;final ArrayList<Path> clips=new ArrayList<>();final Paint paint=new Paint(Paint.ANTI_ALIAS_FLAG);
        Mask(JSONObject json,float density) throws JSONException {Matrix scale=new Matrix();scale.setScale(density,density);path=path(json.getString("path"));path.transform(scale);JSONArray list=json.getJSONArray("clips");for(int i=0;i<list.length();i++){Path p=path(list.getString(i));p.transform(scale);clips.add(p);}paint.setColor(Color.WHITE);paint.setAlpha((int)Math.round(json.optDouble("opacity",1)*255));paint.setXfermode(new PorterDuffXfermode(PorterDuff.Mode.DST_OUT));}
    }
    static Path path(String svg) {return NativePath.parse(svg);}
}

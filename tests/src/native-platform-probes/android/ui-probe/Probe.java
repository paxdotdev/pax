package dev.pax.android_probe;

import android.app.Instrumentation;
import android.app.UiAutomation;
import android.accessibilityservice.AccessibilityServiceInfo;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.SystemClock;
import android.util.Base64;
import android.util.Xml;
import android.text.Spanned;
import android.text.style.*;
import android.view.accessibility.AccessibilityNodeInfo;
import android.view.accessibility.AccessibilityWindowInfo;
import android.view.InputDevice;
import android.view.KeyCharacterMap;
import android.view.KeyEvent;
import java.io.StringWriter;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.zip.GZIPOutputStream;
import org.xmlpull.v1.XmlSerializer;

/** Snapshot the real accessibility tree without requiring animations to stop. */
public final class Probe extends Instrumentation {
    int count;
    Bundle arguments;
    UiAutomation automation;
    @Override public void onCreate(Bundle arguments) {super.onCreate(arguments);this.arguments=arguments;start();}
    @Override public void onStart() {
        Bundle result=new Bundle();
        try {
            // Ordinary UiAutomation suppresses services, which would turn a
            // TalkBack inspection into a test without the screen reader.
            boolean preserve=arguments!=null && "true".equals(arguments.getString("preserve_services"));
            boolean windows=arguments!=null && "true".equals(arguments.getString("all_windows"));
            automation=getUiAutomation(preserve?UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES:0);
            if(windows) {
                AccessibilityServiceInfo info=automation.getServiceInfo();
                info.flags|=AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS;
                automation.setServiceInfo(info);
            }
            long started=android.os.SystemClock.elapsedRealtime();
            AccessibilityNodeInfo root=null;
            for(int attempt=0;attempt<20 && root==null;attempt++) {
                root=automation.getRootInActiveWindow();
                if(root==null) Thread.sleep(100);
            }
            if(root==null) throw new IllegalStateException("No active accessibility root");
            long rooted=android.os.SystemClock.elapsedRealtime();
            if(arguments!=null && arguments.containsKey("type_text_base64")) {
                try {typeText(root,result);} finally {root.recycle();}
                result.putLong("input_ms",SystemClock.elapsedRealtime()-rooted);
                finish(-1,result);return;
            }
            StringWriter output=new StringWriter();XmlSerializer xml=Xml.newSerializer();
            xml.setOutput(output);xml.startDocument("UTF-8",true);xml.startTag(null,"hierarchy");
            try {
                if(windows) {
                    for(AccessibilityWindowInfo window:automation.getWindows()) {
                        try {
                            xml.startTag(null,"window");
                            attribute(xml,"type",Integer.toString(window.getType()));
                            attribute(xml,"title",window.getTitle());
                            attribute(xml,"active",Boolean.toString(window.isActive()));
                            AccessibilityNodeInfo windowRoot=window.getRoot();
                            if(windowRoot!=null) {try {write(xml,windowRoot,0);} finally {windowRoot.recycle();}}
                            xml.endTag(null,"window");
                        } finally {window.recycle();}
                    }
                } else write(xml,root,0);
            } finally {root.recycle();}
            xml.endTag(null,"hierarchy");xml.endDocument();
            byte[] bytes=output.toString().getBytes(StandardCharsets.UTF_8);
            if(bytes.length>8*1024*1024) throw new IllegalStateException("Hierarchy exceeds 8 MiB");
            ByteArrayOutputStream compressed=new ByteArrayOutputStream();
            try(GZIPOutputStream gzip=new GZIPOutputStream(compressed)) {gzip.write(bytes);}
            // Instrumentation results cross Binder as UTF-16 strings. Leave
            // ample room below its shared transaction buffer limit.
            if(compressed.size()>128*1024) throw new IllegalStateException("Compressed hierarchy exceeds 128 KiB");
            result.putLong("root_ms",rooted-started);
            result.putLong("tree_ms",android.os.SystemClock.elapsedRealtime()-rooted);
            result.putInt("node_count",count);
            result.putString("hierarchy_gzip",Base64.encodeToString(compressed.toByteArray(),Base64.NO_WRAP));
            finish(-1,result);
        } catch(Exception error) {
            result.clear();result.putString("error",error.toString());finish(1,result);
        }
    }
    void typeText(AccessibilityNodeInfo root,Bundle result) {
        if(!"dev.pax.native_android_smoke".contentEquals(root.getPackageName()))
            throw new IllegalStateException("Text injection is restricted to the controls fixture");
        AccessibilityNodeInfo focus=root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        try {
            if(focus==null || !focus.isEditable()) throw new IllegalStateException("No focused native editor");
        } finally {if(focus!=null) focus.recycle();}
        String text=new String(Base64.decode(arguments.getString("type_text_base64"),Base64.DEFAULT),StandardCharsets.UTF_8);
        if(text.isEmpty() || text.length()>256) throw new IllegalArgumentException("Input requires 1–256 ASCII characters");
        for(int i=0;i<text.length();i++) if(text.charAt(i)<32 || text.charAt(i)>126)
            throw new IllegalArgumentException("Use the InputConnection probe for non-ASCII input");
        KeyEvent[] events=KeyCharacterMap.load(KeyCharacterMap.VIRTUAL_KEYBOARD).getEvents(text.toCharArray());
        if(events==null) throw new IllegalArgumentException("Unmappable keyboard input");
        java.util.HashMap<Integer,Long> downTimes=new java.util.HashMap<>();
        for(int i=0;i<events.length;i++) {
            KeyEvent original=events[i];long now=SystemClock.uptimeMillis();
            if(original.getAction()==KeyEvent.ACTION_DOWN) downTimes.put(original.getKeyCode(),now);
            long down=downTimes.containsKey(original.getKeyCode())?downTimes.get(original.getKeyCode()):now;
            // The shell's `input text` timestamps the whole sequence up front.
            // Under load its later events can expire before injection. Keep
            // real keyboard timing and fail explicitly if Android rejects one.
            KeyEvent event=new KeyEvent(down,now,original.getAction(),original.getKeyCode(),original.getRepeatCount(),
                original.getMetaState(),KeyCharacterMap.VIRTUAL_KEYBOARD,original.getScanCode(),original.getFlags(),InputDevice.SOURCE_KEYBOARD);
            if(!automation.injectInputEvent(event,true))
                throw new IllegalStateException("Android rejected key event "+i+" of "+events.length);
            if(original.getAction()==KeyEvent.ACTION_UP) downTimes.remove(original.getKeyCode());
        }
        result.putInt("key_events",events.length);
    }
    void write(XmlSerializer xml,AccessibilityNodeInfo node,int depth) throws Exception {
        if(depth>0 && !node.isVisibleToUser()) return;
        if(++count>20000 || depth>256) throw new IllegalStateException("Hierarchy exceeds traversal budget");
        Rect bounds=new Rect();node.getBoundsInScreen(bounds);
        xml.startTag(null,"node");
        attribute(xml,"text",node.getText());attribute(xml,"class",node.getClassName());
        attribute(xml,"package",node.getPackageName());attribute(xml,"content-desc",node.getContentDescription());
        attribute(xml,"bounds","["+bounds.left+","+bounds.top+"]["+bounds.right+","+bounds.bottom+"]");
        attribute(xml,"visible",Boolean.toString(node.isVisibleToUser()));
        attribute(xml,"clickable",Boolean.toString(node.isClickable()));
        attribute(xml,"checked",Boolean.toString(node.isChecked()));
        attribute(xml,"focused",Boolean.toString(node.isFocused()));
        attribute(xml,"accessibility-focused",Boolean.toString(node.isAccessibilityFocused()));
        attribute(xml,"focusable",Boolean.toString(node.isFocusable()));
        attribute(xml,"screen-reader-focusable",Boolean.toString(node.isScreenReaderFocusable()));
        attribute(xml,"checkable",Boolean.toString(node.isCheckable()));
        attribute(xml,"enabled",Boolean.toString(node.isEnabled()));
        attribute(xml,"selected",Boolean.toString(node.isSelected()));
        attribute(xml,"resource-id",node.getViewIdResourceName());
        attribute(xml,"hint",node.getHintText());
        if(android.os.Build.VERSION.SDK_INT>=30) attribute(xml,"state-description",node.getStateDescription());
        StringBuilder actions=new StringBuilder();
        for(AccessibilityNodeInfo.AccessibilityAction action:node.getActionList()) actions.append(action.getId()).append(',');
        attribute(xml,"actions",actions);
        attribute(xml,"scrollable",Boolean.toString(node.isScrollable()));
        attribute(xml,"selection-start",Integer.toString(node.getTextSelectionStart()));
        attribute(xml,"selection-end",Integer.toString(node.getTextSelectionEnd()));
        attribute(xml,"editable",Boolean.toString(node.isEditable()));
        if(node.getText() instanceof Spanned) {
            Spanned text=(Spanned)node.getText();
            StringBuilder colors=new StringBuilder(),styles=new StringBuilder(),fonts=new StringBuilder();
            for(ForegroundColorSpan span:text.getSpans(0,text.length(),ForegroundColorSpan.class))
                colors.append(Integer.toHexString(span.getForegroundColor())).append(',');
            for(StyleSpan span:text.getSpans(0,text.length(),StyleSpan.class))
                styles.append(text.getSpanStart(span)).append(':').append(text.getSpanEnd(span)).append(':').append(span.getStyle()).append(',');
            for(TypefaceSpan span:text.getSpans(0,text.length(),TypefaceSpan.class)) fonts.append(span.getFamily()).append(',');
            attribute(xml,"span-colors",colors);attribute(xml,"span-styles",styles);attribute(xml,"span-fonts",fonts);
            attribute(xml,"span-links",Integer.toString(text.getSpans(0,text.length(),ClickableSpan.class).length));
        }
        for(int i=0;i<node.getChildCount();i++) {
            AccessibilityNodeInfo child=node.getChild(i);
            if(child!=null) {try {write(xml,child,depth+1);} finally {child.recycle();}}
        }
        xml.endTag(null,"node");
    }
    static void attribute(XmlSerializer xml,String name,CharSequence value) throws Exception {
        xml.attribute(null,name,value==null?"":value.toString());
    }
}

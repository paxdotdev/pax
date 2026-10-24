package dev.pax;

import android.app.Instrumentation;
import android.os.Bundle;
import android.graphics.Typeface;
import java.util.Locale;
import org.json.*;

/** Host regression tests against Android's actual JSON, graphics and font APIs. */
public final class HostTests extends Instrumentation {
    @Override public void onCreate(Bundle args) {super.onCreate(args);start();}
    @Override public void onStart() {
        Bundle result=new Bundle();
        try {
            require(NativeRoutes.resolve("/paxflix?page=1#hero","?page=2").equals("/paxflix?page=2"),"Query-only navigation changed the screen");
            require(NativeRoutes.resolve("/paxflix?page=1#hero","").equals("/paxflix?page=1"),"Empty navigation lost the current location");
            result.putBoolean("rawGeorgiaUsesDefault",Typeface.create("Georgia",Typeface.NORMAL).equals(Typeface.DEFAULT));
            if(android.os.Build.VERSION.SDK_INT>=31) {
                result.putString("rawGeorgiaFont",glyphFont(Typeface.create("Georgia",Typeface.BOLD)));
                result.putString("lowercaseGeorgiaFont",glyphFont(Typeface.create("georgia",Typeface.BOLD)));
                result.putString("serifFont",glyphFont(Typeface.create(Typeface.SERIF,Typeface.BOLD)));
                result.putString("resolvedGeorgiaFont",glyphFont(SystemFonts.resolve("Georgia",700,false)));
            }
            Locale previous=Locale.getDefault();
            try {
                // A device's language must not turn SERIF into the Turkish ı.
                Locale.setDefault(new Locale("tr","TR"));
                for(String family:new String[]{"SERIF","SANS-SERIF","MONOSPACE"}) {
                    Typeface expected=Typeface.create(Typeface.create(family.toLowerCase(Locale.ROOT),Typeface.NORMAL),700,true);
                    require(SystemFonts.resolve(family,700,true).equals(expected),"System alias lost family/weight/italic: "+family);
                }
                Typeface exact=Typeface.create("ITC Stone Serif",Typeface.NORMAL);
                require(SystemFonts.resolve("ITC Stone Serif",400,false).equals(Typeface.create(exact,400,false)),"Exact mixed-case platform family was replaced");
                require(SystemFonts.resolve("pax-missing-font-family",400,false).equals(Typeface.create(Typeface.DEFAULT,400,false)),"Unknown family lost system fallback");
            } finally {Locale.setDefault(previous);}
            android.graphics.RectF bounds=new android.graphics.RectF();
            NativePath.parse("M-1e1,+2.5\nL.5 4.").computeBounds(bounds,true);
            require(bounds.equals(new android.graphics.RectF(-10,2.5f,0.5f,4)),"Native path numeric syntax changed");
            NativePath.parse("M0 0 L10 0 10 20 Q8 20 6 18 C4 16 2 16 0 20 Z").computeBounds(bounds,true);
            require(bounds.equals(new android.graphics.RectF(0,0,10,20)),"Native path commands or implicit lines changed");
            for(String invalid:new String[]{"M0 0 H1","m0 0 l1 1","M0 0 L1","M0x0","M0 0 trailing"}) {
                boolean rejected=false;
                try {NativePath.parse(invalid);} catch(IllegalArgumentException expected) {rejected=true;}
                require(rejected,"Invalid native mask accepted: "+invalid);
            }
            PendingUpdates pending=new PendingUpdates();
            StringBuilder content=new StringBuilder();for(int i=0;i<20000;i++) content.append('x');
            pending.add(message("TextUpdate",new JSONObject().put("id",7).put("content",content).put("parent_frame",3)));
            // An unpresented frame can outlast many layout/animation ticks. Its
            // memory must follow live node state, not the count of those ticks.
            for(int i=0;i<1000;i++) pending.add(message("TextUpdate",new JSONObject().put("id",7).put("opacity",i/1000.0).put("content",JSONObject.NULL)));
            require(pending.messages().length()==1 && pending.bytes()<21000,"Pending native state grew with repeated ticks");
            JSONObject patch=pending.messages().getJSONObject(0).getJSONObject("TextUpdate");
            require(patch.getString("content").length()==20000 && patch.getDouble("opacity")==0.999,"Latest patch lost content/state");
            pending.add(message("TextUpdate",new JSONObject().put("id",7).put("parent_frame",JSONObject.NULL)));
            require(patch.isNull("parent_frame"),"Explicit root reparenting was lost");
            pending.observed("Text",7,new JSONObject().put("content","new user edit"));
            require(!patch.has("content") && pending.bytes()<150,"Old content survived newer native input");
            pending.add(message("TextUpdate",new JSONObject().put("id",7).put("content","APP TRANSFORM")));
            require(patch.getString("content").equals("APP TRANSFORM"),"Later application updates were suppressed");
            pending.add(message("TextDelete",7));
            pending.add(message("TextCreate",new JSONObject().put("id",7)));
            pending.add(message("TextUpdate",new JSONObject().put("id",7).put("content","replacement")));
            pending.add(message("Navigate",new JSONObject().put("url","/one")));
            pending.add(message("Navigate",new JSONObject().put("url","/two")));
            require(pending.messages().length()==6,"Lifecycle or navigation ordering was coalesced");
            require(patch.getString("content").equals("APP TRANSFORM"),"Replacement changed the previous node lifetime");
            pending.clear();require(pending.bytes()==0 && pending.messages().length()==0,"Committed state was retained");
            result.putString("checks","native route references; system font aliases/weight/italic/locale/exact names; native path commands/numbers/rejection; bounded pending state; null semantics; native input ownership; lifecycle order; release");
            finish(-1,result);
        } catch(Throwable error) {result.putString("error",error.toString());finish(1,result);}
    }
    static JSONObject message(String kind,Object value) throws JSONException {return new JSONObject().put(kind,value);}
    static String glyphFont(Typeface typeface) {
        android.graphics.Paint paint=new android.graphics.Paint();paint.setTypeface(typeface);paint.setTextSize(48);
        String text="THE LAST SIGNAL";
        android.graphics.text.PositionedGlyphs glyphs=android.graphics.text.TextRunShaper.shapeTextRun(text,0,text.length(),0,text.length(),0,0,false,paint);
        return String.valueOf(glyphs.getFont(0).getFile());
    }
    static void require(boolean condition,String failure) {if(!condition) throw new AssertionError(failure);}
}

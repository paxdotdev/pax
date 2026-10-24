package dev.pax;

import android.graphics.Color;
import android.graphics.Typeface;
import android.text.*;
import android.text.style.*;
import android.view.View;
import android.widget.TextView;
import org.json.*;
import static dev.pax.PaxActivity.json;

/** One node's immutable styled text, shared by measurement and presentation. */
final class RichText {
    String source, linkStyle;
    boolean markdown, ready=true;
    int fontRevision=-1;
    CharSequence value="";
    boolean hasLinks;

    CharSequence get(JSONObject model,PaxScene scene) throws JSONException {
        String next=model.optString("content","");
        // Editing owns the authored source. Replacing editable Markdown with
        // its rendered characters would discard the formatting on the first key.
        boolean rich=model.optBoolean("markdown",false) && !model.optBoolean("editable",false);
        JSONObject links=model.optJSONObject("style_link");
        String nextStyle=links==null?"":links.toString();
        int revision=scene.session.assets.fontRevision;
        if(next.equals(source) && rich==markdown && (!hasLinks || nextStyle.equals(linkStyle) && revision==fontRevision)) return value;
        source=next;markdown=rich;linkStyle=nextStyle;fontRevision=revision;ready=true;hasLinks=false;
        if(!rich) {value=next;return value;}
        JSONObject response=scene.session.call(6,next,0,0,0);
        if(!response.optBoolean("ok")) throw new IllegalArgumentException(response.optString("error"));
        JSONObject document=response.getJSONObject("document");
        SpannableString text=new SpannableString(document.getString("text"));
        JSONArray spans=document.getJSONArray("spans");
        for(int i=0;i<spans.length();i++) {
            JSONObject span=spans.getJSONObject(i);
            int start=span.getInt("start"),end=span.getInt("end");
            if(start<0 || end>text.length() || start>=end) throw new IllegalArgumentException("Invalid Android text span");
            Object style;
            switch(span.getString("kind")) {
                case "bold": style=new StyleSpan(Typeface.BOLD);break;
                case "italic": style=new StyleSpan(Typeface.ITALIC);break;
                case "strike": style=new StrikethroughSpan();break;
                case "code": style=new TypefaceSpan("monospace");break;
                case "color": style=new ForegroundColorSpan(Color.parseColor(span.getString("value")));break;
                case "heading":
                    text.setSpan(new StyleSpan(Typeface.BOLD),start,end,Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
                    style=new RelativeSizeSpan(new float[]{2f,1.5f,1.25f,1.1f,1f,1f}[span.getInt("level")-1]);break;
                case "quote": style=new QuoteSpan(Color.GRAY,(int)(3*scene.density),(int)(8*scene.density));break;
                case "indent": style=new LeadingMarginSpan.Standard((int)(12*scene.density),(int)(28*scene.density));break;
                case "link":
                    hasLinks=true;
                    style=new URLSpan(span.getString("url")) {
                        @Override public void onClick(View view) {scene.activity.navigate(json("url",getURL(),"target","Current"));}
                        @Override public void updateDrawState(TextPaint paint) {
                            // Appearance is supplied separately so the same
                            // font metrics participate in detached measurement.
                            if(links==null) super.updateDrawState(paint);
                        }
                    };
                    if(links!=null) {
                        TextView view=scene.linkStyleView;
                        ready &= scene.style(view,links);
                        text.setSpan(new LinkAppearance(view.getPaint()),start,end,Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
                    }
                    break;
                default: throw new IllegalArgumentException("Unknown Android text span");
            }
            text.setSpan(style,start,end,Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
        }
        value=new SpannedString(text);
        return value;
    }

    static final class LinkAppearance extends MetricAffectingSpan {
        final Typeface font;
        final float size;
        final int color;
        final boolean underline;
        LinkAppearance(TextPaint paint) {font=paint.getTypeface();size=paint.getTextSize();color=paint.getColor();underline=paint.isUnderlineText();}
        @Override public void updateMeasureState(TextPaint paint) {paint.setTypeface(font);paint.setTextSize(size);}
        @Override public void updateDrawState(TextPaint paint) {updateMeasureState(paint);paint.setColor(color);paint.setUnderlineText(underline);}
    }
}

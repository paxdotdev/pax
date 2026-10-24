package dev.pax;

import android.graphics.Path;
import java.util.ArrayList;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** Kurbo's absolute M/L/Q/C/Z projection used by native clipping and masks. */
final class NativePath {
    private static final Pattern TOKEN=Pattern.compile("[MLQCZmlqcz]|[-+]?(?:\\d*\\.\\d+|\\d+\\.?\\d*)(?:[eE][-+]?\\d+)?");
    private static final Pattern SEPARATORS=Pattern.compile("[\\s,]*");

    static Path parse(String svg) {
        Matcher matcher=TOKEN.matcher(svg);
        Matcher separators=SEPARATORS.matcher(svg);
        ArrayList<String> tokens=new ArrayList<>();int end=0;
        while(matcher.find()) {
            // This runs for every coordinate. String.matches would compile an
            // ICU expression for every separator of every animated mask.
            if(!separators.region(end,matcher.start()).matches())
                throw new IllegalArgumentException("Unsupported native mask path: "+svg.substring(0,Math.min(256,svg.length())));
            tokens.add(matcher.group());end=matcher.end();
        }
        if(!svg.substring(end).trim().isEmpty())throw new IllegalArgumentException("Unsupported native mask path");
        Path p=new Path();int i=0;char command=0;
        while(i<tokens.size()) {
            String token=tokens.get(i);if(Character.isLetter(token.charAt(0))){command=token.charAt(0);i++;}
            int n=command=='M'||command=='L'?2:command=='Q'?4:command=='C'?6:command=='Z'?0:-1;
            if(n<0)throw new IllegalArgumentException("Unsupported native mask command "+command);
            if(n==0){p.close();command=0;continue;}
            if(i+n>tokens.size())throw new IllegalArgumentException("Truncated native mask");
            float[] a=new float[n];for(int j=0;j<n;j++)a[j]=Float.parseFloat(tokens.get(i++));
            if(command=='M'){p.moveTo(a[0],a[1]);command='L';}else if(command=='L')p.lineTo(a[0],a[1]);else if(command=='Q')p.quadTo(a[0],a[1],a[2],a[3]);else p.cubicTo(a[0],a[1],a[2],a[3],a[4],a[5]);
        }
        return p;
    }
}

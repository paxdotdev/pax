package dev.pax;

import android.graphics.Typeface;
import java.util.Locale;

/** Resolve Android's installed families and aliases without changing asset names. */
final class SystemFonts {
    static Typeface resolve(String family, int weight, boolean italic) {
        Typeface base = Typeface.create(family, Typeface.NORMAL);
        // Android's family lookup is case-sensitive, while standard aliases
        // such as georgia and arial are lowercase. Keep an exact platform name
        // when it resolves; try the lowercase alias before accepting fallback.
        if (base.equals(Typeface.DEFAULT) && family != null)
            base = Typeface.create(family.toLowerCase(Locale.ROOT), Typeface.NORMAL);
        return Typeface.create(base, weight, italic);
    }
}

# Text, Fonts & Images
<!-- summary: Text styling, typography, and image assets. -->
<!-- tags: text, fonts, images -->

`Text` renders content through the target's native text system. Its `style`
property accepts a `TextStyle` object containing font, size, fill, alignment,
and underline settings:

```pax
<Text text="Hello, Pax" class=heading />

@settings {
    .heading {
        style: {
            font: "Times New Roman"
            font_size: 32px
            fill: WHITE
            align_vertical: TextAlignVertical::Center
            align_horizontal: TextAlignHorizontal::Left
            align_multiline: TextAlignHorizontal::Left
            underline: false
        }
    }
}
```

## Font values

A string selects a locally available font family. It expands to a `Font::Web`
value with an empty stylesheet URL and normal style and weight:

```pax
font: "Times New Roman"
```

A named object configures a hosted font and its optional modifiers:

```pax
font: {
    family: "Inter"
    url: "https://fonts.googleapis.com/css2?family=Inter:wght@400;700&display=swap"
    weight: 700
}
```

The supported fields are:

- `family`: font-family name
- `url`: optional stylesheet URL used to load a hosted font
- `style`: `FontStyle::Normal`, `Italic`, or `Oblique`
- `weight`: a CSS numeric weight from `100` through `900`, in increments of
  `100`; the corresponding `FontWeight` enum variants are also accepted

Fields may appear in any order. Style and weight default to normal when
omitted. Supplying `family` without `url` selects a locally available font,
just like the string shorthand:

```pax
font: {
    family: "Arial"
    weight: 500
}

font: {
    family: "Cormorant Garamond"
    url: "https://example.com/cormorant.css"
    style: FontStyle::Italic
    weight: 500
}
```

`Font::Web(family, URL, style, weight)` remains valid as an explicit longhand.

## Image sources

An `ImageSource` string is shorthand for `ImageSource::Url`. The string can be
an asset path understood by the chassis or a remote URL:

```pax
<Image source="assets/spaceship.png" />
<Image source="https://example.com/avatar.png" />
```

A string expression is coerced the same way, allowing the source to update
reactively:

```pax
<Image source={avatar_url} />
```

The URL shorthand is scalar and therefore has no magic indexes. Omit `source`
to use `ImageSource::Empty`; use the explicit `ImageSource::Data(width, height,
bytes)` constructor for raw RGBA pixels. `ImageSource::Url("...")` remains a
valid explicit longhand.

Keep project-owned files under the project's `assets/` directory and reference
them with an `assets/...` path. Pax copies those assets into the target bundle.

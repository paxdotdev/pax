use pax_std::FontWeight;

pub(crate) const FONT_FAMILIES: &[(&str, &str)] = &[
    (
        "Alegreya",
        "https://fonts.googleapis.com/css2?family=Alegreya:ital,wght@0,400;0,500;0,600;0,700;0,800;0,900;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Arimo",
        "https://fonts.googleapis.com/css2?family=Arimo:ital,wght@0,400;0,500;0,600;0,700;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "Cormorant Garamond",
        "https://fonts.googleapis.com/css2?family=Cormorant+Garamond:ital,wght@0,300;0,400;0,500;0,600;0,700;1,300;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "Crimson Text",
        "https://fonts.googleapis.com/css2?family=Crimson+Text:ital,wght@0,400;0,600;0,700;1,400;1,600;1,700&display=swap",
    ),
    (
        "Fira Code",
        "https://fonts.googleapis.com/css2?family=Fira+Code:wght@300;400;500;600;700&display=swap",
    ),
    (
        "Fira Sans",
        "https://fonts.googleapis.com/css2?family=Fira+Sans:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "IBM Plex Mono",
        "https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;1,100;1,200;1,300;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "IBM Plex Sans",
        "https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;1,100;1,200;1,300;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "IBM Plex Serif",
        "https://fonts.googleapis.com/css2?family=IBM+Plex+Serif:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;1,100;1,200;1,300;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "Inconsolata",
        "https://fonts.googleapis.com/css2?family=Inconsolata:wght@200..900&display=swap",
    ),
    (
        "Inter",
        "https://fonts.googleapis.com/css2?family=Inter:wght@100..900&display=swap",
    ),
    (
        "JetBrains Mono",
        "https://fonts.googleapis.com/css2?family=JetBrains+Mono:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800&display=swap",
    ),
    (
        "Lato",
        "https://fonts.googleapis.com/css2?family=Lato:ital,wght@0,100;0,300;0,400;0,700;0,900;1,100;1,300;1,400;1,700;1,900&display=swap",
    ),
    (
        "Lora",
        "https://fonts.googleapis.com/css2?family=Lora:ital,wght@0,400;0,500;0,600;0,700;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "Merriweather",
        "https://fonts.googleapis.com/css2?family=Merriweather:ital,wght@0,300;0,400;0,700;0,900;1,300;1,400;1,700;1,900&display=swap",
    ),
    (
        "Montserrat",
        "https://fonts.googleapis.com/css2?family=Montserrat:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Noto Sans",
        "https://fonts.googleapis.com/css2?family=Noto+Sans:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Noto Serif",
        "https://fonts.googleapis.com/css2?family=Noto+Serif:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Open Sans",
        "https://fonts.googleapis.com/css2?family=Open+Sans:ital,wght@0,300;0,400;0,500;0,600;0,700;0,800;1,300;1,400;1,500;1,600;1,700;1,800&display=swap",
    ),
    (
        "Oswald",
        "https://fonts.googleapis.com/css2?family=Oswald:wght@200..700&display=swap",
    ),
    (
        "Playfair Display",
        "https://fonts.googleapis.com/css2?family=Playfair+Display:ital,wght@0,400;0,500;0,600;0,700;0,800;0,900;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "PT Sans",
        "https://fonts.googleapis.com/css2?family=PT+Sans:ital,wght@0,400;0,700;1,400;1,700&display=swap",
    ),
    (
        "PT Serif",
        "https://fonts.googleapis.com/css2?family=PT+Serif:ital,wght@0,400;0,700;1,400;1,700&display=swap",
    ),
    (
        "Raleway",
        "https://fonts.googleapis.com/css2?family=Raleway:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Roboto",
        "https://fonts.googleapis.com/css2?family=Roboto:ital,wght@0,100;0,300;0,400;0,500;0,700;0,900;1,100;1,300;1,400;1,500;1,700;1,900&display=swap",
    ),
    (
        "Roboto Mono",
        "https://fonts.googleapis.com/css2?family=Roboto+Mono:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;1,100;1,200;1,300;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "Roboto Slab",
        "https://fonts.googleapis.com/css2?family=Roboto+Slab:wght@100;200;300;400;500;600;700;800;900&display=swap",
    ),
    (
        "Source Code Pro",
        "https://fonts.googleapis.com/css2?family=Source+Code+Pro:ital,wght@0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Source Sans Pro",
        "https://fonts.googleapis.com/css2?family=Source+Sans+Pro:ital,wght@0,200;0,300;0,400;0,600;0,700;0,900;1,200;1,300;1,400;1,600;1,700;1,900&display=swap",
    ),
    (
        "Ubuntu",
        "https://fonts.googleapis.com/css2?family=Ubuntu:ital,wght@0,300;0,400;0,500;0,700;1,300;1,400;1,500;1,700&display=swap",
    ),
    (
        "Ubuntu Mono",
        "https://fonts.googleapis.com/css2?family=Ubuntu+Mono:ital,wght@0,400;0,700;1,400;1,700&display=swap",
    ),
    (
        "Unknown",
        "",
    ),
    (
       "ff-real-headline-pro",
       "https://use.typekit.net/ivu7epf.css",        
    ),
];

pub(crate) const FONT_WEIGHTS: &[(&str, FontWeight)] = &[
    ("Thin", FontWeight::Thin),
    ("ExtraLight", FontWeight::ExtraLight),
    ("Light", FontWeight::Light),
    ("Normal", FontWeight::Normal),
    ("Medium", FontWeight::Medium),
    ("SemiBold", FontWeight::SemiBold),
    ("Bold", FontWeight::Bold),
    ("ExtraBold", FontWeight::ExtraBold),
    ("Black", FontWeight::Black),
];

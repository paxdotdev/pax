// Station order and codes checked against operator sources; see README for provenance.
pub const METRO: &[(&str, &str, &str, &str, &str)] = &[
    ("G", "Ginza", "銀座線", "EF9816",
        "G01 Shibuya|G02 Omote-sando|G03 Gaiemmae|G04 Aoyama-itchome|G05 Akasaka-mitsuke|G06 Tameike-sanno|G07 Toranomon|G08 Shimbashi|G09 Ginza|G10 Kyobashi|G11 Nihombashi|G12 Mitsukoshimae|G13 Kanda|G14 Suehirocho|G15 Ueno-hirokoji|G16 Ueno|G17 Inaricho|G18 Tawaramachi|G19 Asakusa"),
    ("M", "Marunouchi", "丸ノ内線", "EF244F",
        "M01 Ogikubo|M02 Minami-asagaya|M03 Shin-koenji|M04 Higashi-koenji|M05 Shin-nakano|m03 Honancho|m04 Nakano-fujimicho|m05 Nakano-shimbashi|M06 Nakano-sakaue|M07 Nishi-shinjuku|M08 Shinjuku|M09 Shinjuku-sanchome|M10 Shinjuku-gyoemmae|M11 Yotsuya-sanchome|M12 Yotsuya|M13 Akasaka-mitsuke|M14 Kokkai-gijidomae|M15 Kasumigaseki|M16 Ginza|M17 Tokyo|M18 Otemachi|M19 Awajicho|M20 Ochanomizu|M21 Hongo-sanchome|M22 Korakuen|M23 Myogadani|M24 Shin-otsuka|M25 Ikebukuro"),
    ("H", "Hibiya", "日比谷線", "8D969D",
        "H01 Naka-meguro|H02 Ebisu|H03 Hiro-o|H04 Roppongi|H05 Kamiyacho|H06 Toranomon Hills|H07 Kasumigaseki|H08 Hibiya|H09 Ginza|H10 Higashi-ginza|H11 Tsukiji|H12 Hatchobori|H13 Kayabacho|H14 Ningyocho|H15 Kodemmacho|H16 Akihabara|H17 Naka-okachimachi|H18 Ueno|H19 Iriya|H20 Minowa|H21 Minami-senju|H22 Kita-senju"),
    ("T", "Tozai", "東西線", "009BBF",
        "T01 Nakano|T02 Ochiai|T03 Takadanobaba|T04 Waseda|T05 Kagurazaka|T06 Iidabashi|T07 Kudanshita|T08 Takebashi|T09 Otemachi|T10 Nihombashi|T11 Kayabacho|T12 Monzen-nakacho|T13 Kiba|T14 Toyocho|T15 Minami-sunamachi|T16 Nishi-kasai|T17 Kasai|T18 Urayasu|T19 Minami-gyotoku|T20 Gyotoku|T21 Myoden|T22 Baraki-nakayama|T23 Nishi-funabashi"),
    ("C", "Chiyoda", "千代田線", "00A76F",
        "C01 Yoyogi-uehara|C02 Yoyogi-koen|C03 Meiji-jingumae|C04 Omote-sando|C05 Nogizaka|C06 Akasaka|C07 Kokkai-gijidomae|C08 Kasumigaseki|C09 Hibiya|C10 Nijubashimae|C11 Otemachi|C12 Shin-ochanomizu|C13 Yushima|C14 Nezu|C15 Sendagi|C16 Nishi-nippori|C17 Machiya|C18 Kita-senju|C19 Ayase|C20 Kita-ayase"),
    ("Y", "Yurakucho", "有楽町線", "BC9F50",
        "Y01 Wakoshi|Y02 Chikatetsu-narimasu|Y03 Chikatetsu-akatsuka|Y04 Heiwadai|Y05 Hikawadai|Y06 Kotake-mukaihara|Y07 Senkawa|Y08 Kanamecho|Y09 Ikebukuro|Y10 Higashi-ikebukuro|Y11 Gokokuji|Y12 Edogawabashi|Y13 Iidabashi|Y14 Ichigaya|Y15 Kojimachi|Y16 Nagatacho|Y17 Sakuradamon|Y18 Yurakucho|Y19 Ginza-itchome|Y20 Shintomicho|Y21 Tsukishima|Y22 Toyosu|Y23 Tatsumi|Y24 Shin-kiba"),
    ("Z", "Hanzomon", "半蔵門線", "8265AF",
        "Z01 Shibuya|Z02 Omote-sando|Z03 Aoyama-itchome|Z04 Nagatacho|Z05 Hanzomon|Z06 Kudanshita|Z07 Jimbocho|Z08 Otemachi|Z09 Mitsukoshimae|Z10 Suitengumae|Z11 Kiyosumi-shirakawa|Z12 Sumiyoshi|Z13 Kinshicho|Z14 Oshiage"),
    ("N", "Namboku", "南北線", "00A39B",
        "N01 Meguro|N02 Shirokanedai|N03 Shirokane-takanawa|N04 Azabu-juban|N05 Roppongi-itchome|N06 Tameike-sanno|N07 Nagatacho|N08 Yotsuya|N09 Ichigaya|N10 Iidabashi|N11 Korakuen|N12 Todaimae|N13 Hon-komagome|N14 Komagome|N15 Nishigahara|N16 Oji|N17 Oji-kamiya|N18 Shimo|N19 Akabane-iwabuchi"),
    ("F", "Fukutoshin", "副都心線", "986B45",
        "F01 Wakoshi|F02 Chikatetsu-narimasu|F03 Chikatetsu-akatsuka|F04 Heiwadai|F05 Hikawadai|F06 Kotake-mukaihara|F07 Senkawa|F08 Kanamecho|F09 Ikebukuro|F10 Zoshigaya|F11 Nishi-waseda|F12 Higashi-shinjuku|F13 Shinjuku-sanchome|F14 Kita-sando|F15 Meiji-jingumae|F16 Shibuya"),
];

pub const TOEI: &[(&str, &str, &str, &str, &str)] = &[
    ("A", "Asakusa", "浅草線", "DB7098", "Nishi-magome|Magome|Nakanobu|Togoshi|Gotanda|Takanawadai|Sengakuji|Mita|Daimon|Shimbashi|Higashi-ginza|Takaracho|Nihombashi|Ningyocho|Higashi-nihombashi|Asakusabashi|Kuramae|Asakusa|Honjo-azumabashi|Oshiage"),
    ("I", "Mita", "三田線", "0079C2", "Meguro|Shirokanedai|Shirokane-takanawa|Mita|Shibakoen|Onarimon|Uchisaiwaicho|Hibiya|Otemachi|Jimbocho|Suidobashi|Kasuga|Hakusan|Sengoku|Sugamo|Nishi-sugamo|Shin-itabashi|Itabashikuyakushomae|Itabashihoncho|Motohasunuma|Shimura-sakaue|Shimura-sanchome|Hasune|Nishidai|Takashimadaira|Shin-takashimadaira|Nishi-takashimadaira"),
    ("S", "Shinjuku", "新宿線", "81A83D", "Shinjuku|Shinjuku-sanchome|Akebonobashi|Ichigaya|Kudanshita|Jimbocho|Ogawamachi|Iwamotocho|Bakuro-yokoyama|Hamacho|Morishita|Kikukawa|Sumiyoshi|Nishi-ojima|Ojima|Higashi-ojima|Funabori|Ichinoe|Mizue|Shinozaki|Motoyawata"),
    ("E", "Oedo", "大江戸線", "B91575", "Shinjuku-nishiguchi|Higashi-shinjuku|Wakamatsu-kawada|Ushigome-yanagicho|Ushigome-kagurazaka|Iidabashi|Kasuga|Hongo-sanchome|Ueno-okachimachi|Shin-okachimachi|Kuramae|Ryogoku|Morishita|Kiyosumi-shirakawa|Monzen-nakacho|Tsukishima|Kachidoki|Tsukijishijo|Shiodome|Daimon|Akabanebashi|Azabu-juban|Roppongi|Aoyama-itchome|Kokuritsu-kyogijo|Yoyogi|Shinjuku|Tochomae|Nishi-shinjuku-gochome|Nakano-sakaue|Higashi-nakano|Nakai|Ochiai-minami-nagasaki|Shin-egota|Nerima|Toshimaen|Nerima-kasugacho|Hikarigaoka"),
];

// JR numbering order; the final edge closes JY30 → JY01.
pub const YAMANOTE: &str = "Tokyo|Kanda|Akihabara|Okachimachi|Ueno|Uguisudani|Nippori|Nishi-nippori|Tabata|Komagome|Sugamo|Otsuka|Ikebukuro|Mejiro|Takadanobaba|Shin-okubo|Shinjuku|Yoyogi|Harajuku|Shibuya|Ebisu|Meguro|Gotanda|Osaki|Shinagawa|Takanawa Gateway|Tamachi|Hamamatsucho|Shimbashi|Yurakucho";

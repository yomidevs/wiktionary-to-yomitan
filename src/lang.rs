//! Abstractions over language codes.
//!
//! This file was generated and should not be edited directly.
//! The source code can be found at scripts/build.py

use std::{
    fmt::{Debug, Display},
    hash::Hash,
    str::FromStr,
};

use serde::{Deserialize, Serialize};

// The idea is from https://github.com/johnstonskj/rust-codes/tree/main
//
/// Helper trait to ensure that some other traits are implemented.
pub trait Code: Clone + Debug + Display + FromStr + AsRef<str> + PartialEq + Eq + Hash {}

impl Code for Lang {}
impl Code for EditionSpec {}
impl Code for Edition {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Lang {
    /// Afrikaans
    Af,
    /// Albanian
    Sq,
    /// Amharic
    Am,
    /// Egyptian Arabic
    Arz,
    /// Gulf Arabic
    Afb,
    /// Arabic
    Ar,
    /// North Levantine Arabic
    Apc,
    /// South Levantine Arabic
    Ajp,
    /// Armenian
    Hy,
    /// Old Armenian
    Xcl,
    /// Aromanian
    Rup,
    /// Assamese
    As,
    /// Assyrian Neo-Aramaic
    Aii,
    /// Asturian
    Ast,
    /// Azerbaijani
    Az,
    /// Bashkir
    Ba,
    /// Basque
    Eu,
    /// Belarusian
    Be,
    /// Bengali
    Bn,
    /// Central Bikol
    Bcl,
    /// Bulgarian
    Bg,
    /// Burmese
    My,
    /// Catalan
    Ca,
    /// Cebuano
    Ceb,
    /// Chinese
    Zh,
    /// Yue Chinese
    Yue,
    /// Mandarin Chinese
    Cmn,
    /// Cimbrian
    Cim,
    /// Coptic
    Cop,
    /// Cornish
    Kw,
    /// Crimean Tatar
    Crh,
    /// Czech
    Cs,
    /// Old Czech
    Zlwocs,
    /// Danish
    Da,
    /// Dutch
    Nl,
    /// Middle Dutch
    Dum,
    /// Egyptian
    Egy,
    /// English
    En,
    /// Middle English
    Enm,
    /// Old English
    Ang,
    /// Simple English
    Simple,
    /// Esperanto
    Eo,
    /// Estonian
    Et,
    /// Faroese
    Fo,
    /// Finnish
    Fi,
    /// French
    Fr,
    /// Middle French
    Frm,
    /// Old French
    Fro,
    /// Galician
    Gl,
    /// Georgian
    Ka,
    /// German
    De,
    /// Old High German
    Goh,
    /// Gothic
    Got,
    /// Greek
    El,
    /// Ancient Greek
    Grc,
    /// Gujarati
    Gu,
    /// Hawaiian
    Haw,
    /// Hebrew
    He,
    /// Hindi
    Hi,
    /// Hungarian
    Hu,
    /// Icelandic
    Is,
    /// Ido
    Io,
    /// Indonesian
    Id,
    /// Ingrian
    Izh,
    /// Interlingua
    Ia,
    /// Irish
    Ga,
    /// Old Irish
    Sga,
    /// Italian
    It,
    /// Japanese
    Ja,
    /// Javanese
    Jv,
    /// Kannada
    Kn,
    /// Kashubian
    Csb,
    /// Kazakh
    Kk,
    /// Khiamniungan Naga
    Kix,
    /// Khmer
    Km,
    /// Korean
    Ko,
    /// Kurdish
    Ku,
    /// Northern Kurdish
    Kmr,
    /// Kyrgyz
    Ky,
    /// Ladin
    Lld,
    /// Lao
    Lo,
    /// Latin
    La,
    /// Latvian
    Lv,
    /// Laz
    Lzz,
    /// Lithuanian
    Lt,
    /// Livonian
    Liv,
    /// Luxembourgish
    Lb,
    /// Macedonian
    Mk,
    /// Malagasy
    Mg,
    /// Malay
    Ms,
    /// Malayalam
    Ml,
    /// Maltese
    Mt,
    /// Manx
    Gv,
    /// Marathi
    Mr,
    /// Mongolian
    Mn,
    /// Māori
    Mi,
    /// Classical Nahuatl
    Nah,
    /// Navajo
    Nv,
    /// Norman
    Nrf,
    /// Northern Sami
    Se,
    /// Norwegian
    No,
    /// Norwegian Bokmål
    Nb,
    /// Norwegian Nynorsk
    Nn,
    /// Occitan
    Oc,
    /// Odia
    Or,
    /// Ojibwe
    Oj,
    /// Old Church Slavonic
    Cu,
    /// Old Norse
    Non,
    /// Pali
    Pi,
    /// Pannonian Rusyn
    Rsk,
    /// Persian
    Fa,
    /// Plautdietsch
    Pdt,
    /// Polish
    Pl,
    /// Old Polish
    Zlwopl,
    /// Portuguese
    Pt,
    /// Proto-Finnic
    Urjfinpro,
    /// Proto-Germanic
    Gempro,
    /// Proto-Slavic
    Slapro,
    /// Proto-West Germanic
    Gmwpro,
    /// Punjabi
    Pa,
    /// Romanian
    Ro,
    /// Russian
    Ru,
    /// Sanskrit
    Sa,
    /// Scots
    Sco,
    /// Scottish Gaelic
    Gd,
    /// Serbo-Croatian
    Sh,
    /// Sicilian
    Scn,
    /// Slovak
    Sk,
    /// Slovene
    Sl,
    /// Lower Sorbian
    Dsb,
    /// Spanish
    Es,
    /// Sundanese
    Su,
    /// Swahili
    Sw,
    /// Swedish
    Sv,
    /// Classical Syriac
    Syc,
    /// Tagalog
    Tl,
    /// Tajik
    Tg,
    /// Tamil
    Ta,
    /// Tatar
    Tt,
    /// Telugu
    Te,
    /// Thai
    Th,
    /// Tibetan
    Bo,
    /// Toki Pona
    Tok,
    /// Turkish
    Tr,
    /// Ottoman Turkish
    Ota,
    /// Ukrainian
    Uk,
    /// Unami
    Unm,
    /// Urdu
    Ur,
    /// Uyghur
    Ug,
    /// Uzbek
    Uz,
    /// Venetan
    Vec,
    /// Vietnamese
    Vi,
    /// Volapük
    Vo,
    /// Votic
    Vot,
    /// Welsh
    Cy,
    /// West Circassian
    Ady,
    /// West Frisian
    Fy,
    /// Yakut
    Sah,
    /// Yiddish
    Yi,
    /// Yoruba
    Yo,
    /// Zulu
    Zu,
}

impl From<Edition> for Lang {
    fn from(value: Edition) -> Self {
        match value {
            Edition::Zh => Self::Zh,
            Edition::Cs => Self::Cs,
            Edition::Nl => Self::Nl,
            Edition::En => Self::En,
            Edition::Simple => Self::Simple,
            Edition::Fr => Self::Fr,
            Edition::De => Self::De,
            Edition::El => Self::El,
            Edition::Id => Self::Id,
            Edition::It => Self::It,
            Edition::Ja => Self::Ja,
            Edition::Ko => Self::Ko,
            Edition::Ku => Self::Ku,
            Edition::Ms => Self::Ms,
            Edition::Pl => Self::Pl,
            Edition::Pt => Self::Pt,
            Edition::Ru => Self::Ru,
            Edition::Es => Self::Es,
            Edition::Th => Self::Th,
            Edition::Tr => Self::Tr,
            Edition::Vi => Self::Vi,
        }
    }
}

impl Lang {
    pub const fn help_isos() -> &'static str {
        "Supported isos: af | sq | am | arz | afb | ar | apc | ajp | hy | xcl | rup | as | aii | ast | az | ba | eu | be | bn | bcl | bg | my | ca | ceb | zh | yue | cmn | cim | cop | kw | crh | cs | zlw-ocs | da | nl | dum | egy | en | enm | ang | simple | eo | et | fo | fi | fr | frm | fro | gl | ka | de | goh | got | el | grc | gu | haw | he | hi | hu | is | io | id | izh | ia | ga | sga | it | ja | jv | kn | csb | kk | kix | km | ko | ku | kmr | ky | lld | lo | la | lv | lzz | lt | liv | lb | mk | mg | ms | ml | mt | gv | mr | mn | mi | nah | nv | nrf | se | no | nb | nn | oc | or | oj | cu | non | pi | rsk | fa | pdt | pl | zlw-opl | pt | urj-fin-pro | gem-pro | sla-pro | gmw-pro | pa | ro | ru | sa | sco | gd | sh | scn | sk | sl | dsb | es | su | sw | sv | syc | tl | tg | ta | tt | te | th | bo | tok | tr | ota | uk | unm | ur | ug | uz | vec | vi | vo | vot | cy | ady | fy | sah | yi | yo | zu"
    }

    pub const fn help_isos_coloured() -> &'static str {
        "Supported isos: af | sq | am | arz | afb | ar | apc | ajp | hy | xcl | rup | as | aii | ast | az | ba | eu | be | bn | bcl | bg | my | ca | ceb | [32mzh[0m | yue | cmn | cim | cop | kw | crh | [32mcs[0m | zlw-ocs | da | [32mnl[0m | dum | egy | [32men[0m | enm | ang | [32msimple[0m | eo | et | fo | fi | [32mfr[0m | frm | fro | gl | ka | [32mde[0m | goh | got | [32mel[0m | grc | gu | haw | he | hi | hu | is | io | [32mid[0m | izh | ia | ga | sga | [32mit[0m | [32mja[0m | jv | kn | csb | kk | kix | km | [32mko[0m | [32mku[0m | kmr | ky | lld | lo | la | lv | lzz | lt | liv | lb | mk | mg | [32mms[0m | ml | mt | gv | mr | mn | mi | nah | nv | nrf | se | no | nb | nn | oc | or | oj | cu | non | pi | rsk | fa | pdt | [32mpl[0m | zlw-opl | [32mpt[0m | urj-fin-pro | gem-pro | sla-pro | gmw-pro | pa | ro | [32mru[0m | sa | sco | gd | sh | scn | sk | sl | dsb | [32mes[0m | su | sw | sv | syc | tl | tg | ta | tt | te | [32mth[0m | bo | tok | [32mtr[0m | ota | uk | unm | ur | ug | uz | vec | [32mvi[0m | vo | vot | cy | ady | fy | sah | yi | yo | zu"
    }

    pub const fn help_editions() -> &'static str {
        "Supported editions: zh | cs | nl | en | simple | fr | de | el | id | it | ja | ko | ku | ms | pl | pt | ru | es | th | tr | vi"
    }

    pub const fn long(&self) -> &'static str {
        match self {
            Self::Af => "Afrikaans",
            Self::Sq => "Albanian",
            Self::Am => "Amharic",
            Self::Arz => "Egyptian Arabic",
            Self::Afb => "Gulf Arabic",
            Self::Ar => "Arabic",
            Self::Apc => "North Levantine Arabic",
            Self::Ajp => "South Levantine Arabic",
            Self::Hy => "Armenian",
            Self::Xcl => "Old Armenian",
            Self::Rup => "Aromanian",
            Self::As => "Assamese",
            Self::Aii => "Assyrian Neo-Aramaic",
            Self::Ast => "Asturian",
            Self::Az => "Azerbaijani",
            Self::Ba => "Bashkir",
            Self::Eu => "Basque",
            Self::Be => "Belarusian",
            Self::Bn => "Bengali",
            Self::Bcl => "Central Bikol",
            Self::Bg => "Bulgarian",
            Self::My => "Burmese",
            Self::Ca => "Catalan",
            Self::Ceb => "Cebuano",
            Self::Zh => "Chinese",
            Self::Yue => "Yue Chinese",
            Self::Cmn => "Mandarin Chinese",
            Self::Cim => "Cimbrian",
            Self::Cop => "Coptic",
            Self::Kw => "Cornish",
            Self::Crh => "Crimean Tatar",
            Self::Cs => "Czech",
            Self::Zlwocs => "Old Czech",
            Self::Da => "Danish",
            Self::Nl => "Dutch",
            Self::Dum => "Middle Dutch",
            Self::Egy => "Egyptian",
            Self::En => "English",
            Self::Enm => "Middle English",
            Self::Ang => "Old English",
            Self::Simple => "Simple English",
            Self::Eo => "Esperanto",
            Self::Et => "Estonian",
            Self::Fo => "Faroese",
            Self::Fi => "Finnish",
            Self::Fr => "French",
            Self::Frm => "Middle French",
            Self::Fro => "Old French",
            Self::Gl => "Galician",
            Self::Ka => "Georgian",
            Self::De => "German",
            Self::Goh => "Old High German",
            Self::Got => "Gothic",
            Self::El => "Greek",
            Self::Grc => "Ancient Greek",
            Self::Gu => "Gujarati",
            Self::Haw => "Hawaiian",
            Self::He => "Hebrew",
            Self::Hi => "Hindi",
            Self::Hu => "Hungarian",
            Self::Is => "Icelandic",
            Self::Io => "Ido",
            Self::Id => "Indonesian",
            Self::Izh => "Ingrian",
            Self::Ia => "Interlingua",
            Self::Ga => "Irish",
            Self::Sga => "Old Irish",
            Self::It => "Italian",
            Self::Ja => "Japanese",
            Self::Jv => "Javanese",
            Self::Kn => "Kannada",
            Self::Csb => "Kashubian",
            Self::Kk => "Kazakh",
            Self::Kix => "Khiamniungan Naga",
            Self::Km => "Khmer",
            Self::Ko => "Korean",
            Self::Ku => "Kurdish",
            Self::Kmr => "Northern Kurdish",
            Self::Ky => "Kyrgyz",
            Self::Lld => "Ladin",
            Self::Lo => "Lao",
            Self::La => "Latin",
            Self::Lv => "Latvian",
            Self::Lzz => "Laz",
            Self::Lt => "Lithuanian",
            Self::Liv => "Livonian",
            Self::Lb => "Luxembourgish",
            Self::Mk => "Macedonian",
            Self::Mg => "Malagasy",
            Self::Ms => "Malay",
            Self::Ml => "Malayalam",
            Self::Mt => "Maltese",
            Self::Gv => "Manx",
            Self::Mr => "Marathi",
            Self::Mn => "Mongolian",
            Self::Mi => "Māori",
            Self::Nah => "Classical Nahuatl",
            Self::Nv => "Navajo",
            Self::Nrf => "Norman",
            Self::Se => "Northern Sami",
            Self::No => "Norwegian",
            Self::Nb => "Norwegian Bokmål",
            Self::Nn => "Norwegian Nynorsk",
            Self::Oc => "Occitan",
            Self::Or => "Odia",
            Self::Oj => "Ojibwe",
            Self::Cu => "Old Church Slavonic",
            Self::Non => "Old Norse",
            Self::Pi => "Pali",
            Self::Rsk => "Pannonian Rusyn",
            Self::Fa => "Persian",
            Self::Pdt => "Plautdietsch",
            Self::Pl => "Polish",
            Self::Zlwopl => "Old Polish",
            Self::Pt => "Portuguese",
            Self::Urjfinpro => "Proto-Finnic",
            Self::Gempro => "Proto-Germanic",
            Self::Slapro => "Proto-Slavic",
            Self::Gmwpro => "Proto-West Germanic",
            Self::Pa => "Punjabi",
            Self::Ro => "Romanian",
            Self::Ru => "Russian",
            Self::Sa => "Sanskrit",
            Self::Sco => "Scots",
            Self::Gd => "Scottish Gaelic",
            Self::Sh => "Serbo-Croatian",
            Self::Scn => "Sicilian",
            Self::Sk => "Slovak",
            Self::Sl => "Slovene",
            Self::Dsb => "Lower Sorbian",
            Self::Es => "Spanish",
            Self::Su => "Sundanese",
            Self::Sw => "Swahili",
            Self::Sv => "Swedish",
            Self::Syc => "Classical Syriac",
            Self::Tl => "Tagalog",
            Self::Tg => "Tajik",
            Self::Ta => "Tamil",
            Self::Tt => "Tatar",
            Self::Te => "Telugu",
            Self::Th => "Thai",
            Self::Bo => "Tibetan",
            Self::Tok => "Toki Pona",
            Self::Tr => "Turkish",
            Self::Ota => "Ottoman Turkish",
            Self::Uk => "Ukrainian",
            Self::Unm => "Unami",
            Self::Ur => "Urdu",
            Self::Ug => "Uyghur",
            Self::Uz => "Uzbek",
            Self::Vec => "Venetan",
            Self::Vi => "Vietnamese",
            Self::Vo => "Volapük",
            Self::Vot => "Votic",
            Self::Cy => "Welsh",
            Self::Ady => "West Circassian",
            Self::Fy => "West Frisian",
            Self::Sah => "Yakut",
            Self::Yi => "Yiddish",
            Self::Yo => "Yoruba",
            Self::Zu => "Zulu",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Af,
            Self::Sq,
            Self::Am,
            Self::Arz,
            Self::Afb,
            Self::Ar,
            Self::Apc,
            Self::Ajp,
            Self::Hy,
            Self::Xcl,
            Self::Rup,
            Self::As,
            Self::Aii,
            Self::Ast,
            Self::Az,
            Self::Ba,
            Self::Eu,
            Self::Be,
            Self::Bn,
            Self::Bcl,
            Self::Bg,
            Self::My,
            Self::Ca,
            Self::Ceb,
            Self::Zh,
            Self::Yue,
            Self::Cmn,
            Self::Cim,
            Self::Cop,
            Self::Kw,
            Self::Crh,
            Self::Cs,
            Self::Zlwocs,
            Self::Da,
            Self::Nl,
            Self::Dum,
            Self::Egy,
            Self::En,
            Self::Enm,
            Self::Ang,
            Self::Simple,
            Self::Eo,
            Self::Et,
            Self::Fo,
            Self::Fi,
            Self::Fr,
            Self::Frm,
            Self::Fro,
            Self::Gl,
            Self::Ka,
            Self::De,
            Self::Goh,
            Self::Got,
            Self::El,
            Self::Grc,
            Self::Gu,
            Self::Haw,
            Self::He,
            Self::Hi,
            Self::Hu,
            Self::Is,
            Self::Io,
            Self::Id,
            Self::Izh,
            Self::Ia,
            Self::Ga,
            Self::Sga,
            Self::It,
            Self::Ja,
            Self::Jv,
            Self::Kn,
            Self::Csb,
            Self::Kk,
            Self::Kix,
            Self::Km,
            Self::Ko,
            Self::Ku,
            Self::Kmr,
            Self::Ky,
            Self::Lld,
            Self::Lo,
            Self::La,
            Self::Lv,
            Self::Lzz,
            Self::Lt,
            Self::Liv,
            Self::Lb,
            Self::Mk,
            Self::Mg,
            Self::Ms,
            Self::Ml,
            Self::Mt,
            Self::Gv,
            Self::Mr,
            Self::Mn,
            Self::Mi,
            Self::Nah,
            Self::Nv,
            Self::Nrf,
            Self::Se,
            Self::No,
            Self::Nb,
            Self::Nn,
            Self::Oc,
            Self::Or,
            Self::Oj,
            Self::Cu,
            Self::Non,
            Self::Pi,
            Self::Rsk,
            Self::Fa,
            Self::Pdt,
            Self::Pl,
            Self::Zlwopl,
            Self::Pt,
            Self::Urjfinpro,
            Self::Gempro,
            Self::Slapro,
            Self::Gmwpro,
            Self::Pa,
            Self::Ro,
            Self::Ru,
            Self::Sa,
            Self::Sco,
            Self::Gd,
            Self::Sh,
            Self::Scn,
            Self::Sk,
            Self::Sl,
            Self::Dsb,
            Self::Es,
            Self::Su,
            Self::Sw,
            Self::Sv,
            Self::Syc,
            Self::Tl,
            Self::Tg,
            Self::Ta,
            Self::Tt,
            Self::Te,
            Self::Th,
            Self::Bo,
            Self::Tok,
            Self::Tr,
            Self::Ota,
            Self::Uk,
            Self::Unm,
            Self::Ur,
            Self::Ug,
            Self::Uz,
            Self::Vec,
            Self::Vi,
            Self::Vo,
            Self::Vot,
            Self::Cy,
            Self::Ady,
            Self::Fy,
            Self::Sah,
            Self::Yi,
            Self::Yo,
            Self::Zu,
        ]
    }
}

impl TryInto<Edition> for Lang {
    type Error = &'static str;

    fn try_into(self) -> Result<Edition, Self::Error> {
        match self {
            Self::Zh => Ok(Edition::Zh),
            Self::Cs => Ok(Edition::Cs),
            Self::Nl => Ok(Edition::Nl),
            Self::En => Ok(Edition::En),
            Self::Simple => Ok(Edition::Simple),
            Self::Fr => Ok(Edition::Fr),
            Self::De => Ok(Edition::De),
            Self::El => Ok(Edition::El),
            Self::Id => Ok(Edition::Id),
            Self::It => Ok(Edition::It),
            Self::Ja => Ok(Edition::Ja),
            Self::Ko => Ok(Edition::Ko),
            Self::Ku => Ok(Edition::Ku),
            Self::Ms => Ok(Edition::Ms),
            Self::Pl => Ok(Edition::Pl),
            Self::Pt => Ok(Edition::Pt),
            Self::Ru => Ok(Edition::Ru),
            Self::Es => Ok(Edition::Es),
            Self::Th => Ok(Edition::Th),
            Self::Tr => Ok(Edition::Tr),
            Self::Vi => Ok(Edition::Vi),
            _ => Err("language has no edition"),
        }
    }
}

impl FromStr for Lang {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "af" => Ok(Self::Af),
            "sq" => Ok(Self::Sq),
            "am" => Ok(Self::Am),
            "arz" => Ok(Self::Arz),
            "afb" => Ok(Self::Afb),
            "ar" => Ok(Self::Ar),
            "apc" => Ok(Self::Apc),
            "ajp" => Ok(Self::Ajp),
            "hy" => Ok(Self::Hy),
            "xcl" => Ok(Self::Xcl),
            "rup" => Ok(Self::Rup),
            "as" => Ok(Self::As),
            "aii" => Ok(Self::Aii),
            "ast" => Ok(Self::Ast),
            "az" => Ok(Self::Az),
            "ba" => Ok(Self::Ba),
            "eu" => Ok(Self::Eu),
            "be" => Ok(Self::Be),
            "bn" => Ok(Self::Bn),
            "bcl" => Ok(Self::Bcl),
            "bg" => Ok(Self::Bg),
            "my" => Ok(Self::My),
            "ca" => Ok(Self::Ca),
            "ceb" => Ok(Self::Ceb),
            "zh" => Ok(Self::Zh),
            "yue" => Ok(Self::Yue),
            "cmn" => Ok(Self::Cmn),
            "cim" => Ok(Self::Cim),
            "cop" => Ok(Self::Cop),
            "kw" => Ok(Self::Kw),
            "crh" => Ok(Self::Crh),
            "cs" => Ok(Self::Cs),
            "zlw-ocs" => Ok(Self::Zlwocs),
            "da" => Ok(Self::Da),
            "nl" => Ok(Self::Nl),
            "dum" => Ok(Self::Dum),
            "egy" => Ok(Self::Egy),
            "en" => Ok(Self::En),
            "enm" => Ok(Self::Enm),
            "ang" => Ok(Self::Ang),
            "simple" => Ok(Self::Simple),
            "eo" => Ok(Self::Eo),
            "et" => Ok(Self::Et),
            "fo" => Ok(Self::Fo),
            "fi" => Ok(Self::Fi),
            "fr" => Ok(Self::Fr),
            "frm" => Ok(Self::Frm),
            "fro" => Ok(Self::Fro),
            "gl" => Ok(Self::Gl),
            "ka" => Ok(Self::Ka),
            "de" => Ok(Self::De),
            "goh" => Ok(Self::Goh),
            "got" => Ok(Self::Got),
            "el" => Ok(Self::El),
            "grc" => Ok(Self::Grc),
            "gu" => Ok(Self::Gu),
            "haw" => Ok(Self::Haw),
            "he" => Ok(Self::He),
            "hi" => Ok(Self::Hi),
            "hu" => Ok(Self::Hu),
            "is" => Ok(Self::Is),
            "io" => Ok(Self::Io),
            "id" => Ok(Self::Id),
            "izh" => Ok(Self::Izh),
            "ia" => Ok(Self::Ia),
            "ga" => Ok(Self::Ga),
            "sga" => Ok(Self::Sga),
            "it" => Ok(Self::It),
            "ja" => Ok(Self::Ja),
            "jv" => Ok(Self::Jv),
            "kn" => Ok(Self::Kn),
            "csb" => Ok(Self::Csb),
            "kk" => Ok(Self::Kk),
            "kix" => Ok(Self::Kix),
            "km" => Ok(Self::Km),
            "ko" => Ok(Self::Ko),
            "ku" => Ok(Self::Ku),
            "kmr" => Ok(Self::Kmr),
            "ky" => Ok(Self::Ky),
            "lld" => Ok(Self::Lld),
            "lo" => Ok(Self::Lo),
            "la" => Ok(Self::La),
            "lv" => Ok(Self::Lv),
            "lzz" => Ok(Self::Lzz),
            "lt" => Ok(Self::Lt),
            "liv" => Ok(Self::Liv),
            "lb" => Ok(Self::Lb),
            "mk" => Ok(Self::Mk),
            "mg" => Ok(Self::Mg),
            "ms" => Ok(Self::Ms),
            "ml" => Ok(Self::Ml),
            "mt" => Ok(Self::Mt),
            "gv" => Ok(Self::Gv),
            "mr" => Ok(Self::Mr),
            "mn" => Ok(Self::Mn),
            "mi" => Ok(Self::Mi),
            "nah" => Ok(Self::Nah),
            "nv" => Ok(Self::Nv),
            "nrf" => Ok(Self::Nrf),
            "se" => Ok(Self::Se),
            "no" => Ok(Self::No),
            "nb" => Ok(Self::Nb),
            "nn" => Ok(Self::Nn),
            "oc" => Ok(Self::Oc),
            "or" => Ok(Self::Or),
            "oj" => Ok(Self::Oj),
            "cu" => Ok(Self::Cu),
            "non" => Ok(Self::Non),
            "pi" => Ok(Self::Pi),
            "rsk" => Ok(Self::Rsk),
            "fa" => Ok(Self::Fa),
            "pdt" => Ok(Self::Pdt),
            "pl" => Ok(Self::Pl),
            "zlw-opl" => Ok(Self::Zlwopl),
            "pt" => Ok(Self::Pt),
            "urj-fin-pro" => Ok(Self::Urjfinpro),
            "gem-pro" => Ok(Self::Gempro),
            "sla-pro" => Ok(Self::Slapro),
            "gmw-pro" => Ok(Self::Gmwpro),
            "pa" => Ok(Self::Pa),
            "ro" => Ok(Self::Ro),
            "ru" => Ok(Self::Ru),
            "sa" => Ok(Self::Sa),
            "sco" => Ok(Self::Sco),
            "gd" => Ok(Self::Gd),
            "sh" => Ok(Self::Sh),
            "scn" => Ok(Self::Scn),
            "sk" => Ok(Self::Sk),
            "sl" => Ok(Self::Sl),
            "dsb" => Ok(Self::Dsb),
            "es" => Ok(Self::Es),
            "su" => Ok(Self::Su),
            "sw" => Ok(Self::Sw),
            "sv" => Ok(Self::Sv),
            "syc" => Ok(Self::Syc),
            "tl" => Ok(Self::Tl),
            "tg" => Ok(Self::Tg),
            "ta" => Ok(Self::Ta),
            "tt" => Ok(Self::Tt),
            "te" => Ok(Self::Te),
            "th" => Ok(Self::Th),
            "bo" => Ok(Self::Bo),
            "tok" => Ok(Self::Tok),
            "tr" => Ok(Self::Tr),
            "ota" => Ok(Self::Ota),
            "uk" => Ok(Self::Uk),
            "unm" => Ok(Self::Unm),
            "ur" => Ok(Self::Ur),
            "ug" => Ok(Self::Ug),
            "uz" => Ok(Self::Uz),
            "vec" => Ok(Self::Vec),
            "vi" => Ok(Self::Vi),
            "vo" => Ok(Self::Vo),
            "vot" => Ok(Self::Vot),
            "cy" => Ok(Self::Cy),
            "ady" => Ok(Self::Ady),
            "fy" => Ok(Self::Fy),
            "sah" => Ok(Self::Sah),
            "yi" => Ok(Self::Yi),
            "yo" => Ok(Self::Yo),
            "zu" => Ok(Self::Zu),
            _ => Err(format!("unsupported iso code '{s}'\n{}", Self::help_isos())),
        }
    }
}

impl AsRef<str> for Lang {
    fn as_ref(&self) -> &str {
        match self {
            Self::Af => "af",
            Self::Sq => "sq",
            Self::Am => "am",
            Self::Arz => "arz",
            Self::Afb => "afb",
            Self::Ar => "ar",
            Self::Apc => "apc",
            Self::Ajp => "ajp",
            Self::Hy => "hy",
            Self::Xcl => "xcl",
            Self::Rup => "rup",
            Self::As => "as",
            Self::Aii => "aii",
            Self::Ast => "ast",
            Self::Az => "az",
            Self::Ba => "ba",
            Self::Eu => "eu",
            Self::Be => "be",
            Self::Bn => "bn",
            Self::Bcl => "bcl",
            Self::Bg => "bg",
            Self::My => "my",
            Self::Ca => "ca",
            Self::Ceb => "ceb",
            Self::Zh => "zh",
            Self::Yue => "yue",
            Self::Cmn => "cmn",
            Self::Cim => "cim",
            Self::Cop => "cop",
            Self::Kw => "kw",
            Self::Crh => "crh",
            Self::Cs => "cs",
            Self::Zlwocs => "zlw-ocs",
            Self::Da => "da",
            Self::Nl => "nl",
            Self::Dum => "dum",
            Self::Egy => "egy",
            Self::En => "en",
            Self::Enm => "enm",
            Self::Ang => "ang",
            Self::Simple => "simple",
            Self::Eo => "eo",
            Self::Et => "et",
            Self::Fo => "fo",
            Self::Fi => "fi",
            Self::Fr => "fr",
            Self::Frm => "frm",
            Self::Fro => "fro",
            Self::Gl => "gl",
            Self::Ka => "ka",
            Self::De => "de",
            Self::Goh => "goh",
            Self::Got => "got",
            Self::El => "el",
            Self::Grc => "grc",
            Self::Gu => "gu",
            Self::Haw => "haw",
            Self::He => "he",
            Self::Hi => "hi",
            Self::Hu => "hu",
            Self::Is => "is",
            Self::Io => "io",
            Self::Id => "id",
            Self::Izh => "izh",
            Self::Ia => "ia",
            Self::Ga => "ga",
            Self::Sga => "sga",
            Self::It => "it",
            Self::Ja => "ja",
            Self::Jv => "jv",
            Self::Kn => "kn",
            Self::Csb => "csb",
            Self::Kk => "kk",
            Self::Kix => "kix",
            Self::Km => "km",
            Self::Ko => "ko",
            Self::Ku => "ku",
            Self::Kmr => "kmr",
            Self::Ky => "ky",
            Self::Lld => "lld",
            Self::Lo => "lo",
            Self::La => "la",
            Self::Lv => "lv",
            Self::Lzz => "lzz",
            Self::Lt => "lt",
            Self::Liv => "liv",
            Self::Lb => "lb",
            Self::Mk => "mk",
            Self::Mg => "mg",
            Self::Ms => "ms",
            Self::Ml => "ml",
            Self::Mt => "mt",
            Self::Gv => "gv",
            Self::Mr => "mr",
            Self::Mn => "mn",
            Self::Mi => "mi",
            Self::Nah => "nah",
            Self::Nv => "nv",
            Self::Nrf => "nrf",
            Self::Se => "se",
            Self::No => "no",
            Self::Nb => "nb",
            Self::Nn => "nn",
            Self::Oc => "oc",
            Self::Or => "or",
            Self::Oj => "oj",
            Self::Cu => "cu",
            Self::Non => "non",
            Self::Pi => "pi",
            Self::Rsk => "rsk",
            Self::Fa => "fa",
            Self::Pdt => "pdt",
            Self::Pl => "pl",
            Self::Zlwopl => "zlw-opl",
            Self::Pt => "pt",
            Self::Urjfinpro => "urj-fin-pro",
            Self::Gempro => "gem-pro",
            Self::Slapro => "sla-pro",
            Self::Gmwpro => "gmw-pro",
            Self::Pa => "pa",
            Self::Ro => "ro",
            Self::Ru => "ru",
            Self::Sa => "sa",
            Self::Sco => "sco",
            Self::Gd => "gd",
            Self::Sh => "sh",
            Self::Scn => "scn",
            Self::Sk => "sk",
            Self::Sl => "sl",
            Self::Dsb => "dsb",
            Self::Es => "es",
            Self::Su => "su",
            Self::Sw => "sw",
            Self::Sv => "sv",
            Self::Syc => "syc",
            Self::Tl => "tl",
            Self::Tg => "tg",
            Self::Ta => "ta",
            Self::Tt => "tt",
            Self::Te => "te",
            Self::Th => "th",
            Self::Bo => "bo",
            Self::Tok => "tok",
            Self::Tr => "tr",
            Self::Ota => "ota",
            Self::Uk => "uk",
            Self::Unm => "unm",
            Self::Ur => "ur",
            Self::Ug => "ug",
            Self::Uz => "uz",
            Self::Vec => "vec",
            Self::Vi => "vi",
            Self::Vo => "vo",
            Self::Vot => "vot",
            Self::Cy => "cy",
            Self::Ady => "ady",
            Self::Fy => "fy",
            Self::Sah => "sah",
            Self::Yi => "yi",
            Self::Yo => "yo",
            Self::Zu => "zu",
        }
    }
}

impl Lang {
    pub fn iso(&self) -> &str {
        match self {
            Self::Simple => "en",
            _ => self.as_ref(),
        }
    }
}

impl Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EditionSpec {
    /// All editions
    All,
    /// An `Edition`
    One(Edition),
}

impl EditionSpec {
    pub fn variants(&self) -> Vec<Edition> {
        match self {
            Self::All => Edition::all(),
            Self::One(lang) => vec![*lang],
        }
    }
}

impl From<Edition> for EditionSpec {
    fn from(val: Edition) -> Self {
        Self::One(val)
    }
}

impl TryInto<Edition> for EditionSpec {
    type Error = &'static str;

    fn try_into(self) -> Result<Edition, Self::Error> {
        match self {
            Self::All => Err("cannot convert from All"),
            Self::One(lang) => Ok(lang),
        }
    }
}

impl FromStr for EditionSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(Self::All),
            other => Ok(Self::One(Edition::from_str(other)?)),
        }
    }
}

impl AsRef<str> for EditionSpec {
    fn as_ref(&self) -> &str {
        match self {
            Self::All => "all",
            Self::One(lang) => lang.as_ref(),
        }
    }
}

impl Display for EditionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Edition {
    /// Chinese
    Zh,
    /// Czech
    Cs,
    /// Dutch
    Nl,
    /// English
    En,
    /// Simple English
    Simple,
    /// French
    Fr,
    /// German
    De,
    /// Greek
    El,
    /// Indonesian
    Id,
    /// Italian
    It,
    /// Japanese
    Ja,
    /// Korean
    Ko,
    /// Kurdish
    Ku,
    /// Malay
    Ms,
    /// Polish
    Pl,
    /// Portuguese
    Pt,
    /// Russian
    Ru,
    /// Spanish
    Es,
    /// Thai
    Th,
    /// Turkish
    Tr,
    /// Vietnamese
    Vi,
}

impl Edition {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Zh,
            Self::Cs,
            Self::Nl,
            Self::En,
            Self::Simple,
            Self::Fr,
            Self::De,
            Self::El,
            Self::Id,
            Self::It,
            Self::Ja,
            Self::Ko,
            Self::Ku,
            Self::Ms,
            Self::Pl,
            Self::Pt,
            Self::Ru,
            Self::Es,
            Self::Th,
            Self::Tr,
            Self::Vi,
        ]
    }
}

impl FromStr for Edition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zh" => Ok(Self::Zh),
            "cs" => Ok(Self::Cs),
            "nl" => Ok(Self::Nl),
            "en" => Ok(Self::En),
            "simple" => Ok(Self::Simple),
            "fr" => Ok(Self::Fr),
            "de" => Ok(Self::De),
            "el" => Ok(Self::El),
            "id" => Ok(Self::Id),
            "it" => Ok(Self::It),
            "ja" => Ok(Self::Ja),
            "ko" => Ok(Self::Ko),
            "ku" => Ok(Self::Ku),
            "ms" => Ok(Self::Ms),
            "pl" => Ok(Self::Pl),
            "pt" => Ok(Self::Pt),
            "ru" => Ok(Self::Ru),
            "es" => Ok(Self::Es),
            "th" => Ok(Self::Th),
            "tr" => Ok(Self::Tr),
            "vi" => Ok(Self::Vi),
            _ => Err(format!("invalid edition '{s}'")),
        }
    }
}

impl AsRef<str> for Edition {
    fn as_ref(&self) -> &str {
        match self {
            Self::Zh => "zh",
            Self::Cs => "cs",
            Self::Nl => "nl",
            Self::En => "en",
            Self::Simple => "simple",
            Self::Fr => "fr",
            Self::De => "de",
            Self::El => "el",
            Self::Id => "id",
            Self::It => "it",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Ku => "ku",
            Self::Ms => "ms",
            Self::Pl => "pl",
            Self::Pt => "pt",
            Self::Ru => "ru",
            Self::Es => "es",
            Self::Th => "th",
            Self::Tr => "tr",
            Self::Vi => "vi",
        }
    }
}

impl Edition {
    pub fn iso(&self) -> &str {
        match self {
            Self::Simple => "en",
            _ => self.as_ref(),
        }
    }
}

impl Display for Edition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

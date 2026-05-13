#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameRelease {
    SkyrimLe,
    SkyrimSe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Language {
    English,
    German,
    Italian,
    Spanish,
    SpanishMexico,
    French,
    Polish,
    PortugueseBrazil,
    Chinese,
    Russian,
    Japanese,
    Czech,
    Hungarian,
    Danish,
    Finnish,
    Greek,
    Norwegian,
    Swedish,
    Turkish,
    Arabic,
    Korean,
    Thai,
    ChineseSimplified,
}

impl Language {
    pub fn full_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::German => "German",
            Self::Italian => "Italian",
            Self::Spanish => "Spanish",
            Self::SpanishMexico => "Spanish_Mexico",
            Self::French => "French",
            Self::Polish => "Polish",
            Self::PortugueseBrazil => "Portuguese_Brazil",
            Self::Chinese => "Chinese",
            Self::Russian => "Russian",
            Self::Japanese => "Japanese",
            Self::Czech => "Czech",
            Self::Hungarian => "Hungarian",
            Self::Danish => "Danish",
            Self::Finnish => "Finnish",
            Self::Greek => "Greek",
            Self::Norwegian => "Norwegian",
            Self::Swedish => "Swedish",
            Self::Turkish => "Turkish",
            Self::Arabic => "Arabic",
            Self::Korean => "Korean",
            Self::Thai => "Thai",
            Self::ChineseSimplified => "ChineseSimplified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StringsKind {
    Normal,
    Dl,
    Il,
}

impl StringsKind {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Normal => "STRINGS",
            Self::Dl => "DLSTRINGS",
            Self::Il => "ILSTRINGS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LocalizedFieldSource {
    Normal,
    Dl,
    Il,
}

impl LocalizedFieldSource {
    pub fn strings_kind(self) -> StringsKind {
        match self {
            Self::Normal => StringsKind::Normal,
            Self::Dl => StringsKind::Dl,
            Self::Il => StringsKind::Il,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameRelease {
    SkyrimLe,
    SkyrimSe,
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

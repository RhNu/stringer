use crate::LocalizedFieldSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalizedField {
    pub major_record: &'static str,
    pub subrecord: &'static str,
    pub source: LocalizedFieldSource,
}

impl LocalizedField {
    pub fn strings_kind(self) -> crate::StringsKind {
        self.source.strings_kind()
    }
}

pub fn skyrim_localized_fields() -> &'static [LocalizedField] {
    SKYRIM_LOCALIZED_FIELDS
}

pub(crate) fn find_localized_field(
    major_record: &str,
    subrecord: &str,
) -> Option<&'static LocalizedField> {
    SKYRIM_LOCALIZED_FIELDS
        .iter()
        .find(|field| field.major_record == major_record && field.subrecord == subrecord)
}

const N: LocalizedFieldSource = LocalizedFieldSource::Normal;
const D: LocalizedFieldSource = LocalizedFieldSource::Dl;
const I: LocalizedFieldSource = LocalizedFieldSource::Il;

static SKYRIM_LOCALIZED_FIELDS: &[LocalizedField] = &[
    f("ACTI", "FULL", N),
    f("ACTI", "RNAM", N),
    f("AVIF", "FULL", N),
    f("AVIF", "DESC", D),
    f("APPA", "FULL", N),
    f("APPA", "DESC", D),
    f("AMMO", "FULL", N),
    f("AMMO", "DESC", D),
    f("ARMO", "FULL", N),
    f("ARMO", "DESC", D),
    f("BPTD", "BPTN", N),
    f("BOOK", "FULL", N),
    f("BOOK", "DESC", D),
    f("BOOK", "CNAM", D),
    f("CELL", "FULL", N),
    f("CLAS", "FULL", N),
    f("CLAS", "DESC", D),
    f("COLL", "DESC", D),
    f("CLFM", "FULL", N),
    f("CONT", "FULL", N),
    f("INFO", "RNAM", N),
    f("INFO", "NAM1", I),
    f("DIAL", "FULL", N),
    f("DOOR", "FULL", N),
    f("EXPL", "FULL", N),
    f("EYES", "FULL", N),
    f("FACT", "FULL", N),
    f("FACT", "MNAM", N),
    f("FLOR", "FULL", N),
    f("FLOR", "RNAM", N),
    f("FURN", "FULL", N),
    f("GMST", "DATA", N),
    f("HAZD", "FULL", N),
    f("HDPT", "FULL", N),
    f("ALCH", "FULL", N),
    f("ALCH", "DESC", D),
    f("INGR", "FULL", N),
    f("KEYM", "FULL", N),
    f("LIGH", "FULL", N),
    f("LSCR", "DESC", N),
    f("LCTN", "FULL", N),
    f("MGEF", "FULL", N),
    f("MGEF", "DNAM", N),
    f("MESG", "DESC", D),
    f("MESG", "FULL", N),
    f("MESG", "ITXT", N),
    f("MISC", "FULL", N),
    f("MSTT", "FULL", N),
    f("NPC_", "FULL", N),
    f("NPC_", "SHRT", N),
    f("ENCH", "FULL", N),
    f("PERK", "FULL", N),
    f("PERK", "DESC", D),
    f("PERK", "EPF2", N),
    f("PERK", "EPFT", N),
    f("REFR", "FULL", N),
    f("PROJ", "FULL", N),
    f("QUST", "FULL", N),
    f("QUST", "NNAM", D),
    f("QUST", "CNAM", D),
    f("QUST", "NNAM", N),
    f("RACE", "FULL", N),
    f("RACE", "DESC", D),
    f("REGN", "RDMP", N),
    f("SCRL", "FULL", N),
    f("SCRL", "DESC", D),
    f("SHOU", "FULL", N),
    f("SHOU", "DESC", D),
    f("SLGM", "FULL", N),
    f("SNCT", "FULL", N),
    f("SNDR", "FNAM", N),
    f("SPEL", "FULL", N),
    f("SPEL", "DESC", D),
    f("TACT", "FULL", N),
    f("TREE", "FULL", N),
    f("WATR", "FULL", N),
    f("WEAP", "FULL", N),
    f("WEAP", "DESC", D),
    f("WOOP", "FULL", N),
    f("WOOP", "TNAM", N),
    f("WRLD", "FULL", N),
];

const fn f(
    major_record: &'static str,
    subrecord: &'static str,
    source: LocalizedFieldSource,
) -> LocalizedField {
    LocalizedField {
        major_record,
        subrecord,
        source,
    }
}

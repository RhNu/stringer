use std::borrow::Cow;

use encoding_rs::{
    Encoding, SHIFT_JIS, UTF_8, WINDOWS_1250, WINDOWS_1251, WINDOWS_1252, WINDOWS_1253,
    WINDOWS_1254, WINDOWS_1256,
};

use crate::{GameRelease, Language, PluginError};

pub(crate) fn decode_text(
    release: GameRelease,
    language: Language,
    bytes: &[u8],
) -> Result<String, PluginError> {
    let choice = encoding_choice(release, language);
    if let Some(text) = decode_with(choice.primary, bytes) {
        return Ok(text);
    }
    if let Some(fallback) = choice.fallback
        && let Some(text) = decode_with(fallback, bytes)
    {
        return Ok(text);
    }
    Err(PluginError::Encoding {
        release: format!("{release:?}"),
        language: language.full_name().to_string(),
        message: "input bytes could not be decoded".to_string(),
    })
}

pub(crate) fn encode_text(
    release: GameRelease,
    language: Language,
    text: &str,
) -> Result<Vec<u8>, PluginError> {
    let choice = encoding_choice(release, language);
    if let Some(bytes) = encode_with(choice.primary, text) {
        return Ok(bytes);
    }
    if let Some(fallback) = choice.fallback
        && let Some(bytes) = encode_with(fallback, text)
    {
        return Ok(bytes);
    }
    Err(PluginError::Encoding {
        release: format!("{release:?}"),
        language: language.full_name().to_string(),
        message: "text contains characters unsupported by the selected encoding".to_string(),
    })
}

struct EncodingChoice {
    primary: &'static Encoding,
    fallback: Option<&'static Encoding>,
}

fn encoding_choice(release: GameRelease, language: Language) -> EncodingChoice {
    match release {
        GameRelease::SkyrimLe => EncodingChoice {
            primary: skyrim_le_encoding(language),
            fallback: None,
        },
        GameRelease::SkyrimSe => skyrim_se_encoding(language),
    }
}

fn skyrim_le_encoding(language: Language) -> &'static Encoding {
    match language {
        Language::Polish | Language::Hungarian | Language::Czech => WINDOWS_1250,
        Language::Russian => WINDOWS_1251,
        Language::English
        | Language::French
        | Language::German
        | Language::Spanish
        | Language::SpanishMexico
        | Language::Finnish
        | Language::Danish
        | Language::Norwegian
        | Language::Swedish
        | Language::PortugueseBrazil
        | Language::Italian => WINDOWS_1252,
        Language::Greek => WINDOWS_1253,
        Language::Turkish => WINDOWS_1254,
        Language::Arabic => WINDOWS_1256,
        Language::Korean
        | Language::Chinese
        | Language::ChineseSimplified
        | Language::Japanese
        | Language::Thai => UTF_8,
    }
}

fn skyrim_se_encoding(language: Language) -> EncodingChoice {
    match language {
        Language::Japanese => utf8_with_fallback(SHIFT_JIS),
        Language::Czech | Language::Hungarian | Language::Polish => {
            utf8_with_fallback(WINDOWS_1250)
        }
        Language::Russian => utf8_with_fallback(WINDOWS_1251),
        Language::English => EncodingChoice {
            primary: WINDOWS_1252,
            fallback: None,
        },
        Language::French
        | Language::German
        | Language::Italian
        | Language::Spanish
        | Language::SpanishMexico
        | Language::Danish
        | Language::Finnish
        | Language::Norwegian
        | Language::Swedish
        | Language::PortugueseBrazil => utf8_with_fallback(WINDOWS_1252),
        Language::Greek => utf8_with_fallback(WINDOWS_1253),
        Language::Turkish => utf8_with_fallback(WINDOWS_1254),
        Language::Arabic => utf8_with_fallback(WINDOWS_1256),
        Language::Chinese | Language::ChineseSimplified | Language::Korean | Language::Thai => {
            EncodingChoice {
                primary: UTF_8,
                fallback: None,
            }
        }
    }
}

fn utf8_with_fallback(fallback: &'static Encoding) -> EncodingChoice {
    EncodingChoice {
        primary: UTF_8,
        fallback: Some(fallback),
    }
}

fn decode_with(encoding: &'static Encoding, bytes: &[u8]) -> Option<String> {
    encoding
        .decode_without_bom_handling_and_without_replacement(bytes)
        .map(Cow::into_owned)
}

fn encode_with(encoding: &'static Encoding, text: &str) -> Option<Vec<u8>> {
    let (bytes, _, had_errors) = encoding.encode(text);
    (!had_errors).then(|| bytes.into_owned())
}

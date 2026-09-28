use shared_metadata::utils::Multilingual;

/// Returns the value for the first available language in the priority order: en -> de -> fr -> it.
/// Falls back to any available value if none of the preferred languages are present.
///
/// Display-only. For a value used as a lookup key, use
/// [`shared_metadata::multilingual_value`], which is deterministic.
pub fn lang_value(map: &Multilingual) -> Option<&String> {
    ["en", "de", "fr", "it"]
        .iter()
        .find_map(|lang| map.get(*lang))
        .or_else(|| map.values().next())
}

/// Maps a BCP 47 language code to a human-readable English display name.
pub fn language_display_name(code: &str) -> &str {
    match code {
        "ar" => "Arabic",
        "cop" => "Coptic",
        "cu" => "Old Church Slavonic",
        "de" => "German",
        "el" => "Greek",
        "en" => "English",
        "es" => "Spanish",
        "ewo" => "Ewondo",
        "fr" => "French",
        "gez" => "Ge'ez (Ethiopic)",
        "got" => "Gothic",
        "grc" => "Ancient Greek",
        "hy" => "Armenian",
        "it" => "Italian",
        "ka" => "Georgian",
        "la" => "Latin",
        "pez" => "Penan",
        "pt" => "Portuguese",
        "rm" => "Romansh",
        "ru" => "Russian",
        "sw" => "Swahili",
        "syr" => "Syriac",
        "tr" => "Turkish",
        "x-cpa" => "Christian Palestinian Aramaic",
        _ => code,
    }
}

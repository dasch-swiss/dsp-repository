use std::collections::HashMap;

/// Returns true if the value is a data placeholder ("MISSING" or "CALCULATED").
pub fn is_placeholder(value: &str) -> bool {
    value == "MISSING" || value == "CALCULATED"
}

/// Returns the value for the first available language in the priority order: en -> de -> fr -> it.
/// Falls back to any available value if none of the preferred languages are present.
pub fn lang_value(map: &HashMap<String, String>) -> Option<&String> {
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

#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;

#[cfg(not(target_arch = "wasm32"))]
static DATA_DIR: OnceLock<String> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
static SHOW_PLACEHOLDER_VALUES: OnceLock<bool> = OnceLock::new();

/// Set the data directory path at startup. Must be called before any data access.
/// Thread-safe: uses OnceLock (first call wins, subsequent calls are no-ops).
#[cfg(not(target_arch = "wasm32"))]
pub fn set_data_dir(path: &str) {
    if DATA_DIR.set(path.to_string()).is_err() {
        tracing::warn!(
            new = path,
            current = DATA_DIR.get().unwrap().as_str(),
            "set_data_dir called again but data dir is already set"
        );
    }
}

/// Get the data directory path.
///
/// Priority: OnceLock (set by main.rs) → DPE_DATA_DIR env var → DATA_DIR env var → development
/// default. Falls back to setting the OnceLock from env/default on first call.
#[cfg(not(target_arch = "wasm32"))]
pub fn get_data_dir() -> &'static str {
    DATA_DIR.get_or_init(|| {
        std::env::var("DPE_DATA_DIR")
            .or_else(|_| std::env::var("DATA_DIR"))
            .unwrap_or_else(|_| "modules/dpe/server/data".to_string())
    })
}

/// Set whether placeholder values ("MISSING", "CALCULATED") should be shown in the UI.
/// Thread-safe: uses OnceLock (first call wins, subsequent calls are no-ops).
#[cfg(not(target_arch = "wasm32"))]
pub fn set_show_placeholder_values(show: bool) {
    if SHOW_PLACEHOLDER_VALUES.set(show).is_err() {
        tracing::warn!(
            new = show,
            current = SHOW_PLACEHOLDER_VALUES.get().unwrap(),
            "set_show_placeholder_values called again but value is already set"
        );
    }
}

/// Whether placeholder values ("MISSING", "CALCULATED") should be shown in the UI.
///
/// Priority: OnceLock (set by main.rs) → DPE_SHOW_PLACEHOLDER_VALUES env var → false.
/// When true, placeholders are rendered with red styling for QA visibility.
/// When false (default/production), placeholders are hidden entirely.
#[cfg(not(target_arch = "wasm32"))]
pub fn show_placeholder_values() -> bool {
    *SHOW_PLACEHOLDER_VALUES.get_or_init(|| {
        std::env::var("DPE_SHOW_PLACEHOLDER_VALUES")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_placeholder_missing() {
        assert!(is_placeholder("MISSING"));
    }

    #[test]
    fn test_is_placeholder_calculated() {
        assert!(is_placeholder("CALCULATED"));
    }

    #[test]
    fn test_is_placeholder_normal_values() {
        assert!(!is_placeholder("CC BY-SA 4.0"));
        assert!(!is_placeholder("2021-09-02"));
        assert!(!is_placeholder("person-001"));
        assert!(!is_placeholder(""));
        assert!(!is_placeholder("missing")); // case-sensitive
        assert!(!is_placeholder("calculated")); // case-sensitive
    }
}

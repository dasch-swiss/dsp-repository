//! The data languages a project can declare: ISO 639 code and English name.
//! Includes `pez` and `x-cpa`, which the published corpus uses.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    pub code: &'static str,
    pub name: &'static str,
}

/// Sorted by name. Names are unique ignoring case so [`code_for_name`] is
/// unambiguous, which is why `x-cpa` carries a suffix.
pub const LANGUAGES: &[Language] = &[
    Language { code: "ab", name: "Abkhazian" },
    Language { code: "aa", name: "Afar" },
    Language { code: "af", name: "Afrikaans" },
    Language { code: "ak", name: "Akan" },
    Language { code: "sq", name: "Albanian" },
    Language { code: "am", name: "Amharic" },
    Language { code: "grc", name: "Ancient Greek" },
    Language { code: "ar", name: "Arabic" },
    Language { code: "an", name: "Aragonese" },
    Language { code: "hy", name: "Armenian" },
    Language { code: "as", name: "Assamese" },
    Language { code: "av", name: "Avaric" },
    Language { code: "ae", name: "Avestan" },
    Language { code: "ay", name: "Aymara" },
    Language { code: "az", name: "Azerbaijani" },
    Language { code: "bm", name: "Bambara" },
    Language { code: "ba", name: "Bashkir" },
    Language { code: "eu", name: "Basque" },
    Language { code: "be", name: "Belarusian" },
    Language { code: "bn", name: "Bengali" },
    Language { code: "bi", name: "Bislama" },
    Language { code: "bs", name: "Bosnian" },
    Language { code: "br", name: "Breton" },
    Language { code: "bg", name: "Bulgarian" },
    Language { code: "my", name: "Burmese" },
    Language { code: "ca", name: "Catalan" },
    Language { code: "ch", name: "Chamorro" },
    Language { code: "ce", name: "Chechen" },
    Language { code: "ny", name: "Chichewa" },
    Language { code: "zh", name: "Chinese" },
    Language { code: "cpg", name: "Christian Palestinian Aramaic" },
    Language {
        code: "x-cpa",
        name: "Christian Palestinian Aramaic (private use)",
    },
    Language { code: "cv", name: "Chuvash" },
    Language { code: "cop", name: "Coptic" },
    Language { code: "kw", name: "Cornish" },
    Language { code: "co", name: "Corsican" },
    Language { code: "cr", name: "Cree" },
    Language { code: "hr", name: "Croatian" },
    Language { code: "cs", name: "Czech" },
    Language { code: "da", name: "Danish" },
    Language { code: "dv", name: "Divehi" },
    Language { code: "nl", name: "Dutch" },
    Language { code: "dz", name: "Dzongkha" },
    Language { code: "pez", name: "Eastern Penan" },
    Language { code: "en", name: "English" },
    Language { code: "eo", name: "Esperanto" },
    Language { code: "et", name: "Estonian" },
    Language { code: "ee", name: "Ewe" },
    Language { code: "ewo", name: "Ewondo" },
    Language { code: "fo", name: "Faroese" },
    Language { code: "fj", name: "Fijian" },
    Language { code: "fi", name: "Finnish" },
    Language { code: "fr", name: "French" },
    Language { code: "ff", name: "Fulah" },
    Language { code: "gl", name: "Galician" },
    Language { code: "lg", name: "Ganda" },
    Language { code: "gez", name: "Ge'ez (Ethiopic)" },
    Language { code: "ka", name: "Georgian" },
    Language { code: "de", name: "German" },
    Language { code: "got", name: "Gothic" },
    Language { code: "el", name: "Greek" },
    Language { code: "gn", name: "Guarani" },
    Language { code: "gu", name: "Gujarati" },
    Language { code: "ht", name: "Haitian Creole" },
    Language { code: "ha", name: "Hausa" },
    Language { code: "he", name: "Hebrew" },
    Language { code: "hz", name: "Herero" },
    Language { code: "hi", name: "Hindi" },
    Language { code: "ho", name: "Hiri Motu" },
    Language { code: "hu", name: "Hungarian" },
    Language { code: "is", name: "Icelandic" },
    Language { code: "io", name: "Ido" },
    Language { code: "ig", name: "Igbo" },
    Language { code: "id", name: "Indonesian" },
    Language { code: "ia", name: "Interlingua" },
    Language { code: "ie", name: "Interlingue" },
    Language { code: "iu", name: "Inuktitut" },
    Language { code: "ik", name: "Inupiaq" },
    Language { code: "ga", name: "Irish" },
    Language { code: "it", name: "Italian" },
    Language { code: "ja", name: "Japanese" },
    Language { code: "jv", name: "Javanese" },
    Language { code: "kl", name: "Kalaallisut" },
    Language { code: "kn", name: "Kannada" },
    Language { code: "kr", name: "Kanuri" },
    Language { code: "ks", name: "Kashmiri" },
    Language { code: "kk", name: "Kazakh" },
    Language { code: "km", name: "Khmer" },
    Language { code: "ki", name: "Kikuyu" },
    Language { code: "rw", name: "Kinyarwanda" },
    Language { code: "kv", name: "Komi" },
    Language { code: "kg", name: "Kongo" },
    Language { code: "ko", name: "Korean" },
    Language { code: "kj", name: "Kuanyama" },
    Language { code: "ku", name: "Kurdish" },
    Language { code: "ky", name: "Kyrgyz" },
    Language { code: "lo", name: "Lao" },
    Language { code: "la", name: "Latin" },
    Language { code: "lv", name: "Latvian" },
    Language { code: "li", name: "Limburgish" },
    Language { code: "ln", name: "Lingala" },
    Language { code: "lt", name: "Lithuanian" },
    Language { code: "lu", name: "Luba-Katanga" },
    Language { code: "lb", name: "Luxembourgish" },
    Language { code: "mk", name: "Macedonian" },
    Language { code: "mg", name: "Malagasy" },
    Language { code: "ms", name: "Malay" },
    Language { code: "ml", name: "Malayalam" },
    Language { code: "mt", name: "Maltese" },
    Language { code: "gv", name: "Manx" },
    Language { code: "mi", name: "Maori" },
    Language { code: "mr", name: "Marathi" },
    Language { code: "mh", name: "Marshallese" },
    Language { code: "mn", name: "Mongolian" },
    Language { code: "na", name: "Nauru" },
    Language { code: "nv", name: "Navajo" },
    Language { code: "ng", name: "Ndonga" },
    Language { code: "ne", name: "Nepali" },
    Language { code: "nd", name: "North Ndebele" },
    Language { code: "se", name: "Northern Sami" },
    Language { code: "no", name: "Norwegian" },
    Language { code: "nb", name: "Norwegian Bokmål" },
    Language { code: "nn", name: "Norwegian Nynorsk" },
    Language { code: "ii", name: "Nuosu" },
    Language { code: "oc", name: "Occitan" },
    Language { code: "oj", name: "Ojibwe" },
    Language { code: "cu", name: "Old Church Slavonic" },
    Language { code: "or", name: "Oriya" },
    Language { code: "om", name: "Oromo" },
    Language { code: "os", name: "Ossetian" },
    Language { code: "pi", name: "Pali" },
    Language { code: "ps", name: "Pashto" },
    Language { code: "pne", name: "Penan" },
    Language { code: "fa", name: "Persian" },
    Language { code: "pl", name: "Polish" },
    Language { code: "pt", name: "Portuguese" },
    Language { code: "pa", name: "Punjabi" },
    Language { code: "qu", name: "Quechua" },
    Language { code: "ro", name: "Romanian" },
    Language { code: "rm", name: "Romansh" },
    Language { code: "rn", name: "Rundi" },
    Language { code: "ru", name: "Russian" },
    Language { code: "sm", name: "Samoan" },
    Language { code: "sg", name: "Sango" },
    Language { code: "sa", name: "Sanskrit" },
    Language { code: "sc", name: "Sardinian" },
    Language { code: "gd", name: "Scottish Gaelic" },
    Language { code: "sr", name: "Serbian" },
    Language { code: "sn", name: "Shona" },
    Language { code: "sd", name: "Sindhi" },
    Language { code: "si", name: "Sinhala" },
    Language { code: "sk", name: "Slovak" },
    Language { code: "sl", name: "Slovenian" },
    Language { code: "so", name: "Somali" },
    Language { code: "nr", name: "South Ndebele" },
    Language { code: "st", name: "Southern Sotho" },
    Language { code: "es", name: "Spanish" },
    Language { code: "su", name: "Sundanese" },
    Language { code: "sw", name: "Swahili" },
    Language { code: "ss", name: "Swati" },
    Language { code: "sv", name: "Swedish" },
    Language { code: "syr", name: "Syriac" },
    Language { code: "tl", name: "Tagalog" },
    Language { code: "ty", name: "Tahitian" },
    Language { code: "tg", name: "Tajik" },
    Language { code: "ta", name: "Tamil" },
    Language { code: "tt", name: "Tatar" },
    Language { code: "te", name: "Telugu" },
    Language { code: "th", name: "Thai" },
    Language { code: "bo", name: "Tibetan" },
    Language { code: "ti", name: "Tigrinya" },
    Language { code: "to", name: "Tonga" },
    Language { code: "ts", name: "Tsonga" },
    Language { code: "tn", name: "Tswana" },
    Language { code: "tr", name: "Turkish" },
    Language { code: "tk", name: "Turkmen" },
    Language { code: "tw", name: "Twi" },
    Language { code: "uk", name: "Ukrainian" },
    Language { code: "ur", name: "Urdu" },
    Language { code: "ug", name: "Uyghur" },
    Language { code: "uz", name: "Uzbek" },
    Language { code: "ve", name: "Venda" },
    Language { code: "vi", name: "Vietnamese" },
    Language { code: "vo", name: "Volapük" },
    Language { code: "wa", name: "Walloon" },
    Language { code: "cy", name: "Welsh" },
    Language { code: "fy", name: "Western Frisian" },
    Language { code: "wo", name: "Wolof" },
    Language { code: "xh", name: "Xhosa" },
    Language { code: "yi", name: "Yiddish" },
    Language { code: "yo", name: "Yoruba" },
    Language { code: "za", name: "Zhuang" },
    Language { code: "zu", name: "Zulu" },
];

fn find_by_code(code: &str) -> Option<&'static Language> {
    LANGUAGES.iter().find(|language| language.code.eq_ignore_ascii_case(code))
}

/// The listed code a typed code stands for, ignoring ASCII case. The one answer to
/// "is this value a listed code", so the held-value hint, storage and submit agree.
pub fn canonical_code(code: &str) -> Option<&'static str> {
    find_by_code(code).map(|language| language.code)
}

/// The English name of the language with this code, ignoring ASCII case.
pub fn name_for_code(code: &str) -> Option<&'static str> {
    find_by_code(code).map(|language| language.name)
}

/// The code of the language with this name, ignoring surrounding whitespace and case.
pub(crate) fn code_for_name(name: &str) -> Option<&'static str> {
    let name = name.trim().to_lowercase();
    LANGUAGES
        .iter()
        .find(|language| language.name.to_lowercase() == name)
        .map(|language| language.code)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;

    use super::*;
    use crate::checkout_dpe_data_dir;

    #[test]
    fn every_corpus_data_language_is_listed() {
        let projects = checkout_dpe_data_dir().join("projects");
        let mut checked = 0;
        for entry in fs::read_dir(&projects).expect("the committed corpus is readable") {
            let path = entry.expect("directory entry").path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let text = fs::read_to_string(&path).expect("project file is readable");
            let project: serde_json::Value = serde_json::from_str(&text).expect("project file is JSON");
            let codes = project.get("dataLanguage").and_then(|value| value.as_array());
            for code in codes.into_iter().flatten().filter_map(|value| value.as_str()) {
                assert!(
                    name_for_code(code).is_some(),
                    "dataLanguage code `{code}` in {} is not in LANGUAGES",
                    path.display()
                );
            }
            checked += 1;
        }
        assert!(checked > 0, "no project files found under {}", projects.display());
    }

    #[test]
    fn codes_and_names_are_unique() {
        let mut codes = HashSet::new();
        let mut names = HashSet::new();
        for language in LANGUAGES {
            assert!(codes.insert(language.code), "duplicate code `{}`", language.code);
            assert!(
                names.insert(language.name.to_lowercase()),
                "duplicate name `{}` (ignoring case)",
                language.name
            );
        }
    }

    #[test]
    fn the_list_is_sorted_by_name() {
        let names: Vec<String> = LANGUAGES.iter().map(|language| language.name.to_lowercase()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn code_lookups_ignore_case() {
        assert_eq!(canonical_code("HU"), Some("hu"));
        assert_eq!(canonical_code("X-CPA"), Some("x-cpa"));
        assert_eq!(name_for_code("En"), Some("English"));
        assert_eq!(canonical_code("hungarian"), None);
    }

    #[test]
    fn name_lookup_trims_and_ignores_case() {
        assert_eq!(code_for_name(" hungarian "), Some("hu"));
        assert_eq!(code_for_name("Belgian"), None);
        assert_eq!(code_for_name("NORWEGIAN BOKMÅL"), Some("nb"));
    }
}

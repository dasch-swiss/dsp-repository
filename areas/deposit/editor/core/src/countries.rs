//! The countries and Swiss cantons an organisation's address offers, by English name.
//!
//! The contract types both as free text, so a stored value outside these lists
//! is legitimate and stays selectable where the form renders it.

/// The one country whose address carries a canton.
pub const SWITZERLAND: &str = "Switzerland";

/// Whether an address in `country` carries a canton. Ignores case and
/// surrounding whitespace, since a stored country is free text. The form's
/// client-side mirror is the `_swiss` change listener in `editor-web`'s
/// `entity/mod.rs` `address_group`; change both together.
pub fn takes_canton(country: &str) -> bool {
    country.trim().eq_ignore_ascii_case(SWITZERLAND)
}

/// Sorted by name, one spelling each. The short common English form, not the
/// ISO long form (`Tanzania`, `United States`), because that is how committed
/// organisations spell them.
pub const COUNTRIES: &[&str] = &[
    "Afghanistan",
    "Albania",
    "Algeria",
    "Andorra",
    "Angola",
    "Antigua and Barbuda",
    "Argentina",
    "Armenia",
    "Aruba",
    "Australia",
    "Austria",
    "Azerbaijan",
    "Bahamas",
    "Bahrain",
    "Bangladesh",
    "Barbados",
    "Belarus",
    "Belgium",
    "Belize",
    "Benin",
    "Bermuda",
    "Bhutan",
    "Bolivia",
    "Bosnia and Herzegovina",
    "Botswana",
    "Brazil",
    "Brunei",
    "Bulgaria",
    "Burkina Faso",
    "Burundi",
    "Cambodia",
    "Cameroon",
    "Canada",
    "Cape Verde",
    "Central African Republic",
    "Chad",
    "Chile",
    "China",
    "Colombia",
    "Comoros",
    "Congo",
    "Costa Rica",
    "Côte d'Ivoire",
    "Croatia",
    "Cuba",
    "Cyprus",
    "Czechia",
    "Democratic Republic of the Congo",
    "Denmark",
    "Djibouti",
    "Dominica",
    "Dominican Republic",
    "Ecuador",
    "Egypt",
    "El Salvador",
    "Equatorial Guinea",
    "Eritrea",
    "Estonia",
    "Eswatini",
    "Ethiopia",
    "Faroe Islands",
    "Fiji",
    "Finland",
    "France",
    "Gabon",
    "Gambia",
    "Georgia",
    "Germany",
    "Ghana",
    "Gibraltar",
    "Greece",
    "Greenland",
    "Grenada",
    "Guatemala",
    "Guernsey",
    "Guinea",
    "Guinea-Bissau",
    "Guyana",
    "Haiti",
    "Honduras",
    "Hong Kong",
    "Hungary",
    "Iceland",
    "India",
    "Indonesia",
    "Iran",
    "Iraq",
    "Ireland",
    "Isle of Man",
    "Israel",
    "Italy",
    "Jamaica",
    "Japan",
    "Jersey",
    "Jordan",
    "Kazakhstan",
    "Kenya",
    "Kiribati",
    "Kosovo",
    "Kuwait",
    "Kyrgyzstan",
    "Laos",
    "Latvia",
    "Lebanon",
    "Lesotho",
    "Liberia",
    "Libya",
    "Liechtenstein",
    "Lithuania",
    "Luxembourg",
    "Macao",
    "Madagascar",
    "Malawi",
    "Malaysia",
    "Maldives",
    "Mali",
    "Malta",
    "Marshall Islands",
    "Mauritania",
    "Mauritius",
    "Mexico",
    "Micronesia",
    "Moldova",
    "Monaco",
    "Mongolia",
    "Montenegro",
    "Morocco",
    "Mozambique",
    "Myanmar",
    "Namibia",
    "Nauru",
    "Nepal",
    "Netherlands",
    "New Zealand",
    "Nicaragua",
    "Niger",
    "Nigeria",
    "North Korea",
    "North Macedonia",
    "Norway",
    "Oman",
    "Pakistan",
    "Palau",
    "Palestine",
    "Panama",
    "Papua New Guinea",
    "Paraguay",
    "Peru",
    "Philippines",
    "Poland",
    "Portugal",
    "Puerto Rico",
    "Qatar",
    "Romania",
    "Russia",
    "Rwanda",
    "Saint Kitts and Nevis",
    "Saint Lucia",
    "Saint Vincent and the Grenadines",
    "Samoa",
    "San Marino",
    "Sao Tome and Principe",
    "Saudi Arabia",
    "Senegal",
    "Serbia",
    "Seychelles",
    "Sierra Leone",
    "Singapore",
    "Slovakia",
    "Slovenia",
    "Solomon Islands",
    "Somalia",
    "South Africa",
    "South Korea",
    "South Sudan",
    "Spain",
    "Sri Lanka",
    "Sudan",
    "Suriname",
    "Sweden",
    "Switzerland",
    "Syria",
    "Taiwan",
    "Tajikistan",
    "Tanzania",
    "Thailand",
    "Timor-Leste",
    "Togo",
    "Tonga",
    "Trinidad and Tobago",
    "Tunisia",
    "Turkey",
    "Turkmenistan",
    "Tuvalu",
    "Uganda",
    "Ukraine",
    "United Arab Emirates",
    "United Kingdom",
    "United States",
    "Uruguay",
    "Uzbekistan",
    "Vanuatu",
    "Vatican City",
    "Venezuela",
    "Vietnam",
    "Yemen",
    "Zambia",
    "Zimbabwe",
];

/// The 26 Swiss cantons, sorted by name.
pub const CANTONS: &[&str] = &[
    "Aargau",
    "Appenzell Ausserrhoden",
    "Appenzell Innerrhoden",
    "Basel-Landschaft",
    "Basel-Stadt",
    "Bern",
    "Fribourg",
    "Geneva",
    "Glarus",
    "Grisons",
    "Jura",
    "Lucerne",
    "Neuchâtel",
    "Nidwalden",
    "Obwalden",
    "Schaffhausen",
    "Schwyz",
    "Solothurn",
    "St. Gallen",
    "Thurgau",
    "Ticino",
    "Uri",
    "Valais",
    "Vaud",
    "Zug",
    "Zurich",
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::checkout_dpe_data_dir;

    #[test]
    fn every_committed_organisation_country_is_listed() {
        let dir = checkout_dpe_data_dir().join("organizations");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).expect("the committed organisations read") {
            let path = entry.expect("directory entry reads").path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("an organisation file reads");
            let organisation: serde_json::Value = serde_json::from_str(&text).expect("an organisation file parses");
            if let Some(country) = organisation.pointer("/address/country").and_then(serde_json::Value::as_str) {
                assert!(
                    COUNTRIES.contains(&country),
                    "country `{country}` in {} is not in COUNTRIES",
                    path.display()
                );
                checked += 1;
            }
        }
        assert!(checked > 0, "no organisation with a country under {}", dir.display());
    }

    #[test]
    fn the_lists_hold_each_name_once() {
        for list in [COUNTRIES, CANTONS] {
            let mut seen = HashSet::new();
            for name in list {
                assert!(seen.insert(name.to_lowercase()), "duplicate `{name}` (ignoring case)");
            }
        }
        assert_eq!(CANTONS.len(), 26);
        assert!(COUNTRIES.contains(&SWITZERLAND));
    }

    #[test]
    fn only_switzerland_takes_a_canton() {
        for country in ["Switzerland", " switzerland "] {
            assert!(takes_canton(country), "{country:?}");
        }
        for country in ["Germany", "", "CH"] {
            assert!(!takes_canton(country), "{country:?}");
        }
    }
}

//! A multilingual value while it is being edited.
//!
//! The contract's [`Multilingual`] is a `BTreeMap`, alphabetical and silent
//! about the order a form should show. This is the editing view over one:
//! order-preserving, so tags stay where the depositor sees them across a save
//! and reload, and open, so a tag outside the four the UI offers is kept. No
//! serde derives: a draft stores fields as `serde_json` values, so this is
//! built on demand and written back.

use shared_metadata::utils::Multilingual;

/// The language tags the form offers, in the order it offers them.
///
/// Not a closed set: `ar`, `cop` and `grc` are live in the data, and any tag
/// present is kept and editable.
pub const UI_LANGUAGES: [&str; 4] = ["de", "en", "fr", "it"];

/// Language tag to text, in editing order.
///
/// A `Vec` rather than a map because the order is the point. Tags are unique:
/// [`Self::set`] replaces in place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DraftMultilingual {
    entries: Vec<(String, String)>,
}

impl DraftMultilingual {
    /// An empty value, for a field the depositor has not filled in.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds the editing view over a contract value.
    ///
    /// [`UI_LANGUAGES`] come first, in UI order and only where present, so the
    /// form's own fields appear in a stable order regardless of what the file
    /// held. Any other tag follows in the contract value's own order, which is
    /// alphabetical.
    #[must_use]
    pub fn from_contract(map: &Multilingual) -> Self {
        let mut entries: Vec<(String, String)> = UI_LANGUAGES
            .iter()
            .filter_map(|tag| map.get(*tag).map(|text| ((*tag).to_string(), text.clone())))
            .collect();
        entries.extend(
            map.iter()
                .filter(|(tag, _)| !UI_LANGUAGES.contains(&tag.as_str()))
                .map(|(tag, text)| (tag.clone(), text.clone())),
        );
        Self { entries }
    }

    /// The contract value: alphabetical by construction, since [`Multilingual`]
    /// is a `BTreeMap`, with empty texts dropped.
    ///
    /// An empty string is an editing state, not a value: `"en": ""` in a file
    /// makes DPE's `lang_value` render a blank in place of the German the
    /// project still has.
    #[must_use]
    pub fn to_contract(&self) -> Multilingual {
        self.entries.iter().filter(|(_, text)| !text.is_empty()).cloned().collect()
    }

    /// The text for one tag, if set.
    #[must_use]
    pub fn get(&self, tag: &str) -> Option<&str> {
        self.entries.iter().find(|(t, _)| t == tag).map(|(_, text)| text.as_str())
    }

    /// Sets one tag's text, replacing in place or appending, so an edit never
    /// reorders the form.
    ///
    /// An empty `text` is stored, so a tag does not vanish from under the cursor
    /// mid-edit; [`Self::to_contract`] is where it goes away, and
    /// [`Self::remove`] drops one deliberately.
    pub fn set(&mut self, tag: &str, text: impl Into<String>) {
        let text = text.into();
        match self.entries.iter_mut().find(|(t, _)| t == tag) {
            Some(entry) => entry.1 = text,
            None => self.entries.push((tag.to_string(), text)),
        }
    }

    /// Drops one tag. Returns whether it was there.
    pub fn remove(&mut self, tag: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|(t, _)| t != tag);
        self.entries.len() != before
    }

    /// The tags and texts, in editing order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(tag, text)| (tag.as_str(), text.as_str()))
    }

    /// Tags present but outside [`UI_LANGUAGES`], in editing order. The form
    /// renders these as existing rows so they are visible and editable rather
    /// than invisibly carried.
    pub fn extra_tags(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .map(|(tag, _)| tag.as_str())
            .filter(|tag| !UI_LANGUAGES.contains(tag))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(pairs: &[(&str, &str)]) -> Multilingual {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    #[test]
    fn from_contract_orders_ui_languages_first_regardless_of_alphabet() {
        // Alphabetically `en` precedes `fr` precedes `it`, and `de` precedes all
        // three; the UI order happens to match there, so use `it` and `de` to
        // show the ordering is the UI's and not the map's.
        let view = DraftMultilingual::from_contract(&contract(&[("it", "Ciao"), ("de", "Hallo")]));
        assert_eq!(view.iter().map(|(t, _)| t).collect::<Vec<_>>(), ["de", "it"]);
    }

    /// `ar` is live in two committed files. It must survive the round trip and
    /// be reported as an extra tag so the form can render it.
    #[test]
    fn from_contract_retains_a_tag_the_ui_does_not_offer() {
        let view = DraftMultilingual::from_contract(&contract(&[("en", "Hello"), ("ar", "مرحبا")]));
        assert_eq!(view.get("ar"), Some("مرحبا"));
        assert_eq!(view.extra_tags().collect::<Vec<_>>(), ["ar"]);
        assert_eq!(view.to_contract(), contract(&[("en", "Hello"), ("ar", "مرحبا")]));
    }

    #[test]
    fn extra_tags_follow_the_ui_languages() {
        let view = DraftMultilingual::from_contract(&contract(&[("ar", "a"), ("en", "e"), ("cop", "c")]));
        assert_eq!(view.iter().map(|(t, _)| t).collect::<Vec<_>>(), ["en", "ar", "cop"]);
    }

    #[test]
    fn set_replaces_in_place_and_appends_new_tags_at_the_end() {
        let mut view = DraftMultilingual::from_contract(&contract(&[("de", "Hallo"), ("en", "Hello")]));
        view.set("de", "Guten Tag");
        view.set("rm", "Bun di");
        assert_eq!(
            view.iter().collect::<Vec<_>>(),
            [("de", "Guten Tag"), ("en", "Hello"), ("rm", "Bun di")]
        );
    }

    #[test]
    fn set_keeps_an_emptied_tag_so_the_form_row_survives() {
        let mut view = DraftMultilingual::from_contract(&contract(&[("en", "Hello")]));
        view.set("en", "");
        assert_eq!(view.get("en"), Some(""));
        assert!(!view.is_empty());
    }

    /// The row survives editing, but an empty string must not reach the file:
    /// `"en": ""` would make DPE render a blank value instead of falling back to
    /// a language the project still has.
    #[test]
    fn to_contract_drops_an_emptied_tag() {
        let mut view = DraftMultilingual::from_contract(&contract(&[("de", "Hallo"), ("en", "Hello")]));
        view.set("en", "");
        assert_eq!(view.to_contract(), contract(&[("de", "Hallo")]));
    }

    #[test]
    fn to_contract_is_empty_when_every_tag_was_cleared() {
        let mut view = DraftMultilingual::from_contract(&contract(&[("en", "Hello")]));
        view.set("en", "");
        assert!(view.to_contract().is_empty());
    }

    #[test]
    fn remove_drops_a_tag_and_reports_whether_it_was_there() {
        let mut view = DraftMultilingual::from_contract(&contract(&[("en", "Hello")]));
        assert!(view.remove("en"));
        assert!(!view.remove("en"));
        assert!(view.is_empty());
    }

    #[test]
    fn to_contract_is_alphabetical_whatever_the_editing_order() {
        let mut view = DraftMultilingual::new();
        view.set("it", "Ciao");
        view.set("ar", "مرحبا");
        view.set("de", "Hallo");
        assert_eq!(view.iter().map(|(t, _)| t).collect::<Vec<_>>(), ["it", "ar", "de"]);
        assert_eq!(view.to_contract().keys().collect::<Vec<_>>(), ["ar", "de", "it"]);
    }
}

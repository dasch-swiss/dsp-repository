//! Reading a posted form body into a draft.
//!
//! The record form is a plain `<form>` submitted with `contentType: 'form'`, so
//! what arrives is `application/x-www-form-urlencoded` and **no signals**
//! (Datastar transmits none on a form-content-type request, and `ReadSignals` is
//! JSON-only and cannot read a form body).
//!
//! ## Why the body is a list of pairs and not a struct
//!
//! `axum::Form` deserializes with `serde_urlencoded` 0.7, which this workspace
//! pins through axum 0.8. That parser cannot express what this form needs, and
//! not by a small margin — measured against the pinned version:
//!
//! | body | target | result |
//! |---|---|---|
//! | `row=a&row=b` | `Vec<String>` | `Err` — "invalid type: string, expected a sequence" |
//! | `row=a` | `Vec<String>` | `Err` — same |
//! | `name=a&name=b` | `String` | `Err` — "duplicate field `name`" |
//! | `row=a&row=b` | `Vec<(String, String)>` | **`Ok`**, in order, duplicates kept |
//!
//! So a repeated key — a checkbox group, a repeatable list's row keys — is an
//! *error* rather than a `Vec`, and a struct with a `Vec` field cannot deserialize
//! at all. (`serde_html_form`, which does decode repeated keys into a `Vec`, is
//! what the plan assumed `axum::Form` used; it is not in this tree.)
//!
//! The last row is the way through: `Form<Vec<(String, String)>>` gives every
//! pair in body order with duplicates intact, which is exactly what opaque row
//! keys plus DOM order need — and it adds no second urlencoded parser with its
//! own edge cases beside the one axum already uses.
//!
//! ## What this module is not
//!
//! It has no idea which field is a text field and which is a language map.
//! [`FormBody`] reads names and values; the appliers below each know one
//! *shape*. Choosing a shape per field is the field registry's job, in
//! `editor-web`, keyed by the same field ids the renderers are — so a field's
//! control and its decoder are declared together and cannot drift. That also
//! keeps the audience check (which fields a depositor may write at all) in one
//! place rather than duplicated here.
//!
//! ## Absent is not empty
//!
//! A name missing from the body means "this section did not carry that field",
//! and every applier leaves such a field alone. A name present with an empty
//! value means "the depositor cleared it". The distinction is load-bearing
//! because a section only posts its own fields, and treating absent as cleared
//! would have saving one section wipe every other.
//!
//! Two HTML shapes submit nothing when empty and therefore need a same-named
//! hidden marker beside them, or a clear cannot be expressed: an unchecked
//! checkbox group, and a repeatable list with no rows. [`FormBody::all`] returns
//! the marker along with the values, and the appliers drop empties.
//!
//! ## Saving without changing anything must change nothing
//!
//! A depositor opens a section to read it and presses save. The file has to come
//! back byte-identical, and three separate things stand in the way — all three
//! found by `editor-web/tests/untouched_form_round_trip.rs`, which puts the form
//! in the middle of the corpus round-trip. It is in the *sibling* crate because
//! it derives which field takes which shape from the field registry, and the
//! registry lives in `editor-web`:
//!
//! - **A newline arrives as CRLF on the no-JavaScript path.** The urlencoded serializer normalises
//!   line breaks, so a `textarea` holding `"a\nb"` posts `a%0D%0Ab` on a native submit — verified
//!   in Chromium, WebKit and Firefox. Datastar's own path does *not*: `new FormData(form)`
//!   preserves `\n` in all three. So the two paths disagree byte-for-byte on 26 of the 85 committed
//!   files, and only one of them is wrong. [`normalise_newlines`] settles it on `\n`, which is what
//!   the files hold — and it is applied to both sides of the comparison but only to a value being
//!   stored, because a bare `\r` (which 10 committed abstracts hold) cannot survive a `<textarea>`
//!   in any engine.
//! - **Trimming rewrites values nobody edited.** A value that differs from the stored one *only* in
//!   surrounding whitespace is one the depositor did not change, so the stored bytes are kept; a
//!   genuinely new value is stored trimmed. 20 of the 85 committed files carry such a space
//!   somewhere, and every field taken over from here brings more of them into range.
//! - **A stored placeholder renders as an empty control**, so an untouched form posts an empty
//!   value for it — see [`apply_text`].

use std::collections::HashSet;

use platform_metadata::is_placeholder;
use serde_json::Value;

use crate::draft::ProjectDraft;
use crate::multilingual::DraftMultilingual;

/// The longest a language tag may be, and the longest an opaque row key may be.
///
/// Both arrive inside a field *name*, which the server then uses to build map
/// keys, so an unbounded one is a way to put arbitrary bulk into a stored draft.
/// Twelve characters holds every tag in the data (`en`, `de`, `ar`, `cop`,
/// `grc`) with room for a subtag, and every key this service mints.
const MAX_NAME_SEGMENT: usize = 12;

/// The most suffixed values [`FormBody::entries`] will read under one prefix —
/// in practice, the most language tags one field may carry.
///
/// A bound on work, not a product rule: `DraftMultilingual` is an
/// order-preserving `Vec`, so its `get` and `set` scan, which is right for a map
/// the data holds two entries of and ruinous for one holding twenty thousand.
/// Sixteen times the four tags the UI offers, so no real submission reaches it
/// and the excess is dropped rather than refused — refusing needs a field-level
/// error, which arrives with submit validation.
const MAX_VALUES_PER_PREFIX: usize = 64;

/// A posted form body: every name/value pair, in the order the browser sent
/// them, duplicates intact.
///
/// Every reader here is **linear in the number of pairs**, which is a property
/// to keep: nothing bounds how many arrive under one name but Axum's 2 MB body
/// limit, so an `O(n²)` reader turns one request into billions of comparisons.
/// Hence the `HashSet` beside each de-duplicating result, and hence
/// [`Self::entries`] returning values *with* their suffixes rather than leaving
/// the caller to fetch each with [`Self::get`].
///
/// A *product* cap — at most so many keywords, refused with a field-level error
/// — is a different thing and belongs with the route. This is only the bound on
/// work.
#[derive(Debug, Default, Clone)]
pub struct FormBody {
    pairs: Vec<(String, String)>,
}

impl FormBody {
    /// Wrap what `Form<Vec<(String, String)>>` extracted.
    #[must_use]
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Self {
        Self { pairs }
    }

    /// The first value posted under `name`, or `None` when the name is absent.
    ///
    /// First rather than last: no browser sends a scalar field twice, so a
    /// repeat is either a bug in our own markup or a hand-built request, and
    /// picking deterministically without an error path is enough for both. Which
    /// end is picked matters for neither — the whole body is the client's, so
    /// there is nothing an appended second value could override that the first
    /// could not have said.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.pairs.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    /// Every value posted under `name`, in body order.
    ///
    /// The name is copied into the closure rather than borrowed, so a caller can
    /// pass a name it built on the spot — `format!("{field}.row")` — without the
    /// temporary having to outlive the iterator.
    pub fn all<'a>(&'a self, name: &str) -> impl Iterator<Item = &'a str> + 'a {
        let name = name.to_string();
        self.pairs
            .iter()
            .filter(move |(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Whether `name` was posted at all, whatever its value.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.pairs.iter().any(|(key, _)| key == name)
    }

    /// The row keys of a repeatable field, in DOM order, as the
    /// `{field}.row` hidden inputs carried them.
    ///
    /// Order comes from the repetition, never from a number: an index in a name
    /// goes stale the moment a middle row is removed, and per-row errors keyed
    /// by index then point at the wrong rows.
    ///
    /// Keys that are empty, over-long or not `[A-Za-z0-9_-]+` are dropped: a row
    /// key becomes part of a field name the server reads back, and the ones this
    /// service mints are none of those. Duplicates are dropped too, keeping the
    /// first — two rows with one key would collapse into one wherever the key is
    /// used as a map key, which is a silent loss of a row.
    #[must_use]
    pub fn rows(&self, field: &str) -> Vec<&str> {
        let name = format!("{field}.row");
        let mut keys: Vec<&str> = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();
        for key in self.all(&name) {
            if is_valid_segment(key) && seen.insert(key) {
                keys.push(key);
            }
        }
        keys
    }

    /// The suffixed values posted under `{prefix}.<suffix>`, in body order,
    /// without duplicates.
    ///
    /// How a multilingual field's tags are discovered: the form renders whatever
    /// tags the value had plus the four it offers, so the body is the only thing
    /// that knows which are present. A suffix containing a further `.` is
    /// skipped — that is a deeper path this shape does not own.
    ///
    /// **The value comes back with the suffix, in one pass**, because returning
    /// the suffixes alone and fetching each with [`Self::get`] is quadratic in
    /// the number of pairs — see [`MAX_VALUES_PER_PREFIX`] for the other half of
    /// the bound. At most that many are returned.
    ///
    /// First occurrence wins, matching [`Self::get`].
    #[must_use]
    pub fn entries(&self, prefix: &str) -> Vec<(&str, &str)> {
        let prefix = format!("{prefix}.");
        let mut found: Vec<(&str, &str)> = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();
        for (key, value) in &self.pairs {
            let Some(suffix) = key.strip_prefix(&prefix) else {
                continue;
            };
            if suffix.contains('.') || !is_valid_segment(suffix) || !seen.insert(suffix) {
                continue;
            }
            found.push((suffix, value.as_str()));
            if found.len() == MAX_VALUES_PER_PREFIX {
                break;
            }
        }
        found
    }
}

/// A name segment the server will use as a map key: a language tag or a row key.
///
/// Must start with an ASCII alphanumeric. Without that, `-` and `_` on their own
/// pass — and neither is a language tag or a row key this service mints, so
/// accepting one only lets a hand-built body put a junk key into a stored
/// language map, from where it rides out into a published file.
fn is_valid_segment(segment: &str) -> bool {
    segment.len() <= MAX_NAME_SEGMENT
        && segment.starts_with(|c: char| c.is_ascii_alphanumeric())
        && segment.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A submitted value, once compared against what is stored.
#[derive(Debug, PartialEq, Eq)]
enum Resolved {
    /// The submitted value differs from the stored one only in surrounding
    /// whitespace or in how a newline was encoded. The depositor did not change
    /// this field, so the stored bytes stay exactly as they are.
    Unchanged,
    /// Nothing but whitespace was submitted: the field was cleared.
    Cleared,
    /// A new value, trimmed.
    Value(String),
}

/// Line breaks as the files hold them.
///
/// A form body encodes a newline as CRLF on a native submit (the urlencoded
/// serializer normalises them; confirmed in Chromium, WebKit and Firefox), while
/// `new FormData(form)` — Datastar's path — preserves whatever the control held.
/// Without this the two paths write different bytes for the same untouched
/// value, in 26 of the 85 committed files.
///
/// A lone CR is normalised too: it is not a line break any of these files use,
/// and leaving it would put a bare control character in published JSON.
fn normalise_newlines(value: &str) -> String {
    if !value.contains('\r') {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    out
}

/// Decide what a submitted value means for a field currently holding `stored`.
///
/// **Both sides are normalised before the comparison, and only the comparison.**
/// A stored value is kept byte-for-byte when it matches; a genuinely new one is
/// stored normalised. That asymmetry is what preserves the committed corpus: a
/// bare `\r` is not representable in a `<textarea>` at all — its value
/// sanitisation turns one into `\n` before any submit, in Chromium, WebKit and
/// Firefox alike — and 10 committed abstracts hold one. Normalising for storage
/// as well would rewrite all 10 on the first save of an unrelated field, and
/// there is no rendering of those bytes that could come back unchanged.
fn resolve(submitted: &str, stored: Option<&str>) -> Resolved {
    let normalised = normalise_newlines(submitted);
    let trimmed = normalised.trim();
    if trimmed.is_empty() {
        return Resolved::Cleared;
    }
    if stored.is_some_and(|stored| normalise_newlines(stored).trim() == trimmed) {
        return Resolved::Unchanged;
    }
    Resolved::Value(trimmed.to_string())
}

/// What a cleared scalar field becomes.
///
/// The contract types some fields as a plain `String` and others as an
/// `Option<String>`, and the two have different empty states. Dropping a
/// required `String` leaves a draft that cannot be published until the field is
/// filled in again — right for a field that genuinely has to be there, wrong for
/// one whose absence the data already spells another way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhenCleared {
    /// Drop the member. The right answer for an `Option` field: absent is what
    /// unset means.
    Drop,
    /// Write the `MISSING` placeholder.
    ///
    /// For a field the contract types as a required `String` whose empty state
    /// the committed data spells that way — `endDate` on an ongoing project is
    /// `"MISSING"` in 24 of the 85 files. Dropping it instead would make every
    /// ongoing project unpublishable until an end date it does not have is
    /// entered.
    ///
    /// Which field gets which is declared once, on `editor_web::form::registry`'s
    /// `Field`, as part of its [`Shape`] — never passed per call.
    Placeholder,
}

/// The sentinel [`WhenCleared::Placeholder`] writes, and the one a stored value
/// is recognised by.
const MISSING: &str = "MISSING";

/// How the form reads one field back out of a posted body.
///
/// One arm per applier, so naming a shape is the only way to reach one. The
/// field registry gives every field it takes over exactly one, which is what
/// keeps a field's control and its decoder from drifting.
///
/// [`WhenCleared`] rides inside [`Self::Text`] rather than beside it because it
/// is meaningful for nothing else — a language map's empty state is "no tags".
/// Beside it, a `Multilingual` field could declare one and a `Text` field none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A scalar string member, read by [`apply_text`]. The [`WhenCleared`] is
    /// the contract's own distinction: a required `String` whose empty state the
    /// data spells as a sentinel takes [`WhenCleared::Placeholder`], an
    /// `Option<String>` takes [`WhenCleared::Drop`].
    Text(WhenCleared),
    /// A language map, from `{field}.<tag>` pairs. Read by
    /// [`apply_multilingual`].
    Multilingual,
}

/// Apply one field, whichever shape it is.
///
/// The only door out of this crate — the appliers are `pub(crate)` — so a
/// handler cannot pick a `WhenCleared` the registry does not declare. That
/// mistake (`Drop` on a required `String`) leaves every ongoing project
/// unpublishable with nothing failing.
pub fn apply(shape: Shape, body: &FormBody, draft: &mut ProjectDraft, field: &str) {
    match shape {
        Shape::Text(when_cleared) => apply_text(body, draft, field, when_cleared),
        Shape::Multilingual => apply_multilingual(body, draft, field),
    }
}

/// Apply a submitted scalar to `field`.
///
/// Absent from the body leaves the field alone (see the module docs). The value
/// is trimmed, because a form posts what was typed and a trailing space is not a
/// value anybody meant.
///
/// **A stored placeholder survives an empty submit.** `MISSING` and `CALCULATED`
/// are the platform's "no value yet" sentinels, filtered out of DPE's UI and of
/// OAI-PMH output, so a control holding one renders empty — which means an
/// untouched form posts an empty value for it. Writing `""` back would change a
/// published field nobody edited: 131 sentinels across 8 paths in the 85
/// committed files, 24 of them `endDate`. This is the one rule that keeps a save
/// in an unrelated section from rewriting them.
pub(crate) fn apply_text(body: &FormBody, draft: &mut ProjectDraft, field: &str, when_cleared: WhenCleared) {
    let Some(submitted) = body.get(field) else { return };
    let stored = draft.get(field).and_then(Value::as_str).map(str::to_string);
    match resolve(submitted, stored.as_deref()) {
        Resolved::Unchanged => {}
        Resolved::Value(value) => draft.set(field, Value::String(value)),
        Resolved::Cleared => {
            // A stored sentinel is already the empty state, so leave it.
            if stored.as_deref().is_some_and(is_placeholder) {
                return;
            }
            match when_cleared {
                WhenCleared::Drop => {
                    draft.remove(field);
                }
                WhenCleared::Placeholder => draft.set(field, Value::String(MISSING.to_string())),
            }
        }
    }
}

/// Apply a submitted language map to `field`, from `{field}.<tag>` pairs.
///
/// The tags come from the body rather than from a fixed list, so a tag outside
/// the four the UI offers is kept: `ar` is live in two committed files, and a
/// closed set would drop it on the first save. Editing order is preserved
/// here; the canonical writer sorts on the way out.
///
/// An empty text drops its tag — [`DraftMultilingual::to_contract`] does that
/// too, so `"en": ""` never reaches a file, where DPE's language fallback would
/// render a blank description in place of the German the project still has. A
/// map left entirely empty removes the field.
pub(crate) fn apply_multilingual(body: &FormBody, draft: &mut ProjectDraft, field: &str) {
    let posted = body.entries(field);
    if posted.is_empty() {
        return;
    }
    let stored = draft.multilingual(field);
    let mut value = DraftMultilingual::new();
    for (tag, text) in posted {
        // Folded, because the tag becomes a key in the stored map and in the
        // published file. `description.EN` beside `description.en` would
        // otherwise write both, and BCP 47 treats them as one language — every
        // tag in the committed corpus is lowercase, so folding changes no
        // existing value. Two case variants collapse to one entry, body order
        // deciding which text wins.
        let tag = &tag.to_ascii_lowercase();
        let stored_text = stored.get(tag);
        match resolve(text, stored_text) {
            // Only non-empty texts reach the value. `DraftMultilingual::set`
            // deliberately keeps an empty one — that is the editing view, which
            // must not drop a tag from under the cursor — but a map of nothing
            // but empty texts is not "empty" to `set_multilingual`, so it would
            // be stored as `{}`.
            Resolved::Cleared => {}
            // `Unchanged` is only returned when there *is* a stored text, so the
            // `if let` never falls through. Written this way rather than with an
            // `unwrap_or_default` so that if it ever did, the tag keeps what it
            // had instead of being replaced by an empty string.
            Resolved::Unchanged => {
                if let Some(kept) = stored_text {
                    value.set(tag, kept);
                }
            }
            Resolved::Value(text) => value.set(tag, text),
        }
    }
    draft.set_multilingual(field, &value);
}

/// Apply a submitted list of strings to `field`, from repeated `{field}` pairs.
///
/// The shape a checkbox group and a plain repeated input both post. Empty values
/// are dropped, which is what makes the hidden marker work: a group with nothing
/// checked posts only the marker, so the field arrives present and empty rather
/// than absent, and is cleared rather than left alone.
///
/// Order is the body's. Duplicates are dropped, keeping the first: two checked
/// controls with one value is a rendering bug, and writing the value twice would
/// put it in the file twice.
/// No [`Shape`] arm names this yet: no field declares that shape until the
/// repeatable and checkbox-group widgets land. The attribute goes with the arm.
#[allow(dead_code)]
pub(crate) fn apply_string_list(body: &FormBody, draft: &mut ProjectDraft, field: &str) {
    if !body.has(field) {
        return;
    }
    let mut values: Vec<String> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for value in body.all(field) {
        let value = value.trim();
        if !value.is_empty() && seen.insert(value) {
            values.push(value.to_string());
        }
    }
    if values.is_empty() {
        draft.remove(field);
    } else {
        draft.set(field, Value::Array(values.into_iter().map(Value::String).collect()));
    }
}

/// Apply a submitted list of language maps to `field` — `keywords` and
/// `alternativeNames`.
///
/// Rows come from `{field}.row` in DOM order and each row's texts from
/// `{field}.<key>.<tag>`. A row whose every language is empty is dropped rather
/// than written as `{}`: an added-but-unfilled row is an editing state, and a
/// file full of empty objects is not.
///
/// Absent `{field}.row` leaves the field alone. Present with no valid key —
/// which is what the hidden marker of an empty list posts — clears it.
/// No [`Shape`] arm names this yet: no field declares that shape until the
/// repeatable and checkbox-group widgets land. The attribute goes with the arm.
#[allow(dead_code)]
pub(crate) fn apply_multilingual_rows(body: &FormBody, draft: &mut ProjectDraft, field: &str) {
    let row_name = format!("{field}.row");
    if !body.has(&row_name) {
        return;
    }
    let mut rows: Vec<Value> = Vec::new();
    for key in body.rows(field) {
        let prefix = format!("{field}.{key}");
        let mut value = DraftMultilingual::new();
        for (tag, text) in body.entries(&prefix) {
            // No stored counterpart to compare against: a row's identity is its
            // opaque key, and the stored list is positional, so there is nothing
            // here to preserve bytes from. The submitted text is the value.
            // Tag folded for the same reason as in `apply_multilingual`.
            if let Resolved::Value(text) = resolve(text, None) {
                value.set(&tag.to_ascii_lowercase(), text);
            }
        }
        let contract = value.to_contract();
        if contract.is_empty() {
            continue;
        }
        if let Ok(row) = serde_json::to_value(&contract) {
            rows.push(row);
        }
    }
    if rows.is_empty() {
        draft.remove(field);
    } else {
        draft.set(field, Value::Array(rows));
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::test_support::sample_raw;

    /// A body from `name=value` pairs written the way a browser would send them.
    fn body(pairs: &[(&str, &str)]) -> FormBody {
        FormBody::from_pairs(
            pairs
                .iter()
                .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                .collect(),
        )
    }

    fn draft() -> ProjectDraft {
        ProjectDraft::from_raw(&sample_raw())
    }

    // --- FormBody ---------------------------------------------------------

    #[test]
    fn a_repeated_name_keeps_every_value_in_body_order() {
        // The whole reason the body is a pair list: `serde_urlencoded` errors on
        // this shape, both into a `Vec` and into a scalar.
        let body = body(&[("typeOfData", "Text"), ("typeOfData", "Image")]);
        assert_eq!(body.all("typeOfData").collect::<Vec<_>>(), ["Text", "Image"]);
    }

    #[test]
    fn get_returns_the_first_value_and_none_for_an_absent_name() {
        let body = body(&[("name", "first"), ("name", "second")]);
        assert_eq!(body.get("name"), Some("first"));
        assert_eq!(body.get("nope"), None);
    }

    #[test]
    fn has_distinguishes_present_and_empty_from_absent() {
        // The distinction every applier turns on: a section posts its own fields,
        // so absent means "not this section's" and empty means "cleared".
        let body = body(&[("provenance", "")]);
        assert!(body.has("provenance"));
        assert_eq!(body.get("provenance"), Some(""));
        assert!(!body.has("abstract"));
    }

    #[test]
    fn rows_are_the_row_keys_in_dom_order() {
        let body = body(&[
            ("keywords.row", "k7f3"),
            ("keywords.k7f3.en", "manuscripts"),
            ("keywords.row", "k2b9"),
            ("keywords.k2b9.en", "palaeography"),
        ]);
        assert_eq!(body.rows("keywords"), ["k7f3", "k2b9"]);
    }

    #[test]
    fn a_row_key_that_could_not_have_been_minted_here_is_dropped() {
        // A row key becomes part of a field name the server reads back and a
        // key in a stored draft, so an unbounded or punctuated one is a way to
        // put arbitrary bulk and arbitrary keys into the payload.
        let long = "x".repeat(MAX_NAME_SEGMENT + 1);
        let body = body(&[
            ("keywords.row", "k1"),
            ("keywords.row", ""),
            ("keywords.row", "has space"),
            ("keywords.row", "dots.are.paths"),
            ("keywords.row", &long),
            ("keywords.row", "k2"),
        ]);
        assert_eq!(body.rows("keywords"), ["k1", "k2"]);
    }

    #[test]
    fn a_duplicate_row_key_is_kept_once() {
        // Two rows under one key collapse wherever the key is a map key, which
        // silently loses a row.
        let body = body(&[("keywords.row", "k1"), ("keywords.row", "k1")]);
        assert_eq!(body.rows("keywords"), ["k1"]);
    }

    #[test]
    fn entries_finds_the_language_tags_a_field_actually_carries_with_their_texts() {
        // The tags come from the body because the form renders whatever the
        // value had, plus the four the UI offers. The text comes back with the
        // tag so the caller never has to scan the body again per tag.
        let body = body(&[
            ("description.de", "Beschreibung"),
            ("description.en", "Description"),
            ("description.ar", "وصف"),
            ("descriptionOther.en", "not this field"),
        ]);
        assert_eq!(
            body.entries("description"),
            [("de", "Beschreibung"), ("en", "Description"), ("ar", "وصف")]
        );
    }

    #[test]
    fn entries_skips_a_deeper_path_and_a_segment_that_is_no_tag() {
        let long = "x".repeat(MAX_NAME_SEGMENT + 1);
        let body = body(&[
            ("keywords.k1.en", "deeper"),
            ("keywords.-", "punctuation only"),
            ("keywords.", "empty"),
            (&format!("keywords.{long}"), "over-long"),
            ("keywords.en", "shallow"),
        ]);
        // A segment that could not be a language tag is dropped rather than
        // becoming a key in a stored map, and from there in a published file.
        assert_eq!(body.entries("keywords"), [("en", "shallow")]);
        // The deeper path is not a tag of `keywords`; it is a tag of
        // `keywords.k1`.
        assert_eq!(body.entries("keywords.k1"), [("en", "deeper")]);
    }

    #[test]
    fn entries_keeps_the_first_of_a_repeated_suffix_matching_get() {
        // No browser posts one name twice, so a repeat is our own markup or a
        // hand-built request; picking the same end as `get` is what keeps the
        // two readers from disagreeing about one body.
        let body = body(&[("description.en", "first"), ("description.en", "second")]);
        assert_eq!(body.entries("description"), [("en", "first")]);
        assert_eq!(body.get("description.en"), Some("first"));
    }

    #[test]
    fn entries_stops_at_the_cap_so_one_body_cannot_buy_unbounded_work() {
        // The bound on work, at a size a single request can reach: Axum's 2 MB
        // body limit holds roughly 100,000 short pairs, and the readers behind
        // this scan a `Vec` per value. 20,000 tags under one prefix measured
        // 2.6 s of CPU in a debug build before the cap.
        let pairs: Vec<(String, String)> = (0..20_000)
            .map(|i| (format!("description.t{i:x}"), format!("text {i}")))
            .collect();
        let body = FormBody::from_pairs(pairs);
        assert_eq!(body.entries("description").len(), MAX_VALUES_PER_PREFIX);
    }

    #[test]
    fn a_body_within_the_cap_keeps_every_tag() {
        // The cap must not be reachable by anything real: the UI offers four
        // tags and no committed field carries more than two.
        let below = MAX_VALUES_PER_PREFIX - 1;
        let pairs: Vec<(String, String)> = (0..below)
            .map(|i| (format!("description.t{i:x}"), format!("text {i}")))
            .collect();
        let body = FormBody::from_pairs(pairs);
        let entries = body.entries("description");
        assert_eq!(entries.len(), below);
        assert_eq!(entries[0], ("t0", "text 0"));
        assert_eq!(entries[below - 1].1, format!("text {}", below - 1));
    }

    // --- apply_text -------------------------------------------------------

    #[test]
    fn a_submitted_value_that_differs_from_the_stored_one_is_stored_trimmed() {
        let mut draft = draft();
        apply_text(&body(&[("name", "  A Project  ")]), &mut draft, "name", WhenCleared::Drop);
        assert_eq!(draft.get("name"), Some(&json!("A Project")));
    }

    #[test]
    fn a_field_absent_from_the_body_is_left_alone() {
        // Saving one section must not wipe every other, and a section posts only
        // its own fields.
        let mut draft = draft();
        let before = draft.get("name").cloned();
        apply_text(&body(&[("abstract.en", "x")]), &mut draft, "name", WhenCleared::Drop);
        assert_eq!(draft.get("name").cloned(), before);
    }

    #[test]
    fn clearing_an_optional_field_drops_it() {
        let mut draft = draft();
        assert!(draft.get("dataManagementPlan").is_some(), "the fixture sets a DMP link");
        apply_text(
            &body(&[("dataManagementPlan", "")]),
            &mut draft,
            "dataManagementPlan",
            WhenCleared::Drop,
        );
        assert!(draft.get("dataManagementPlan").is_none());
    }

    #[test]
    fn clearing_a_required_string_writes_the_placeholder_rather_than_dropping_it() {
        // `endDate` is a required `String`; dropping it would make an ongoing
        // project unpublishable until an end date it does not have is entered.
        let mut draft = draft();
        apply_text(&body(&[("endDate", "")]), &mut draft, "endDate", WhenCleared::Placeholder);
        assert_eq!(draft.get("endDate"), Some(&json!("MISSING")));
        assert!(draft.to_raw().is_ok(), "a placeholder end date is still publishable");
    }

    #[test]
    fn a_stored_placeholder_survives_an_empty_submit_unchanged() {
        // The failure this prevents: a control holding `MISSING` renders empty,
        // so an untouched form posts an empty value for it, and writing `""`
        // back changes a published field nobody edited — in 24 of the 85
        // committed files for `endDate` alone.
        for when_cleared in [WhenCleared::Drop, WhenCleared::Placeholder] {
            let mut draft = draft();
            draft.set("endDate", json!("MISSING"));
            apply_text(&body(&[("endDate", "")]), &mut draft, "endDate", when_cleared);
            assert_eq!(draft.get("endDate"), Some(&json!("MISSING")), "{when_cleared:?}");
        }
    }

    #[test]
    fn the_other_placeholder_sentinel_survives_too() {
        // `CALCULATED` is the more common of the two in the corpus (88 of 131).
        let mut draft = draft();
        draft.set("howToCite", json!("CALCULATED"));
        apply_text(&body(&[("howToCite", "")]), &mut draft, "howToCite", WhenCleared::Drop);
        assert_eq!(draft.get("howToCite"), Some(&json!("CALCULATED")));
    }

    #[test]
    fn a_real_value_replaces_a_stored_placeholder() {
        // The sentinel is the empty state, not a lock: filling the field in is
        // the point.
        let mut draft = draft();
        draft.set("endDate", json!("MISSING"));
        apply_text(
            &body(&[("endDate", "2026-03-31")]),
            &mut draft,
            "endDate",
            WhenCleared::Placeholder,
        );
        assert_eq!(draft.get("endDate"), Some(&json!("2026-03-31")));
    }

    #[test]
    fn a_crlf_newline_is_stored_as_the_lf_the_files_hold() {
        // The no-JavaScript path posts CRLF: the urlencoded serializer
        // normalises line breaks, confirmed in Chromium, WebKit and Firefox.
        // Datastar's own path posts LF. Both have to store the same bytes.
        let mut draft = draft();
        apply_text(
            &body(&[("provenance", "first\r\nsecond")]),
            &mut draft,
            "provenance",
            WhenCleared::Drop,
        );
        assert_eq!(draft.get("provenance"), Some(&json!("first\nsecond")));
    }

    #[test]
    fn a_lone_cr_is_normalised_too_rather_than_left_in_published_json() {
        let mut draft = draft();
        apply_text(&body(&[("provenance", "a\rb")]), &mut draft, "provenance", WhenCleared::Drop);
        assert_eq!(draft.get("provenance"), Some(&json!("a\nb")));
    }

    #[test]
    fn a_value_differing_only_in_its_newline_encoding_leaves_the_stored_bytes_alone() {
        // 10 committed abstracts hold a bare `\r`, which a `<textarea>` cannot
        // represent — its value sanitisation turns one into `\n` before any
        // submit. Storing the normalised form would rewrite all 10 on the first
        // save of an unrelated field.
        let mut draft = draft();
        draft.set("provenance", json!("first\r second"));
        apply_text(
            &body(&[("provenance", "first\n second")]),
            &mut draft,
            "provenance",
            WhenCleared::Drop,
        );
        assert_eq!(
            draft.get("provenance"),
            Some(&json!("first\r second")),
            "the stored bytes should survive a value the depositor did not change"
        );
    }

    #[test]
    fn a_value_differing_only_in_surrounding_whitespace_leaves_the_stored_bytes_alone() {
        // Four committed files carry a leading or trailing space in a field the
        // form owns, and a control posts it back verbatim.
        let mut draft = draft();
        draft.set("provenance", json!("Digitised from slides "));
        apply_text(
            &body(&[("provenance", "Digitised from slides ")]),
            &mut draft,
            "provenance",
            WhenCleared::Drop,
        );
        assert_eq!(draft.get("provenance"), Some(&json!("Digitised from slides ")));
    }

    #[test]
    fn a_genuinely_new_value_is_stored_trimmed() {
        // The other half: a value the depositor did type is stored tidily.
        let mut draft = draft();
        draft.set("provenance", json!("old"));
        apply_text(&body(&[("provenance", "  new  ")]), &mut draft, "provenance", WhenCleared::Drop);
        assert_eq!(draft.get("provenance"), Some(&json!("new")));
    }

    // --- apply_multilingual ----------------------------------------------

    #[test]
    fn a_language_map_is_read_from_its_tag_suffixes() {
        let mut draft = draft();
        let posted = body(&[("description.de", "Beschreibung"), ("description.en", "Description")]);
        apply_multilingual(&posted, &mut draft, "description");
        let value = draft.multilingual("description");
        assert_eq!(value.get("de"), Some("Beschreibung"));
        assert_eq!(value.get("en"), Some("Description"));
    }

    #[test]
    fn a_tag_outside_the_four_the_ui_offers_is_kept() {
        // `ar` is live in two committed files; a closed language set would drop
        // it on the first save.
        let mut draft = draft();
        apply_multilingual(&body(&[("description.ar", "وصف")]), &mut draft, "description");
        assert_eq!(draft.multilingual("description").get("ar"), Some("وصف"));
    }

    #[test]
    fn an_empty_language_drops_its_tag_rather_than_storing_an_empty_string() {
        // `"en": ""` in a file makes DPE's language fallback render a blank
        // description in place of the German the project still has.
        let mut draft = draft();
        let posted = body(&[("description.de", "Beschreibung"), ("description.en", "  ")]);
        apply_multilingual(&posted, &mut draft, "description");
        let stored = draft.get("description").expect("description").clone();
        assert_eq!(stored, json!({"de": "Beschreibung"}));
    }

    #[test]
    fn a_wholly_empty_language_map_removes_the_field() {
        let mut draft = draft();
        apply_multilingual(
            &body(&[("description.de", ""), ("description.en", "")]),
            &mut draft,
            "description",
        );
        assert!(draft.get("description").is_none());
    }

    #[test]
    fn a_language_map_absent_from_the_body_is_left_alone() {
        let mut draft = draft();
        let before = draft.get("description").cloned();
        apply_multilingual(&body(&[("name", "x")]), &mut draft, "description");
        assert_eq!(draft.get("description").cloned(), before);
    }

    #[test]
    fn two_case_variants_of_one_language_tag_collapse_to_one_entry() {
        // A tag becomes a key in the stored map and in the published file, and
        // BCP 47 treats `EN` and `en` as one language. Writing both would put a
        // duplicate language in a project file.
        let mut draft = draft();
        let posted = body(&[("description.EN", "First"), ("description.en", "Second")]);
        apply_multilingual(&posted, &mut draft, "description");
        assert_eq!(draft.get("description"), Some(&json!({"en": "Second"})));
    }

    #[test]
    fn a_folded_tag_still_matches_its_stored_counterpart() {
        // The unchanged-value rule compares against the stored tag, which is
        // lowercase, so an upper-cased submission of the same text must still
        // read as unchanged rather than as an edit.
        let mut draft = draft();
        draft.set("description", json!({"en": "Kept verbatim "}));
        apply_multilingual(&body(&[("description.EN", "Kept verbatim ")]), &mut draft, "description");
        assert_eq!(draft.get("description"), Some(&json!({"en": "Kept verbatim "})));
    }

    // --- apply_string_list -----------------------------------------------

    #[test]
    fn a_checkbox_group_becomes_a_list_in_body_order() {
        let mut draft = draft();
        let posted = body(&[("typeOfData", ""), ("typeOfData", "Image"), ("typeOfData", "Text")]);
        apply_string_list(&posted, &mut draft, "typeOfData");
        assert_eq!(draft.get("typeOfData"), Some(&json!(["Image", "Text"])));
    }

    #[test]
    fn a_group_with_nothing_checked_clears_the_field_via_its_marker() {
        // An unchecked checkbox submits nothing, so without the hidden marker
        // the field would be absent and therefore left alone — a field that
        // cannot be cleared.
        let mut draft = draft();
        draft.set("typeOfData", json!(["Text"]));
        apply_string_list(&body(&[("typeOfData", "")]), &mut draft, "typeOfData");
        assert!(draft.get("typeOfData").is_none());
    }

    #[test]
    fn a_group_absent_from_the_body_entirely_is_left_alone() {
        let mut draft = draft();
        draft.set("typeOfData", json!(["Text"]));
        apply_string_list(&body(&[("name", "x")]), &mut draft, "typeOfData");
        assert_eq!(draft.get("typeOfData"), Some(&json!(["Text"])));
    }

    #[test]
    fn a_duplicate_value_is_stored_once() {
        let mut draft = draft();
        let posted = body(&[("typeOfData", "Text"), ("typeOfData", "Text")]);
        apply_string_list(&posted, &mut draft, "typeOfData");
        assert_eq!(draft.get("typeOfData"), Some(&json!(["Text"])));
    }

    #[test]
    fn a_row_s_language_tag_is_folded_too() {
        let mut draft = draft();
        let posted = body(&[("keywords.row", "k1"), ("keywords.k1.DE", "Handschriften")]);
        apply_multilingual_rows(&posted, &mut draft, "keywords");
        assert_eq!(draft.get("keywords"), Some(&json!([{"de": "Handschriften"}])));
    }

    // --- apply_multilingual_rows -----------------------------------------

    #[test]
    fn rows_of_language_maps_are_read_in_dom_order() {
        let mut draft = draft();
        let posted = body(&[
            ("keywords.row", "k7f3"),
            ("keywords.k7f3.en", "manuscripts"),
            ("keywords.k7f3.de", "Handschriften"),
            ("keywords.row", "k2b9"),
            ("keywords.k2b9.en", "palaeography"),
        ]);
        apply_multilingual_rows(&posted, &mut draft, "keywords");
        assert_eq!(
            draft.get("keywords"),
            Some(&json!([
                {"de": "Handschriften", "en": "manuscripts"},
                {"en": "palaeography"},
            ]))
        );
    }

    #[test]
    fn removing_a_middle_row_does_not_disturb_the_others() {
        // The index trap, from the other side: with opaque keys the surviving
        // rows keep their own values whatever went from between them.
        let mut draft = draft();
        let posted = body(&[
            ("keywords.row", "k1"),
            ("keywords.k1.en", "first"),
            ("keywords.row", "k3"),
            ("keywords.k3.en", "third"),
            // k2's fields are still in the body — a removed row's inputs are
            // gone from the DOM, but a stale one must not resurrect it either.
            ("keywords.k2.en", "second"),
        ]);
        apply_multilingual_rows(&posted, &mut draft, "keywords");
        assert_eq!(
            draft.get("keywords"),
            Some(&json!([{"en": "first"}, {"en": "third"}])),
            "only rows named by a row key are kept"
        );
    }

    #[test]
    fn an_added_but_unfilled_row_is_not_written_as_an_empty_object() {
        let mut draft = draft();
        let posted = body(&[
            ("keywords.row", "k1"),
            ("keywords.k1.en", "manuscripts"),
            ("keywords.row", "k2"),
            ("keywords.k2.en", ""),
            ("keywords.k2.de", "  "),
        ]);
        apply_multilingual_rows(&posted, &mut draft, "keywords");
        assert_eq!(draft.get("keywords"), Some(&json!([{"en": "manuscripts"}])));
    }

    #[test]
    fn a_list_emptied_of_rows_clears_the_field() {
        let mut draft = draft();
        apply_multilingual_rows(&body(&[("keywords.row", "")]), &mut draft, "keywords");
        assert!(draft.get("keywords").is_none());
    }

    #[test]
    fn a_row_list_absent_from_the_body_is_left_alone() {
        let mut draft = draft();
        let before = draft.get("keywords").cloned();
        apply_multilingual_rows(&body(&[("name", "x")]), &mut draft, "keywords");
        assert_eq!(draft.get("keywords").cloned(), before);
    }

    #[test]
    fn a_row_field_named_after_an_unlisted_key_is_ignored() {
        // Row keys are the server's; a body naming one the form did not render
        // must not create a row.
        let mut draft = draft();
        let posted = body(&[
            ("keywords.row", "k1"),
            ("keywords.k1.en", "kept"),
            ("keywords.zz.en", "injected"),
        ]);
        apply_multilingual_rows(&posted, &mut draft, "keywords");
        assert_eq!(draft.get("keywords"), Some(&json!([{"en": "kept"}])));
    }

    // --- the shapes together ---------------------------------------------

    #[test]
    fn a_draft_nobody_edited_survives_a_full_round_trip_unchanged() {
        // The property that matters most: opening a section and saving it
        // without typing anything must not change the project. Every applier
        // has an "absent is not empty" branch and a placeholder branch, and this
        // is the case where all of them have to agree.
        let original = draft();
        let mut draft = original.clone();
        let posted = body(&[
            ("name", "A Test Project"),
            ("endDate", ""),
            ("dataManagementPlan", "https://doi.org/10.5281/zenodo.7038186"),
        ]);
        draft.set("endDate", json!("MISSING"));
        let with_placeholder = draft.clone();
        apply_text(&posted, &mut draft, "name", WhenCleared::Drop);
        apply_text(&posted, &mut draft, "endDate", WhenCleared::Placeholder);
        apply_text(&posted, &mut draft, "dataManagementPlan", WhenCleared::Drop);
        // `dataManagementPlan` was submitted exactly as it already stood.
        assert_eq!(draft.get("dataManagementPlan"), original.get("dataManagementPlan"));
        assert_eq!(draft.get("endDate"), with_placeholder.get("endDate"));
        assert_eq!(draft.get("name"), Some(&json!("A Test Project")));
        assert!(original.get("name").is_some());
    }
}

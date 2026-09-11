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
use platform_metadata::utils::Multilingual;
use serde_json::{Map, Value};

use crate::draft::{ProjectDraft, UrlSlot};
use crate::multilingual::DraftMultilingual;

/// The longest a language tag may be, and the longest an opaque row key may be.
///
/// Both arrive inside a field *name*, which the server then uses to build map
/// keys, so an unbounded one is a way to put arbitrary bulk into a stored draft.
/// Twelve characters holds every tag in the data (`en`, `de`, `ar`, `cop`,
/// `grc`) with room for a subtag, and every key this service mints.
const MAX_NAME_SEGMENT: usize = 12;

/// The most values one field may carry, whatever shape it is: language tags under a prefix, or
/// repeated values under one name.
///
/// One number, deliberately: it is both the bound on work — `DraftMultilingual` is an
/// order-preserving `Vec` whose `get` and `set` scan, so an unbounded map is a way to spend a
/// request's worth of CPU — and the cap a depositor is shown. A visible cap set *above* the work
/// bound would silently drop every value between the two, which is the failure a visible cap exists
/// to remove.
///
/// The route refuses a body over it ([`FormBody::exceeds_entries`]) before any applier runs, so the
/// truncation in [`FormBody::entries`] is a fail-safe floor rather than the behaviour: unreachable
/// through the route, kept because this type is public and a caller might not check first.
///
/// `submit::tests::the_cap_is_above_anything_the_committed_corpus_holds` keeps it clear of the
/// corpus, so a corpus that grows past it fails there rather than refusing a depositor.
pub const MAX_VALUES_PER_FIELD: usize = 250;

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

    /// The posted pairs with one row of `field_id` dropped, ready for [`Self::from_pairs`].
    ///
    /// **The `{field}.row` marker survives removing the last row.** With no marker in the body an
    /// applier reads the field as absent and leaves the stored value alone, so removing the only
    /// row would not stick — an empty marker is what says "this field has zero rows now" rather
    /// than "this field was not posted".
    ///
    /// Here rather than in a handler because both write surfaces need it: the project form's
    /// section rows and the entity form's. Two copies of a rule this subtle can drift silently, and
    /// the failure is a removal that appears to work and does not.
    #[must_use]
    pub fn pairs_without_row(pairs: Vec<(String, String)>, field_id: &str, key: &str) -> Vec<(String, String)> {
        let row_name = format!("{field_id}.row");
        let mut kept: Vec<(String, String)> = pairs
            .into_iter()
            .filter(|(name, value)| name != &row_name || value != key)
            .collect();
        if !kept.iter().any(|(name, _)| name == &row_name) {
            kept.push((row_name, String::new()));
        }
        kept
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

    /// Whether more than `cap` values arrived under `name` itself.
    ///
    /// The repeated-name counterpart of [`Self::exceeds_entries`], for a
    /// checkbox group or a plain repeated input. Counts every pair rather than
    /// distinct values, because a body carrying a thousand copies of one value
    /// is the same amount of work to read whatever it stores.
    #[must_use]
    pub fn exceeds_all(&self, name: &str, cap: usize) -> bool {
        self.pairs.iter().filter(|(key, _)| key == name).nth(cap).is_some()
    }

    /// Whether more than `cap` distinct suffixes arrived under `{prefix}.`.
    ///
    /// The counting half of the visible cap, and it cannot be done with
    /// [`Self::entries`] — that stops at [`MAX_VALUES_PER_FIELD`], so a body
    /// carrying twenty thousand tags and one carrying sixty-four are
    /// indistinguishable through it. Stops as soon as the answer is known, so
    /// the work stays linear in the pairs and bounded by `cap` in memory
    /// however large the body is.
    #[must_use]
    pub fn exceeds_entries(&self, prefix: &str, cap: usize) -> bool {
        let prefix = format!("{prefix}.");
        let mut seen: HashSet<&str> = HashSet::new();
        for (key, _) in &self.pairs {
            let Some(suffix) = key.strip_prefix(&prefix) else {
                continue;
            };
            if suffix.contains('.') || !is_valid_segment(suffix) {
                continue;
            }
            if seen.insert(suffix) && seen.len() > cap {
                return true;
            }
        }
        false
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
    /// the number of pairs — see [`MAX_VALUES_PER_FIELD`] for the other half of
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
            if found.len() == MAX_VALUES_PER_FIELD {
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

/// Whether a list field accepts a value the form did not offer.
///
/// A closed set is the safer default and the wrong answer for most of these
/// fields: closing `dataLanguage` to the four tags the UI offers would drop
/// `la` from ten committed projects on their first save, silently, because a
/// value with no control posts nothing and a list rebuilt from the body would
/// not carry it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceSet {
    /// Only these values are stored; anything else is dropped.
    Closed(&'static [&'static str]),
    /// These are what the form offers, and any other value is kept.
    ///
    /// The slice is the *offer*, not the accepted set — the widget unions it
    /// with whatever the project already holds, so an unusual tag keeps its
    /// control instead of vanishing on the next save.
    Open(&'static [&'static str]),
}

impl ChoiceSet {
    /// The values the form offers for this set.
    #[must_use]
    pub const fn offered(self) -> &'static [&'static str] {
        match self {
            Self::Closed(values) | Self::Open(values) => values,
        }
    }

    /// Whether a submitted value may be stored.
    #[must_use]
    pub fn accepts(self, value: &str) -> bool {
        match self {
            Self::Closed(values) => values.contains(&value),
            Self::Open(_) => true,
        }
    }
}

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
    /// One of the project's two URLs, read by [`apply_url`].
    ///
    /// A slot rather than a member, because *where* the pair is stored depends on the project's
    /// vintage: some keep it positionally in a `url` array, some keep the secondary in its own
    /// member, and one project holds both. The slot is the only difference between the two
    /// fields, and they are applied independently because they belong to different audiences:
    /// `url` is RDU-only and `secondaryUrl` is a depositor's.
    Url(UrlSlot),
    /// A list of strings from repeated `{field}` values, read by
    /// [`apply_string_list`].
    ///
    /// The [`ChoiceSet`] decides whether a value the form did not offer is kept — a contract
    /// question, not a UI one: `typeOfData` is a closed vocabulary, while `dataLanguage` is
    /// open because the corpus holds far more tags than the UI offers.
    StringList(ChoiceSet),
    /// A list of authority references, read by [`apply_reference_rows`] -
    /// `spatialCoverage`.
    ///
    /// The reference half of [`Self::TextOrReferenceRows`] and **not** that
    /// shape with an unused branch: the contract types this member as
    /// `Vec<AuthorityFileReference>`, so a free-text row would not deserialize at all.
    ReferenceRows(&'static [&'static str]),
    /// A list of bibliographic references, read by [`apply_publication_rows`] -
    /// `publications`.
    ///
    /// A row is a citation plus an optional persistent identifier. Every committed identifier is a
    /// `url`; the `text` the contract allows beside it is unused, so the form does not offer it
    /// rather than adding a control nothing fills.
    PublicationRows,
    /// Funding, read by [`apply_funding`] - either a list of grants or one free
    /// text.
    ///
    /// The discriminant is on the **field**, not per row, which is what makes this its own shape
    /// rather than another [`Self::TextOrReferenceRows`]: a project holds either a list of grants
    /// or one free text, never a mix.
    FundingRows,
    /// A list of rows that are each **either** an authority reference **or** free
    /// text per language, read by [`apply_text_or_reference_rows`] -
    /// `temporalCoverage` and `disciplines`.
    ///
    /// The slice is the reference *types* the field offers, which differ per field.
    ///
    /// Both variants are live in the committed data, so neither can be dropped and an entry must
    /// never be coerced from one into the other. Narrowing consults **only the discriminant**
    /// the row posts, never the shape of the values beside it: serde's untagged enums would
    /// read a half-filled reference as text, which is the silent coercion this shape exists to
    /// prevent.
    TextOrReferenceRows(&'static [&'static str]),
    /// A list of contributor rows, read by [`apply_attribution_rows`] -
    /// `attributions`.
    ///
    /// Each row is an agent id plus a list of roles, so it is the one shape here whose row holds
    /// two different kinds of thing. Roles are an open set
    /// (`platform_metadata::project::CONTRIBUTOR_ROLES` is the *offer*), because the corpus
    /// spells the same role several ways and anything closed would rewrite published files.
    AttributionRows,
    /// A list of agent ids as editable rows, read by [`apply_string_rows`] —
    /// `contactPoint`.
    ///
    /// Stored and posted exactly as [`Self::StringRows`] is, which is why it shares the applier:
    /// the difference is not how the value is read but what it *means*. The form resolves each
    /// id to a name and submit refuses one that resolves to nothing — neither of which the
    /// applier can do, because a draft is allowed to hold a value that does not validate.
    ///
    /// A variant rather than a list of field ids elsewhere, so the submit check reads it off the
    /// shape rather than off a second list that could drift.
    AgentRows,
    /// A list of single strings as editable rows, read by
    /// [`apply_string_rows`] — `additionalMaterial` and
    /// `documentationMaterial`.
    ///
    /// Rows rather than a checkbox group, even though the contract member is the
    /// same `Option<Vec<String>>` [`Self::StringList`] reads: these hold URLs,
    /// which DPE renders as links, and a URL has to be *editable*. Through a
    /// checkbox group a typo means unticking the value and retyping the whole
    /// address.
    StringRows,
    /// A list of language maps, read by [`apply_multilingual_rows`] —
    /// `keywords` and `alternativeNames`.
    ///
    /// Rows come from `{field}.row` in DOM order and each row's texts from
    /// `{field}.<key>.<tag>`, so neither the count nor the order is carried by a
    /// number anywhere.
    MultilingualRows,
    /// One of a fixed set of wire values, read by [`apply_choice`].
    ///
    /// The set rides inside the shape because it is what makes the applier
    /// safe: a submitted value outside it is never stored, so a hand-built body
    /// cannot put `status: "Cancelled"` into a file that `ProjectStatus` will
    /// then refuse to deserialize. It comes from `platform_metadata` — the
    /// contract's own vocabulary — rather than being written out in the
    /// registry, so it cannot drift from the enum it has to satisfy.
    Choice(&'static [&'static str]),
}

impl Shape {
    /// Whether a field of this shape renders a control that refers to a person or organisation.
    ///
    /// **Exhaustive on purpose**, and declared here so a new shape cannot be added without
    /// answering the question. It gates the proposals summary, which exists to be acted on beside
    /// a picker; it used to gate the shared `<datalist>` those pickers pointed at, and the
    /// silent failure then was a page whose inputs referenced a list that was never rendered.
    #[must_use]
    pub const fn has_agent_picker(self) -> bool {
        match self {
            Self::AgentRows | Self::AttributionRows | Self::FundingRows => true,
            Self::Text(_)
            | Self::Multilingual
            | Self::Choice(_)
            | Self::Url(_)
            | Self::StringList(_)
            | Self::StringRows
            | Self::MultilingualRows
            | Self::TextOrReferenceRows(_)
            | Self::ReferenceRows(_)
            | Self::PublicationRows => false,
        }
    }

    /// Whether a field of this shape renders as a list of rows, and therefore carries the add and
    /// remove controls the row-action routes serve.
    ///
    /// **Exhaustive for the same reason [`Self::has_agent_picker`] is**, and it is the
    /// same failure: the renderer emits an add control whose `formaction` the route then has to
    /// accept, and a hand-written allowlist that misses a shape renders a button that answers
    /// `404` and discards the whole form with it. Nothing compiles differently and no test that
    /// only renders can see it. This is the list that was missed: `PublicationRows`,
    /// `ReferenceRows`, `TextOrReferenceRows` and `FundingRows` all arrived with their controls
    /// and without their routes.
    #[must_use]
    pub const fn has_rows(self) -> bool {
        match self {
            Self::AgentRows
            | Self::AttributionRows
            | Self::FundingRows
            | Self::MultilingualRows
            | Self::PublicationRows
            | Self::ReferenceRows(_)
            | Self::StringRows
            | Self::TextOrReferenceRows(_) => true,
            Self::Text(_) | Self::Multilingual | Self::Choice(_) | Self::Url(_) | Self::StringList(_) => false,
        }
    }
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
        Shape::Choice(values) => apply_choice(body, draft, field, values),
        Shape::Url(slot) => apply_url(body, draft, field, slot),
        Shape::StringList(set) => apply_string_list(body, draft, field, set),
        Shape::MultilingualRows => apply_multilingual_rows(body, draft, field),
        Shape::StringRows | Shape::AgentRows => apply_string_rows(body, draft, field),
        Shape::AttributionRows => apply_attribution_rows(body, draft, field),
        Shape::TextOrReferenceRows(types) => apply_text_or_reference_rows(body, draft, field, types),
        Shape::ReferenceRows(types) => apply_reference_rows(body, draft, field, types),
        Shape::PublicationRows => apply_publication_rows(body, draft, field),
        Shape::FundingRows => apply_funding(body, draft, field),
    }
}

/// Apply a submitted list of single strings to `field`, from `{field}.row` keys
/// and one `{field}.<key>` value each.
///
/// The same row protocol [`apply_multilingual_rows`] reads, with one text per row instead of a
/// language map, so the tile, the add and remove routes and the empty marker are all shared.
///
/// An empty row is dropped: an added-but-unfilled row is an editing state, and a list of empty
/// strings is not data. Absent `{field}.row` leaves the field alone; present with no valid key —
/// the marker an empty list posts — clears it.
///
/// A submitted text that differs from a stored one only in surrounding
/// whitespace keeps the **stored** bytes, matched across the whole list for the
/// reason [`unchanged_in`] gives: a row key is opaque and need not map to a
/// position, so resolving positionally would preserve one row's bytes into
/// another.
pub(crate) fn apply_string_rows(body: &FormBody, draft: &mut ProjectDraft, field: &str) {
    let row_name = format!("{field}.row");
    if !body.has(&row_name) {
        return;
    }
    let stored: Vec<String> = draft
        .get(field)
        .and_then(Value::as_array)
        .map(|rows| rows.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();

    let mut rows: Vec<Value> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for key in body.rows(field) {
        let Some(submitted) = body.get(&format!("{field}.{key}")) else {
            continue;
        };
        let Some(text) = resolve_against(&stored, submitted) else {
            continue;
        };
        // Two rows holding one value would write it to the file twice.
        if seen.insert(text.clone()) {
            rows.push(Value::String(text));
        }
    }
    if rows.is_empty() {
        draft.remove(field);
    } else {
        draft.set(field, Value::Array(rows));
    }
}

/// Apply a submitted URL to one slot of the project's URL pair.
///
/// Absent from the body leaves the slot alone. An empty value clears it — the
/// contract types both members as `Option`, so absence is what unset means —
/// **except** against a stored placeholder, which is already the empty state
/// and renders as an empty control — a committed project holds `url: ["MISSING"]`, and writing `""`
/// over it would change a published field nobody edited.
///
/// The write goes through [`ProjectDraft::set_url_slot`], which preserves the
/// project's stored representation and never touches the other slot.
pub(crate) fn apply_url(body: &FormBody, draft: &mut ProjectDraft, field: &str, slot: UrlSlot) {
    let Some(submitted) = body.get(field) else { return };
    let stored = draft.url_slot(slot).map(str::to_string);
    match resolve(submitted, stored.as_deref()) {
        Resolved::Unchanged => {}
        Resolved::Value(value) => draft.set_url_slot(slot, Some(&value)),
        Resolved::Cleared => {
            if stored.as_deref().is_some_and(is_placeholder) {
                return;
            }
            draft.set_url_slot(slot, None);
        }
    }
}

/// Apply a submitted choice to `field`, accepting only one of `values`.
///
/// Absent from the body leaves the field alone, as everywhere else. So does an **empty or
/// unrecognised** value, and that is the whole difference from [`apply_text`]: these fields are
/// required contract members typed as enums, so there is no empty state to write — `ProjectStatus`
/// has no "unset" variant and `accessRights` has no "none", and storing one would produce a draft
/// that cannot become a `ProjectRaw`.
///
/// A value outside `values` is dropped rather than refused: no control renders one, so it can only
/// arrive from a hand-built body, and dropping is the fail-safe direction taken by
/// [`FormBody::rows`] and [`FormBody::entries`] too.
pub(crate) fn apply_choice(body: &FormBody, draft: &mut ProjectDraft, field: &str, values: &[&str]) {
    let Some(submitted) = body.get(field) else { return };
    let submitted = submitted.trim();
    if !values.contains(&submitted) {
        return;
    }
    // No `resolve` here, and do not add an "only write if it changed" guard. A choice comes from a
    // closed list, so writing an unchanged value back is already byte-identical. The guard
    // would make the write unreachable for an untouched submit, which is the only path
    // `untouched_form_round_trip` exercises for this applier — under it a corrupted applier
    // passes that test.
    draft.set(field, Value::String(submitted.to_string()));
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
/// A value outside a [`ChoiceSet::Closed`] set is dropped, for the reason
/// [`apply_choice`] drops one: no control offers it, so it can only come from a
/// hand-built body, and storing it would put a value in the file the contract's
/// own vocabulary does not admit.
pub(crate) fn apply_string_list(body: &FormBody, draft: &mut ProjectDraft, field: &str, set: ChoiceSet) {
    if !body.has(field) {
        return;
    }
    let mut values: Vec<String> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for value in body.all(field) {
        let value = value.trim();
        if !value.is_empty() && set.accepts(value) && seen.insert(value) {
            values.push(value.to_string());
        }
    }
    if values.is_empty() {
        draft.remove(field);
    } else {
        draft.set(field, Value::Array(values.into_iter().map(Value::String).collect()));
    }
}

/// The stored bytes of a text that a submitted one only differs from in
/// whitespace or line endings, if any row holds one.
///
/// The row-list counterpart of the `stored` argument to [`resolve`]. Returns the
/// first match: two rows holding texts that differ only in surrounding space are
/// already indistinguishable to a reader, and collapsing to one of them cannot
/// lose a row.
fn unchanged_in<'a>(stored: &'a [DraftMultilingual], tag: &str, submitted: &str) -> Option<&'a str> {
    stored.iter().find_map(|row| {
        let text = row.get(tag)?;
        matches!(resolve(submitted, Some(text)), Resolved::Unchanged).then_some(text)
    })
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
pub(crate) fn apply_multilingual_rows(body: &FormBody, draft: &mut ProjectDraft, field: &str) {
    let row_name = format!("{field}.row");
    if !body.has(&row_name) {
        return;
    }
    // Every stored row, as an editing view, so a submitted text can be matched
    // against what the field already holds.
    let stored: Vec<DraftMultilingual> = draft
        .get(field)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| serde_json::from_value::<Multilingual>(row.clone()).ok())
                .map(|contract| DraftMultilingual::from_contract(&contract))
                .collect()
        })
        .unwrap_or_default();

    let mut rows: Vec<Value> = Vec::new();
    for key in body.rows(field) {
        let prefix = format!("{field}.{key}");
        let mut value = DraftMultilingual::new();
        for (tag, text) in body.entries(&prefix) {
            // Tag folded for the same reason as in `apply_multilingual`.
            let tag = tag.to_ascii_lowercase();
            // The stored counterpart is looked up **by text, across every row**, not by the row's
            // position: a row key is opaque and need not map to a position — after one
            // removal the body carries `r0` and `r2` against a two-row list — so
            // resolving positionally would preserve one row's bytes into another.
            match unchanged_in(&stored, &tag, text) {
                Some(kept) => value.set(&tag, kept),
                None => {
                    if let Resolved::Value(text) = resolve(text, None) {
                        value.set(&tag, text);
                    }
                }
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

/// A submitted value as it should be stored, given every value the field already
/// holds, or `None` when it is not a value at all.
///
/// The list counterpart of [`resolve`]'s `stored` argument, searching the whole list rather than
/// one position: a row key is opaque, so after one removal the body carries `r0` and `r2` against a
/// two-row list and matching by position would preserve one row's bytes into another.
///
/// An exact match is preferred over a whitespace-insensitive one, because a list may hold two
/// values that differ only in whitespace and the loose comparison cannot tell them apart.
fn resolve_against(stored: &[String], submitted: &str) -> Option<String> {
    // An exact match first, and it is not redundant: one committed project
    // holds both `"Project Member, Data Collector"` and the same value with a
    // trailing space, so a list can contain two values the normalised
    // comparison below cannot tell apart. Taking whichever came first then
    // rewrote the other, which the corpus round trip caught.
    if let Some(held) = stored.iter().find(|held| held.as_str() == submitted) {
        return Some(held.clone());
    }
    if let Some(held) = stored
        .iter()
        .find(|held| matches!(resolve(submitted, Some(held)), Resolved::Unchanged))
    {
        return Some(held.clone());
    }
    match resolve(submitted, None) {
        Resolved::Value(text) => Some(text),
        // Empty, or read as unchanged against nothing - neither is a value.
        Resolved::Cleared | Resolved::Unchanged => None,
    }
}

/// Apply a submitted list of contributor rows to `field` - `attributions`.
///
/// Rows come from `{field}.row` in DOM order; each row's agent id from
/// `{field}.<key>.contributor` and its roles from repeated
/// `{field}.<key>.role` values, which is what lets one checkbox group and an
/// "add another" input beside it both post into the same list.
///
/// **A row is defined by its contributor.** One with no id is dropped, because
/// that is what an added-but-unfilled row looks like and a contribution
/// attributed to nobody is not data. An empty role list is *kept*: the contract
/// types `contributorType` as a `Vec<String>`, so a contributor whose role
/// nobody has stated yet is expressible, and dropping the row would lose the
/// person instead of the gap.
///
/// Roles keep their stored bytes where a submitted one differs only in whitespace, through
/// [`resolve_against`] over every role the field holds: committed values end in a space and spell
/// the same role several ways, so neither trimming nor case-folding may touch what is there.
pub(crate) fn apply_attribution_rows(body: &FormBody, draft: &mut ProjectDraft, field: &str) {
    let row_name = format!("{field}.row");
    if !body.has(&row_name) {
        return;
    }
    let stored_roles: Vec<String> = draft
        .get(field)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("contributorType"))
                .filter_map(Value::as_array)
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut rows: Vec<Value> = Vec::new();
    for key in body.rows(field) {
        let contributor = body.get(&format!("{field}.{key}.contributor")).unwrap_or_default().trim();
        if contributor.is_empty() {
            continue;
        }
        let mut roles: Vec<Value> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for submitted in body.all(&format!("{field}.{key}.role")) {
            if let Some(role) = resolve_against(&stored_roles, submitted) {
                // Two ticked controls with one value is a rendering bug; writing
                // it twice would put the role in the file twice.
                if seen.insert(role.clone()) {
                    roles.push(Value::String(role));
                }
            }
        }
        let mut row = Map::new();
        // `Attribution`'s declaration order. `canonical::write_draft` reorders
        // on the way out anyway, but a draft that already matches is one fewer
        // difference between what is stored and what is published.
        row.insert("contributor".to_string(), Value::String(contributor.to_string()));
        row.insert("contributorType".to_string(), Value::Array(roles));
        rows.push(Value::Object(row));
    }
    if rows.is_empty() {
        draft.remove(field);
    } else {
        draft.set(field, Value::Array(rows));
    }
}

/// The discriminant value a row posts to choose the reference variant.
///
/// Anything else means free text, which is the fail-safe direction: text is
/// storable whatever was typed, while a reference needs a URL the row may not
/// have.
pub const REFERENCE_KIND: &str = "reference";

/// Apply a submitted list of reference-or-text rows to `field`.
///
/// Rows come from `{field}.row`; each row carries a discriminant at
/// `{field}.<key>.kind`, a reference at `{field}.<key>.ref.{type,url,label}`,
/// and free text at `{field}.<key>.text.<tag>`.
///
/// **The text branch is namespaced under `.text.`**, which is not cosmetic:
/// [`FormBody::entries`] reads every dotted suffix under a prefix as a language
/// tag, so language texts posted directly under `{field}.<key>` would have made
/// `kind`, `type` and `url` into languages named "kind", "type" and "url".
///
/// **Only the discriminant narrows.** The inactive branch is still submitted -
/// it is rendered `hidden`, not `disabled`, so a depositor who switches back
/// finds what they had - which means both candidates arrive on every save and
/// the row's own `kind` is the only thing that may decide between them. Reading
/// the values instead is how a half-filled reference gets silently stored as
/// text.
pub(crate) fn apply_text_or_reference_rows(body: &FormBody, draft: &mut ProjectDraft, field: &str, types: &[&str]) {
    let row_name = format!("{field}.row");
    if !body.has(&row_name) {
        return;
    }
    let stored = stored_strings(draft, field);

    let mut rows: Vec<Value> = Vec::new();
    for key in body.rows(field) {
        let prefix = format!("{field}.{key}");
        if body.get(&format!("{prefix}.kind")) == Some(REFERENCE_KIND) {
            // Shared with `apply_reference_rows`, so the two cannot disagree
            // about what a reference is.
            if let Some(row) = reference_row(body, &stored, &prefix, types) {
                rows.push(row);
            }
        } else {
            let mut value = DraftMultilingual::new();
            for (tag, text) in body.entries(&format!("{prefix}.text")) {
                if let Some(text) = resolve_against(&stored, text) {
                    value.set(&tag.to_ascii_lowercase(), &text);
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
    }
    if rows.is_empty() {
        draft.remove(field);
    } else {
        draft.set(field, Value::Array(rows));
    }
}

/// Every string this field already holds, at any depth.
///
/// What [`resolve_against`] needs to keep an untouched save byte-identical, and
/// flattened rather than walked per row for the reason that function gives: a
/// row key is opaque and need not map to a position. A URL and a label can only
/// be confused for each other when they are the same string, in which case
/// storing either is the same bytes.
fn stored_strings(draft: &ProjectDraft, field: &str) -> Vec<String> {
    fn walk(value: &Value, into: &mut Vec<String>) {
        match value {
            Value::String(text) => into.push(text.clone()),
            Value::Array(items) => items.iter().for_each(|item| walk(item, into)),
            Value::Object(members) => members.values().for_each(|member| walk(member, into)),
            _ => {}
        }
    }
    let mut strings = Vec::new();
    if let Some(value) = draft.get(field) {
        walk(value, &mut strings);
    }
    strings
}

/// One authority reference from a row's `{prefix}.ref.{type,url,label}`, or
/// `None` when it has no URL or names a source the field does not offer.
///
/// Shared by [`apply_reference_rows`] and [`apply_text_or_reference_rows`], so
/// the two cannot disagree about what a reference is.
fn reference_row(body: &FormBody, stored: &[String], prefix: &str, types: &[&str]) -> Option<Value> {
    let url = body.get(&format!("{prefix}.ref.url"))?;
    let kind = body.get(&format!("{prefix}.ref.type")).unwrap_or_default();
    if !types.contains(&kind) {
        return None;
    }
    // A reference is its URL: without one there is nothing to dereference, and
    // the type alone would publish a link to nowhere.
    let url = resolve_against(stored, url)?;
    let mut row = Map::new();
    // `AuthorityFileReference`'s declaration order.
    row.insert("type".to_string(), Value::String(kind.to_string()));
    row.insert("url".to_string(), Value::String(url));
    if let Some(label) = resolve_against(stored, body.get(&format!("{prefix}.ref.label")).unwrap_or_default()) {
        row.insert("text".to_string(), Value::String(label));
    }
    Some(Value::Object(row))
}

/// Apply a submitted list of authority references to `field`.
pub(crate) fn apply_reference_rows(body: &FormBody, draft: &mut ProjectDraft, field: &str, types: &[&str]) {
    if !body.has(&format!("{field}.row")) {
        return;
    }
    let stored = stored_strings(draft, field);
    let rows: Vec<Value> = body
        .rows(field)
        .into_iter()
        .filter_map(|key| reference_row(body, &stored, &format!("{field}.{key}"), types))
        .collect();
    if rows.is_empty() {
        draft.remove(field);
    } else {
        draft.set(field, Value::Array(rows));
    }
}

/// Apply a submitted list of bibliographic references to `field`.
///
/// **A row is its citation.** One with no text is dropped, because that is what
/// an added-but-unfilled row looks like and a persistent identifier with no
/// citation beside it is not a publication. The identifier is optional and its
/// member is omitted entirely when absent, rather than written as an object
/// with an empty URL.
pub(crate) fn apply_publication_rows(body: &FormBody, draft: &mut ProjectDraft, field: &str) {
    if !body.has(&format!("{field}.row")) {
        return;
    }
    let stored = stored_strings(draft, field);
    let mut rows: Vec<Value> = Vec::new();
    for key in body.rows(field) {
        let prefix = format!("{field}.{key}");
        let Some(text) = resolve_against(&stored, body.get(&format!("{prefix}.text")).unwrap_or_default()) else {
            continue;
        };
        let mut row = Map::new();
        // `Publication`'s declaration order.
        row.insert("text".to_string(), Value::String(text));
        if let Some(url) = resolve_against(&stored, body.get(&format!("{prefix}.pid")).unwrap_or_default()) {
            let mut pid = Map::new();
            row.insert("pid".to_string(), {
                pid.insert("url".to_string(), Value::String(url));
                Value::Object(pid)
            });
        }
        rows.push(Value::Object(row));
    }
    if rows.is_empty() {
        draft.remove(field);
    } else {
        draft.set(field, Value::Array(rows));
    }
}

/// Apply submitted funding to `field` - a list of grants, or one free text.
///
/// The discriminant is on the field rather than per row, so unlike
/// [`apply_text_or_reference_rows`] there is one `{field}.kind` for the whole
/// control. Both branches are submitted for the same reason and narrowed the
/// same way: only the discriminant decides.
///
/// A grant is its funders. One with none is dropped - that is what an
/// added-but-unfilled row looks like, and a grant number attributed to no funder is not a grant.
/// `number`, `name` and `url` are each omitted when empty rather than written as `""`, which is
/// what the committed data holds.
pub(crate) fn apply_funding(body: &FormBody, draft: &mut ProjectDraft, field: &str) {
    let has_rows = body.has(&format!("{field}.row"));
    let has_text = body.has(&format!("{field}.text"));
    if !has_rows && !has_text {
        return;
    }
    let stored = stored_strings(draft, field);

    if body.get(&format!("{field}.kind")) != Some(GRANTS_KIND) {
        match resolve_against(&stored, body.get(&format!("{field}.text")).unwrap_or_default()) {
            Some(text) => draft.set(field, Value::String(text)),
            None => {
                draft.remove(field);
            }
        }
        return;
    }

    let mut grants: Vec<Value> = Vec::new();
    for key in body.rows(field) {
        let prefix = format!("{field}.{key}");
        let mut funders: Vec<Value> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for submitted in body.all(&format!("{prefix}.funder")) {
            if let Some(id) = resolve_against(&stored, submitted) {
                if seen.insert(id.clone()) {
                    funders.push(Value::String(id));
                }
            }
        }
        if funders.is_empty() {
            continue;
        }
        let mut grant = Map::new();
        // `Grant`'s declaration order.
        grant.insert("funders".to_string(), Value::Array(funders));
        for (member, name) in [("number", "number"), ("name", "name"), ("url", "url")] {
            if let Some(value) = resolve_against(&stored, body.get(&format!("{prefix}.{name}")).unwrap_or_default()) {
                grant.insert(member.to_string(), Value::String(value));
            }
        }
        grants.push(Value::Object(grant));
    }
    if grants.is_empty() {
        draft.remove(field);
    } else {
        draft.set(field, Value::Array(grants));
    }
}

/// The discriminant value `funding` posts to choose the grants variant.
pub const GRANTS_KIND: &str = "grants";

#[cfg(test)]
mod tests {
    /// The marker rule both row-removal call sites depend on.
    ///
    /// Removing the last row must leave an empty `{field}.row` behind: with no marker an applier
    /// reads the field as not posted and leaves the stored value alone, so the removal would
    /// appear to work and not stick. This is why the rule is one function and not a copy in each
    /// handler.
    #[test]
    fn removing_the_last_row_leaves_an_empty_marker_behind() {
        let pairs = vec![
            ("keywords.row".to_string(), "r0".to_string()),
            ("keywords.r0.en".to_string(), "only".to_string()),
        ];
        let kept = super::FormBody::pairs_without_row(pairs, "keywords", "r0");
        assert!(
            kept.contains(&("keywords.row".to_string(), String::new())),
            "the field must still be posted, with zero rows: {kept:?}"
        );
    }

    #[test]
    fn removing_one_of_several_rows_adds_no_marker() {
        let pairs = vec![
            ("keywords.row".to_string(), "r0".to_string()),
            ("keywords.row".to_string(), "r1".to_string()),
        ];
        let kept = super::FormBody::pairs_without_row(pairs, "keywords", "r0");
        assert_eq!(kept, vec![("keywords.row".to_string(), "r1".to_string())]);
    }

    /// Another field's rows are untouched: the marker name is scoped to `field_id`.
    #[test]
    fn a_removal_leaves_another_fields_rows_alone() {
        let pairs = vec![
            ("keywords.row".to_string(), "r0".to_string()),
            ("attributions.row".to_string(), "r0".to_string()),
        ];
        let kept = super::FormBody::pairs_without_row(pairs, "keywords", "r0");
        assert!(kept.contains(&("attributions.row".to_string(), "r0".to_string())), "{kept:?}");
    }

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
        assert_eq!(body.entries("description").len(), MAX_VALUES_PER_FIELD);
    }

    #[test]
    fn a_body_within_the_cap_keeps_every_tag() {
        // The cap must not be reachable by anything real: the UI offers four
        // tags and no committed field carries more than two.
        let below = MAX_VALUES_PER_FIELD - 1;
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

    /// The closed vocabulary `typeOfData` declares, as these tests need it.
    /// Spelled out rather than imported from `platform_metadata`, so a change
    /// to the real vocabulary cannot quietly change what these assert.
    const DATA_KINDS: ChoiceSet = ChoiceSet::Closed(&["Text", "Image", "XML", "Video", "Audio"]);

    #[test]
    fn a_checkbox_group_becomes_a_list_in_body_order() {
        let mut draft = draft();
        let posted = body(&[("typeOfData", ""), ("typeOfData", "Image"), ("typeOfData", "Text")]);
        apply_string_list(&posted, &mut draft, "typeOfData", DATA_KINDS);
        assert_eq!(draft.get("typeOfData"), Some(&json!(["Image", "Text"])));
    }

    #[test]
    fn a_group_with_nothing_checked_clears_the_field_via_its_marker() {
        // An unchecked checkbox submits nothing, so without the hidden marker
        // the field would be absent and therefore left alone — a field that
        // cannot be cleared.
        let mut draft = draft();
        draft.set("typeOfData", json!(["Text"]));
        apply_string_list(&body(&[("typeOfData", "")]), &mut draft, "typeOfData", DATA_KINDS);
        assert!(draft.get("typeOfData").is_none());
    }

    #[test]
    fn a_group_absent_from_the_body_entirely_is_left_alone() {
        let mut draft = draft();
        draft.set("typeOfData", json!(["Text"]));
        apply_string_list(&body(&[("name", "x")]), &mut draft, "typeOfData", DATA_KINDS);
        assert_eq!(draft.get("typeOfData"), Some(&json!(["Text"])));
    }

    #[test]
    fn a_duplicate_value_is_stored_once() {
        let mut draft = draft();
        let posted = body(&[("typeOfData", "Text"), ("typeOfData", "Text")]);
        apply_string_list(&posted, &mut draft, "typeOfData", DATA_KINDS);
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

    #[test]
    fn a_choice_stores_only_a_value_the_shape_offers() {
        // The set rides inside the shape so a hand-built body cannot put
        // `status: "Cancelled"` into a file `ProjectStatus` then refuses to
        // deserialize — the failure would land on `to_raw` at submit, nowhere
        // near the value that caused it.
        let values: &[&str] = &["Ongoing", "Finished"];
        let mut draft = ProjectDraft::default();
        draft.set("status", Value::String("Ongoing".to_string()));

        apply_choice(
            &FormBody::from_pairs(vec![("status".to_string(), "Cancelled".to_string())]),
            &mut draft,
            "status",
            values,
        );
        assert_eq!(draft.get("status").and_then(Value::as_str), Some("Ongoing"), "unchanged");

        apply_choice(
            &FormBody::from_pairs(vec![("status".to_string(), "Finished".to_string())]),
            &mut draft,
            "status",
            values,
        );
        assert_eq!(draft.get("status").and_then(Value::as_str), Some("Finished"));
    }

    #[test]
    fn an_absent_or_empty_choice_leaves_the_stored_value_alone() {
        // Absent is "this section did not carry the field", as everywhere else.
        // Empty is the difference from `apply_text`: these are required
        // contract members typed as enums, so there is no unset variant to
        // write, and storing one would produce a draft that cannot become a
        // `ProjectRaw` at all. A radio group with nothing checked posts
        // nothing, so this is the shape a project with no value renders as.
        let values: &[&str] = &["Ongoing", "Finished"];
        let mut draft = ProjectDraft::default();
        draft.set("status", Value::String("Ongoing".to_string()));

        apply_choice(&FormBody::from_pairs(vec![]), &mut draft, "status", values);
        assert_eq!(draft.get("status").and_then(Value::as_str), Some("Ongoing"));

        apply_choice(
            &FormBody::from_pairs(vec![("status".to_string(), String::new())]),
            &mut draft,
            "status",
            values,
        );
        assert_eq!(draft.get("status").and_then(Value::as_str), Some("Ongoing"));
    }

    #[test]
    fn a_choice_writes_through_a_dotted_id_without_disturbing_its_siblings() {
        // `accessRights.accessRights` is the registry's dotted choice, and it
        // shares its object with `embargoDate`. A write that replaced the
        // object would take the date with it.
        let values: &[&str] = &["Full Open Access", "Embargoed Access"];
        let mut draft = ProjectDraft::default();
        draft.set(
            "accessRights",
            serde_json::json!({"accessRights": "Full Open Access", "embargoDate": "2030-01-01"}),
        );

        apply_choice(
            &FormBody::from_pairs(vec![("accessRights.accessRights".to_string(), "Embargoed Access".to_string())]),
            &mut draft,
            "accessRights.accessRights",
            values,
        );
        assert_eq!(
            draft.get("accessRights"),
            Some(&serde_json::json!({"accessRights": "Embargoed Access", "embargoDate": "2030-01-01"}))
        );
    }

    #[test]
    fn exceeds_entries_counts_distinct_suffixes_and_stops_early() {
        // `entries` cannot answer this — it stops at `MAX_VALUES_PER_FIELD`,
        // so through it a body of twenty thousand tags and one of sixty-four
        // look the same.
        let mut pairs: Vec<(String, String)> =
            (0..10).map(|n| (format!("description.l{n}"), "text".to_string())).collect();
        let body = FormBody::from_pairs(pairs.clone());
        assert!(!body.exceeds_entries("description", 10));
        assert!(body.exceeds_entries("description", 9));

        // A repeated suffix is one value, because that is what gets stored.
        pairs.extend((0..500).map(|_| ("description.l0".to_string(), "again".to_string())));
        assert!(!FormBody::from_pairs(pairs).exceeds_entries("description", 10));
    }

    #[test]
    fn a_closed_list_drops_a_value_it_does_not_offer_and_an_open_one_keeps_it() {
        // The distinction the corpus forces: `typeOfData` has no tail, while `dataLanguage` holds
        // far more tags than the UI offers, so closing the second would strip values from
        // published projects on their first save.
        let open = ChoiceSet::Open(&["de", "en"]);

        let body = FormBody::from_pairs(vec![
            ("typeOfData".to_string(), "Text".to_string()),
            ("typeOfData".to_string(), "Hologram".to_string()),
        ]);
        let mut draft = ProjectDraft::default();
        apply_string_list(&body, &mut draft, "typeOfData", DATA_KINDS);
        assert_eq!(draft.get("typeOfData"), Some(&serde_json::json!(["Text"])));

        let body = FormBody::from_pairs(vec![
            ("dataLanguage".to_string(), "de".to_string()),
            ("dataLanguage".to_string(), "la".to_string()),
        ]);
        let mut draft = ProjectDraft::default();
        apply_string_list(&body, &mut draft, "dataLanguage", open);
        assert_eq!(draft.get("dataLanguage"), Some(&serde_json::json!(["de", "la"])));
    }

    #[test]
    fn exceeds_all_counts_repeated_values_under_one_name() {
        let pairs: Vec<(String, String)> = (0..10).map(|n| ("typeOfData".to_string(), format!("v{n}"))).collect();
        let body = FormBody::from_pairs(pairs);
        assert!(!body.exceeds_all("typeOfData", 10));
        assert!(body.exceeds_all("typeOfData", 9));
        assert!(!body.exceeds_all("dataLanguage", 0), "a name that is absent exceeds nothing");
    }

    // --- apply_string_rows -----------------------------------------------

    #[test]
    fn string_rows_keep_body_order_and_drop_an_empty_row() {
        // An added-but-unfilled row is an editing state, not data: a list of
        // empty strings would reach a published file.
        let mut draft = draft();
        let posted = body(&[
            ("additionalMaterial.row", "r0"),
            ("additionalMaterial.r0", "https://second.example/"),
            ("additionalMaterial.row", "r1"),
            ("additionalMaterial.r1", ""),
            ("additionalMaterial.row", "r2"),
            ("additionalMaterial.r2", "https://first.example/"),
        ]);
        apply_string_rows(&posted, &mut draft, "additionalMaterial");
        assert_eq!(
            draft.get("additionalMaterial"),
            Some(&json!(["https://second.example/", "https://first.example/"]))
        );
    }

    #[test]
    fn string_rows_absent_from_the_body_are_left_alone_and_the_marker_clears_them() {
        // The marker is the only way a depositor can clear the last row: with
        // no `{field}.row` at all the field reads as absent and is left alone.
        let mut draft = draft();
        draft.set("additionalMaterial", json!(["https://kept.example/"]));

        apply_string_rows(&body(&[("name", "x")]), &mut draft, "additionalMaterial");
        assert_eq!(draft.get("additionalMaterial"), Some(&json!(["https://kept.example/"])));

        apply_string_rows(&body(&[("additionalMaterial.row", "")]), &mut draft, "additionalMaterial");
        assert!(draft.get("additionalMaterial").is_none());
    }

    #[test]
    fn a_string_row_matching_a_stored_value_but_for_whitespace_keeps_the_stored_bytes() {
        // The same rule the language-map rows follow, and for the same reason:
        // a value that differs only in surrounding space is one the depositor
        // did not change, so trimming it would rewrite a published field
        // nobody edited. Matched across the whole list, because a row key is
        // opaque and need not map to a position.
        let mut draft = draft();
        draft.set("additionalMaterial", json!(["https://kept.example/ "]));
        apply_string_rows(
            &body(&[
                ("additionalMaterial.row", "r0"),
                ("additionalMaterial.r0", "https://kept.example/"),
            ]),
            &mut draft,
            "additionalMaterial",
        );
        assert_eq!(
            draft.get("additionalMaterial"),
            Some(&json!(["https://kept.example/ "])),
            "the stored bytes survive"
        );
    }

    #[test]
    fn two_string_rows_holding_one_value_store_it_once() {
        let mut draft = draft();
        apply_string_rows(
            &body(&[
                ("additionalMaterial.row", "r0"),
                ("additionalMaterial.r0", "https://one.example/"),
                ("additionalMaterial.row", "r1"),
                ("additionalMaterial.r1", "https://one.example/"),
            ]),
            &mut draft,
            "additionalMaterial",
        );
        assert_eq!(draft.get("additionalMaterial"), Some(&json!(["https://one.example/"])));
    }

    // --- apply_attribution_rows -------------------------------------------

    #[test]
    fn an_attribution_row_is_defined_by_its_contributor() {
        // A row with no id is what an added-but-unfilled row looks like, and a
        // contribution attributed to nobody is not data. An empty role list is
        // kept, because the contract types `contributorType` as a `Vec` and a
        // contributor whose role nobody has stated is expressible.
        let mut draft = draft();
        let posted = body(&[
            ("attributions.row", "r0"),
            ("attributions.r0.contributor", "person-001"),
            ("attributions.r0.role", "Editor"),
            ("attributions.row", "r1"),
            ("attributions.r1.contributor", ""),
            ("attributions.r1.role", "Author"),
            ("attributions.row", "r2"),
            ("attributions.r2.contributor", "organization-008"),
            ("attributions.r2.role", ""),
        ]);
        apply_attribution_rows(&posted, &mut draft, "attributions");
        assert_eq!(
            draft.get("attributions"),
            Some(&json!([
                { "contributor": "person-001", "contributorType": ["Editor"] },
                { "contributor": "organization-008", "contributorType": [] },
            ]))
        );
    }

    #[test]
    fn a_role_keeps_its_stored_casing_and_its_stored_spaces() {
        // The corpus spells the same role several ways and four values end in a
        // space, so neither trimming nor folding may touch what is there: an
        // untouched save must not rewrite `Data curator` to `Data Curator`.
        let mut draft = draft();
        draft.set(
            "attributions",
            json!([{ "contributor": "person-001", "contributorType": ["Data curator", "Editor "] }]),
        );
        apply_attribution_rows(
            &body(&[
                ("attributions.row", "r0"),
                ("attributions.r0.contributor", "person-001"),
                ("attributions.r0.role", "Data curator"),
                ("attributions.r0.role", "Editor "),
            ]),
            &mut draft,
            "attributions",
        );
        assert_eq!(
            draft.get("attributions"),
            Some(&json!([{ "contributor": "person-001", "contributorType": ["Data curator", "Editor "] }]))
        );
    }

    #[test]
    fn two_roles_differing_only_in_a_trailing_space_both_survive() {
        // `0121_societesavoie` holds exactly this: `"Project Member, Data
        // Collector"` and the same value with a trailing space, in one
        // project. A whitespace-insensitive lookup cannot tell them apart, so
        // without an exact match first, whichever came second was rewritten to
        // the first - caught by the corpus round trip, not by a unit test.
        let mut draft = draft();
        draft.set(
            "attributions",
            json!([{ "contributor": "person-001", "contributorType": ["A role", "A role "] }]),
        );
        apply_attribution_rows(
            &body(&[
                ("attributions.row", "r0"),
                ("attributions.r0.contributor", "person-001"),
                ("attributions.r0.role", "A role"),
                ("attributions.r0.role", "A role "),
            ]),
            &mut draft,
            "attributions",
        );
        assert_eq!(
            draft.get("attributions"),
            Some(&json!([{ "contributor": "person-001", "contributorType": ["A role", "A role "] }])),
            "both variants survive rather than collapsing to one"
        );
    }

    #[test]
    fn a_role_typed_into_the_add_input_joins_the_ticked_ones() {
        // The group and the "add another" input post under one name, so
        // `FormBody::all` collects them together - which is what makes a role
        // outside the offer expressible without a second wire name.
        let mut draft = draft();
        apply_attribution_rows(
            &body(&[
                ("attributions.row", "r0"),
                ("attributions.r0.contributor", "person-001"),
                ("attributions.r0.role", "Editor"),
                ("attributions.r0.role", "Conservation, restoration"),
            ]),
            &mut draft,
            "attributions",
        );
        assert_eq!(
            draft.get("attributions"),
            Some(&json!([{
                "contributor": "person-001",
                "contributorType": ["Editor", "Conservation, restoration"],
            }]))
        );
    }

    #[test]
    fn attributions_absent_from_the_body_are_left_alone_and_the_marker_clears_them() {
        let mut draft = draft();
        draft.set("attributions", json!([{ "contributor": "person-001", "contributorType": [] }]));

        apply_attribution_rows(&body(&[("name", "x")]), &mut draft, "attributions");
        assert!(draft.get("attributions").is_some(), "absent leaves it");

        apply_attribution_rows(&body(&[("attributions.row", "")]), &mut draft, "attributions");
        assert!(draft.get("attributions").is_none(), "the marker clears it");
    }

    // --- apply_text_or_reference_rows -------------------------------------

    const SOURCES: &[&str] = &["Chronontology", "Periodo", "URL"];

    #[test]
    fn only_the_discriminant_decides_which_variant_a_row_becomes() {
        // The coercion this shape exists to prevent. Both branches are
        // submitted on every save because the inactive one is `hidden` rather
        // than `disabled`, so the values alone cannot say which was chosen:
        // here a fully filled reference sits beside filled text, and the
        // discriminant is the only difference between the two expectations.
        let filled = &[
            ("temporalCoverage.row", "r0"),
            ("temporalCoverage.r0.ref.type", "Periodo"),
            ("temporalCoverage.r0.ref.url", "https://perio.do/p0"),
            ("temporalCoverage.r0.ref.label", "Bronze Age"),
            ("temporalCoverage.r0.text.en", "Bronze Age, roughly"),
        ];

        let mut as_reference = draft();
        let mut pairs = filled.to_vec();
        pairs.push(("temporalCoverage.r0.kind", "reference"));
        apply_text_or_reference_rows(&body(&pairs), &mut as_reference, "temporalCoverage", SOURCES);
        assert_eq!(
            as_reference.get("temporalCoverage"),
            Some(&json!([{ "type": "Periodo", "url": "https://perio.do/p0", "text": "Bronze Age" }]))
        );

        let mut as_text = draft();
        let mut pairs = filled.to_vec();
        pairs.push(("temporalCoverage.r0.kind", "text"));
        apply_text_or_reference_rows(&body(&pairs), &mut as_text, "temporalCoverage", SOURCES);
        assert_eq!(
            as_text.get("temporalCoverage"),
            Some(&json!([{ "en": "Bronze Age, roughly" }])),
            "the reference values beside it are ignored, not merged"
        );
    }

    #[test]
    fn an_unchosen_branch_is_kept_in_the_form_and_out_of_the_file() {
        // What `hidden` rather than `disabled` buys: switching to a reference
        // and back finds the text intact, because the body carried it the whole
        // time — while the file only ever holds the chosen variant.
        let mut draft = draft();
        let pairs = vec![
            ("temporalCoverage.row", "r0"),
            ("temporalCoverage.r0.kind", "reference"),
            ("temporalCoverage.r0.ref.type", "Periodo"),
            ("temporalCoverage.r0.ref.url", "https://perio.do/p0"),
            ("temporalCoverage.r0.text.en", "kept in the form"),
        ];
        apply_text_or_reference_rows(&body(&pairs), &mut draft, "temporalCoverage", SOURCES);
        let stored = draft.get("temporalCoverage").expect("a row").to_string();
        assert!(!stored.contains("kept in the form"), "{stored}");
        assert!(stored.contains("perio.do"), "{stored}");
    }

    #[test]
    fn a_reference_without_a_url_or_with_an_unoffered_source_is_dropped() {
        // A reference is its URL: without one there is nothing to dereference
        // and the type alone publishes a link to nowhere. An unoffered source
        // can only come from a hand-built body, since no control renders it.
        let mut draft = draft();
        apply_text_or_reference_rows(
            &body(&[
                ("temporalCoverage.row", "r0"),
                ("temporalCoverage.r0.kind", "reference"),
                ("temporalCoverage.r0.ref.type", "Periodo"),
                ("temporalCoverage.r0.ref.url", ""),
                ("temporalCoverage.row", "r1"),
                ("temporalCoverage.r1.kind", "reference"),
                ("temporalCoverage.r1.ref.type", "MadeUp"),
                ("temporalCoverage.r1.ref.url", "https://example.test/"),
            ]),
            &mut draft,
            "temporalCoverage",
            SOURCES,
        );
        assert!(draft.get("temporalCoverage").is_none());
    }

    #[test]
    fn a_language_named_kind_or_url_is_not_invented_from_a_reference_member() {
        // The reason the text branch is namespaced under `.text.`:
        // `FormBody::entries` reads every dotted suffix under a prefix as a
        // language tag, so texts posted directly under the row would have made
        // `kind`, `type` and `url` into languages of those names.
        let mut draft = draft();
        apply_text_or_reference_rows(
            &body(&[
                ("temporalCoverage.row", "r0"),
                ("temporalCoverage.r0.kind", "text"),
                ("temporalCoverage.r0.ref.type", "Periodo"),
                ("temporalCoverage.r0.ref.url", "https://perio.do/p0"),
                ("temporalCoverage.r0.text.en", "A period"),
            ]),
            &mut draft,
            "temporalCoverage",
            SOURCES,
        );
        assert_eq!(draft.get("temporalCoverage"), Some(&json!([{ "en": "A period" }])));
    }

    #[test]
    fn a_variant_row_keeps_the_stored_bytes_of_a_value_nobody_changed() {
        let mut draft = draft();
        draft.set("temporalCoverage", json!([{ "en": "A period " }]));
        apply_text_or_reference_rows(
            &body(&[
                ("temporalCoverage.row", "r0"),
                ("temporalCoverage.r0.kind", "text"),
                ("temporalCoverage.r0.text.en", "A period"),
            ]),
            &mut draft,
            "temporalCoverage",
            SOURCES,
        );
        assert_eq!(draft.get("temporalCoverage"), Some(&json!([{ "en": "A period " }])));
    }

    // --- the last three shapes --------------------------------------------

    const PLACES: &[&str] = &["Geonames", "Pleiades", "Gazetteer", "URL"];

    #[test]
    fn a_reference_row_needs_a_url_and_an_offered_source() {
        let mut draft = draft();
        apply_reference_rows(
            &body(&[
                ("spatialCoverage.row", "r0"),
                ("spatialCoverage.r0.ref.type", "Geonames"),
                ("spatialCoverage.r0.ref.url", "https://www.geonames.org/2658434"),
                ("spatialCoverage.r0.ref.label", "Switzerland"),
                // No URL: nothing to dereference.
                ("spatialCoverage.row", "r1"),
                ("spatialCoverage.r1.ref.type", "Geonames"),
                ("spatialCoverage.r1.ref.url", ""),
                // A source no control offers.
                ("spatialCoverage.row", "r2"),
                ("spatialCoverage.r2.ref.type", "MadeUp"),
                ("spatialCoverage.r2.ref.url", "https://example.test/"),
            ]),
            &mut draft,
            "spatialCoverage",
            PLACES,
        );
        assert_eq!(
            draft.get("spatialCoverage"),
            Some(&json!([
                { "type": "Geonames", "url": "https://www.geonames.org/2658434", "text": "Switzerland" }
            ]))
        );
    }

    #[test]
    fn a_publication_row_is_its_citation_and_its_identifier_is_optional() {
        // The identifier's member is omitted entirely when absent rather than
        // written as an object with an empty URL, which is what the committed
        // data does: 85 of 334 rows carry one.
        let mut draft = draft();
        apply_publication_rows(
            &body(&[
                ("publications.row", "r0"),
                ("publications.r0.text", "Sartori, M., 2022"),
                ("publications.r0.pid", "https://doi.org/10.0000/x"),
                ("publications.row", "r1"),
                ("publications.r1.text", "Basile, C., 2020"),
                ("publications.r1.pid", ""),
                // No citation: an added-but-unfilled row.
                ("publications.row", "r2"),
                ("publications.r2.text", ""),
                ("publications.r2.pid", "https://doi.org/10.0000/orphan"),
            ]),
            &mut draft,
            "publications",
        );
        assert_eq!(
            draft.get("publications"),
            Some(&json!([
                { "text": "Sartori, M., 2022", "pid": { "url": "https://doi.org/10.0000/x" } },
                { "text": "Basile, C., 2020" },
            ]))
        );
    }

    #[test]
    fn funding_narrows_on_its_own_discriminant_and_keeps_the_other_branch_out_of_the_file() {
        // The discriminant is on the field rather than per row, and both
        // branches are submitted because the inactive one is only `hidden`.
        let filled = &[
            ("funding.text", "No funding"),
            ("funding.row", "r0"),
            ("funding.r0.funder", "organization-002"),
            ("funding.r0.number", "166072"),
            ("funding.r0.name", "Project funding"),
            ("funding.r0.url", "https://data.snf.ch/grants/grant/166072"),
        ];

        let mut as_grants = draft();
        let mut pairs = filled.to_vec();
        pairs.push(("funding.kind", "grants"));
        apply_funding(&body(&pairs), &mut as_grants, "funding");
        assert_eq!(
            as_grants.get("funding"),
            Some(&json!([{
                "funders": ["organization-002"],
                "number": "166072",
                "name": "Project funding",
                "url": "https://data.snf.ch/grants/grant/166072",
            }]))
        );

        let mut as_text = draft();
        let mut pairs = filled.to_vec();
        pairs.push(("funding.kind", "text"));
        apply_funding(&body(&pairs), &mut as_text, "funding");
        assert_eq!(as_text.get("funding"), Some(&json!("No funding")));
    }

    #[test]
    fn a_grant_is_its_funders_and_its_empty_members_are_omitted() {
        // A grant's optional members are absent in the committed data when unset, never `""`, so
        // writing an empty one as `""` rewrites the file on the first save.
        let mut draft = draft();
        apply_funding(
            &body(&[
                ("funding.kind", "grants"),
                ("funding.row", "r0"),
                ("funding.r0.funder", "organization-128"),
                ("funding.r0.funder", ""),
                ("funding.r0.number", ""),
                ("funding.r0.name", ""),
                ("funding.r0.url", ""),
                // No funder: a grant number attributed to nobody.
                ("funding.row", "r1"),
                ("funding.r1.funder", ""),
                ("funding.r1.number", "999"),
            ]),
            &mut draft,
            "funding",
        );
        assert_eq!(draft.get("funding"), Some(&json!([{ "funders": ["organization-128"] }])));
    }

    #[test]
    fn a_repeated_funder_is_stored_once() {
        let mut draft = draft();
        apply_funding(
            &body(&[
                ("funding.kind", "grants"),
                ("funding.row", "r0"),
                ("funding.r0.funder", "organization-002"),
                ("funding.r0.funder", "organization-002"),
            ]),
            &mut draft,
            "funding",
        );
        assert_eq!(draft.get("funding"), Some(&json!([{ "funders": ["organization-002"] }])));
    }
}

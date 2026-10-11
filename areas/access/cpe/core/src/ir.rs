//! Typed, validated IR produced by `validate::validate_with_data`.
//!
//! Every field is **owned** (`String`, `Vec<_>`, enums) — no borrowed
//! `&str` fields — so `ValidatedConfig` carries no lifetime and can
//! live inside `Arc<Project>` unchanged. This is what `Project::config()`
//! returns.
//!
//! Types are defined once and reused across component variants (no
//! per-variant duplication).
//!
//! No `kdl`/`miette` types appear in this module's public surface
//! (per dsp-incubator cpe/docs/adr/0001-crate-layout.md boundary rule).

// ── Query-layer domain types ──────────────────────────────────────────────────

/// A single Resource fetched from the project's SQLite, with all its typed
/// properties. Returned by the query layer (`query::get_resource`); consumed
/// by the property renderer. The `properties` vec preserves the data-defined
/// order (name, then seq within each name group).
#[derive(Debug, PartialEq)]
pub struct Resource {
    pub id: String,
    pub class: String,
    /// Properties in name order, then seq order within each name. Each
    /// `Property` groups all values for that name across all langs/seqs.
    pub properties: Vec<Property>,
    /// Media representations in `seq` order (images, audio, video, documents).
    /// Image representations render as a real `<img>` from the resolved IIIF
    /// URL, and — via the embedded-media axis (`media="…"` on
    /// `resource-detail`/`resource-list`) — feed the record viewer
    /// (`component::media`). Audio/video/document are silently
    /// skipped.
    pub representations: Vec<Representation>,
}

/// One media representation row from the `representations` table.
#[derive(Debug, PartialEq)]
pub struct Representation {
    /// The media kind as stored in the `representations.kind` column.
    pub kind: RepresentationKind,
    /// The media URI (IIIF manifest URL, file path, etc.).
    pub uri: String,
    /// Optional pixel width (for image representations).
    pub width: Option<i64>,
    /// Optional pixel height (for image representations).
    pub height: Option<i64>,
    /// Optional alt text for image representations (accessibility).
    pub alt: Option<String>,
    /// Optional attribution text (e.g. "John Tenniel · Public Domain").
    pub attribution: Option<String>,
    /// Sequence order within the resource's representation list.
    pub seq: i64,
}

/// A browse-row/grid-tile thumbnail's raw representation URI plus the
/// target image's own stored pixel size (`representations.width` and
/// `.height`, both nullable).
///
/// Returned by `query::media_thumb_uris`/`ProjectReads::media_thumbs`, one entry
/// per host id that resolved a thumbnail. `width` feeds the IIIF request-size
/// clamp (`iiif::iiif_thumb_url_clamped`/`iiif_grid_tile_url_clamped`)
/// — IIIF v3 (Sipi) rejects a `/full/<w>,/` request wider than the image's own
/// native width with a 400, so a thumbnail fetch must never ask for more than
/// `min(<fetch constant>, width)` when `width` is known.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaThumb {
    /// The raw stored representation URI (unsized — `/full/max/…`).
    pub uri: String,
    /// The target image's stored pixel width, when known.
    pub width: Option<i64>,
    /// The target image's stored pixel height, when known. A tall grid tile
    /// is requested best-fit and needs both sides for its clamp.
    pub height: Option<i64>,
}

/// The one meaning of "tall" for a box `w` wide and `h` high: at least three
/// times as high as wide. A 1:3 box is tall. It decides `data-tile="tall"`,
/// which selects the tall-tile rules in `areas/access/cpe/components/style/grid.css`, and
/// `data-frame="tall"` (`component::media::frame_is_tall`), which selects the
/// tall-frame rules in `areas/access/cpe/components/style/media.css`.
#[doc(hidden)]
pub fn is_tall(w: i64, h: i64) -> bool {
    w > 0 && h >= 3 * w
}

/// A grid tile's aspect ratio, width to height, from `resource-list
/// tile-ratio="W:H"`. Both sides are 1..=9999; [`TileRatio::parse`] is the
/// only constructor, so a value is always safe to write into a `style`
/// attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRatio {
    w: u32,
    h: u32,
}

impl TileRatio {
    /// Parses `W:H`: two integers from 1 to 9999 joined by one colon, each
    /// ASCII digits only, with no sign, white space, decimal point or leading
    /// zero. The characters are checked by hand because `u32::from_str`
    /// accepts `+1` and `01`.
    pub fn parse(s: &str) -> Option<Self> {
        fn side(part: &str) -> Option<u32> {
            if part.is_empty() || part.len() > 4 || part.starts_with('0') || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            part.parse().ok()
        }
        let (w, h) = s.split_once(':')?;
        Some(Self { w: side(w)?, h: side(h)? })
    }

    /// Whether the tile is tall ([`is_tall`]).
    pub fn is_tall(self) -> bool {
        is_tall(i64::from(self.w), i64::from(self.h))
    }
}

/// The CSS `aspect-ratio` value, `W / H`.
impl std::fmt::Display for TileRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} / {}", self.w, self.h)
    }
}

/// The closed set of representation kinds recognised by the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentationKind {
    Image,
    Audio,
    Video,
    Document,
}

impl RepresentationKind {
    /// Parse from the string stored in `representations.kind`.
    #[doc(hidden)]
    pub fn from_schema_str(s: &str) -> Option<Self> {
        match s {
            "image" => Some(Self::Image),
            "audio" => Some(Self::Audio),
            "video" => Some(Self::Video),
            "document" => Some(Self::Document),
            _ => None,
        }
    }
}

/// A named property with one or more typed values (multi-valued when the
/// property has several rows — e.g. `text` in EN and DE, or a list with
/// multiple values).
///
/// Values are in `seq` order within this property name.
#[derive(Debug, PartialEq)]
pub struct Property {
    pub name: String,
    pub values: Vec<PropertyValue>,
}

/// One typed value for a property row. Variant names match the `value_type`
/// column string in `schema.sql`.
#[derive(Debug, PartialEq)]
pub enum PropertyValue {
    /// A plain-text value, possibly language-tagged. `lang` is `None` for
    /// language-universal values (`value_lang IS NULL`).
    Text { text: String, lang: Option<String> },
    /// A Markdown/richtext value. `lang` is `None` when `value_lang IS NULL`
    /// (language-universal richtext).
    Richtext { text: String, lang: Option<String> },
    /// A 64-bit signed integer (stored as SQLite INTEGER).
    Integer(i64),
    /// A floating-point decimal (stored as SQLite REAL).
    Decimal(f64),
    /// A boolean (stored as SQLite INTEGER 0/1).
    Boolean(bool),
    /// A date value, optionally a range, with display precision.
    ///
    /// Whether the value *is* a range is derived, never stored: see
    /// `ir::date_is_range` (there is no `range` precision).
    Date {
        start: String,
        end: Option<String>,
        precision: DatePrecision,
        /// The curated display string (`date_display`), e.g. a catalogue's
        /// `28. Juli 1491`. `None` when the row carries none.
        display: Option<String>,
        /// The language `display` is written in — the row's `value_lang`.
        /// `None` means language-universal (or no display string at all).
        display_lang: Option<String>,
        /// The calendar system the ISO bounds were recorded in
        /// (`date_calendar`). Stored, never computed with.
        calendar: Option<Calendar>,
    },
    /// A link to another resource. `target_label` is `None` when the target
    /// does not exist or has no resolvable `name` property. `target_class` is
    /// the resolved Class of the target resource — `None` when the target does
    /// not exist.
    Link {
        target_id: String,
        target_label: Option<String>,
        /// The Class of the target resource, used to look up its route in the
        /// class→route map for clickable cross-resource links. `None` when the
        /// target does not exist in the data.
        target_class: Option<String>,
    },
    /// A list node reference. `label` is the resolved label for the requested
    /// language (with fallback), or `None` if no label exists.
    ListValue { node_id: String, label: Option<String> },
}

/// The display precision for a date value. Controls how much of the date is
/// shown (year only, year+month, or full date).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePrecision {
    Year,
    Month,
    Day,
}

impl DatePrecision {
    /// Parse from the string form stored in `schema.sql`'s `date_precision` column.
    #[doc(hidden)]
    pub fn from_schema_str(s: &str) -> Option<Self> {
        match s {
            "year" => Some(Self::Year),
            "month" => Some(Self::Month),
            "day" => Some(Self::Day),
            _ => None,
        }
    }

    /// Truncate an ISO date string (`YYYY`, `YYYY-MM` or `YYYY-MM-DD`) to
    /// this precision's significant prefix. A string shorter than the prefix
    /// is returned whole.
    #[doc(hidden)]
    pub fn truncate(self, date_iso: &str) -> &str {
        let len = match self {
            Self::Year => 4,
            Self::Month => 7,
            Self::Day => return date_iso,
        };
        date_iso.get(..len).unwrap_or(date_iso)
    }
}

/// The calendar system a date value's ISO bounds were recorded in. Parsed
/// once at hydration from `schema.sql`'s `date_calendar` column, whose CHECK
/// constraint admits exactly these two values (or NULL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Calendar {
    Julian,
    Gregorian,
}

impl Calendar {
    /// Parse from the string form stored in `schema.sql`'s `date_calendar` column.
    #[doc(hidden)]
    pub fn from_schema_str(s: &str) -> Option<Self> {
        match s {
            "julian" => Some(Self::Julian),
            "gregorian" => Some(Self::Gregorian),
            _ => None,
        }
    }
}

/// Whether a date value is a range: `true` iff `end` is `Some` and its
/// `precision`-truncated form differs from `start`'s.
///
/// Compares truncated parts, not raw strings, so a year-precise
/// `1471-01-01`/`1471-12-31` is a single year. This is the ONE place that
/// decides range-ness (there is no `range` precision) — the date renderer
/// and the timeline both call it.
#[doc(hidden)]
pub fn date_is_range(start: &str, end: Option<&str>, precision: DatePrecision) -> bool {
    match end {
        Some(end) => precision.truncate(start) != precision.truncate(end),
        None => false,
    }
}

/// A page of `Resource`s returned by `ProjectReads::list_resources`, together with
/// the pagination metadata the renderer needs to decide whether to show a pager.
///
/// The renderer shows a pager iff `pagination == Pagination::PerPage(_)`. For
/// `Pagination::Limit`, `page` is always `1` and `total` equals `items.len()`
/// (no pager, no total count query needed).
#[derive(Debug, PartialEq)]
pub struct ResourceListPage {
    /// The hydrated resources for this page window.
    pub items: Vec<Resource>,
    /// Total count of resources matching the filter (used for pager math).
    /// For `Pagination::Limit` this equals `items.len()`.
    ///
    /// LATENT: under `Pagination::Limit`, `total` is capped at the limit, so a
    /// facet-narrowed `Limit` list would understate the surf-head "N of M"
    /// count. Unreachable today (all faceted Alice lists are `PerPage`); revisit
    /// if a `Limit` list ever gains a paired `FacetPanel`.
    pub total: usize,
    /// 1-based current page. Always `1` for `Pagination::Limit`.
    pub page: u32,
    /// The pagination strategy, carried from the config. The renderer uses
    /// this to decide whether to render a pager: show iff `PerPage(_)`.
    pub pagination: Pagination,
    /// The unfiltered match count (class + config `where`s only, no active
    /// facet narrowing), for the "N of M" surf-head header.
    ///
    /// `Some(m)` when a facet panel has narrowed this list — `total` is the
    /// narrowed count and `m` is what it would be with no facets active.
    /// `None` when no facets are active (or the list has no paired
    /// `FacetPanel`) — the header shows a plain "N resources" instead.
    ///
    /// `list_resources` never sets this itself: it has no notion of "config
    /// wheres" vs. "facet wheres" (both arrive merged in one `wheres` slice).
    /// The store's page builder computes it post-hoc, after `list_resources`
    /// returns, via `count_resources` against the config wheres alone — see
    /// `query::page`.
    pub unfiltered_total: Option<usize>,
}

/// A lightweight header for one member of a compound, used to render the
/// contents rail (`.cv-toc__list`) without hydrating every member's
/// full `Resource`.
///
/// Returned in `compound_parts.seq, child_id` order by
/// `query::compound_part_headers`. `name` is resolved from the literal
/// `"name"` property (requested lang → universal → any) and falls back to
/// `id` when the member has no `name` value at all — `name` is never empty.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberHeader {
    /// The member resource's id.
    pub id: String,
    /// The member resource's class.
    pub class: String,
    /// Display name for the contents-rail entry. Never empty — falls back
    /// to `id` when the member has no `"name"` property value.
    pub name: String,
}

/// A single member view of a compound Resource, returned by
/// `ProjectReads::compound_parts`.
///
/// Invariant: `member.is_none() ⟺ total == 0`. `member` is `None` only
/// when the compound source has no parts (the renderer's empty-state path).
/// When `total >= 1`, `member` is always `Some` — the hydrated Resource at
/// position `index`.
///
/// Invariant: `members.len() == total`. `members` carries a lightweight
/// header (id/class/name) for every member in the compound (in
/// `compound_parts.seq, child_id` order), used to render the contents
/// rail; `member` is the hydrated `Resource` at the 1-based `index`.
#[derive(Debug, PartialEq)]
pub struct CompoundPage {
    /// The hydrated member at `index`, or `None` when `total == 0`.
    pub member: Option<Resource>,
    /// 1-based position of the shown member within the compound.
    pub index: u32,
    /// Total number of members in the compound (mirrors `ResourceListPage.total` typing).
    pub total: usize,
    /// Lightweight headers for every member, in compound order. `len() == total`.
    pub members: Vec<MemberHeader>,
    /// The current member's linked Image resources, in Seqnum order
    /// (`ProjectReads::media_items` over the `compound-viewer media="…"`
    /// property). Empty unless a media property was passed to
    /// `ProjectReads::compound_parts` (and always empty when `total == 0`). The
    /// renderer shows only the first — see `render_compound_viewer`.
    pub member_media: Vec<Resource>,
    /// The decoded `?page=` value when it was non-empty and named no page
    /// (unknown or ambiguous), else `None` (book-page Choice H). The page
    /// shown is still the `?member=` fallback; the `pages` reader renders a
    /// status line naming both. Always `None` when `total == 0`.
    pub stale_page: Option<String>,
    /// The shown member's annotation register (`ProjectReads::page_annotations`).
    /// Filled only when `ProjectReads::compound_parts` is asked for annotations
    /// (`compound-viewer annotations=#true`); empty otherwise and when
    /// `total == 0`.
    pub annotations: Vec<AnnotationEntry>,
    /// The members that have a register (`ProjectReads::annotated_members`), for
    /// the reader's signposts. Same gate as `annotations`.
    pub annotated_members: std::collections::BTreeSet<String>,
}

impl CompoundPage {
    /// Whether the member has a register: the one rule every reader
    /// signpost (pager step, picker entry, picker group) marks by.
    #[doc(hidden)]
    pub fn is_annotated(&self, member_id: &str) -> bool {
        self.annotated_members.contains(member_id)
    }
}

/// The ordering of an `incoming-links` list (`query::incoming_links`).
///
/// Every variant is a total, deterministic order: ties always break on the
/// source id, then on the matching link property's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncomingOrder {
    /// By the source's display name, then id.
    Name,
    /// By the source's id.
    Id,
    /// By the source's position in its compound parent: grouped by parent id
    /// (members of different parents never interleave), then by
    /// `compound_parts.seq`; sources with no compound parent sort last.
    CompoundSeq,
}

/// The compound parent of an incoming-link source (`IncomingLink::parent`)
/// or of an annotation partner's resource (`AnnotationPartner::parent`).
#[derive(Debug, Clone, PartialEq)]
pub struct MemberParent {
    /// The parent resource's id.
    pub id: String,
    /// The parent resource's class.
    pub class: String,
    /// The source's `compound_parts.seq` within THIS parent.
    pub seq: i64,
}

/// One resource that links TO a target resource, returned by
/// `query::incoming_links` (reverse resolution of `link`-typed properties).
///
/// One row per `(source, via)` pair — a source linking to the target through
/// two different properties yields two rows.
#[derive(Debug, Clone, PartialEq)]
pub struct IncomingLink {
    /// The source resource's id.
    pub id: String,
    /// The source resource's class.
    pub class: String,
    /// Display name, from the literal `"name"` property (requested lang →
    /// universal → any). Never empty — falls back to `id`.
    pub name: String,
    /// The link property name that matched (e.g. `strip-left`).
    pub via: String,
    /// The source's compound parent, if it is a compound member. A source in
    /// several compounds takes its lexicographically-first parent, with that
    /// parent's own `seq`.
    pub parent: Option<MemberParent>,
    /// The row's group key when the spec groups by a source PROPERTY
    /// (`group-by="<property>"`). `None` when the spec is
    /// ungrouped, groups by `via` (the key is [`Self::via`] itself), or the
    /// source carries no value for the group property (the trailing,
    /// headless group). A multi-valued group property contributes its first
    /// value only, so the row appears in exactly one group.
    pub group: Option<GroupKey>,
    /// The `row-label` property's value (requested lang → universal → any,
    /// first value). `None` when the spec has no `row-label` or the source
    /// lacks the property — the renderer then falls back to [`Self::name`].
    pub label: Option<String>,
    /// The `row-aside` property's value: a `text` value (requested lang →
    /// universal → any), else a `list_value`'s node label (requested lang →
    /// any; list labels have no universal rung). First value only. `None`
    /// when the spec has no `row-aside` or the source lacks the property.
    pub aside: Option<String>,
}

/// The group an [`IncomingLink`] row belongs to under a property
/// `group-by`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupKey {
    /// A `list_value` group property: the list node.
    List {
        /// The list node id (`properties.target_id`).
        key: String,
        /// The node's label (requested lang → any label → the node id).
        label: String,
        /// The node's `list_nodes.seq` — the group order. A node with no
        /// `list_nodes` row sorts last (`i64::MAX`).
        seq: i64,
    },
    /// A `text` group property: the value (requested lang → universal →
    /// any). Case variants of one value share a key — the first spelling in
    /// row order wins — so groups compare case-insensitively.
    Text {
        /// The group's text value.
        key: String,
    },
}

// ── Annotations ───────────────────────────────────────────────────────────────
// dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md

/// The drawn shape of an [`Annotation`] (`annotations.shape`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationShape {
    Rectangle,
    /// Its vertices are [`Annotation::points`].
    Polygon,
    /// The ellipse inscribed in the bounding box; its two radii may differ.
    Circle,
}

impl AnnotationShape {
    /// Parse the stored `annotations.shape` value; `None` for anything else.
    #[doc(hidden)]
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "rectangle" => Some(Self::Rectangle),
            "polygon" => Some(Self::Polygon),
            "circle" => Some(Self::Circle),
            _ => None,
        }
    }
}

/// Parse an `annotations.points` value — whitespace-separated `x,y` pairs —
/// into vertices. `None` when any pair is not two finite numbers. Shared by
/// the query layer and the `MalformedAnnotation` boot check, so both read the
/// same grammar; the vertex count and box containment are the check's.
#[doc(hidden)]
pub fn parse_annotation_points(text: &str) -> Option<Vec<(f64, f64)>> {
    text.split_whitespace()
        .map(|pair| {
            let (x, y) = pair.split_once(',')?;
            let x: f64 = x.parse().ok()?;
            let y: f64 = y.parse().ok()?;
            (x.is_finite() && y.is_finite()).then_some((x, y))
        })
        .collect()
}

/// One marked area on a resource's displayed image (`annotations` row).
///
/// The box and the points are fractions (0..1) of the image, top-left origin;
/// boot validation (`MalformedAnnotation`) guarantees the box lies inside the
/// image with `w, h > 0`, a polygon has at least three vertices inside the
/// box, and `color` is lowercase `#rrggbb`.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub id: String,
    /// The annotated resource (a Page), never its `Image` resource.
    pub resource_id: String,
    pub shape: AnnotationShape,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// A polygon's vertices; empty for every other shape.
    pub points: Vec<(f64, f64)>,
    pub color: Option<String>,
    pub remark: Option<String>,
    pub remark_lang: Option<String>,
}

/// A named link joining annotations or resources (at least one target)
/// (`annotation_links` row).
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationLink {
    pub id: String,
    /// The link's short display name.
    pub name: String,
    pub name_lang: Option<String>,
    /// The source's own comment on the link.
    pub comment: Option<String>,
    pub comment_lang: Option<String>,
}

/// Another target of an annotation link, seen from one register entry
/// (`query::annotation_partners`). One row per `(link, target)`: two links
/// joining the same pair yield two partners, and grouping or deduplicating
/// them is the renderer's.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationPartner {
    /// The link joining the entry to this partner.
    pub link_id: String,
    pub link_name: String,
    pub link_name_lang: Option<String>,
    /// The partner annotation's id; `None` when the target is a resource.
    pub annotation_id: Option<String>,
    /// That annotation's number on its own resource (the register order);
    /// `None` when the target is a resource.
    pub number: Option<u32>,
    /// The partner's resource: the annotated resource of an annotation
    /// target, else the target resource itself.
    pub resource_id: String,
    pub resource_class: String,
    /// The resource's display name, from the literal `"name"` property
    /// (requested lang → universal → any). Never empty — falls back to the id.
    pub resource_label: String,
    /// The resource's compound parent (its lexicographically-first one), if
    /// it is a compound member.
    pub parent: Option<MemberParent>,
    /// The parent's display name, resolved like `resource_label`.
    pub parent_title: Option<String>,
    /// The resource comes first in that parent (`compound_parts.seq`, then
    /// child id): its reader's page 1, whose address may carry no `page=`.
    pub first_member: bool,
}

/// One entry of a resource's annotation register (`ProjectReads::page_annotations`).
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationEntry {
    /// An annotation on the resource. `number` is 1-based: larger box first,
    /// ties by id.
    Annotation {
        number: u32,
        annotation: Annotation,
        partners: Vec<AnnotationPartner>,
    },
    /// A link targeting the resource itself (a page-level entry): unnumbered
    /// and after every numbered entry, in link-id order.
    Link { link: AnnotationLink, partners: Vec<AnnotationPartner> },
}

/// One annotation with what a figure cut from its resource's image needs
/// (`ProjectReads::annotation_crops`).
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationCrop {
    pub annotation: Annotation,
    /// The annotated resource's class.
    pub resource_class: String,
    /// Its display name, from the literal `"name"` property (requested lang →
    /// universal → any), falling back to its id.
    pub resource_label: String,
    /// Its compound parent (the lexicographically-first one), if it is a
    /// compound member.
    pub parent: Option<MemberParent>,
    /// The parent's display name, resolved like `resource_label`.
    pub parent_title: Option<String>,
    /// The resource's image: the first image representation of its media
    /// property's first target. `None` when it has none.
    pub image: Option<CropImage>,
}

/// A resource with what a whole-page figure needs (`ProjectReads::page_plates`):
/// the fields of an [`AnnotationCrop`] without the annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct PagePlate {
    pub resource_id: String,
    pub resource_class: String,
    /// Its display name, resolved like [`AnnotationCrop::resource_label`].
    pub resource_label: String,
    pub parent: Option<MemberParent>,
    pub parent_title: Option<String>,
    pub image: Option<CropImage>,
}

/// The stored image an [`AnnotationCrop`] or a [`PagePlate`] is cut from.
#[derive(Debug, Clone, PartialEq)]
pub struct CropImage {
    /// The raw representation URI (`…/full/max/…`).
    pub uri: String,
    /// The stored pixel size, when known.
    pub width: Option<i64>,
    pub height: Option<i64>,
}

/// A resource-level link partner of a record (`ProjectReads::resource_links`): a
/// resource that shares an annotation link with it. Annotation targets of the
/// same link are not resource-level partners and are omitted.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceLink {
    pub link_id: String,
    pub link_name: String,
    pub link_name_lang: Option<String>,
    pub resource_id: String,
    pub resource_class: String,
    /// Display name from the literal `"name"` property (requested lang →
    /// universal → any), falling back to the id.
    pub resource_label: String,
}

// ── Data ARKs ─────────────────────────────────────────────────────────────────

/// Every plain DSP data ARK URL starts with this: the fixed resolver, DaSCH's
/// assigned number and dsp-api's ARK version. `dsp-incubator cpe/tools/dao-lift/src/ark.rs` and
/// `cpe_ports::DATA_ARK_PREFIX`, from which the ARKs of
/// Incunabula's store are built, spell the same prefix; the three must stay equal.
pub(crate) const DATA_ARK_PREFIX: &str = "https://ark.dasch.swiss/ark:/72163/1/";

/// The stored data ARKs of a project, keyed by id: `resources.ark` and
/// `annotations.ark`, NULL rows omitted. Built once by `Project::load`
/// (`Project::data_arks`); every value has passed the `MalformedDataArk` boot
/// check. Resource and annotation ids are separate namespaces, hence two maps.
/// The fields are private and a value is built only through `from_stored`, so
/// no host can assemble one from unchecked strings; read it through the lookups.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DataArks {
    resources: std::collections::HashMap<String, String>,
    annotations: std::collections::HashMap<String, String>,
}

impl DataArks {
    /// Build the ARKs from the two stored maps (`resources.ark`, `annotations.ark`).
    ///
    /// A value may reach a renderer only after the `MalformedDataArk` boot check
    /// passed for the same store, which `Project::load`'s validate-then-read
    /// order guarantees: it validates, then reads the ARKs with
    /// `query::data_arks`. This is for the store and for tests, not for a host.
    #[doc(hidden)]
    pub fn from_stored(
        resources: std::collections::HashMap<String, String>,
        annotations: std::collections::HashMap<String, String>,
    ) -> Self {
        Self { resources, annotations }
    }

    /// The data ARK of the resource `id`, if it has one.
    pub fn resource(&self, id: &str) -> Option<&str> {
        self.resources.get(id).map(String::as_str)
    }

    /// The data ARK of the annotation `id`, if it has one.
    pub fn annotation(&self, id: &str) -> Option<&str> {
        self.annotations.get(id).map(String::as_str)
    }
}

// ── Top-level ─────────────────────────────────────────────────────────────────

/// The fully-validated, typed IR produced by `validate::validate_with_data`.
/// Replaces the loose `RawConfig` for rendering.
#[derive(Debug, PartialEq)]
pub struct ValidatedConfig {
    pub site: SiteConfig,
    pub pages: Vec<Page>,
}

// ── Site ──────────────────────────────────────────────────────────────────────

/// Project-level metadata and chrome, validated and typed.
#[derive(Debug, PartialEq)]
pub struct SiteConfig {
    /// Project display name (`title "…"` in the `site` block).
    pub title: String,
    /// Optional project description.
    pub description: Option<String>,
    /// Optional cover image path, shown on the site index card. Authored
    /// INCLUDING the `assets/` segment itself (e.g. `"assets/cover.webp"`),
    /// not relative to (i.e. excluding) that directory.
    pub cover_image: Option<String>,
    /// Optional attribution/credit text for `cover_image`.
    pub image_credit: Option<String>,
    /// Optional short eyebrow label shown above the title on the site
    /// index card.
    pub eyebrow: Option<String>,
    /// ISO 639-1 language codes declared in `languages "…" "…"`.
    /// Always contains at least `"de"` and `"en"`.
    pub languages: Vec<String>,
    /// The resolved default language. Defaults to the first declared
    /// language if not explicitly set.
    pub default_language: String,
    /// Navigation items from the `nav { … }` block.
    pub nav: Vec<NavItem>,
    /// Chrome strings (UI labels with per-language translations).
    pub chrome: Vec<ChromeString>,
    /// Per-property display labels with per-language translations, declared
    /// in the top-level `labels { property "…" { … } }` block.
    pub property_labels: Vec<PropertyLabel>,
    /// Per-class display labels with per-language translations, declared
    /// in the top-level `labels { class "…" { … } }` block.
    pub class_labels: Vec<ClassLabel>,
    /// Per-project visual design knob from the `theme { … }` block.
    ///
    /// Absent `theme {}` block ⇒ `ThemeKnob::default()` (Ochre/Tint).
    /// Emitted as `data-accent` and `data-bg` on `<html>` by `render_page`.
    pub theme: crate::theme::ThemeKnob,
    /// `site { date-precision-word #true }`: append the localized precision word
    /// (`Tag`/`Monat`/`Jahr`/`Zeitraum`, plus the Gregorian calendar word, not at year precision)
    /// after every rendered date value as `span.prec`. Default `false` — a project that does not
    /// ask renders its dates without the word.
    pub date_precision_word: bool,
    /// `site { footer-attribution "…" }`: a project override for the footer's attribution line.
    /// Chrome-id-or-literal. `None` ⇒ `ChromeMsg::FooterAttribution`.
    pub footer_attribution: Option<String>,
    /// `site { not-found { browse route="…" label="…" } }`: the project 404's browse links, in
    /// declaration order, at most three (`TooManyNotFoundBrowseLinks`). Each
    /// `route` matches a declared page route exactly
    /// (`UnresolvedNotFoundBrowseRoute`). Empty ⇒ the engine's default (the
    /// first three nav items).
    pub not_found_browse: Vec<NotFoundBrowse>,
    /// `site { narrative-companion "…" }`: what stands beside a narrative
    /// page's reading column unless the page's first `narrative` says
    /// otherwise.
    pub narrative_companion: NarrativeCompanion,
    /// `site { narrative-side "…" }`: the side that companion stands on,
    /// unless the page's first `narrative` says otherwise.
    pub narrative_side: NarrativeSide,
    /// `site { text-align "…" }`: how running prose is aligned.
    pub text_align: TextAlign,
}

/// What stands beside a narrative page's reading column from the desk
/// breakpoint up (`companion=` on a page's first `narrative`,
/// `site { narrative-companion }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NarrativeCompanion {
    /// The "On this page" list of the page's top-level `##` headings.
    #[default]
    Rail,
    /// Nothing: one centred column.
    Text,
}

impl NarrativeCompanion {
    /// Every config value, in the order diagnostics list them.
    pub const ALLOWED: [&'static str; 2] = ["rail", "text"];

    /// Parse a `config.kdl` value; `None` for anything else.
    pub fn parse_config_str(s: &str) -> Option<Self> {
        match s {
            "rail" => Some(Self::Rail),
            "text" => Some(Self::Text),
            _ => None,
        }
    }

    /// The `data-narr` value. Round-trips with `parse_config_str`.
    pub fn as_data_attr(self) -> &'static str {
        match self {
            Self::Rail => "rail",
            Self::Text => "text",
        }
    }
}

/// The side of the reading column a narrative page's companion stands on
/// (`side=` on a page's first `narrative`, `site { narrative-side }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NarrativeSide {
    /// After the reading column in the inline direction.
    #[default]
    End,
    /// Before it: the mirrored layout.
    Start,
}

impl NarrativeSide {
    /// Every config value, in the order diagnostics list them.
    pub const ALLOWED: [&'static str; 2] = ["end", "start"];

    /// Parse a `config.kdl` value; `None` for anything else.
    pub fn parse_config_str(s: &str) -> Option<Self> {
        match s {
            "end" => Some(Self::End),
            "start" => Some(Self::Start),
            _ => None,
        }
    }

    /// The `data-narr-side` value. Round-trips with `parse_config_str`.
    pub fn as_data_attr(self) -> &'static str {
        match self {
            Self::End => "end",
            Self::Start => "start",
        }
    }
}

/// How running prose is aligned (`site { text-align }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// Ragged at the line end.
    #[default]
    Start,
    /// Justified and hyphenated.
    Justify,
}

impl TextAlign {
    /// Every config value, in the order diagnostics list them.
    pub const ALLOWED: [&'static str; 2] = ["start", "justify"];

    /// Parse a `config.kdl` value; `None` for anything else.
    pub fn parse_config_str(s: &str) -> Option<Self> {
        match s {
            "start" => Some(Self::Start),
            "justify" => Some(Self::Justify),
            _ => None,
        }
    }

    /// The `data-align` value. Round-trips with `parse_config_str`.
    pub fn as_data_attr(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Justify => "justify",
        }
    }
}

/// One `site { not-found { browse route="…" label="…" } }` entry.
#[derive(Debug, Clone, PartialEq)]
pub struct NotFoundBrowse {
    /// A declared page route, matched exactly (the nav-route rule).
    pub route: String,
    /// Optional link-text override, chrome-id-or-literal (a declared
    /// `chrome { string }` id renders its translation, any other value renders
    /// verbatim). `None` ⇒ the matching nav item's label, else the route.
    pub label: Option<String>,
}

/// A navigation item declared in `nav { item label="…" route="…" }`.
#[derive(Debug, Clone, PartialEq)]
pub struct NavItem {
    pub label: String,
    pub route: String,
}

/// One chrome string: an identifier and its per-language translations.
///
/// Maps from `chrome { string "id" { de "…" en "…" } }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ChromeString {
    pub id: String,
    pub translations: Vec<(String, String)>, // (lang, text), order preserved
}

/// One captioned property group on a `resource-detail`. Generic — any project may partition its
/// detail properties this way.
///
/// Maps from `resource-detail { group label="<chrome-id>" "a" "b" … }`.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyGroup {
    /// A project `chrome { string }` **id** (id-only, never literal text) for
    /// the group's cap. An undeclared id is the Warning `UnknownGroupLabel`
    /// and falls back to `ChromeMsg::PropertyGroupTitle`.
    pub label: String,
    /// Member property names, in display order (positional string args,
    /// same shape as `show-properties`).
    pub properties: Vec<String>,
    /// Optional chrome-string **id** of an italic sentence shown under the
    /// caption when EVERY member is absent on the record. `None` ⇒ an all-absent group renders
    /// nothing. An undeclared id is the Warning `UnknownGroupEmptyLabel`; the
    /// group then renders as if `empty=` were unset.
    pub empty: Option<String>,
    /// Optional chrome-string **id** captioning a closing row that lists the
    /// record's resource-level annotation-link partners
    /// (`ProjectReads::resource_links`). `None` ⇒ no row.
    /// An undeclared id is the Warning `UnknownGroupLinksLabel`.
    pub links_label: Option<String>,
}

/// The `compound-viewer`'s `member-links` row: a caption
/// and the member `link` properties whose targets it lists, in order.
///
/// Maps from `compound-viewer { member-links label="<chrome-id>" "a" "b" … }`.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberLinks {
    /// A project `chrome { string }` **id** for the caption. An undeclared id
    /// is the Warning `UnknownGroupLabel` and falls back to
    /// `ChromeMsg::PropertyGroupTitle`, as a `group` does.
    pub label: String,
    /// Member `link` property names, in display order.
    pub properties: Vec<String>,
}

/// One per-resource introduction on a `resource-detail`. Generic — any project may attach a
/// narrative file to specific resources this way.
///
/// Maps from `resource-detail { narrative-for "<resource-id>" file="…" }`.
#[derive(Debug, Clone, PartialEq)]
pub struct NarrativeFor {
    /// The literal resource id this intro attaches to (never a `$param`).
    pub resource_id: String,
    /// A bare narrative filename (same contract as `narrative file="…"`),
    /// resolved through the shared 3-rung language fallback at render time.
    pub file: String,
}

/// The reverse-link list on a `resource-detail` —
/// the resources of one source `class` that point AT the rendered resource
/// through any of the named `link` properties. Generic — any project may
/// declare one.
///
/// Maps from `resource-detail { incoming-links label="<chrome-id>"
/// class="<Class>" order="name"|"id"|"compound-seq" "a" "b" … }`. At most one
/// per `resource-detail` (`DuplicateIncomingLinks`).
#[derive(Debug, Clone, PartialEq)]
pub struct IncomingLinksSpec {
    /// A project `chrome { string }` **id** (id-only, never literal text) for
    /// the list's caption. An undeclared id is the Warning
    /// `UnknownIncomingLinksLabel`.
    pub label: String,
    /// The source Class whose resources are listed (a query filter, not
    /// decoration).
    pub class: String,
    /// The `link` property names to reverse-resolve (positional string args,
    /// same shape as `show-properties`). Never empty — lowering rejects zero
    /// (`IncomingLinksWithoutProperty`).
    pub properties: Vec<String>,
    /// The list's ordering; `IncomingOrder::Name` when `order=` is absent.
    pub order: IncomingOrder,
    /// The row cap. Always resolved — never `Option` — to
    /// [`DEFAULT_INCOMING_LINKS_LIMIT`] when the optional `limit=<int>`
    /// attribute is absent. A present-but-invalid value (not an integer, or an
    /// integer outside `1..=u32::MAX`) is `InvalidAttributeType` at lowering
    /// time (`validate::lower_incoming_links`), so this field is always a valid
    /// positive cap by the time rendering sees it.
    ///
    /// In grouped mode (`grouping: Some`) an absent `limit=` resolves to
    /// [`GROUPED_INCOMING_LIMIT_CEILING`] instead, so a cut never silently
    /// drops trailing groups; an explicit `limit=` (including `limit=50`) is
    /// always honoured.
    pub limit: u32,
    /// Optional partitioning of the rows into headed groups, from `group-by=`. `None` ⇒ a flat
    /// list. The grouped-only options (`layout=`, `note-label=`) live inside,
    /// so an option without a grouping is unrepresentable; written without
    /// `group-by` they are ignored.
    pub grouping: Option<IncomingGrouping>,
    /// Optional `row-label="<property>"`: a `text` property of the source
    /// resource used as the row's link text instead of its `name` (fallback
    /// `name` when a row lacks it).
    pub row_label: Option<String>,
    /// Optional `row-aside="<property>"`: a `text` or `list_value` property
    /// of the source resource rendered faint at the row's right.
    pub row_aside: Option<String>,
    /// Optional `sort-property="<property>"`: a `text` property of the source
    /// resource the rows are ordered by (case-insensitive, missing values
    /// last, then id). Wins over `order`.
    pub sort_property: Option<String>,
}

/// What an `incoming-links` list is partitioned by, from
/// `group-by="<name>"`.
#[derive(Debug, Clone, PartialEq)]
pub enum IncomingGroupBy {
    /// The reserved word `via`: group by the matching link property, in the
    /// spec's declared property order.
    Via,
    /// Any other value: a `list_value` (list-node `seq` order) or `text`
    /// (alphabetical) property of the linking (source) resource.
    Property(String),
}

/// The grouped `incoming-links` layout, from `layout=`. A
/// closed vocabulary; meaningful only with `group-by`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IncomingLayout {
    /// A grid of per-group cards (HdM's Work page: performances by decade).
    Cards,
    /// Per-group sections over a multi-column list (HdM's Person page: works
    /// by role). The default.
    #[default]
    Columns,
}

/// The grouping options of an `incoming-links` list.
#[derive(Debug, Clone, PartialEq)]
pub struct IncomingGrouping {
    /// What the rows are partitioned by.
    pub by: IncomingGroupBy,
    /// The grouped layout; [`IncomingLayout::Columns`] when `layout=` is
    /// absent.
    pub layout: IncomingLayout,
    /// Optional `note-label="<chrome-id>"`: a project `chrome { string }`
    /// **id** whose text (which must contain `{n}`, the row count) renders as
    /// a note on the list's section head. An undeclared id is the Warning
    /// `UnknownIncomingLinksNoteLabel`; a declared one lacking `{n}` in any
    /// language is the Error `IncomingLinksTextMissingPlaceholder`.
    pub note_label: Option<String>,
}

/// The `incoming-links limit=` default in grouped mode: a safety ceiling above any real fan-in
/// (HdM's largest is 105), so group counts are true. Counts are exact only when
/// no overall cut occurred (`rows.len() == total`).
pub const GROUPED_INCOMING_LIMIT_CEILING: u32 = 1000;

/// The built-in default for `incoming-links limit=` when the attribute is
/// absent. Chosen to comfortably exceed every shipped project's largest
/// reverse-link list (Incunabula's biggest border-strip fan-in is 23 rows)
/// while still bounding a future large-fan-in class (e.g. HdM's Person ←
/// Performance) from rendering unboundedly.
pub const DEFAULT_INCOMING_LINKS_LIMIT: u32 = 50;

/// The hydrated result of
/// `query::incoming_links`/`ProjectReads::incoming_links`: the capped `rows` (at
/// most `limit` long) alongside the UNCAPPED `total` match count, so the
/// renderer can show an honest "N of M" line when `rows.len() < total`. `total
/// == 0` iff `rows` is empty.
#[derive(Debug, Clone, PartialEq)]
pub struct IncomingLinks {
    /// The capped rows, in the spec's declared `order`.
    pub rows: Vec<IncomingLink>,
    /// The full, uncapped match count.
    pub total: usize,
}

/// One property display label: a property name and its per-language
/// translations.
///
/// Maps from `labels { property "name" { de "…" en "…" } }`.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyLabel {
    pub property: String,
    pub translations: Vec<(String, String)>, // (lang, text), order preserved
}

/// One class display label: a class name, its per-language (singular)
/// translations, and an optional per-language plural.
///
/// Maps from `labels { class "name" { de "…" en "…" plural { de "…" en
/// "…" } } }`. The `plural` child is optional — `plural` is `Vec::new()` when undeclared, which
/// is not an error; `labels::resolve_class_plural` falls back
/// to the singular `translations` (and beyond that, humanization) in that
/// case. There is deliberately no `plural` field on `PropertyLabel` —
/// pluralization is a class-noun concept only.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassLabel {
    pub class: String,
    pub translations: Vec<(String, String)>, // (lang, text), order preserved
    pub plural: Vec<(String, String)>,       // (lang, text), order preserved; empty ⇒ undeclared
}

// ── Page ──────────────────────────────────────────────────────────────────────

/// A validated page: route, extracted params, and typed body components.
#[derive(Debug, PartialEq)]
pub struct Page {
    /// String identifier used by other components to reference this page.
    pub id: String,
    /// Route pattern, e.g. `"/"` or `"/books/:id"`.
    pub route: String,
    /// Route parameters extracted from the route pattern (`:param` → `"param"`).
    /// Populated from the route at parse time; used by the `$param` checks.
    pub params: Vec<String>,
    /// Typed, validated body components in render order. Lowered from the raw
    /// KDL component nodes by the validator; components that fail validation
    /// are dropped (and diagnostics pushed) rather than included.
    pub body: Vec<Component>,
    /// The figure-block slot of each `body` component, index-aligned with
    /// `body` (and so with the render-time `ComponentData` slice): lowering
    /// pushes or drops a component and its slot together. Read it through
    /// [`Page::figure_slot`], which treats a missing entry as no slot.
    pub figure_slots: Vec<Option<FigureSlot>>,
}

impl Page {
    /// The figure-block slot of `body[index]`, if it has one.
    pub fn figure_slot(&self, index: usize) -> Option<&FigureSlot> {
        self.figure_slots.get(index).and_then(Option::as_ref)
    }

    /// The index of the narrative a slotted `body[index]` renders into: the
    /// nearest `narrative` before it. `None` when `body[index]` has no slot
    /// or no narrative precedes it (boot rejects the latter).
    pub fn figure_host(&self, index: usize) -> Option<usize> {
        self.figure_slot(index)?;
        self.body
            .get(..index)?
            .iter()
            .rposition(|c| matches!(c, Component::Narrative { .. }))
    }
}

/// Where a per-project component renders inside the narrative before it on
/// its page: after the prose of the `## … {#section}` heading, at full width
/// (the slot attributes `section=`, `count=`, `more-label=` + `more-route=`,
/// dsp-incubator cpe/docs/adr/0011-per-project-components.md). A component with a slot renders only
/// there, never at its own body position.
#[derive(Debug, Clone, PartialEq)]
pub struct FigureSlot {
    /// The heading id (`{#id}`) of a `##` in every variant of the nearest
    /// preceding `narrative`.
    pub section: String,
    /// A chrome-string id: the count line under the block.
    pub count: Option<String>,
    /// The "more" link under the block.
    pub more: Option<FigureMore>,
}

/// The "more" link of a [`FigureSlot`]: both halves are required together.
#[derive(Debug, Clone, PartialEq)]
pub struct FigureMore {
    /// A chrome-string id: the link text.
    pub label: String,
    /// A declared page route, project-relative (`"/strips"`).
    pub route: String,
}

// ── Component enum ────────────────────────────────────────────────────────────

/// One typed component variant. The six generic v1 body components (see
/// `V1_COMPONENTS` in `validate.rs`) are represented, plus the per-project
/// custom components `Timeline`, `WoodcutCases`, `EditionCards` and
/// `StripRow` (`page` itself is the
/// `Page` struct, not a body `Component` variant, so it is not counted among
/// either the seven v1 components in `dsp-incubator cpe/docs/COMPONENT_LIBRARY.md` or this enum's
/// variants).
///
/// Mapping to `dsp-incubator cpe/docs/COMPONENT_LIBRARY.md`:
/// - `Narrative` → the `narrative` component.
/// - `ResourceList` → `resource-list`.
/// - `ResourceDetail` → `resource-detail`.
/// - `CompoundViewer` → `compound-viewer`.
/// - `FacetPanel` → `facet-panel`.
/// - `SearchBar` → `search-bar`.
/// - `Timeline` → `timeline` (a per-project custom component registered in the `(project, name)`
///   registry, see dsp-incubator cpe/docs/adr/0011-per-project-components.md — not one of the six
///   generic v1 components above).
/// - `WoodcutCases` → `woodcut-cases`, `EditionCards` → `edition-cards`, `StripRow` → `strip-row`
///   (Incunabula's, per-project).
#[derive(Debug, PartialEq)]
pub enum Component {
    /// Renders a Markdown file from the project's `narrative/` directory.
    ///
    /// Maps `narrative file="…" companion="…" side="…"`.
    Narrative {
        /// Relative path under `narrative/`, e.g. `"home.md"`.
        file: String,
        /// `companion=`: this page's override of
        /// `SiteConfig::narrative_companion`. `Some` only on a page's first
        /// narrative (`NarrativeLayoutNotFirst`).
        companion: Option<NarrativeCompanion>,
        /// `side=`: this page's override of `SiteConfig::narrative_side`,
        /// under the same first-narrative rule.
        side: Option<NarrativeSide>,
    },

    /// Paginated, sortable, optionally facetable list of Resources of one Class.
    ///
    /// Maps `resource-list class="…" { … }`.
    ResourceList {
        /// Allows `facet-panel`/`search-bar` to bind to this list.
        id: Option<ResourceListId>,
        /// The Class to list (required).
        class: String,
        /// Static `where` filters (AND-combined).
        wheres: Vec<WhereClause>,
        /// Sort criteria.
        sorts: Vec<Sort>,
        /// Pagination strategy; defaults to `PerPage(20)`.
        pagination: Pagination,
        /// The row gloss (annotated rows). `None` ⇒ no gloss line. Maps
        /// `gloss property="…"` ([`Gloss::Property`]) or a `gloss { part …;
        /// teaser … }` block ([`Gloss::Composed`]).
        gloss: Option<Gloss>,
        /// Row chips (`.rl-annot__meta`) with an optional per-chip shape
        /// override. Absent `kind` ⇒ the cardinality rule: a
        /// single-valued list property renders as a `.term` chip, a
        /// multi-valued one as `.keyword` chips (same single-vs-multi split as
        /// `resource-detail`). Empty ⇒ no chips. Maps repeatable
        /// `chip property="…" kind="term"|"keyword"`.
        chips: Vec<ChipSpec>,
        /// Chrome-string **id** override for the "Name A–Z" ordering label
        /// — id only, never literal text (`search-bar`'s
        /// `placeholder`/`scope` precedent). `None`, or an id that does not
        /// resolve, ⇒ the `ChromeMsg::OrderNameAsc` default.
        /// Maps `order-name-label="…"`.
        order_name_label: Option<String>,
        /// Chrome-string **id** override for the "Name Z–A" ordering label
        /// — id only, never literal text. `None`, or an id that does
        /// not resolve, ⇒ the `ChromeMsg::OrderNameDesc` default.
        /// Maps `order-name-desc-label="…"`.
        order_name_desc_label: Option<String>,
        /// Chrome-string **id** override for the built-in "As curated"
        /// (`default`) ordering row's label — id only, never literal text.
        /// `None`, or an id that does not resolve, ⇒ the
        /// `ChromeMsg::OrderCurated` default. No boot check ties the label to
        /// what the list's `sort` actually orders by: that is the author's
        /// responsibility. Maps `order-default-label="…"`.
        order_default_label: Option<String>,
        /// The property the built-in `name`/`name-desc` rows (`?sort=name`,
        /// `?sort=name-desc`) order by, in place of `name`. `None` ⇒ `name`.
        /// The wire vocabulary is unchanged. Boot-validated for existence on
        /// the class like `sort`. Maps `order-name-property="…"`.
        order_name_property: Option<String>,
        /// Project-declared ordering options (`order-by` children), in config order. Each adds a
        /// `?sort=<key>` to this list's vocabulary on top of the built-ins. Empty ⇒ the
        /// built-in ordering only.
        order_options: Vec<OrderOption>,
        /// The `link` property naming the embedded-media set for each row
        /// (browse-row thumbs). `None` ⇒ no thumb. Maps `media="…"`.
        /// Boot-validated for existence/link-type/image-likeness (same
        /// treatment as `where`/`sort`), and for consistency against the
        /// class→media-property map built from `resource-detail media="…"`
        /// declarations (a mismatch is a boot hard-fail; a list-only
        /// declaration with no matching detail-page entry is fine).
        media: Option<String>,
        /// Grid-mode opt-in (`grid=#true`). `false` when absent. Requires
        /// `media` to be `Some` on the same `resource-list` —
        /// `check_grid_without_media` hard-fails a `grid: true` with no
        /// `media`.
        grid: bool,
        /// The grid tile's aspect ratio (`tile-ratio="W:H"`). `None` ⇒ the
        /// stylesheet's default. Requires `grid: true`
        /// (`check_tile_ratio_without_grid`).
        tile_ratio: Option<TileRatio>,
    },

    /// Full view of one Resource.
    ///
    /// Maps `resource-detail id="…" { show-properties … }`.
    ResourceDetail {
        /// Resource id or route param reference (`"$id"`).
        /// Required when invoked outside a `:id` route.
        id: Option<String>,
        /// Explicit property display ordering.
        /// If `None`, properties render in their data-defined order.
        show_properties: Option<Vec<String>>,
        /// The property name whose value to use as the page `<h1>` title.
        /// Falls back to the resource id when `None` or the property is absent.
        title_property: Option<String>,
        /// The Class this detail page renders, used to build the class→route map
        /// for clickable cross-resource links. Optional and backward-compatible:
        /// absent ⇒ `None` (this page's resources are simply not link targets).
        class: Option<String>,
        /// The `link` property naming this resource's embedded-media set (the
        /// record viewer). `None` ⇒ no media. Maps `media="…"`. Boot-validated:
        /// the property must exist on the resolved class, be `link`-typed, and
        /// target a class whose resources carry `representations` rows. `media`
        /// requires a resolvable class — either this component's own `class`
        /// attribute, or (for a literal `id`) the data-looked-up class of that
        /// resource — a class-less `$param` page with no `class=` hard-fails
        /// (`UnresolvableMediaClass`).
        media: Option<String>,
        /// Captioned property groups (`group label="…" "a" "b" …` children), in declaration
        /// order. Each renders as a `section.pgroup` after the flat `div.props`.
        /// Empty ⇒ no group renders. When non-empty and `show_properties` is
        /// `None`, the flat list is **empty** (not "every property") — see
        /// `render_resource_detail`.
        groups: Vec<PropertyGroup>,
        /// Per-resource introductions (`narrative-for "<id>" file="…"`
        /// children), in declaration order. When the rendered resource's id
        /// matches an entry, its narrative file renders as `div.record__intro`
        /// inside `.record__body`. Empty ⇒ no intro on any record.
        narrative_for: Vec<NarrativeFor>,
        /// The reverse-link list (`incoming-links …` child). `None` ⇒ no list.
        incoming_links: Option<IncomingLinksSpec>,
        /// The optional header slots (`alt-title-property=`,
        /// `author-property=`, an `imprint { part … }` child). `None` when
        /// none is declared ⇒ today's header exactly.
        head: Option<HeadSlots>,
        /// Whether this record is paired with a same-`id` `compound-viewer`
        /// on its page — a **boot fact**, set once per page in
        /// `check_and_lower_pages` from `pairs_with_navigator`
        /// (dsp-incubator cpe/docs/adr/0012-one-member-navigator-declared-layout.md),
        /// never a config attribute and never a per-request sibling scan.
        /// `true` ⇒ the record drops the `.rv-contents` member list (the
        /// navigator is the one member list) and, when the navigator is a
        /// populated `layout="pages"` reader, renders it inside its own article
        /// (`render_body`); `false` otherwise.
        navigator: bool,
    },

    /// Viewer for a compound Resource (paginated prev/next member view).
    ///
    /// Maps `compound-viewer source="…"`.
    CompoundViewer {
        /// Resource id or route param reference (`"$id"`). Points at the
        /// compound parent.
        source: String,
        /// Optional member→image `link` property naming each member's scan
        /// (`compound-viewer media="…"`). Absent ⇒ no member image is rendered.
        /// When set, the reader shows the current member's first linked Image
        /// as a plain, non-interactive `figure.cv-member__scan > img`, and the
        /// property is suppressed from the member's property list.
        /// Boot-validated like `resource-detail`'s `media` against the distinct
        /// classes of the compound's members, taken from the data
        /// (`check_compound_viewer_media`).
        media: Option<String>,
        /// The `pages` reader's page picker (`compound-viewer picker="…"`,
        /// book-page Choice E), in the pager row's centre at `total >= 2`.
        /// `None` ⇒ no picker: the centre keeps its count. Valid only with
        /// `layout="pages"` (`PagesLayoutOnly`).
        picker: Option<PickerMode>,
        /// The reader's declared layout (`compound-viewer layout="…"`).
        /// Required: an absent or unknown value is a boot Error and the
        /// component is skipped — no default layout exists.
        layout: CompoundLayout,
        /// How the member index is shown (`compound-viewer index="…"`). Required, like `layout`.
        index: MemberIndex,
        /// Whether this compound-viewer is paired with a same-`id`
        /// `resource-detail` on its page — the mirror image of
        /// `ResourceDetail.navigator`, a **boot fact** set once per page in
        /// `check_and_lower_pages` from the SAME `pairs_with_navigator` call
        /// call, never a config attribute and never a per-request sibling scan. `true` ⇒ the
        /// empty (`total == 0`) branch renders the "no members" sentence alone — the paired
        /// `resource-detail` already owns the title/eyebrow/properties, so showing
        /// `cfg.parent`'s head+props here would duplicate them (dsp-incubator
        /// cpe/docs/adr/0012-one-member-navigator-declared-layout.md). `false` ⇒ a standalone
        /// compound-viewer with no paired record, e.g. Alice's `/wonderland`, degrades to the
        /// parent-head-and-properties branch.
        paired: bool,
        /// The member's outgoing links shown as one captioned row under the
        /// `pages` pager (`compound-viewer { member-links label="…" "p" … }`).
        /// `None` ⇒ no row. Valid only with `layout="pages"`
        /// (`PagesLayoutOnly`): the `contents` layout already lists every
        /// member property.
        member_links: Option<MemberLinks>,
        /// Whether the reader renders the shown page's annotation register
        /// (`compound-viewer annotations=#true`, dsp-incubator
        /// cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md). `false` ⇒ no
        /// register and no `#cv-annotations` patch target, so the markup is
        /// unchanged. Valid only with `layout="pages"` (`PagesLayoutOnly`),
        /// and rendered only when the reader is paired with its record (the
        /// book composition); an unpaired reader shows no register.
        annotations: bool,
    },

    /// Facet controls bound to a sibling `resource-list` by id.
    ///
    /// Maps `facet-panel target="…" { facet … }`.
    FacetPanel {
        /// The `id` of a `resource-list` in the same page.
        target: String,
        /// One or more facet controls.
        facets: Vec<Facet>,
    },

    /// Free-text search across configured text/richtext properties.
    ///
    /// Maps `search-bar target="…" searches="…" "…" scope="…" { … }`.
    SearchBar {
        /// The `id` of a `resource-list` in the same page.
        target: String,
        /// Property names to search.
        searches: Vec<String>,
        /// Optional id of a project-declared `chrome { string }` holding the
        /// input's placeholder text — an id, not a literal, resolved at
        /// render time via `chrome_lookup`. Mirrors `scope`'s "id, not
        /// literal" convention below.
        placeholder: Option<String>,
        /// Optional id of a project-declared `chrome { string }` holding the
        /// scope note (e.g. "Searches name and description") — an id, not a
        /// literal, resolved at render time via `chrome_lookup`. Mirrors
        /// `placeholder`'s "id, not literal" convention; like `placeholder`, it is not
        /// validated for id-existence.
        scope: Option<String>,
    },

    /// Incunabula's timeline (a per-project component): one marker
    /// per resource of `class`, positioned on a fixed year axis by a `date`
    /// property and grouped into lanes by a `list_value` property.
    ///
    /// Maps `timeline class="…" date-property="…" lane-property="…"
    /// extent-property="…" axis-start=<int> axis-end=<int>`. The axis bounds
    /// are explicit config, never derived from the data.
    Timeline {
        /// The Class whose resources become markers (required).
        class: String,
        /// The `date`-typed property positioning each marker (required).
        date_property: String,
        /// The `list_value`-typed property naming each marker's lane (required).
        lane_property: String,
        /// Optional `integer`-typed property for the extent segment of the
        /// marker's accessible name. `None` ⇒ the segment is omitted.
        extent_property: Option<String>,
        /// Inclusive first year of the axis (required).
        axis_start: i32,
        /// Last year of the axis (required); boot-validated to exceed
        /// `axis_start` by at least 3 (one three-year grid cell).
        axis_end: i32,
    },

    /// Incunabula's woodcut crop figures (a per-project component):
    /// per case, the impressions of one woodblock cut from their pages' scans
    /// by annotation, or two pages joined by a page-level link.
    ///
    /// Maps `woodcut-cases section="…" layout="…" media="…" { case … {
    /// crop … } page-case … { page … } }`. Always rendered in a figure-block
    /// slot (`section=` is required).
    WoodcutCases {
        /// The annotated resources' `link` property to their image (`media=`).
        media: String,
        layout: CaseLayout,
        /// In config order; never empty.
        cases: Vec<ShowcaseCase>,
    },

    /// Incunabula's edition cards (a per-project component): one
    /// card per book, its cover plate, a language eyebrow, its title and
    /// imprint, linking to the book's reader.
    ///
    /// Maps `edition-cards section="…" media="…" { book "<id>"
    /// lang-label="…"; imprint { part property="…" unit="…" } }`. Always
    /// rendered in a figure-block slot (`section=` is required).
    EditionCards {
        /// The books' `link` property to their cover image (`media=`).
        media: String,
        /// In config order; never empty.
        books: Vec<EditionCard>,
        /// The imprint line, the record header's part grammar. Empty ⇒ no
        /// imprint line.
        imprint: Vec<GlossPart>,
    },

    /// Incunabula's border-strip row (a per-project component): the
    /// resources of `class` used most often, as a row of tiles.
    ///
    /// Maps `strip-row section="…" class="…" media="…" uses-property="…"
    /// min-uses=<int> limit=<int>`. Always rendered in a figure-block slot.
    StripRow {
        class: String,
        /// The resources' `link` property to their tile image (`media=`).
        media: String,
        /// The integer property counting a resource's uses; the row lists
        /// resources with at least `min_uses`, most-used first, then by id.
        uses_property: String,
        min_uses: i64,
        /// At most this many tiles, 1 to 100 (boot-checked in lowering).
        limit: u32,
    },
}

impl Component {
    /// The `(project, name)` registry name of a per-project component
    /// (`validate::project_components`, `component::project`), `None` for the
    /// generic v1 components. The IR variant carries no name string, so this
    /// is how the render dispatch and the success-arm data check find a
    /// variant's registry row.
    /// Exhaustive on purpose: a new variant must say which kind it is.
    #[doc(hidden)]
    pub fn registry_name(&self) -> Option<&'static str> {
        match self {
            Component::Narrative { .. }
            | Component::ResourceList { .. }
            | Component::ResourceDetail { .. }
            | Component::CompoundViewer { .. }
            | Component::FacetPanel { .. }
            | Component::SearchBar { .. } => None,
            Component::Timeline { .. } => Some("timeline"),
            Component::WoodcutCases { .. } => Some("woodcut-cases"),
            Component::EditionCards { .. } => Some("edition-cards"),
            Component::StripRow { .. } => Some("strip-row"),
        }
    }
}

// ── Showcase figures (Incunabula) ─────────────────────────────────────────────

/// How a `woodcut-cases` block sets its cases (`layout=`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseLayout {
    /// One case per row, crops 24rem tall.
    Edition,
    /// One case of four crops in one row, 24rem tall.
    Four,
    /// Two cases per row, crops 17rem tall (whole pages 15rem); no detail
    /// crop.
    Compact,
}

impl CaseLayout {
    /// Parse a `layout=` value; `None` for anything else.
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "edition" => Some(Self::Edition),
            "four" => Some(Self::Four),
            "compact" => Some(Self::Compact),
            _ => None,
        }
    }

    /// The `layout=` value, also the block's class modifier.
    #[doc(hidden)]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Edition => "edition",
            Self::Four => "four",
            Self::Compact => "compact",
        }
    }
}

/// One case of a `woodcut-cases` block.
#[derive(Debug, Clone, PartialEq)]
pub enum ShowcaseCase {
    /// `case`: one woodblock's impressions, each cut from its page by an
    /// annotation's box.
    Crops {
        /// The Schramm catalogue number, shown in the id line.
        schramm: Option<u32>,
        /// The case title (`lang="de"`, reformatted from the description).
        title: Option<String>,
        remark: Option<ShowcaseRemark>,
        /// Two to four, in row order.
        crops: Vec<ShowcaseCrop>,
        /// A crop backing the remark, set under the row (never `Compact`).
        detail: Option<ShowcaseCrop>,
    },
    /// `page-case`: two whole pages joined by the page-level link `link`.
    Pages {
        link: String,
        remark: Option<ShowcaseRemark>,
        /// Exactly two Page resource ids, in row order.
        pages: Vec<String>,
    },
}

impl ShowcaseCase {
    /// The case's DOM key (`figure#case-<key>`): its first crop's annotation
    /// id, or a page case's link id. Unique on its page (boot-checked).
    pub fn key(&self) -> &str {
        match self {
            ShowcaseCase::Crops { crops, .. } => crops.first().map_or("", |c| c.annotation.as_str()),
            ShowcaseCase::Pages { link, .. } => link,
        }
    }

    /// The case's remark source, if any.
    pub fn remark(&self) -> Option<&ShowcaseRemark> {
        match self {
            ShowcaseCase::Crops { remark, .. } | ShowcaseCase::Pages { remark, .. } => remark.as_ref(),
        }
    }
}

/// One crop of a case: an annotation id and its optional `alt` (verbatim
/// from the page's description; absent ⇒ a composed fallback).
#[derive(Debug, Clone, PartialEq)]
pub struct ShowcaseCrop {
    pub annotation: String,
    pub alt: Option<String>,
}

/// Where a case's remark comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum ShowcaseRemark {
    /// `remark-link=`: the link's own comment, verbatim.
    Link(String),
    /// `remark=`: an authored chrome-string id.
    Chrome(String),
    /// `remark-excerpt=`: a verbatim line of a case page's description.
    Excerpt(String),
}

/// The data a `woodcut-cases` block renders from, pre-fetched by the store:
/// every crop and detail annotation it names, every link it names, and every
/// page its page cases name.
#[derive(Debug, Clone, PartialEq)]
pub struct WoodcutCasesPayload {
    pub crops: Vec<AnnotationCrop>,
    pub links: Vec<AnnotationLink>,
    pub pages: Vec<PagePlate>,
}

/// One card of an `edition-cards` block: a book id and the chrome-string id
/// of its language eyebrow (`lang-label=`; `None` ⇒ no eyebrow).
#[derive(Debug, Clone, PartialEq)]
pub struct EditionCard {
    pub book: String,
    pub lang_label: Option<String>,
}

/// The data an `edition-cards` block renders from, pre-fetched by the store:
/// each card's book that exists, in card order.
#[derive(Debug, PartialEq)]
pub struct EditionCardsPayload {
    pub books: Vec<EditionBook>,
}

/// One book of an edition card, hydrated, with its cover image (the first
/// image representation of its media property's first target).
#[derive(Debug, PartialEq)]
pub struct EditionBook {
    pub resource: Resource,
    pub cover: Option<CropImage>,
}

/// The data a `strip-row` renders from, pre-fetched by the store.
#[derive(Debug, PartialEq)]
pub struct StripRowPayload {
    /// The row's resources, most-used first, then by id.
    pub strips: Vec<Resource>,
    /// Each strip's tile image, keyed by resource id; a strip with none is
    /// absent.
    pub thumbs: std::collections::HashMap<String, MediaThumb>,
    /// Every resource of the row's class, not only the listed ones: the
    /// `{n}` of the slot's "all N" more link.
    pub class_total: usize,
}

// ── Timeline payload (Incunabula) ─────────────────────────────────────────────

/// One lane of a `timeline`: a node of the lane property's list vocabulary,
/// in vocabulary (`list_nodes.seq`) order.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneNode {
    /// The list node's id (`target_id` of the lane property's rows).
    pub node_id: String,
    /// The node's label in the requested language (else any), if one exists.
    pub label: Option<String>,
}

/// The data a `timeline` renders from, pre-fetched by the store: every resource
/// of the timeline's class, plus the lane vocabulary in `seq` order.
#[derive(Debug, PartialEq)]
pub struct TimelinePayload {
    /// The class's resources (fully hydrated).
    pub resources: ResourceListPage,
    /// The lane vocabulary, in `list_nodes.seq` order.
    pub lanes: Vec<LaneNode>,
}

// ── Row gloss ─────────────────────────────────────────────────────────────────

/// A `resource-list` row gloss.
#[derive(Debug, Clone, PartialEq)]
pub enum Gloss {
    /// `gloss property="…"`: the property's first language-picked
    /// text/richtext/date value.
    Property(String),
    /// `gloss { part property="…" unit="…"; teaser property="…" }`: every
    /// part's language-picked values joined with ", ", the parts joined with
    /// " · " (a part with no value is skipped), then the teaser appended
    /// after " — " in the same line.
    Composed {
        /// At least one, each naming a distinct property.
        parts: Vec<GlossPart>,
        /// The authored text appended after the parts.
        teaser: Option<String>,
    },
}

/// One property's contribution to a composed [`Gloss`].
#[derive(Debug, Clone, PartialEq)]
pub struct GlossPart {
    /// The property whose values this part renders.
    pub property: String,
    /// A chrome-string **id** whose template (e.g. `"{n} Seiten"`) wraps a
    /// numeric value, `{n}` standing for the number. Only valid on an
    /// `integer`/`decimal` part. `None` ⇒ the bare value.
    pub unit: Option<String>,
}

// ── Record header slots ───────────────────────────────────────────────────────

/// A `resource-detail`'s optional header slots, rendered inside
/// `header.record__head` after the title, in this order. Each slot property
/// leaves the implicit flat property list, as `title-property` does; an
/// explicit `show-properties`/`group` that still names one renders it twice.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HeadSlots {
    /// `alt-title-property="…"`: a text/richtext property, values joined ", ".
    pub alt_title: Option<String>,
    /// `author-property="…"`: a text/richtext property, values joined ", ".
    pub author: Option<String>,
    /// `imprint { part property="…" unit="…" … }`: the composed-gloss part
    /// grammar — values joined ", ", parts " · ", an empty part skipped.
    /// Empty ⇒ no imprint.
    pub imprint: Vec<GlossPart>,
}

// ── Row chip shape ────────────────────────────────────────────────────────────

/// One configured row chip: the source property, plus an optional shape
/// override. Maps repeatable `chip property="…" kind="…"` on `resource-list`.
#[derive(Debug, Clone, PartialEq)]
pub struct ChipSpec {
    /// The property whose list value(s) render as this chip.
    pub property: String,
    /// Explicit shape override. `None` ⇒ derive from cardinality (today's
    /// rule): a single-valued list property renders `.term`, multi-valued
    /// renders `.keyword` per label.
    pub kind: Option<ChipKind>,
}

/// The closed set of reader layouts a `compound-viewer layout="…"` attribute
/// may name. Deliberately no `Default` impl: the
/// attribute is required, so no lowering path may invent a layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompoundLayout {
    /// `layout="contents"` — the text-first reader (member head, props, pager),
    /// optionally beside a contents rail.
    Contents,
    /// `layout="pages"` — the scan-first reader (requires `media=`).
    Pages,
}

/// The closed set of member-index presentations a `compound-viewer
/// index="…"` attribute may name. Deliberately no `Default` impl: the attribute
/// is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberIndex {
    /// `index="rail"` — the sticky contents rail beside the reader.
    Rail,
    /// `index="collapsed"` — a closed `details.cv-index` after the reader.
    Collapsed,
    /// `index="none"` — no member list at all.
    Off,
}

/// How the `pages` reader's page picker groups a book's pages
/// (`compound-viewer picker="…"`, book-page Choice E). Absent ⇒ no picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    /// `picker="numbered"` — runs of 20 pages by position.
    Numbered,
    /// `picker="signatures"` — the book's own collation, read from its page
    /// labels (`a1r` → gathering `a`, leaf 1, recto); a book whose labels
    /// mostly do not parse falls back to numbered runs, per book.
    Signatures,
}

/// The closed set of row-chip shapes a `chip kind="…"` attribute may name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipKind {
    Term,
    Keyword,
}

// ── Scalar value type ─────────────────────────────────────────────────────────

/// A KDL scalar value, decoupled from `kdl::KdlValue` (per dsp-incubator
/// cpe/docs/adr/0001-crate-layout.md boundary).
///
/// Used both in the raw parser output and in the typed IR's filter clauses,
/// so it lives here (public) and is re-used by `config.rs` internally.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Integer(i128),
    Float(f64),
    Bool(bool),
    Null,
}

// ── Child / support types ─────────────────────────────────────────────────────

/// A stable, string-typed identifier for a `resource-list` component.
/// Newtype so `target` references can be typed against it.
pub type ResourceListId = String;

/// A `where` filter child node on a `resource-list`.
///
/// Maps the three `where` node shapes in `docs/src/cpe/config-language.md`:
/// - `where property="…" equals=<v>` → `Equals`
/// - `where property="…" in="<v1>" "<v2>" …` → `In`
/// - `where property="…" range-start=<v> range-end=<v>` → `Range`
///
/// Additionally, `TargetIn` and `Search` are injected at request time (never
/// stored in the config IR):
/// - `TargetIn { property, target_ids }` → `EXISTS (… target_id IN (…))`, injected by the store's
///   page builder (`query::page`) for `list_value`/`link` facet filtering.
/// - `Search { table, query }` → `r.id IN (SELECT … FROM <table> WHERE <table> MATCH ?)`, injected
///   by the store's page builder for free-text `search-bar` narrowing.
#[derive(Debug, Clone, PartialEq)]
pub enum WhereClause {
    /// `where property="name" equals=<value>`
    Equals { property: String, value: Value },
    /// `where property="name" in="v1" "v2" …`
    In { property: String, values: Vec<Value> },
    /// `where property="name" range-start=<v> range-end=<v>`
    Range { property: String, start: Option<Value>, end: Option<Value> },
    /// `list_value` / `link` membership filter: the resource has a property
    /// row whose `target_id` is one of `target_ids`. Used by `list`-kind facets.
    ///
    /// Injected at request time by the store's page builder; never appears in
    /// the static config IR. An empty `target_ids` is a defensive dead branch
    /// (the page builder must not push an empty-ids clause — caller contract).
    TargetIn { property: String, target_ids: Vec<String> },
    /// Free-text FTS5 prefix search: the resource has a matching row in the
    /// named FTS5 virtual table. `table` is the boot-sanitized, resolved FTS5
    /// table name (see `query::search_index_name`) — mirrors `TargetIn`
    /// carrying resolved `target_ids` rather than raw config. `query` is the
    /// RAW, un-tokenized user search string (request data); `query::build_where`
    /// tokenizes it into a MATCH expression via `fts_match_expr` at bind time,
    /// emitting no clause at all if the query has zero usable terms (see
    /// `fts_match_expr`'s doc comment). Injected at request time by the store;
    /// never appears in the static config IR (mirrors `TargetIn`).
    Search { table: String, query: String },
}

/// The property or field a `sort` clause orders by.
///
/// Only `Property` is ever produced from static KDL config — `sort
/// property="…"` in `config.kdl` always lowers to `SortKey::Property`
/// (`lower_sort` in `validate.rs`). `Class` and `Id` arrive only at request
/// time via the `?sort=` query control, parsed by [`Sort::parse_wire`]; they
/// never appear in the static config IR. Mirrors the request-time-only
/// status of [`WhereClause::TargetIn`] elsewhere in this file.
#[derive(Debug, Clone, PartialEq)]
pub enum SortKey {
    /// Sort by a named EAV property's value (language-aware — see
    /// `query::list_resource_ids`).
    Property(String),
    /// Sort by the resource's Class. Request-time only.
    Class,
    /// Sort by the resource's id. Request-time only.
    Id,
}

/// A `sort` child node on a `resource-list`, or a request-time `?sort=`
/// override lowered by [`Sort::parse_wire`].
///
/// Maps `sort property="name"` or `sort property="name" direction="desc"`
/// (always `key: SortKey::Property(_)` when built from config).
#[derive(Debug, Clone, PartialEq)]
pub struct Sort {
    pub key: SortKey,
    pub direction: SortDir,
}

impl Sort {
    /// Parse a `?sort=` query-string value into a `Sort`.
    ///
    /// Mirrors the `parse_config_str` idiom used by `Accent`/`Background`/
    /// `Typeface` in `theme.rs` — named `parse_wire` (not `from_str`) since
    /// this parses a runtime wire value, not a `config.kdl` string.
    ///
    /// Mapping: `"name"` → `Property("name") Asc`; `"name-desc"` →
    /// `Property("name") Desc`; `"class"` → `Class Asc`; `"id"` → `Id Asc`.
    /// Returns `None` for `"default"`, empty, or any unrecognised value; the
    /// caller treats `None` as "fall back to the page's configured sort".
    pub fn parse_wire(s: &str) -> Option<Sort> {
        match s {
            "name" => Some(Sort {
                key: SortKey::Property("name".to_owned()),
                direction: SortDir::Asc,
            }),
            "name-desc" => Some(Sort {
                key: SortKey::Property("name".to_owned()),
                direction: SortDir::Desc,
            }),
            "class" => Some(Sort { key: SortKey::Class, direction: SortDir::Asc }),
            "id" => Some(Sort { key: SortKey::Id, direction: SortDir::Asc }),
            _ => None,
        }
    }
}

/// Sort direction for a `sort` child node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    /// Ascending order (default when `direction` is omitted).
    Asc,
    /// Descending order (`direction="desc"`).
    Desc,
}

/// One project-declared ordering option on a `resource-list`: a named `?sort=<key>` that orders
/// the list by one property.
///
/// Maps `order-by key="…" property="…" direction="asc"|"desc" label="…"`
/// children, in config order. `?sort=` never takes a raw property name — it
/// takes a declared `key`, so the wire vocabulary stays closed per list.
/// Boot-validated: `key` is `[a-z0-9-]+` (`InvalidOrderKey`), not a built-in
/// (`ReservedOrderKey`) and unique within its list (`DuplicateOrderKey`);
/// `property` gets the `where`/`sort` two-tier existence check.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderOption {
    /// The `?sort=` wire token that selects this option.
    pub key: String,
    /// The property to order by.
    pub property: String,
    /// The direction; `SortDir::Asc` when `direction=` is absent.
    pub direction: SortDir,
    /// A project `chrome { string }` **id** for the option's row label —
    /// id-only, never literal text (the `order-name-label` convention).
    /// Required, and an undeclared id is the Error `UnknownOrderLabel`: the
    /// engine has no default word for an arbitrary property.
    pub label: String,
}

impl OrderOption {
    /// The `Sort` this option selects: `SortKey::Property(property)` in the
    /// option's direction (no dedicated `SortKey` variant).
    pub fn sort(&self) -> Sort {
        Sort {
            key: SortKey::Property(self.property.clone()),
            direction: self.direction,
        }
    }
}

/// Pagination strategy for a `resource-list`.
///
/// Maps the mutually-exclusive `per-page <n>` / `limit <n>` child nodes.
/// `per-page` is the default paginated form; `limit` is for non-paginated
/// cases. Both present simultaneously is a `PaginationConflict` error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pagination {
    /// Show `n` items per page (default 20).
    PerPage(u32),
    /// Show at most `n` items with no pagination UI.
    Limit(u32),
}

impl Default for Pagination {
    fn default() -> Self {
        Self::PerPage(20)
    }
}

/// Hydrated render-payload for one facet group (one property's options).
///
/// Produced by `ProjectReads::facet_options` and carried in
/// `ComponentData::Facets(Vec<FacetGroup>)` for the renderer. `Clone`: the store's page builder
/// (`query::page`) pre-resolves a page's facet groups once (so the `ResourceList` arm can
/// gate `has_active_facets` on the SAME resolved selection data the
/// `FacetPanel` arm renders, regardless of which comes first in `page.body`)
/// and needs an owned copy for the latter's `ComponentData::Facets` payload.
#[derive(Debug, Clone, PartialEq)]
pub struct FacetGroup {
    /// Raw property name — used as the query-param key in toggle hrefs.
    pub property: String,
    /// Humanized display label for the group heading.
    pub label: String,
    /// Facet kind — determines the control type rendered.
    pub kind: FacetKind,
    /// Ordered list of options (sorted by count DESC, label, node id).
    pub options: Vec<FacetOption>,
}

/// One option within a `FacetGroup`.
///
/// `value` is the list node id (the query-param value in toggle hrefs);
/// `label` is the display label (node label when available, else `value`);
/// `count` is the number of resources of the target class that have this node;
/// `selected` is set when this value is present in the current query string.
#[derive(Debug, Clone, PartialEq)]
pub struct FacetOption {
    /// List node id — the query-param value in toggle hrefs.
    pub value: String,
    /// Display label: resolved node label, else node id as fallback (resolved
    /// at build time in `ProjectReads::facet_options`; renderer does no derivation).
    pub label: String,
    /// Count of distinct resources of the target class that have this node.
    pub count: usize,
    /// Whether this option is currently selected (present in the query string).
    pub selected: bool,
}

/// Whether at least one option across any group is currently selected.
///
/// The ONE facet-activity predicate every caller that needs to know
/// "are the facets actually narrowing this list" should use — engine-side
/// (the binary has the same resolved `FacetGroup` data too, through
/// `any_facet_selected`, which `dsp-incubator cpe/engine` re-exports).
/// `FacetOption::selected` is set in
/// `ProjectReads::facet_options` from whether the query-param value matches a
/// REAL node id that occurs in the data — a hand-crafted bogus value
/// (`?role=NOPE`) never sets any option's `selected`, so this predicate
/// correctly reports "not active" for it. Do not derive "facets active" from
/// raw query-param presence (e.g. `facet_wheres` non-emptiness in the page
/// builder) — that conflates "a facet param was supplied" with "a facet param
/// actually matched something", which is exactly what this predicate is for.
pub fn any_facet_selected(groups: &[FacetGroup]) -> bool {
    groups.iter().any(|g| g.options.iter().any(|o| o.selected))
}

/// A `facet` child node on a `facet-panel`.
///
/// Maps `facet property="name" kind="kind"`.
#[derive(Debug, Clone, PartialEq)]
pub struct Facet {
    pub property: String,
    pub kind: FacetKind,
}

/// The closed set of facet control kinds.
///
/// Sourced from `dsp-incubator cpe/docs/COMPONENT_LIBRARY.md` §`facet-panel` → Facet `kind`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetKind {
    /// Checkboxes listing distinct values.
    Text,
    /// Tree control for list-valued properties.
    List,
    /// Checkboxes grouped by decade.
    DateDecade,
    /// Checkboxes grouped by year.
    DateYear,
    /// Yes/no toggle.
    Boolean,
    /// Min/max sliders for integer/decimal/date properties.
    Range,
}

impl FacetKind {
    /// Parse from the string form used in `config.kdl`.
    ///
    /// Named `parse_config_str` (not `from_str`) to avoid confusion with the
    /// `std::str::FromStr` trait, which this type does not implement.
    pub fn parse_config_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "list" => Some(Self::List),
            "date-decade" => Some(Self::DateDecade),
            "date-year" => Some(Self::DateYear),
            "boolean" => Some(Self::Boolean),
            "range" => Some(Self::Range),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tall_starts_at_three_times_the_width() {
        assert!(!is_tall(1, 2));
        assert!(!is_tall(100, 299));
        assert!(is_tall(1, 3));
        assert!(is_tall(1, 9));
        assert!(!is_tall(0, 5), "an unsized box is not tall");
    }

    #[test]
    fn tile_ratio_displays_as_a_css_ratio_and_knows_tall() {
        let strip = TileRatio::parse("1:9").expect("valid");
        assert_eq!(strip.to_string(), "1 / 9");
        assert!(strip.is_tall());
        assert!(TileRatio::parse("1:3").expect("valid").is_tall());
        let page = TileRatio::parse("3:4").expect("valid");
        assert_eq!(page.to_string(), "3 / 4");
        assert!(!page.is_tall());
    }

    // ── Sort::parse_wire ──────────────────────────────────────────────────────

    #[test]
    fn parse_wire_name_is_property_asc() {
        assert_eq!(
            Sort::parse_wire("name"),
            Some(Sort {
                key: SortKey::Property("name".to_owned()),
                direction: SortDir::Asc,
            })
        );
    }

    #[test]
    fn parse_wire_name_desc_is_property_desc() {
        assert_eq!(
            Sort::parse_wire("name-desc"),
            Some(Sort {
                key: SortKey::Property("name".to_owned()),
                direction: SortDir::Desc,
            })
        );
    }

    #[test]
    fn parse_wire_class_is_class_asc() {
        assert_eq!(
            Sort::parse_wire("class"),
            Some(Sort { key: SortKey::Class, direction: SortDir::Asc })
        );
    }

    #[test]
    fn parse_wire_id_is_id_asc() {
        assert_eq!(Sort::parse_wire("id"), Some(Sort { key: SortKey::Id, direction: SortDir::Asc }));
    }

    #[test]
    fn parse_wire_default_returns_none() {
        assert_eq!(Sort::parse_wire("default"), None);
    }

    #[test]
    fn parse_wire_bogus_returns_none() {
        assert_eq!(Sort::parse_wire("bogus"), None);
    }

    #[test]
    fn parse_wire_empty_returns_none() {
        assert_eq!(Sort::parse_wire(""), None);
    }

    // ── date_is_range ─────────────────────────────────────────────────────────

    #[test]
    fn date_is_range_table() {
        use DatePrecision::{Day, Month, Year};
        let cases: &[(&str, Option<&str>, DatePrecision, bool, &str)] = &[
            ("1471-03-21", Some("1471-03-21"), Day, false, "equal bounds"),
            ("1471", Some("1471"), Year, false, "equal year bounds"),
            ("1471-01-01", Some("1471-12-31"), Year, false, "year-truncated-equal bounds"),
            ("1491-02-01", Some("1491-02-28"), Month, false, "month-truncated-equal bounds"),
            ("1475-01-01", Some("1499-12-31"), Year, true, "a real year range"),
            ("1491-02-01", Some("1491-03-31"), Month, true, "a real month range"),
            ("1491-07-27", Some("1491-07-28"), Day, true, "a real day range"),
            ("1471-03-21", None, Day, false, "end = None"),
            ("1471", None, Year, false, "end = None, year"),
        ];
        for (start, end, precision, want, why) in cases {
            assert_eq!(
                date_is_range(start, *end, *precision),
                *want,
                "{why}: {start} / {end:?} @ {precision:?}"
            );
        }
    }

    #[test]
    fn calendar_from_schema_str() {
        assert_eq!(Calendar::from_schema_str("julian"), Some(Calendar::Julian));
        assert_eq!(Calendar::from_schema_str("gregorian"), Some(Calendar::Gregorian));
        assert_eq!(Calendar::from_schema_str("JULIAN"), None);
        assert_eq!(Calendar::from_schema_str(""), None);
    }
}

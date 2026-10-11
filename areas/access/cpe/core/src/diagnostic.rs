//! Diagnostic infrastructure: span types, severity, error kinds, and the
//! validation report accumulator. This is the **leaf module** — it has zero
//! crate-internal imports and no `kdl`/`miette` types in its public surface.
//! Conversion from `kdl`/`miette` spans happens at the call site in `config.rs`
//! and (eventually) `validate.rs`, never here.

// ── Span ─────────────────────────────────────────────────────────────────────

/// A byte range into a source string, decoupled from `miette::SourceSpan`.
/// `offset` is the byte index of the first character; `len` is the byte count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub offset: usize,
    pub len: usize,
}

// ── Severity ──────────────────────────────────────────────────────────────────

/// Whether a diagnostic blocks loading or is advisory.
///
/// An `Error` blocks boot. A `Warning` does not: it is logged on the serve
/// path, and `--check` does not print it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

// ── DiagnosticKind ────────────────────────────────────────────────────────────

/// Which config-declared surface a `labels {}` entry names — a property (its
/// `<dt>` label / facet-group heading) or a class (the resource-detail eyebrow
/// / resource-list surf-head title). Carried on both label-related
/// `DiagnosticKind` variants so a single message-formatting call site can
/// render "property" or "class" without two parallel kind families.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LabelTarget {
    Property,
    Class,
}

/// One variant per category of diagnostic, config-internal or data-dependent.
/// Tests assert on the *kind set* so they remain stable when message wording
/// changes.
///
/// `#[non_exhaustive]`, so that a new kind does not break a match in another
/// crate.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticKind {
    /// A top-level KDL node is none of `site`, `labels`, `page` and `remodel`,
    /// or a second `remodel` (the block occurs at most once).
    UnexpectedTopLevelNode,
    /// The `site` block has no `title` child.
    MissingSiteTitle,
    /// The `site` block has no `languages` child or the list is empty.
    MissingSiteLanguages,
    /// The language list is present but does not include a required language
    /// (e.g. `de` or `en`).
    MissingRequiredLanguage,
    /// A field that requires a specific value type (e.g. strings for
    /// `languages`) received a different type.
    InvalidFieldType,
    /// `default-language` names a language not in the `languages` list.
    UnknownDefaultLanguage,
    /// Two pages share the same `id`.
    DuplicatePageId,
    /// Two pages share the same `route`.
    DuplicatePageRoute,
    /// A page `route` has a first path segment beginning with `_`. The engine
    /// reserves the `_`-prefixed route namespace for engine-registered routes
    /// (e.g. `/_static/…`, `/_assets/…`, the `_<component>` fragment
    /// segments); a config page must not claim one, or it would silently
    /// collide with the reserved route.
    ReservedRoute,
    /// Two `resource-list` components on the same page share the same `id`.
    DuplicateResourceListId,
    /// A component name is not one of the eight recognised component kinds.
    UnknownComponent,
    /// A required attribute is absent on a component.
    MissingRequiredAttribute,
    /// An attribute value has the wrong type (e.g. integer where a string is
    /// expected).
    InvalidAttributeType,
    /// A `facet` child's `kind` attribute is not a known facet kind string.
    UnknownFacetKind,
    /// A component declares both `per-page` and `limit`, which are mutually
    /// exclusive pagination strategies.
    PaginationConflict,
    /// A `facet-panel` or `search-bar` `target` attribute names a
    /// `resource-list` that does not exist on the same page.
    UnresolvedTarget,
    /// A `$param` reference in a string attribute value names a route
    /// parameter not declared in the page's `route`.
    UndeclaredRouteParam,
    /// A `chrome` block is missing translations for one or more declared
    /// languages.
    IncompleteChromeTranslation,
    /// A narrative component references a file that is not present in the
    /// bundle's `narrative/` directory.
    NarrativeFileNotFound,
    /// A narrative path contains `..` or a path separator, which would escape
    /// the `narrative/` directory.
    InvalidNarrativePath,
    /// A site `cover-image` path contains `..` or a leading `/`, which would
    /// escape the project's `assets/` directory.
    InvalidCoverImagePath,
    /// A site `cover-image` references a file that is not present in the
    /// bundle's `assets/` directory.
    CoverImageNotFound,
    /// The project id matches one of the engine-reserved ids (e.g. `_static`).
    ReservedProjectId,
    /// The project id is not valid kebab-case (`^[a-z0-9][a-z0-9-]*$`).
    InvalidProjectId,
    /// A `resource-detail` component references a literal resource id that
    /// does not exist in the project's data. This is a hard-fail: the page
    /// would always render "not found" at runtime, which is almost certainly
    /// a configuration error.
    ///
    /// Only fires for literal ids (not `$param` references).
    UnknownResource,
    /// A property name listed in `show-properties` or `title-property` on a
    /// `resource-detail` component is not present on any resource of the
    /// target resource's class. The page still renders (the property is
    /// silently absent), so this is advisory (`Severity::Warning`).
    ///
    /// Only fires for literal ids where the resource exists.
    SparseProperty,
    /// A key or value in the `theme {}` block is not recognised.
    ///
    /// `key` is the theme key (e.g. `"accent"` or `"background"`), and
    /// `value` is the string the author supplied. `allowed` lists the
    /// recognised values for that key so the error message is actionable.
    /// Fires for both an unknown key (unknown key; allowed is empty) and an
    /// unknown value for a known key. This is a hard-fail — an unrecognised
    /// knob value indicates a likely config authoring error and the knob
    /// cannot default safely to something the author didn't intend.
    UnknownThemeValue { key: String, value: String, allowed: Vec<String> },
    /// A closed-vocabulary `site {}` child (`narrative-companion`,
    /// `narrative-side`, `text-align`) holds a string outside its vocabulary.
    /// Hard-fail, config-internal.
    ///
    /// `key` is the child's name, `value` what was written, `allowed` the
    /// vocabulary.
    UnknownSiteValue { key: String, value: String, allowed: Vec<String> },
    /// A `narrative` that is not its page's first sets `companion=` or
    /// `side=`. The page's layout is read from the first narrative only, so
    /// the attribute would be silently ignored. Hard-fail, config-internal.
    NarrativeLayoutNotFirst { page: String },
    /// A `properties` row has a `value_type` whose required value column is
    /// NULL. For example, a row with `value_type='text'` must have a non-NULL
    /// `value_text`; a row with `value_type='date'` must have a non-NULL
    /// `date_start`. This is a hard-fail: the data is malformed and
    /// `build_property_value` would produce an incorrect result if the row
    /// were returned at request time.
    ///
    /// Documented in `dsp-incubator cpe/docs/SCHEMA.md` §"Constraints not
    /// enforced in DDL" as a boot-validator check. Enforced here so
    /// `build_property_value` can use `expect()` calls backed by this boot-time
    /// invariant.
    MalformedPropertyRow,
    /// An `annotations` or `annotation_links` row breaks a rule the DDL does
    /// not enforce (dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md,
    /// `dsp-incubator cpe/docs/SCHEMA.md` §"Constraints not enforced in DDL"): a box
    /// outside the image or with no area, a polygon whose points are missing, unparseable,
    /// fewer than three or outside the box, points on a non-polygon, a colour that is not
    /// lowercase `#rrggbb`, or a link with no target. One diagnostic per violated rule.
    /// Hard-fail: the annotation renderers rely on these invariants.
    MalformedAnnotation,
    /// A non-NULL `resources.ark` or `annotations.ark` is not a plain DSP data
    /// ARK URL, `https://ark.dasch.swiss/ark:/72163/1/<4 uppercase hex>/<id>`
    /// with an id of `[A-Za-z0-9_=]+` (`dsp-incubator cpe/docs/SCHEMA.md`
    /// §"Constraints not enforced in DDL"): a version ARK, another scheme or
    /// host, a trailing `/`, a lowercase shortcode or `''`. One diagnostic per
    /// row. Hard-fail, so a renderer may put a stored ARK into an `href` and
    /// rely on its prefix.
    MalformedDataArk,
    /// A `nav` item's `route` does not match any declared page route.
    ///
    /// Per `docs/src/cpe/config-language.md` §nav, a nav item's
    /// `route` must match a declared page route exactly (the renderer's
    /// `is_active` comparison is exact string equality). A nav item pointing at
    /// a nonexistent route would never be highlighted as active and would
    /// produce a broken link. This is a hard-fail.
    UnresolvedNavRoute,
    /// Two pages both declare the same `class` on their `resource-detail`
    /// component with a parameterised (`:param`) route. The class→route map
    /// requires an unambiguous binding: one class ↔ one route template. When
    /// two competing pages exist, the engine cannot determine which route to
    /// use for link construction, so this is a boot hard-fail.
    ///
    /// `class` is the duplicated class name; `first_route` and
    /// `second_route` are the two competing page route templates (in
    /// discovery order).
    AmbiguousClassRoute { class: String, first_route: String, second_route: String },
    /// A `link`-typed property value has a `target_id` that does not exist in
    /// the `resources` table. The renderer already degrades gracefully to a
    /// `span.unresolved` plain-text label at request time, so this is
    /// advisory (`Severity::Warning`) rather than a hard-fail.
    ///
    /// `source_id` is the resource that owns the dangling link property;
    /// `target_id` is the absent target id that was referenced.
    UnknownLinkTarget { source_id: String, target_id: String },
    /// A `resource-list`'s `class` attribute names a class that does not exist
    /// in the data. The page would always render an empty list at runtime, which
    /// is almost certainly a configuration error. This is a hard-fail.
    ///
    /// `class` is the class name declared on the `resource-list`;
    /// `page` is the page route where the component appears.
    UnknownResourceListClass { class: String, page: String },
    /// A property named in a `where` or `sort` clause on a `resource-list`
    /// does not exist anywhere in the data (no resource of any class has it).
    /// The filter or sort would always silently produce wrong results. This is
    /// a hard-fail.
    ///
    /// `property` is the property name; `page` is the page route; `class` is
    /// the list's declared class.
    UnknownResourceListProperty { property: String, class: String, page: String },
    /// A property named in a `where` or `sort` clause on a `resource-list`
    /// exists somewhere in the data but is not present on any resource of the
    /// list's declared class. The filter or sort will silently produce no
    /// effect for that class. This is advisory (`Severity::Warning`).
    ///
    /// `property` is the property name; `page` is the page route; `class` is
    /// the list's declared class.
    SparseResourceListProperty { property: String, class: String, page: String },
    /// A `compound-viewer` component's `source` is a literal Resource id that
    /// has no `compound_parts` rows. The viewer will always show its empty
    /// state at runtime, which is almost certainly a configuration error
    /// (the source is not a compound, or the compound has no members yet).
    ///
    /// This is advisory (`Severity::Warning`) — an empty/non-compound source
    /// renders the empty state and does not crash.
    ///
    /// Only fires for literal ids (not `$param` references, which are
    /// runtime-data-dependent).
    ///
    /// `source` is the literal Resource id declared on the `compound-viewer`.
    EmptyCompoundSource { source: String },
    /// A `facet` declares a `kind` that is valid but not implemented
    /// (everything except `list`). The page cannot function until the
    /// facet kind is supported. This is a hard-fail.
    ///
    /// `kind` is the raw kind string from the facet; `page` is the page route.
    UnsupportedFacetKind { kind: String, page: String },
    /// A `list`-kind facet's `property` exists on the target class but none of
    /// its rows have `value_type = 'list_value'`. The facet would return no
    /// options at runtime (the data is the wrong type). This is a hard-fail.
    ///
    /// `property` is the facet property name; `kind` is the facet kind string;
    /// `value_type` is the actual (incompatible) value type found;
    /// `target_class` is the resource-list's class; `page` is the page route.
    IncompatibleFacetKind {
        property: String,
        kind: String,
        value_type: String,
        target_class: String,
        page: String,
    },
    /// A facet's `property` does not exist anywhere in the data (no resource of
    /// any class has it). The facet would produce no options at runtime, which
    /// is almost certainly a configuration error. This is a hard-fail.
    ///
    /// `property` is the property name; `target_class` is the resource-list's
    /// class; `page` is the page route.
    UnknownFacetProperty { property: String, target_class: String, page: String },
    /// A facet's `property` exists somewhere in the data but is not present on
    /// any resource of the target class. The facet will return no options for
    /// this class at runtime (the property is on another class). This is
    /// advisory (`Severity::Warning`) — the page still loads.
    ///
    /// `property` is the property name; `target_class` is the resource-list's
    /// class; `page` is the page route.
    SparseFacetProperty { property: String, target_class: String, page: String },
    /// A `search-bar`'s `searches` property does not exist anywhere in the
    /// data (no resource of any class has it). The search would silently
    /// match nothing for that term at runtime, which is almost certainly a
    /// configuration error. This is a hard-fail. Modeled on
    /// `UnknownFacetProperty`.
    ///
    /// `property` is the searchable property name; `target_class` is the
    /// resource-list's class; `page` is the page route.
    UnknownSearchProperty { property: String, target_class: String, page: String },
    /// A `search-bar`'s `searches` property exists on the target class but
    /// with a value type that is neither `text` nor `richtext` — FTS5
    /// indexing requires one of those two. The search would silently never
    /// match against this property at runtime. This is a hard-fail. Modeled
    /// on `IncompatibleFacetKind`, but search-bar has no `kind` concept (it
    /// has exactly one behaviour), so there is no `kind` field here.
    ///
    /// `property` is the searchable property name; `value_type` is the
    /// actual (incompatible) value type found; `target_class` is the
    /// resource-list's class; `page` is the page route.
    IncompatibleSearchProperty {
        property: String,
        value_type: String,
        target_class: String,
        page: String,
    },
    /// A `search-bar`'s `searches` property exists somewhere in the data but
    /// is not present on any resource of the target class. The search will
    /// never match on this property for this class at runtime (the property
    /// is on another class). This is advisory (`Severity::Warning`) — the
    /// page still loads. Modeled on `SparseFacetProperty`.
    ///
    /// `property` is the searchable property name; `target_class` is the
    /// resource-list's class; `page` is the page route.
    SparseSearchProperty { property: String, target_class: String, page: String },
    /// A `resource-list`'s `where` clause compares a scalar value against a
    /// property whose actual stored `value_type`(s) on the list's class never
    /// populate the column that value's kind would compare against (e.g. an
    /// integer `equals` against a property stored only as `value_text`). The
    /// filter can never match by type alone — this is purely structural, not
    /// data-content-dependent. The page still renders (an always-empty
    /// filtered result), so this is advisory (`Severity::Warning`).
    ///
    /// Only fires for a property confirmed present on the list's class
    /// (`UnknownResourceListProperty`/`SparseResourceListProperty` already
    /// cover absent/sparse properties); applies to `Equals` and each `Range`
    /// bound, not `TargetIn` (membership filters compare `target_id`, not a
    /// scalar column) or `In` (mixed-type lists are a separate, deferred
    /// concern — see `build_where`).
    ///
    /// `property` is the where-clause property name; `class` is the
    /// resource-list's declared class; `page` is the page route.
    IncompatibleWhereValue { property: String, class: String, page: String },
    /// A `facet`'s `property` shadows one of the query-string keys the host
    /// reads for its own purposes (theme/appearance overrides, pagination,
    /// sort, i18n, …). Facet values are read back at request time via the
    /// property name used as a query key, so a collision would silently
    /// shadow (or be shadowed by) the host's own handling of that key. This is
    /// a hard-fail: the page would misbehave at runtime in a way boot
    /// validation can and should catch.
    ///
    /// Pure config check — fires regardless of data, for every facet whose
    /// `property` is in the reserved set, independent of `kind`.
    ///
    /// `property` is the facet's declared property name; `page` is the page
    /// route.
    ReservedFacetProperty { property: String, page: String },
    /// The query itself failed to execute during a data-dependent validator
    /// check (not "the checked condition is true/false" — the underlying
    /// SQLite query call returned `Err`). Distinguishes an internal query
    /// failure from the feature-specific kind the check would otherwise push
    /// on a genuine finding, so a `kind`-based consumer isn't misled into
    /// thinking (e.g.) a resource-list class was confirmed missing when the
    /// check couldn't even run. This should be unreachable on a freshly
    /// loaded, schema-valid database; if it fires, the database itself is
    /// broken.
    ///
    /// `context` is a short human label naming which check failed (e.g.
    /// "property-row consistency", "link target existence", "compound
    /// membership").
    InternalQueryError { context: String },
    /// A page's body declares more than one instance of a component the
    /// engine assumes is a page-level singleton. Only `resource-detail`
    /// (the record viewer emits the fixed `id="record-figure"` — a second
    /// instance would already emit a duplicate id on the rendered page)
    /// and `facet-panel` (the `_facets` fragment handler locates only
    /// the first `FacetPanel` on the page, so a second panel's target list
    /// would never update) are checked. This is a boot-time hard-fail —
    /// letting it through would produce broken HTML or silently-inert UI at
    /// render time.
    ///
    /// `component` is the colliding component name (`"resource-detail"`,
    /// `"facet-panel"`, `"resource-list"`, or `"search-bar"`); `page` is the
    /// page route.
    DuplicateSingletonComponent { component: String, page: String },
    /// A `labels` block entry is missing a translation for one or more declared
    /// site languages. Cloned from `IncompleteChromeTranslation`'s completeness
    /// contract — every declared label must supply every declared language.
    /// This is a hard-fail.
    ///
    /// `target` distinguishes a `property` label from a `class` label; `key`
    /// is the property or class name the label was declared under; `lang` is
    /// the missing declared site language.
    IncompleteLabelTranslation { target: LabelTarget, key: String, lang: String },
    /// A `labels` block entry names a property or class that is absent from
    /// the project's data entirely. The label is simply unused at render time
    /// (falls back to nothing extra — the site just never displays it), so
    /// this is advisory. Mirrors the sparse-property warning family; catches
    /// typos without blocking boot. This is advisory (`Severity::Warning`).
    ///
    /// `target` distinguishes a `property` label from a `class` label; `key`
    /// is the property or class name the label was declared under.
    UnknownLabel { target: LabelTarget, key: String },
    /// A `labels { class "…" }` entry declares no `plural { … }` block.
    /// `labels::resolve_class_plural` then falls back to the capitalized
    /// SINGULAR label on all four plural-bearing surfaces (the search-bar
    /// no-match copy, the browse breadcrumb/eyebrow, `resource-detail`'s class
    /// crumb, and the 404 "browse all" exit), which reads wrong in every
    /// language with an inflected plural (for Incunabula, "Alle Buch
    /// durchsuchen"). The site still renders, so this is advisory
    /// (`Severity::Warning`) rather than a hard-fail.
    ///
    /// `class` is the class name the label was declared under.
    MissingClassPlural { class: String },
    /// A project's desk header row (brand, nav and controls in one row) is
    /// estimated wider than the room the desk breakpoint leaves it
    /// (`validate::header_budget`). The project title ellipsises there, so
    /// this is advisory. Config-internal: checked once per declared language
    /// in `validate_inner`, before the `validate_with_data` fork.
    ///
    /// `lang` is the language code; `estimated_px` the estimate and
    /// `budget_px` the room, both in CSS px.
    HeaderRowOverBudget { lang: String, estimated_px: u32, budget_px: u32 },
    /// A `media="…"` attribute names a property that does not exist anywhere in
    /// the data. The plate/thumb would always be empty at runtime, which is
    /// almost certainly a configuration error. This is a hard-fail.
    ///
    /// `property` is the declared name; `class` is the resolved class;
    /// `page` is the page route.
    UnknownMediaProperty { property: String, class: String, page: String },
    /// A `media` property exists on the resolved class but with a value type
    /// other than `link` — the record-viewer/thumb mechanism only works over
    /// `link` properties (a host→Image reference). This is a hard-fail.
    ///
    /// `property` is the declared name; `class` is the resolved class;
    /// `value_type` is the actual (incompatible) value type found; `page` is
    /// the page route.
    IncompatibleMediaProperty {
        property: String,
        class: String,
        value_type: String,
        page: String,
    },
    /// A `media` property exists somewhere in the data but is not present on
    /// any resource of the resolved class. The plate/thumb will always be
    /// empty for this class at runtime (the property is on another class).
    /// This is advisory (`Severity::Warning`) — the page still loads.
    ///
    /// `property` is the declared name; `class` is the resolved class;
    /// `page` is the page route.
    SparseMediaProperty { property: String, class: String, page: String },
    /// A `media` property is `link`-typed and present on the resolved class,
    /// but its resolvable link targets belong to a class whose resources
    /// carry no `representations` rows at all — i.e. the target class is not
    /// "image-like". The plate/thumb mechanism has nothing to render. This is
    /// a hard-fail.
    ///
    /// `property` is the declared name; `class` is the resolved (host)
    /// class; `target_class` is the offending link-target class; `page` is
    /// the page route.
    MediaPropertyTargetNotImagelike {
        property: String,
        class: String,
        target_class: String,
        page: String,
    },
    /// A `resource-detail`'s `media` attribute was declared without a
    /// resolvable class: the component has no `class="…"` attribute, and its
    /// `id` is either absent or a `$param` reference (so no literal-id
    /// class lookup is possible either). `media` on `resource-detail` requires
    /// a resolvable class to validate and to contribute to the
    /// class→media-property map. This is a hard-fail.
    ///
    /// `page` is the page route.
    UnresolvableMediaClass { page: String },
    /// A `media` declaration conflicts with the class→media-property map
    /// entry already registered for the same class (registration order:
    /// `resource-detail`, then `compound-viewer`, then `resource-list`, which
    /// only checks). The declarations must agree — a mismatch would mean the
    /// record viewer, the reader and the browse-row thumb disagree about which
    /// property carries the embedded-media set for the class. Hard-fail.
    ///
    /// `class` is the shared class; `registered_property` is the property
    /// already registered for this class (e.g. by a `resource-detail`
    /// binding); `declared_property` is the conflicting property named by
    /// this declaration (e.g. a `resource-list`); `page` is this
    /// declaration's page route. Field names are source-neutral (not
    /// `detail_property`/`list_property`) because a `compound-viewer`
    /// registration is neither a "detail" nor a "list".
    ConflictingMediaProperty {
        class: String,
        registered_property: String,
        declared_property: String,
        page: String,
    },
    /// A `resource-list` declares `grid=#true` (grid-mode opt-in) but has no
    /// `media="…"` attribute. Grid mode renders a wall of image tiles — with
    /// no `media` property there is nothing to source the tile images from,
    /// so a grid of pure text tiles would never be a designed surface (the
    /// text tile is a *fallback for sparse images within* an image-bearing
    /// class, not a mode on its own). This is a hard-fail, checked by a
    /// config-internal pages walk (`check_grid_without_media`) so both the
    /// data-validated and raw-fallback validation paths inherit it.
    ///
    /// `class` is the resource-list's declared class; `page` is the page route.
    GridWithoutMedia { class: String, page: String },
    /// A `resource-list` declares `tile-ratio="…"` without `grid=#true`: the
    /// ratio shapes grid tiles only, so it would be read and never used.
    /// Config-internal (`check_tile_ratio_without_grid`).
    ///
    /// `class` is the resource-list's declared class; `page` is the page route.
    TileRatioWithoutGrid { class: String, page: String },
    /// A single page declares both a `compound-viewer` and a paginating
    /// `resource-list` (`Pagination::PerPage` — which is also the default when
    /// neither `per-page` nor `limit` is given). Both read the `?page=` query
    /// key with different meanings: the compound-viewer addresses a member
    /// (`?page=<member-id|name|prefix>`), the resource-list pages its browse
    /// (`?page=N`). On one page the two would silently fight over the same
    /// key, so this is a config-internal hard-fail, checked in
    /// `check_and_lower_pages` before the success/raw fork. A `resource-list`
    /// with `limit` (no pager) alongside a compound-viewer is fine.
    ///
    /// `page` is the page route.
    CompoundViewerWithPagedList { page: String },
    /// A `resource-detail` names the same property in two `group` children,
    /// or in both a `group` and `show-properties`. The property would render
    /// twice on one record — a config-internal mistake no data can cause, so
    /// this is a hard-fail, checked in `check_and_lower_pages` before the
    /// success/raw fork.
    ///
    /// `property` is the repeated property name; `page` is the page route.
    DuplicateGroupedProperty { property: String, page: String },
    /// A `resource-detail { group label="…" }` names a chrome id that is not
    /// a declared `chrome { string }` of this project. The group still renders,
    /// captioned with the engine fallback `ChromeMsg::PropertyGroupTitle`, so
    /// this is advisory (`Severity::Warning`) — the `UnknownLabel` /
    /// `MissingClassPlural` precedent. Config-internal: checked in
    /// `validate_inner` inside the `site_result` guard, before the
    /// `validate_with_data` fork.
    ///
    /// `label` is the undeclared chrome id; `page` is the page route.
    UnknownGroupLabel { label: String, page: String },
    /// A `resource-detail { group empty="…" }` names a chrome id that is not
    /// a declared `chrome { string }` of this project. The group renders as if
    /// `empty=` were unset, so this is advisory (`Severity::Warning`) — the
    /// `UnknownGroupLabel` tier. Config-internal: checked beside
    /// `UnknownGroupLabel`.
    ///
    /// `label` is the undeclared chrome id; `page` is the page route.
    UnknownGroupEmptyLabel { label: String, page: String },
    /// A `resource-detail { group links-label="…" }` names a chrome id that
    /// is not a declared `chrome { string }` of this project. The links row
    /// still renders, captioned with the engine fallback
    /// `ChromeMsg::AnnoRelated`, so this is advisory (`Severity::Warning`) —
    /// the `UnknownGroupLabel` tier. Config-internal: checked beside
    /// `UnknownGroupLabel`.
    ///
    /// `label` is the undeclared chrome id; `page` is the page route.
    UnknownGroupLinksLabel { label: String, page: String },
    /// A `resource-detail { narrative-for "<id>" file="…" }` names a resource
    /// id that does not exist in the data. The intro simply never attaches —
    /// every record renders as if it had none, a legal state — so this is
    /// advisory (`Severity::Warning`), the `UnknownLabel` / `UnknownLinkTarget`
    /// shape. Data-dependent: checked in both `validate_with_data` arms. The id
    /// is always a literal, so it is checkable even on a `$param` page.
    ///
    /// `resource_id` is the unknown id; `page` is the page route.
    UnknownNarrativeForResource { resource_id: String, page: String },
    /// A `resource-detail` declares two `narrative-for` entries for the same
    /// resource id. Config-internal — which file should win is undecidable — so
    /// a hard-fail, checked in `check_and_lower_pages` before the success/raw
    /// fork.
    ///
    /// `resource_id` is the repeated id; `page` is the page route.
    DuplicateNarrativeFor { resource_id: String, page: String },
    /// A `compound-viewer layout="pages"` declares no `media=`. The `pages`
    /// layout is scan-first, so without a member image property it has nothing
    /// to show. Config-internal — a hard-fail, checked in
    /// `check_and_lower_pages` before the success/raw fork.
    ///
    /// `page` is the page route.
    PagesLayoutWithoutMedia { page: String },
    /// A `compound-viewer` feature only the `pages` layout renders, declared
    /// on another layout (book-page Choice E): the `picker` attribute (it
    /// lives in the pages reader's pager row), the `member-links` child (its
    /// row sits under that pager) or the `annotations` attribute (the
    /// register belongs to the pages reader's page). None could ever take
    /// effect, so each is rejected rather than ignored. Config-internal — a
    /// hard-fail, checked in `check_and_lower_pages` before the success/raw
    /// fork, like `PagesLayoutWithoutMedia`.
    ///
    /// `feature` is `"picker"`, `"member-links"` or `"annotations"`; `page`
    /// is the page route.
    PagesLayoutOnly { feature: String, page: String },
    /// A `resource-detail` with `media=` whose `id` expression is identical to
    /// a sibling `compound-viewer`'s `source` (dsp-incubator
    /// cpe/docs/adr/0012-one-member-navigator-declared-layout.md): the navigator owns
    /// the compound's images, so the record would show them a second time. Config-internal — a
    /// hard-fail, checked in `check_and_lower_pages` before the success/raw fork. Known limit:
    /// the record's image comes from the class map, so a `resource-detail
    /// media=` for the same class on *another* page is not caught.
    ///
    /// `page` is the page route.
    CompoundMediaOwnership { page: String },
    /// A page declares a `compound-viewer layout="pages"` **and** any
    /// `resource-detail` with `media=` (dsp-incubator
    /// cpe/docs/adr/0012-one-member-navigator-declared-layout.md). Both mount the record
    /// viewer's unit, whose signals (`rail`, `expand`, `plate`, the Adjust values, the zoom
    /// flags) are page-global, so the two viewers would drive each other. Config-internal — a
    /// hard-fail, checked in `check_and_lower_pages` before the success/raw fork, and reported
    /// only when `CompoundMediaOwnership` has not already fired for the page
    /// (one mistake, one diagnostic).
    ///
    /// `page` is the page route.
    MultipleImageViewers { page: String },
    /// A `resource-detail` declares more than one `incoming-links` child. At
    /// most one reverse-link list per record, so a config-internal hard-fail,
    /// raised by `lower_resource_detail` (which is not given the page route —
    /// the diagnostic's span locates it).
    DuplicateIncomingLinks,
    /// A `resource-detail { incoming-links … }` child names no link property
    /// — zero positional string args. The list would query nothing, so a
    /// config-internal hard-fail, raised by `lower_resource_detail`
    /// (span-located, like `DuplicateIncomingLinks`).
    IncomingLinksWithoutProperty,
    /// A `resource-detail { incoming-links label="…" }` names a chrome id that
    /// is not a declared `chrome { string }` of this project. The list still
    /// renders, captioned with the source class's capitalised plural, so this
    /// is advisory (`Severity::Warning`) — the `UnknownGroupLabel` tier.
    /// Config-internal: checked in `validate_inner` inside the `site_result`
    /// guard, before the `validate_with_data` fork.
    ///
    /// `label` is the undeclared chrome id; `page` is the page route.
    UnknownIncomingLinksLabel { label: String, page: String },
    /// An `incoming-links class="…"` names a class that does not exist in the
    /// data. The class is a query input — like a `resource-list`'s `class` — so
    /// this is a hard-fail. Data-dependent: checked in both
    /// `validate_with_data` arms.
    ///
    /// `class` is the unknown source class; `page` is the page route.
    UnknownIncomingLinksClass { class: String, page: String },
    /// An `incoming-links` property does not exist anywhere in the data. A
    /// query input, so a hard-fail. Data-dependent: checked in both
    /// `validate_with_data` arms.
    ///
    /// `property` is the unknown property; `class` the declared source class;
    /// `page` the page route.
    UnknownIncomingLinksProperty { property: String, class: String, page: String },
    /// An `incoming-links` property exists in the data but is carried by no
    /// resource of the declared source class — "sparse" in the codebase's
    /// sense: absent from the whole class, not uneven across it. The list would
    /// always be empty for it, so advisory (`Severity::Warning`).
    /// Data-dependent: checked in both arms.
    ///
    /// `property` is the property; `class` the declared source class; `page`
    /// the page route.
    SparseIncomingLinksProperty { property: String, class: String, page: String },
    /// An `incoming-links` property is stored on the source class with a
    /// `value_type` other than `link`. Only `link` rows carry a `target_id`, so
    /// the list could never match — a hard-fail. Data-dependent: checked in
    /// both `validate_with_data` arms.
    ///
    /// `property` is the property; `value_type` the (lexicographically
    /// smallest) offending stored type; `class` the declared source class;
    /// `page` the page route.
    IncomingLinksPropertyNotLink {
        property: String,
        value_type: String,
        class: String,
        page: String,
    },
    /// An `incoming-links` property is `link`-typed and present on the
    /// declared source class, but its resolvable targets never actually
    /// belong to the page's own host class — the list this
    /// property drives will always be empty on this page. Advisory
    /// (`Severity::Warning`): every existing tier already passed, so this is
    /// a design smell, not a query-input mistake. Data-dependent: checked in
    /// both `validate_with_data` arms.
    ///
    /// `property` is the property; `class` the declared source class;
    /// `host_class` is the resolved class of the page's own `resource-detail`
    /// (the class hosting the incoming-links list); `page` the page route.
    IncomingLinksPropertyMissesHost {
        property: String,
        class: String,
        host_class: String,
        page: String,
    },
    /// An `incoming-links note-label="…"` names a chrome id that is not a
    /// declared `chrome { string }` of this project. The list
    /// still renders, without its note, so advisory (`Severity::Warning`) —
    /// the `UnknownIncomingLinksLabel` tier. Config-internal: checked in
    /// `validate_inner` (`check_group_labels`) inside the `site_result`
    /// guard, before the `validate_with_data` fork.
    ///
    /// `label` is the undeclared chrome id; `page` is the page route.
    UnknownIncomingLinksNoteLabel { label: String, page: String },
    /// An `incoming-links note-label=` chrome string is declared, but its
    /// translation for `lang` has no `{n}` placeholder, so the row count
    /// would never appear (the `GlossUnitMissingPlaceholder`
    /// tier). Config-internal, hard-fail.
    ///
    /// `label` is the chrome id; `lang` the offending language.
    IncomingLinksTextMissingPlaceholder { label: String, lang: String },
    /// An `incoming-links` row property — `row-label=`, `row-aside=`,
    /// `sort-property=` or a property `group-by=` — is absent from the
    /// declared source class entirely (absent from the data, or carried by no
    /// resource of the class). A query input: every row would
    /// silently lose its label/aside/sort key/group, so a hard-fail.
    /// Data-dependent: checked in both `validate_with_data` arms.
    ///
    /// `attribute` names the attribute (`"row-label"`, `"row-aside"`,
    /// `"sort-property"`, `"group-by"`); `property` the property; `class` the
    /// declared source class; `page` the page route.
    UnknownIncomingLinksRowProperty {
        attribute: String,
        property: String,
        class: String,
        page: String,
    },
    /// An `incoming-links` row property is carried by the source class, but
    /// not by every source resource that links through the declared
    /// properties — those rows fall back (name label, no aside,
    /// sorted last, trailing headless group). Advisory (`Severity::Warning`).
    /// Data-dependent: checked in both arms.
    ///
    /// Fields as `UnknownIncomingLinksRowProperty`.
    SparseIncomingLinksRowProperty {
        attribute: String,
        property: String,
        class: String,
        page: String,
    },
    /// An `incoming-links` row property is stored on the source class with a
    /// `value_type` its role cannot use: `group-by`/`row-aside`
    /// need `list_value` or `text`, `row-label`/`sort-property` need `text`.
    /// Any other type would lower cleanly and silently put every row in the
    /// no-key group, so a hard-fail. Data-dependent: checked in both arms.
    ///
    /// `value_type` is the (lexicographically smallest) offending stored
    /// type; other fields as `UnknownIncomingLinksRowProperty`.
    IncomingLinksRowPropertyWrongType {
        attribute: String,
        property: String,
        value_type: String,
        class: String,
        page: String,
    },
    /// A `resource-list { order-by key="…" }` names a key that collides with
    /// a built-in `?sort=` token — `default`, `name`, `name-desc`, `class` or
    /// `id`. The built-in would always win, so the option could never be
    /// selected: a config-internal hard-fail, raised by `lower_resource_list`
    /// (not given the page route — span-located).
    ///
    /// `key` is the colliding key.
    ReservedOrderKey { key: String },
    /// A `resource-list` declares two `order-by` options with the same `key`.
    /// Which one `?sort=<key>` selects would be undecidable, so a
    /// config-internal hard-fail, span-located.
    ///
    /// `key` is the repeated key.
    DuplicateOrderKey { key: String },
    /// A `resource-list { order-by key="…" }` key is empty or contains a
    /// character outside `[a-z0-9-]`. The host never percent-decodes `?sort=`,
    /// so such a key would be form-encoded on submit, never match, and silently
    /// fall back to "As curated" — a config-internal hard-fail, span-located.
    ///
    /// `key` is the offending key.
    InvalidOrderKey { key: String },
    /// A `resource-list { order-by label="…" }` names a chrome id that is not
    /// a declared `chrome { string }` of this project. `label=` is required and
    /// id-only and the engine has no default word for an arbitrary property, so
    /// a typo would render the raw key in the UI: a hard-fail (unlike the
    /// advisory `UnknownGroupLabel`). Config-internal: checked in
    /// `validate_inner` inside the `site_result` guard, before the
    /// `validate_with_data` fork.
    ///
    /// `label` is the undeclared chrome id; `page` is the page route.
    UnknownOrderLabel { label: String, page: String },
    /// A `site { not-found { browse route="…" } }` entry's `route` does not
    /// match any declared page route. Same exact string rule as
    /// `UnresolvedNavRoute` — a browse link to a nonexistent route would be a
    /// broken link on the 404 page. Hard-fail.
    UnresolvedNotFoundBrowseRoute,
    /// `site { not-found { … } }` declares more than three `browse` entries.
    /// The 404 page offers at most three browse exits, never
    /// the whole nav. Hard-fail.
    TooManyNotFoundBrowseLinks,
    /// A `timeline axis-start=`/`axis-end=` integer does not fit in an `i32`.
    /// KDL integers arrive as `i128`; the value is range-checked before any
    /// cast. A config-internal hard-fail, raised by
    /// `validate::timeline::lower_timeline` (not given the page route —
    /// span-located).
    ///
    /// `attribute` is the offending attribute name (`axis-start`/`axis-end`).
    TimelineAxisOutOfRange { attribute: String },
    /// A `timeline`'s axis is empty or too short: `axis-end <= axis-start`, or
    /// a span below three years — the grid-line rule draws one line every
    /// three years, so a shorter axis has no whole grid cell. A config-internal
    /// hard-fail, span-located.
    InvalidTimelineAxis { axis_start: i32, axis_end: i32 },
    /// A `timeline class="…"` names a class that does not exist in the data.
    /// The class is a query input, so a hard-fail. Data-dependent: checked in
    /// both `validate_with_data` arms.
    ///
    /// `class` is the unknown class; `page` is the page route.
    UnknownTimelineClass { class: String, page: String },
    /// A `timeline` `date-property`/`lane-property`/`extent-property` does
    /// not exist anywhere in the data. Hard-fail. Data-dependent: checked in
    /// both `validate_with_data` arms.
    ///
    /// `property` is the unknown property; `class` the timeline's class;
    /// `page` the page route.
    UnknownTimelineProperty { property: String, class: String, page: String },
    /// A `timeline` `date-property` or `lane-property` exists in the data but
    /// is carried by no resource of the timeline's class
    /// — "absent from the whole class", the codebase's sparse sense. A
    /// deliberate **hard-fail**, unlike the other sparse tiers: without a
    /// date every marker is skipped, without a lane every marker drops into
    /// the `—` lane, so the component would be useless. (`extent-property`
    /// keeps the Warning tier — `SparseTimelineProperty`.) Data-dependent:
    /// checked in both `validate_with_data` arms.
    ///
    /// `property` is the property; `class` the timeline's class; `page` the
    /// page route.
    MissingTimelineProperty { property: String, class: String, page: String },
    /// A `timeline` `extent-property` exists in the data but is carried by no
    /// resource of the timeline's class. The extent segment is optional, so
    /// advisory (`Severity::Warning`) — every marker simply omits it.
    /// Data-dependent: checked in both arms.
    ///
    /// `property` is the property; `class` the timeline's class; `page` the
    /// page route.
    SparseTimelineProperty { property: String, class: String, page: String },
    /// A `timeline` property is stored on the timeline's class with a
    /// `value_type` other than the one its role requires: `date-property`
    /// needs `date`, `lane-property` needs `list_value`,
    /// `extent-property` needs `integer`. Hard-fail. Data-dependent: checked
    /// in both `validate_with_data` arms.
    ///
    /// `property` is the property; `value_type` the (lexicographically
    /// smallest) offending stored type; `expected` the required type; `class`
    /// the timeline's class; `page` the page route.
    IncompatibleTimelineProperty {
        property: String,
        value_type: String,
        expected: String,
        class: String,
        page: String,
    },
    /// A component declares a child node it does not recognise — a child
    /// node the component does not recognise. Config-internal: checked
    /// during lowering, before the `validate_with_data` fork, so it reaches
    /// both arms via the shared report. Named generically: its tenants are
    /// `resource-detail` and its `imprint`, and `resource-list`'s composed
    /// `gloss`.
    ///
    /// `component` is the component's node name (e.g. `"resource-detail"`);
    /// `child` is the unrecognised child node's name.
    UnknownChildNode { component: String, child: String },
    /// A `resource-detail class="…"` names a class that does not exist in the
    /// data. Mirrors `UnknownResourceListClass`/`UnknownIncomingLinksClass`/
    /// `UnknownTimelineClass`: the class is a query input, so a typo'd value
    /// is a hard-fail rather than a silently-always-empty page. Data-
    /// dependent: checked in both `validate_with_data` arms.
    ///
    /// `class` is the declared class; `page` is the page route.
    UnknownResourceDetailClass { class: String, page: String },
    /// A composed `gloss { part … unit="<id>" }`, or a `resource-detail` or
    /// `edition-cards` imprint part, names a unit that is not a declared
    /// `chrome { string }` id. Config-internal, hard-fail.
    ///
    /// `unit` is the undeclared id; `page` is the page route.
    UnknownGlossUnit { unit: String, page: String },
    /// A composed-gloss or imprint unit is declared, but its translation for `lang` has
    /// no `{n}` placeholder, so the number would never appear. Config-internal,
    /// hard-fail.
    GlossUnitMissingPlaceholder { unit: String, lang: String },
    /// A composed `gloss` or a `resource-detail` header slot names a property
    /// whose stored type cannot play its role: a gloss or imprint `part`
    /// stored as `list_value`/`link`/`boolean` (it would render nothing), a
    /// `teaser`, alt title or author that is not `text`/`richtext`, or a
    /// `unit=` on a part that is not `integer`/`decimal`. Hard-fail.
    /// Data-dependent: checked in both `validate_with_data` arms.
    ///
    /// `role` is `"part"`, `"teaser"` or `"unit"` (a gloss), or
    /// `"alt-title"`, `"author"`, `"imprint part"` or `"imprint unit"` (a
    /// header slot); `value_type` the (lexicographically smallest) offending
    /// stored type; `class` the list's or the record's class; `page` the page
    /// route.
    IncompatibleGlossPart {
        property: String,
        role: String,
        value_type: String,
        class: String,
        page: String,
    },
    /// A per-project component's figure-block slot (`section="…"`) has no
    /// `narrative` before it in its page body, so there is no narrative to
    /// render it in. Config-internal hard-fail, span-located, raised by
    /// `lower_figure_slot`.
    ///
    /// `section` is the declared heading id.
    FigureSectionWithoutNarrative { section: String },
    /// A figure-block slot's `count=` or `more-label=` names a chrome id that
    /// is not a declared `chrome { string }`. A hard-fail (unlike the advisory
    /// `UnknownGroupLabel`): the count line or more link would silently drop.
    /// Config-internal: checked in `validate_inner` inside the `site_result`
    /// guard, before the `validate_with_data` fork.
    ///
    /// `label` is the undeclared chrome id; `page` is the page route.
    UnknownFigureLabel { label: String, page: String },
    /// A figure-block slot's `more-route=` matches no declared page route —
    /// the `UnresolvedNavRoute` rule (a broken "more" link) — or is a
    /// parameterised route, which a more link cannot fill. Hard-fail,
    /// config-internal, run before the `validate_with_data` fork.
    ///
    /// `route` is the declared more route; `page` is the page route.
    UnresolvedFigureMoreRoute { route: String, page: String },
    /// A figure-block slot's `section=` is not the id of a top-level `##`
    /// heading in a present variant of the narrative before it, so figure
    /// mode would never split there and the block would silently vanish. An
    /// id on a `#` or `###` counts as absent. Hard-fail; reads narrative
    /// content, so checked in both `validate_with_data` arms.
    ///
    /// `section` is the declared id; `file` the narrative variant that lacks
    /// it; `page` the page route.
    UnknownFigureSection { section: String, file: String, page: String },
    /// A figure-block slot's `section=` names a `##` heading followed by an
    /// invite list: that heading renders as an invite section, which figure
    /// mode never splits into a part, so the block has nowhere to attach.
    /// Hard-fail; checked in both `validate_with_data` arms.
    ///
    /// `section` is the declared id; `file` the narrative variant; `page` the
    /// page route.
    FigureSectionHoldsInviteList { section: String, file: String, page: String },
    /// A heading id (`{#id}`) in a narrative a `narrative` component names is
    /// not `[a-z0-9]+(-[a-z0-9]+)*`, repeats an earlier id of the same file,
    /// or is (or starts with the prefix of) an id the engine renders itself,
    /// so the page would carry a malformed or duplicate id. Hard-fail; checked
    /// in both `validate_with_data` arms.
    ///
    /// `id` is the offending id; `file` the narrative variant.
    InvalidHeadingId { id: String, file: String },
    /// Two narratives of one page carry the same heading id (`{#id}`) in the
    /// content one visitor is served, so the page would hold the id twice
    /// and an in-page link to it would reach only the first. Compared per
    /// declared language through the narrative fallback; the same file twice
    /// on a page collides with itself. Hard-fail; checked in both
    /// `validate_with_data` arms.
    DuplicateHeadingIdOnPage { id: String, page: String },
    /// A `woodcut-cases` block or case is malformed: an unknown `layout=`, a
    /// case with fewer than two or more than four crops, two remark sources,
    /// a second `detail`, a `detail` in a `compact` block, a page case
    /// without exactly two pages, a block with no case, or a case key (first
    /// crop's annotation id, a page case's link id) used twice on one page.
    /// Config-internal hard-fail.
    ///
    /// `case` is the case key, when the case has one.
    InvalidShowcaseCase { case: Option<String> },
    /// A showcase case's `remark=` or an edition card's `lang-label=` names a
    /// chrome id that is not a declared `chrome { string }`, so the text
    /// would silently drop. Config-internal hard-fail, checked before the
    /// `validate_with_data` fork. (The edition imprint's `unit=` is
    /// `UnknownGlossUnit`'s, like the record header's.)
    ///
    /// `label` is the undeclared chrome id; `page` the page route.
    UnknownShowcaseLabel { label: String, page: String },
    /// A showcase `crop`/`detail` names an annotation that is not in the
    /// data. Hard-fail; checked in both `validate_with_data` arms.
    UnknownShowcaseAnnotation { annotation: String, page: String },
    /// A showcase `remark-link=` or `page-case link=` names an annotation
    /// link that is not in the data. Hard-fail; both arms.
    UnknownShowcaseLink { link: String, page: String },
    /// A showcase link does not join its case: a `remark-link` that targets
    /// none of the case's crops (or, on a page case, none of its pages), or a
    /// page case whose pages are not exactly its link's resource targets.
    /// Hard-fail; both arms.
    ///
    /// `case` is the case key.
    ShowcaseLinkMismatch { link: String, case: String, page: String },
    /// A showcase `page` names a resource that is not in the data.
    /// Hard-fail; both arms.
    ///
    /// `id` is the declared resource id; `page` the page route.
    UnknownShowcasePage { id: String, page: String },
    /// A showcase text that must be verbatim data is not: a crop `alt` or a
    /// `remark-excerpt` that is not part of one line of its page's
    /// description, or a `detail` `alt` that is not the detail annotation's
    /// own remark. Hard-fail; both arms.
    ///
    /// `text` is the declared text; `case` the case key.
    ShowcaseExcerptMismatch { text: String, case: String, page: String },
    /// A case's `schramm=N` while its first crop's page comment names no
    /// `Abb. N`. Hard-fail; both arms.
    ShowcaseSchrammMismatch { schramm: u32, case: String, page: String },
    /// A showcase crop's page image has no stored width or height (or one is
    /// 0), so its region cannot be sized or its aspect computed. Hard-fail;
    /// both arms.
    MissingCropDimensions { annotation: String, page: String },
    /// A showcase crop's page image is sized, but its URI fails
    /// `is_safe_external_url` (http, https or root-relative) or lacks the
    /// IIIF `/full/max/` segment the region URL is built from, so the crop
    /// would render as a failed bed. Hard-fail; both arms.
    CropImageNotIiif { annotation: String, page: String },
    /// An `edition-cards` `book` names a resource that is not in the data.
    /// Hard-fail; both arms.
    ///
    /// `book` is the declared resource id; `page` the page route.
    UnknownShowcaseBook { book: String, page: String },
    /// A `strip-row` `class=` names a class that is not in the data.
    /// Hard-fail; both arms.
    UnknownShowcaseClass { class: String, page: String },
    /// A `strip-row` `uses-property` that is not an integer on its class.
    /// Absent from the class counts as incompatible: the row would silently
    /// render empty. Hard-fail; both arms. (Both showcase components' `media=`
    /// goes through the shared media checks, `UnknownMediaProperty` and
    /// kin.)
    ///
    /// `role` names the attribute (`uses-property`).
    IncompatibleShowcaseProperty { property: String, role: String, class: String, page: String },
    /// `validate::load_remodel` found no top-level `remodel` block in the
    /// config. Raised only there, never by `validate` or at boot: the block is
    /// optional for the web binary.
    MissingRemodelBlock,
    /// A `remodel` entry repeats one that must be unique in its scope: a node
    /// that occurs more often than the grammar allows, an attribute written
    /// twice on one node, a list name, a list-node slug, a label language in
    /// one list-node, a class name or `iri=`, a `property` `iri=` on one
    /// class, a CPE property name on one class (of a `property`, a
    /// `curated`, a `member-count`, an `incoming-count` or a `member-of`
    /// `seq=`, also once by each kind), a curated key (`curated-key=` of a
    /// `property`, an `id` or a `cover`, `display-key=`, or a `curated`
    /// node's `key=`, or a `key` of the `hook` node) on one class, a class
    /// named twice in one `on` or `link-targets`, a property named by two
    /// `via` nodes of one `incoming-count`, a second `id`, `member-of` or
    /// `cover` on one class; and of the `hook` node: a second `hook`, a
    /// `provides` that takes a name its class already has, two `drops` of one
    /// class, two `overrides` of one property. Hard-fail.
    DuplicateRemodelEntry,
    /// A `remodel` node takes a name the block reserves: a `class` named
    /// like the `images` block's class, or, in a block with an `images`
    /// node, a property named `image`. Hard-fail.
    ReservedRemodelName,
    /// A `remodel` node carries an attribute the grammar does not list for
    /// it. Hard-fail.
    UnknownRemodelAttribute,
    /// A `remodel` `property` `type=` is a word that is not a value kind at
    /// all. Hard-fail.
    UnknownRemodelType,
    /// A `remodel` `property` `type=` is a port value kind the language does
    /// not support yet (`integer`, `decimal`, `boolean`, `list-node`,
    /// `geometry`, `color`). Hard-fail.
    UnsupportedRemodelType,
    /// A `remodel` value is out of range or malformed: a shortcode that is
    /// not four characters of `0-9A-F`, a `sentinel` that is empty or starts
    /// or ends with whitespace, an empty text where one must name something
    /// (a class name of `on` or `link-targets` and the argument or `class=`
    /// of a `hook` child among them), an `images` `iiif-base=` that is not
    /// `https://`, a host and `/` (so also one without a host), a positional
    /// argument beyond the one a node takes, or any on a node that takes none;
    /// a `property` `max` below its `min`, a `min` or `max` that is negative
    /// or above `u32::MAX`, a word outside the closed vocabulary of `from`,
    /// `empty`, `sentinel` or `sentinel-within`; a `curated` `type` outside
    /// `text`, `list` and `per-language`, or a `curated` `min` other than 0
    /// or 1; an `id` `from` outside `label`, `property` and `iri`. Hard-fail.
    InvalidRemodelValue,
    /// A `list-node` lacks a label for a language the site declares.
    /// Hard-fail; not reported when the site declares no language.
    IncompleteListTranslation,
    /// A `remodel` node names a class, a property, a list or a language
    /// that is not declared, or not as the node needs it: a class in `on` or
    /// `link-targets`, a `property` `target=`, a `member-of` parent or an
    /// `incoming-count` `from=` that no `class` declares; a `member-count`
    /// `of=` that names no class that is `member-of` this one; an `id`
    /// `property=` that names no `text` property of its class with `max=1`;
    /// a `via` that names no property of the `from` class; a `curated`
    /// `list=` that no `list` declares; a `list-node` label in a language
    /// the site does not declare; a `class=` of a `hook` child that no
    /// `class` declares; or an `overrides` that names no `property` node of
    /// its class. Hard-fail.
    UnknownRemodelReference,
    /// A `remodel` node combines attributes the grammar does not allow
    /// together, or omits one the combination needs: a `property` with both
    /// an `iri` and a `from` or with neither, with an `iri` and no `type`, a
    /// `link` without a `target`, an attribute its form does not take (a
    /// `type` beside `from="label"` among them), `exclude-label` beside
    /// `curated-key`, `sentinel` or `sentinel-within` in a block without a
    /// `sentinel` node; a `curated` with `list=` beside a `type` other than
    /// `list`, or with `type="list"` and no `list=`; an `id` with neither a
    /// `curated-key` nor a `from`, with `property` and no `from="property"`
    /// or the reverse, or with `under-parent` on a class without a
    /// `member-of` or beside `curated-key`; a class `image` or a `cover` in a
    /// block without an `images` node, a `cover` on a class that has no
    /// member class or beside its class's `image`; a `via` that names a
    /// property that is no `link` or whose `target` is another class; a
    /// `drops` of the `hook` node that names a class another class is
    /// `member-of`. An attribute being written decides, whatever its value.
    /// Hard-fail.
    ConflictingRemodelAttributes,
    /// A `member-of` leads back to its own class: it names that class, or
    /// following the parents' own `member-of` nodes returns to it. Reported at
    /// each `member-of` on the cycle. Hard-fail.
    RemodelMembershipCycle,
}

impl DiagnosticKind {
    /// Canonical severity for this kind: `Warning` for the advisory kinds the
    /// match names, `Error` for every other.
    pub fn severity(&self) -> Severity {
        // Severity is carried on `Diagnostic` as a field rather than derived
        // solely from `kind.severity()`, because a future kind could have
        // configurable severity. The method here is a convenience default used
        // by the constructors below.
        match self {
            // Exhaustive match so the compiler reminds us when a new variant
            // needs a severity decision.
            DiagnosticKind::UnexpectedTopLevelNode
            | DiagnosticKind::MissingSiteTitle
            | DiagnosticKind::MissingSiteLanguages
            | DiagnosticKind::MissingRequiredLanguage
            | DiagnosticKind::InvalidFieldType
            | DiagnosticKind::UnknownDefaultLanguage
            | DiagnosticKind::DuplicatePageId
            | DiagnosticKind::DuplicatePageRoute
            | DiagnosticKind::ReservedRoute
            | DiagnosticKind::DuplicateResourceListId
            | DiagnosticKind::UnknownComponent
            | DiagnosticKind::MissingRequiredAttribute
            | DiagnosticKind::InvalidAttributeType
            | DiagnosticKind::UnknownFacetKind
            | DiagnosticKind::PaginationConflict
            | DiagnosticKind::UnresolvedTarget
            | DiagnosticKind::UndeclaredRouteParam
            | DiagnosticKind::IncompleteChromeTranslation
            | DiagnosticKind::NarrativeFileNotFound
            | DiagnosticKind::InvalidNarrativePath
            | DiagnosticKind::InvalidCoverImagePath
            | DiagnosticKind::CoverImageNotFound
            | DiagnosticKind::ReservedProjectId
            | DiagnosticKind::InvalidProjectId
            | DiagnosticKind::UnknownResource
            | DiagnosticKind::UnknownThemeValue { .. }
            | DiagnosticKind::UnknownSiteValue { .. }
            | DiagnosticKind::NarrativeLayoutNotFirst { .. }
            | DiagnosticKind::MalformedPropertyRow
            | DiagnosticKind::MalformedAnnotation
            | DiagnosticKind::MalformedDataArk
            | DiagnosticKind::UnresolvedNavRoute
            | DiagnosticKind::AmbiguousClassRoute { .. }
            | DiagnosticKind::UnknownResourceListClass { .. }
            | DiagnosticKind::UnknownResourceListProperty { .. }
            | DiagnosticKind::UnsupportedFacetKind { .. }
            | DiagnosticKind::IncompatibleFacetKind { .. }
            | DiagnosticKind::UnknownFacetProperty { .. }
            | DiagnosticKind::ReservedFacetProperty { .. }
            | DiagnosticKind::UnknownSearchProperty { .. }
            | DiagnosticKind::IncompatibleSearchProperty { .. }
            | DiagnosticKind::InternalQueryError { .. }
            | DiagnosticKind::DuplicateSingletonComponent { .. }
            | DiagnosticKind::IncompleteLabelTranslation { .. }
            | DiagnosticKind::UnknownMediaProperty { .. }
            | DiagnosticKind::IncompatibleMediaProperty { .. }
            | DiagnosticKind::MediaPropertyTargetNotImagelike { .. }
            | DiagnosticKind::UnresolvableMediaClass { .. }
            | DiagnosticKind::ConflictingMediaProperty { .. }
            | DiagnosticKind::GridWithoutMedia { .. }
            | DiagnosticKind::TileRatioWithoutGrid { .. }
            | DiagnosticKind::CompoundViewerWithPagedList { .. }
            | DiagnosticKind::DuplicateGroupedProperty { .. }
            | DiagnosticKind::DuplicateNarrativeFor { .. }
            | DiagnosticKind::PagesLayoutWithoutMedia { .. }
            | DiagnosticKind::PagesLayoutOnly { .. }
            | DiagnosticKind::CompoundMediaOwnership { .. }
            | DiagnosticKind::MultipleImageViewers { .. }
            | DiagnosticKind::DuplicateIncomingLinks
            | DiagnosticKind::IncomingLinksWithoutProperty
            | DiagnosticKind::UnknownIncomingLinksClass { .. }
            | DiagnosticKind::UnknownIncomingLinksProperty { .. }
            | DiagnosticKind::IncomingLinksPropertyNotLink { .. }
            | DiagnosticKind::IncomingLinksTextMissingPlaceholder { .. }
            | DiagnosticKind::UnknownIncomingLinksRowProperty { .. }
            | DiagnosticKind::IncomingLinksRowPropertyWrongType { .. }
            | DiagnosticKind::ReservedOrderKey { .. }
            | DiagnosticKind::DuplicateOrderKey { .. }
            | DiagnosticKind::InvalidOrderKey { .. }
            | DiagnosticKind::UnknownOrderLabel { .. }
            | DiagnosticKind::UnresolvedNotFoundBrowseRoute
            | DiagnosticKind::TooManyNotFoundBrowseLinks
            | DiagnosticKind::TimelineAxisOutOfRange { .. }
            | DiagnosticKind::InvalidTimelineAxis { .. }
            | DiagnosticKind::UnknownTimelineClass { .. }
            | DiagnosticKind::UnknownTimelineProperty { .. }
            | DiagnosticKind::MissingTimelineProperty { .. }
            | DiagnosticKind::IncompatibleTimelineProperty { .. }
            | DiagnosticKind::UnknownChildNode { .. }
            | DiagnosticKind::UnknownResourceDetailClass { .. }
            | DiagnosticKind::UnknownGlossUnit { .. }
            | DiagnosticKind::GlossUnitMissingPlaceholder { .. }
            | DiagnosticKind::IncompatibleGlossPart { .. }
            | DiagnosticKind::FigureSectionWithoutNarrative { .. }
            | DiagnosticKind::UnknownFigureLabel { .. }
            | DiagnosticKind::UnresolvedFigureMoreRoute { .. }
            | DiagnosticKind::UnknownFigureSection { .. }
            | DiagnosticKind::FigureSectionHoldsInviteList { .. }
            | DiagnosticKind::InvalidHeadingId { .. }
            | DiagnosticKind::DuplicateHeadingIdOnPage { .. }
            | DiagnosticKind::InvalidShowcaseCase { .. }
            | DiagnosticKind::UnknownShowcaseLabel { .. }
            | DiagnosticKind::UnknownShowcaseAnnotation { .. }
            | DiagnosticKind::UnknownShowcaseLink { .. }
            | DiagnosticKind::ShowcaseLinkMismatch { .. }
            | DiagnosticKind::UnknownShowcasePage { .. }
            | DiagnosticKind::ShowcaseExcerptMismatch { .. }
            | DiagnosticKind::ShowcaseSchrammMismatch { .. }
            | DiagnosticKind::MissingCropDimensions { .. }
            | DiagnosticKind::CropImageNotIiif { .. }
            | DiagnosticKind::UnknownShowcaseBook { .. }
            | DiagnosticKind::UnknownShowcaseClass { .. }
            | DiagnosticKind::IncompatibleShowcaseProperty { .. }
            | DiagnosticKind::MissingRemodelBlock
            | DiagnosticKind::DuplicateRemodelEntry
            | DiagnosticKind::ReservedRemodelName
            | DiagnosticKind::UnknownRemodelAttribute
            | DiagnosticKind::UnknownRemodelType
            | DiagnosticKind::UnsupportedRemodelType
            | DiagnosticKind::InvalidRemodelValue
            | DiagnosticKind::IncompleteListTranslation
            | DiagnosticKind::UnknownRemodelReference
            | DiagnosticKind::ConflictingRemodelAttributes
            | DiagnosticKind::RemodelMembershipCycle => Severity::Error,
            DiagnosticKind::SparseProperty
            | DiagnosticKind::UnknownLinkTarget { .. }
            | DiagnosticKind::SparseResourceListProperty { .. }
            | DiagnosticKind::EmptyCompoundSource { .. }
            | DiagnosticKind::SparseFacetProperty { .. }
            | DiagnosticKind::SparseSearchProperty { .. }
            | DiagnosticKind::IncompatibleWhereValue { .. }
            | DiagnosticKind::UnknownLabel { .. }
            | DiagnosticKind::MissingClassPlural { .. }
            | DiagnosticKind::HeaderRowOverBudget { .. }
            | DiagnosticKind::UnknownGroupLabel { .. }
            | DiagnosticKind::UnknownGroupEmptyLabel { .. }
            | DiagnosticKind::UnknownGroupLinksLabel { .. }
            | DiagnosticKind::UnknownNarrativeForResource { .. }
            | DiagnosticKind::UnknownIncomingLinksLabel { .. }
            | DiagnosticKind::SparseIncomingLinksProperty { .. }
            | DiagnosticKind::IncomingLinksPropertyMissesHost { .. }
            | DiagnosticKind::UnknownIncomingLinksNoteLabel { .. }
            | DiagnosticKind::SparseIncomingLinksRowProperty { .. }
            | DiagnosticKind::SparseMediaProperty { .. }
            | DiagnosticKind::SparseTimelineProperty { .. } => Severity::Warning,
        }
    }
}

// ── Diagnostic ────────────────────────────────────────────────────────────────

/// A single validation finding: a kind, a human message, an optional source
/// location, and a severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    /// `None` for diagnostics whose source is not inside `config.kdl`
    /// (e.g. project-id checks derived from the directory name).
    pub span: Option<Span>,
    pub severity: Severity,
}

impl Diagnostic {
    /// Construct an error-severity diagnostic with a source location.
    pub fn error(kind: DiagnosticKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: kind.severity(),
            kind,
            message: message.into(),
            span: Some(span),
        }
    }

    /// Construct an error-severity diagnostic with no source location.
    /// Use for checks whose input comes from outside `config.kdl` (e.g.
    /// `ReservedProjectId`, `InvalidProjectId` — the id is the directory name).
    pub fn error_spanless(kind: DiagnosticKind, message: impl Into<String>) -> Self {
        Self {
            severity: kind.severity(),
            kind,
            message: message.into(),
            span: None,
        }
    }
}

// ── ValidationReport ─────────────────────────────────────────────────────────

/// An accumulator for zero or more [`Diagnostic`]s produced by the validator.
///
/// Diagnostics are pushed in discovery order and preserved in that order.
/// `has_errors()` determines whether validation failed; `render()` produces a
/// located string for human output (requires the original source text).
#[derive(Debug, Default)]
pub struct ValidationReport {
    diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    /// Create an empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one diagnostic to the report.
    pub fn push(&mut self, d: Diagnostic) {
        self.diagnostics.push(d);
    }

    /// Return `true` if no diagnostics have been pushed.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Return `true` if any diagnostic has `Severity::Error`.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    /// Iterate over all diagnostics in push order.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    /// Render all diagnostics as a human-readable multi-line string.
    ///
    /// For located diagnostics (span present): `"<file_label>:<line>:<col>: <message>"`.
    /// Line and column are **1-based**; column counts Unicode scalar values
    /// (chars) within the line, so non-ASCII configs report correct columns.
    ///
    /// For span-less diagnostics: `"<file_label>: <message>"` — no fake `:0:0`.
    /// (The project-id checks that produce span-less diagnostics pass the
    /// project id as `file_label` at the call site, so the output reads as
    /// `"my-project: reserved project id …"`.)
    pub fn render(&self, source: &str, file_label: &str) -> String {
        let mut out = String::new();
        for d in &self.diagnostics {
            if let Some(span) = d.span {
                let (line, col) = offset_to_line_col(source, span.offset);
                out.push_str(&format!("{file_label}:{line}:{col}: {}\n", d.message));
            } else {
                out.push_str(&format!("{file_label}: {}\n", d.message));
            }
        }
        // Trim trailing newline for cleaner multi-report joining.
        if out.ends_with('\n') {
            out.pop();
        }
        out
    }
}

/// Convert a byte offset into a source string to a 1-based (line, column)
/// pair. Column counts Unicode scalar values (chars) — not bytes — within the
/// line, so multi-byte characters count as one column each.
///
/// The offset is clamped to `source.len()` so callers with out-of-bounds spans
/// do not panic.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let clamped = offset.min(source.len());
    let before = &source[..clamped];
    let line = 1 + before.chars().filter(|&c| c == '\n').count();
    let col = {
        // Find the byte index just after the last newline before `clamped`
        // (or 0 if we are on the first line). Count chars from there to
        // `clamped` — this is correct for both single-byte and multi-byte
        // characters.
        let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        1 + before[line_start..].chars().count()
    };
    (line, col)
}

// ── Display + Error for ValidationReport ─────────────────────────────────────

/// Degraded display: kind name + message + raw byte offset (no line:col
/// because the source text is not available here). This form is used when
/// `ValidationReport` sits inside a `LoadError::Validation(report)` and
/// something calls `report.to_string()` without the source — e.g. the
/// auto-derived `Display` from `#[error(transparent)]`. The host calls
/// `report.render(source, label)` for the located form.
impl std::fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.diagnostics.is_empty() {
            return write!(f, "validation passed (no diagnostics)");
        }
        for d in &self.diagnostics {
            let offset = d.span.map(|s| s.offset);
            match offset {
                Some(o) => writeln!(f, "{:?} at byte {}: {}", d.kind, o, d.message)?,
                None => writeln!(f, "{:?}: {}", d.kind, d.message)?,
            }
        }
        Ok(())
    }
}

impl std::error::Error for ValidationReport {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: build a report from a slice of (kind, msg, span-or-none).
    fn make_report(entries: &[(DiagnosticKind, &str, Option<Span>)]) -> ValidationReport {
        let mut r = ValidationReport::new();
        for (kind, msg, span) in entries {
            let d = match span {
                Some(s) => Diagnostic::error(kind.clone(), *msg, *s),
                None => Diagnostic::error_spanless(kind.clone(), *msg),
            };
            r.push(d);
        }
        r
    }

    // ── offset_to_line_col ────────────────────────────────────────────────

    #[test]
    fn line_col_at_offset_zero() {
        // The very first character is line 1, column 1.
        assert_eq!(offset_to_line_col("hello", 0), (1, 1));
    }

    #[test]
    fn line_col_mid_first_line() {
        // "hello" — offset 3 → line 1, col 4 (0-indexed byte 3 → 1-based col 4).
        assert_eq!(offset_to_line_col("hello", 3), (1, 4));
    }

    #[test]
    fn line_col_immediately_after_newline() {
        // "foo\nbar" — the 'b' at offset 4 is line 2, col 1.
        assert_eq!(offset_to_line_col("foo\nbar", 4), (2, 1));
    }

    #[test]
    fn line_col_end_of_file() {
        // Offset equal to source.len() — clamped; still reports a valid location.
        let src = "abc\ndef";
        // len = 7; last char 'f' is at 6; offset 7 is one past end.
        assert_eq!(offset_to_line_col(src, src.len()), (2, 4));
    }

    #[test]
    fn line_col_multibyte_utf8_char() {
        // "ä" is 2 bytes (U+00E4). In "aäb", 'b' is at byte offset 3.
        // Line 1, but column must be 3 (three chars: a, ä, b) — NOT 4 (byte count).
        let src = "a\u{00E4}b";
        assert_eq!(src.len(), 4, "sanity: 'a'(1) + 'ä'(2) + 'b'(1) = 4 bytes");
        // offset of 'b' is 3 (after 1 + 2 bytes).
        assert_eq!(offset_to_line_col(src, 3), (1, 3));
    }

    #[test]
    fn line_col_em_dash_multibyte() {
        // "—" is 3 bytes (U+2014). "x—y\nz":
        //   'x' → 1 byte (offset 0)
        //   '—' → 3 bytes (offsets 1-3)
        //   'y' → 1 byte (offset 4)
        //   '\n'→ 1 byte (offset 5)
        //   'z' → 1 byte (offset 6)
        // Total: 7 bytes.
        let src = "x\u{2014}y\nz";
        assert_eq!(src.len(), 7, "sanity: 'x'(1) + '—'(3) + 'y'(1) + '\n'(1) + 'z'(1) = 7 bytes");
        assert_eq!(offset_to_line_col(src, 4), (1, 3)); // 'y': line 1, col 3 (x, —, y)
        assert_eq!(offset_to_line_col(src, 6), (2, 1)); // 'z': line 2, col 1
    }

    // ── render ────────────────────────────────────────────────────────────

    #[test]
    fn render_located_diagnostic() {
        // Source: "foo\nbar" — 'b' at offset 4 → line 2, col 1.
        let mut r = ValidationReport::new();
        r.push(Diagnostic::error(
            DiagnosticKind::UnknownComponent,
            "unknown component 'widget'",
            Span { offset: 4, len: 3 },
        ));
        let out = r.render("foo\nbar", "config.kdl");
        assert_eq!(out, "config.kdl:2:1: unknown component 'widget'");
    }

    #[test]
    fn render_located_diagnostic_after_multibyte_utf8_char_uses_char_column() {
        // Source line: "ä" is U+00E4 — 2 bytes, 1 char.
        // "äx" on line 1: 'x' is at byte offset 2 (after 2-byte 'ä'),
        // but it is character column 2 (not byte column 3).
        // We span to 'x' at byte offset 2, asserting render gives ":1:2:" not ":1:3:".
        let src = "\u{00E4}x";
        assert_eq!(src.len(), 3, "sanity: 'ä'(2) + 'x'(1) = 3 bytes");
        // Byte offset of 'x' is 2.
        let mut r = ValidationReport::new();
        r.push(Diagnostic::error(
            DiagnosticKind::UnknownComponent,
            "unexpected node",
            Span { offset: 2, len: 1 },
        ));
        let out = r.render(src, "config.kdl");
        // Column must be 2 (character column of 'x'), NOT 3 (byte offset + 1).
        assert_eq!(
            out, "config.kdl:1:2: unexpected node",
            "render must use char column (2) not byte column (3) after multi-byte char"
        );
    }

    #[test]
    fn render_spanless_diagnostic_no_fake_location() {
        // Span-less diagnostics must NOT produce `:0:0` or any line:col.
        let mut r = ValidationReport::new();
        r.push(Diagnostic::error_spanless(
            DiagnosticKind::ReservedProjectId,
            "project id '_static' is reserved",
        ));
        let out = r.render("", "_static");
        assert_eq!(out, "_static: project id '_static' is reserved");
        assert!(!out.contains(":0:0"), "span-less render must not contain ':0:0'");
    }

    // ── DiagnosticKind::severity ──────────────────────────────────────────

    #[test]
    fn malformed_property_row_severity_is_error() {
        assert_eq!(
            DiagnosticKind::MalformedPropertyRow.severity(),
            Severity::Error,
            "MalformedPropertyRow must be Error — a panic at request time is worse than boot failure"
        );
    }

    #[test]
    fn malformed_annotation_severity_is_error() {
        assert_eq!(DiagnosticKind::MalformedAnnotation.severity(), Severity::Error);
    }

    #[test]
    fn malformed_data_ark_severity_is_error() {
        assert_eq!(DiagnosticKind::MalformedDataArk.severity(), Severity::Error);
    }

    #[test]
    fn unresolved_nav_route_severity_is_error() {
        assert_eq!(
            DiagnosticKind::UnresolvedNavRoute.severity(),
            Severity::Error,
            "UnresolvedNavRoute must be Error — a nav item pointing at a nonexistent route \
             produces a broken link and should block boot"
        );
    }

    #[test]
    fn internal_query_error_severity_is_error() {
        assert_eq!(
            DiagnosticKind::InternalQueryError { context: "test".to_owned() }.severity(),
            Severity::Error,
            "InternalQueryError must be Error — the query itself failed to execute, \
             which is worse than any data-dependent condition the check was trying to verify"
        );
    }

    #[test]
    fn duplicate_singleton_component_severity_is_error() {
        assert_eq!(
            DiagnosticKind::DuplicateSingletonComponent {
                component: "resource-detail".to_owned(),
                page: "/test".to_owned(),
            }
            .severity(),
            Severity::Error,
            "DuplicateSingletonComponent must be Error — a second resource-detail \
             or facet-panel on one page would produce broken HTML (duplicate DOM id) \
             or silently-inert UI at render time, not just an advisory concern"
        );
    }

    #[test]
    fn duplicate_grouped_property_severity_is_error() {
        assert_eq!(
            DiagnosticKind::DuplicateGroupedProperty { property: "creator".to_owned(), page: "/test".to_owned() }
                .severity(),
            Severity::Error,
            "DuplicateGroupedProperty must be Error — a config-internal mistake \
             that would render one property twice on the record"
        );
    }

    #[test]
    fn unknown_group_empty_label_severity_is_warning() {
        assert_eq!(
            DiagnosticKind::UnknownGroupEmptyLabel { label: "empty-typo".to_owned(), page: "/test".to_owned() }
                .severity(),
            Severity::Warning,
            "UnknownGroupEmptyLabel must be Warning — the group renders as if unset"
        );
    }

    #[test]
    fn unknown_group_links_label_severity_is_warning() {
        assert_eq!(
            DiagnosticKind::UnknownGroupLinksLabel { label: "links-typo".to_owned(), page: "/test".to_owned() }
                .severity(),
            Severity::Warning,
            "UnknownGroupLinksLabel must be Warning — the row falls back to the engine caption"
        );
    }

    #[test]
    fn unknown_group_label_severity_is_warning() {
        assert_eq!(
            DiagnosticKind::UnknownGroupLabel { label: "group-typo".to_owned(), page: "/test".to_owned() }.severity(),
            Severity::Warning,
            "UnknownGroupLabel must be Warning — the group still renders under \
             the PropertyGroupTitle fallback cap"
        );
    }

    #[test]
    fn unknown_narrative_for_resource_severity_is_warning() {
        assert_eq!(
            DiagnosticKind::UnknownNarrativeForResource { resource_id: "nope".to_owned(), page: "/test".to_owned() }
                .severity(),
            Severity::Warning,
            "UnknownNarrativeForResource must be Warning — the intro never \
             attaches, which is a legal no-intro render"
        );
    }

    #[test]
    fn duplicate_narrative_for_severity_is_error() {
        assert_eq!(
            DiagnosticKind::DuplicateNarrativeFor { resource_id: "b1".to_owned(), page: "/test".to_owned() }.severity(),
            Severity::Error,
            "DuplicateNarrativeFor must be Error — which file wins is undecidable"
        );
    }

    #[test]
    fn incoming_links_diagnostics_severities() {
        let (p, c, pg) = ("strip-left".to_owned(), "Page".to_owned(), "/t".to_owned());
        for kind in [
            DiagnosticKind::DuplicateIncomingLinks,
            DiagnosticKind::IncomingLinksWithoutProperty,
            DiagnosticKind::UnknownIncomingLinksClass { class: c.clone(), page: pg.clone() },
            DiagnosticKind::UnknownIncomingLinksProperty { property: p.clone(), class: c.clone(), page: pg.clone() },
            DiagnosticKind::IncomingLinksPropertyNotLink {
                property: p.clone(),
                value_type: "text".to_owned(),
                class: c.clone(),
                page: pg.clone(),
            },
            DiagnosticKind::IncomingLinksTextMissingPlaceholder { label: "note".to_owned(), lang: "de".to_owned() },
            DiagnosticKind::UnknownIncomingLinksRowProperty {
                attribute: "row-label".to_owned(),
                property: p.clone(),
                class: c.clone(),
                page: pg.clone(),
            },
            DiagnosticKind::IncomingLinksRowPropertyWrongType {
                attribute: "group-by".to_owned(),
                property: p.clone(),
                value_type: "integer".to_owned(),
                class: c.clone(),
                page: pg.clone(),
            },
        ] {
            assert_eq!(
                kind.severity(),
                Severity::Error,
                "{kind:?} must be Error — a config typo or an impossible query input"
            );
        }
        for kind in [
            DiagnosticKind::UnknownIncomingLinksLabel { label: "typo".to_owned(), page: pg.clone() },
            DiagnosticKind::SparseIncomingLinksProperty { property: p.clone(), class: c.clone(), page: pg.clone() },
            DiagnosticKind::UnknownIncomingLinksNoteLabel { label: "typo".to_owned(), page: pg.clone() },
            DiagnosticKind::SparseIncomingLinksRowProperty {
                attribute: "sort-property".to_owned(),
                property: p.clone(),
                class: c.clone(),
                page: pg.clone(),
            },
            DiagnosticKind::IncomingLinksPropertyMissesHost { property: p, class: c.clone(), host_class: c, page: pg },
        ] {
            assert_eq!(
                kind.severity(),
                Severity::Warning,
                "{kind:?} must be Warning — the list still renders (fallback caption / empty)"
            );
        }
    }

    #[test]
    fn order_by_diagnostics_severities_are_error() {
        let k = "date".to_owned();
        for kind in [
            DiagnosticKind::ReservedOrderKey { key: k.clone() },
            DiagnosticKind::DuplicateOrderKey { key: k.clone() },
            DiagnosticKind::InvalidOrderKey { key: k.clone() },
            DiagnosticKind::UnknownOrderLabel { label: k, page: "/t".to_owned() },
        ] {
            assert_eq!(
                kind.severity(),
                Severity::Error,
                "{kind:?} must be Error — the option could never be selected or labelled"
            );
        }
    }

    #[test]
    fn composed_gloss_diagnostics_severities_are_error() {
        for kind in [
            DiagnosticKind::UnknownGlossUnit { unit: "gloss-extent".to_owned(), page: "/books".to_owned() },
            DiagnosticKind::GlossUnitMissingPlaceholder { unit: "gloss-extent".to_owned(), lang: "de".to_owned() },
            DiagnosticKind::IncompatibleGlossPart {
                property: "office".to_owned(),
                role: "part".to_owned(),
                value_type: "list_value".to_owned(),
                class: "Book".to_owned(),
                page: "/books".to_owned(),
            },
        ] {
            assert_eq!(
                kind.severity(),
                Severity::Error,
                "{kind:?} must be Error — the gloss would render a wrong or empty part"
            );
        }
    }

    #[test]
    fn figure_slot_diagnostics_severities_are_error() {
        let s = |v: &str| v.to_owned();
        for kind in [
            DiagnosticKind::FigureSectionWithoutNarrative { section: s("four") },
            DiagnosticKind::UnknownFigureLabel { label: s("count"), page: s("/woodcuts") },
            DiagnosticKind::UnresolvedFigureMoreRoute { route: s("/nowhere"), page: s("/woodcuts") },
            DiagnosticKind::UnknownFigureSection {
                section: s("four"),
                file: s("woodcuts.en.md"),
                page: s("/woodcuts"),
            },
            DiagnosticKind::FigureSectionHoldsInviteList {
                section: s("four"),
                file: s("woodcuts.en.md"),
                page: s("/woodcuts"),
            },
            DiagnosticKind::InvalidHeadingId { id: s("main"), file: s("woodcuts.de.md") },
            DiagnosticKind::UnknownSiteValue {
                key: s("narrative-companion"),
                value: s("figures"),
                allowed: vec![s("rail"), s("text")],
            },
            DiagnosticKind::NarrativeLayoutNotFirst { page: s("/") },
            DiagnosticKind::TileRatioWithoutGrid { class: s("Strip"), page: s("/strips") },
            DiagnosticKind::DuplicateHeadingIdOnPage { id: s("four"), page: s("/") },
        ] {
            assert_eq!(
                kind.severity(),
                Severity::Error,
                "{kind:?} must be Error — a figure block would silently vanish or misplace"
            );
        }
    }

    #[test]
    fn showcase_diagnostics_severities_are_error() {
        let s = |v: &str| v.to_owned();
        for kind in [
            DiagnosticKind::InvalidShowcaseCase { case: None },
            DiagnosticKind::UnknownShowcaseLabel { label: s("remark"), page: s("/woodcuts") },
            DiagnosticKind::UnknownShowcaseAnnotation { annotation: s("a"), page: s("/woodcuts") },
            DiagnosticKind::UnknownShowcaseLink { link: s("l"), page: s("/woodcuts") },
            DiagnosticKind::ShowcaseLinkMismatch { link: s("l"), case: s("a"), page: s("/woodcuts") },
            DiagnosticKind::UnknownShowcasePage { id: s("p"), page: s("/woodcuts") },
            DiagnosticKind::ShowcaseExcerptMismatch { text: s("t"), case: s("a"), page: s("/woodcuts") },
            DiagnosticKind::ShowcaseSchrammMismatch { schramm: 1, case: s("a"), page: s("/woodcuts") },
            DiagnosticKind::CropImageNotIiif { annotation: s("a"), page: s("/woodcuts") },
            DiagnosticKind::MissingCropDimensions { annotation: s("a"), page: s("/woodcuts") },
            DiagnosticKind::UnknownShowcaseBook { book: s("b"), page: s("/narrenschiff") },
            DiagnosticKind::UnknownShowcaseClass { class: s("C"), page: s("/narrenschiff") },
            DiagnosticKind::IncompatibleShowcaseProperty {
                property: s("p"),
                role: s("uses-property"),
                class: s("C"),
                page: s("/narrenschiff"),
            },
        ] {
            assert_eq!(
                kind.severity(),
                Severity::Error,
                "{kind:?} must be Error — a case would render wrong or unverified data"
            );
        }
    }

    #[test]
    fn not_found_browse_diagnostics_severities_are_error() {
        for kind in [
            DiagnosticKind::UnresolvedNotFoundBrowseRoute,
            DiagnosticKind::TooManyNotFoundBrowseLinks,
        ] {
            assert_eq!(
                kind.severity(),
                Severity::Error,
                "{kind:?} must be Error — a broken or over-long 404 exit list must block boot"
            );
        }
    }

    #[test]
    fn timeline_diagnostics_severities() {
        let (p, c, pg) = ("pub-date".to_owned(), "Book".to_owned(), "/".to_owned());
        for kind in [
            DiagnosticKind::TimelineAxisOutOfRange { attribute: "axis-start".to_owned() },
            DiagnosticKind::InvalidTimelineAxis { axis_start: 1506, axis_end: 1470 },
            DiagnosticKind::UnknownTimelineClass { class: c.clone(), page: pg.clone() },
            DiagnosticKind::UnknownTimelineProperty { property: p.clone(), class: c.clone(), page: pg.clone() },
            DiagnosticKind::MissingTimelineProperty { property: p.clone(), class: c.clone(), page: pg.clone() },
            DiagnosticKind::IncompatibleTimelineProperty {
                property: p.clone(),
                value_type: "text".to_owned(),
                expected: "date".to_owned(),
                class: c.clone(),
                page: pg.clone(),
            },
        ] {
            assert_eq!(
                kind.severity(),
                Severity::Error,
                "{kind:?} must be Error — the timeline would be useless or unplaceable"
            );
        }
        assert_eq!(
            DiagnosticKind::SparseTimelineProperty { property: "extent".to_owned(), class: c, page: pg }.severity(),
            Severity::Warning,
            "SparseTimelineProperty must be Warning — the optional extent segment is just omitted"
        );
    }

    #[test]
    fn compound_viewer_with_paged_list_severity_is_error() {
        assert_eq!(
            DiagnosticKind::CompoundViewerWithPagedList { page: "/test".to_owned() }.severity(),
            Severity::Error,
            "CompoundViewerWithPagedList must be Error — the two components would \
             silently fight over the `?page=` query key at request time"
        );
    }

    #[test]
    fn compound_layout_diagnostics_severity_is_error() {
        assert_eq!(
            DiagnosticKind::PagesLayoutWithoutMedia { page: "/test".to_owned() }.severity(),
            Severity::Error,
            "PagesLayoutWithoutMedia must be Error — the pages layout has no scan to show"
        );
        assert_eq!(
            DiagnosticKind::PagesLayoutOnly { feature: "picker".to_owned(), page: "/test".to_owned() }.severity(),
            Severity::Error,
            "PagesLayoutOnly must be Error — the feature could never render"
        );
        assert_eq!(
            DiagnosticKind::CompoundMediaOwnership { page: "/test".to_owned() }.severity(),
            Severity::Error,
            "CompoundMediaOwnership must be Error — the compound's images would render twice"
        );
        assert_eq!(
            DiagnosticKind::MultipleImageViewers { page: "/test".to_owned() }.severity(),
            Severity::Error,
            "MultipleImageViewers must be Error — two viewers would share page-global signals"
        );
    }

    #[test]
    fn incomplete_label_translation_severity_is_error() {
        assert_eq!(
            DiagnosticKind::IncompleteLabelTranslation {
                target: LabelTarget::Property,
                key: "role".to_owned(),
                lang: "fr".to_owned(),
            }
            .severity(),
            Severity::Error,
            "IncompleteLabelTranslation must be Error — cloned from \
             IncompleteChromeTranslation's completeness contract, every declared \
             label must supply every declared site language, and a partial \
             translation set is a config authoring error, not advisory"
        );
    }

    #[test]
    fn unknown_label_severity_is_warning() {
        assert_eq!(
            DiagnosticKind::UnknownLabel { target: LabelTarget::Class, key: "Character".to_owned() }.severity(),
            Severity::Warning,
            "UnknownLabel must be Warning — a label naming a property/class \
             absent from the data is simply unused at render time (falls back \
             to humanize_property_name), mirroring the sparse-property warning \
             family; it should not block boot"
        );
    }

    #[test]
    fn header_row_over_budget_severity_is_warning() {
        assert_eq!(
            DiagnosticKind::HeaderRowOverBudget { lang: "de".to_owned(), estimated_px: 1300, budget_px: 1280 }
                .severity(),
            Severity::Warning,
            "an over-budget desk row ellipsises the title; it should not block boot"
        );
    }

    // ── has_errors / is_empty ─────────────────────────────────────────────

    #[test]
    fn empty_report_is_empty_and_no_errors() {
        let r = ValidationReport::new();
        assert!(r.is_empty());
        assert!(!r.has_errors());
    }

    #[test]
    fn report_with_error_has_errors() {
        let mut r = ValidationReport::new();
        r.push(Diagnostic::error_spanless(DiagnosticKind::MissingSiteTitle, "missing title"));
        assert!(!r.is_empty());
        assert!(r.has_errors());
    }

    #[test]
    fn report_with_warning_only_has_no_errors() {
        let mut r = ValidationReport::new();
        r.push(Diagnostic {
            kind: DiagnosticKind::MissingSiteTitle,
            message: "advisory".to_owned(),
            span: None,
            severity: Severity::Warning,
        });
        assert!(!r.is_empty());
        assert!(!r.has_errors());
    }

    // ── accumulation + ordering ───────────────────────────────────────────

    #[test]
    fn diagnostics_preserve_push_order() {
        let entries = [
            (DiagnosticKind::MissingSiteTitle, "missing title", None),
            (DiagnosticKind::MissingSiteLanguages, "missing languages", None),
            (
                DiagnosticKind::UnknownComponent,
                "bad component",
                Some(Span { offset: 10, len: 5 }),
            ),
        ];
        let r = make_report(&entries);
        let kinds: Vec<_> = r.iter().map(|d| &d.kind).collect();
        assert_eq!(kinds[0], &DiagnosticKind::MissingSiteTitle);
        assert_eq!(kinds[1], &DiagnosticKind::MissingSiteLanguages);
        assert_eq!(kinds[2], &DiagnosticKind::UnknownComponent);
    }

    #[test]
    fn multi_diagnostic_render() {
        // Two located diagnostics on the same source should produce two lines.
        let src = "foo\nbar\nbaz";
        let mut r = ValidationReport::new();
        r.push(Diagnostic::error(
            DiagnosticKind::MissingSiteTitle,
            "no title",
            Span { offset: 0, len: 3 },
        ));
        r.push(Diagnostic::error(
            DiagnosticKind::UnknownComponent,
            "bad node",
            Span { offset: 4, len: 3 },
        ));
        let out = r.render(src, "test.kdl");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "test.kdl:1:1: no title");
        assert_eq!(lines[1], "test.kdl:2:1: bad node");
    }
}

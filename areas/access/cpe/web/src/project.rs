//! A loaded project: parsed config + the in-memory SQLite + narrative
//! content, all immutable after load (no hot-reload).

#[cfg(test)]
use std::collections::BTreeSet;

use cpe_components::component::context::NarrativeStore;
use cpe_core::diagnostic::{Diagnostic, ValidationReport};
#[cfg(test)]
use cpe_core::ir::{
    AnnotationEntry, CompoundPage, Facet, FacetGroup, MemberHeader, Pagination, ResourceLink, ResourceListPage, Sort,
    WhereClause,
};
use cpe_core::ir::{DataArks, IncomingLinks, Page, Resource, ValidatedConfig};
use cpe_core::lang::Lang;
use cpe_core::validate::context::ValidationContext;
use cpe_core::validate::{self, build_class_list_routes, build_class_routes, build_media_properties, ConfigLoadError};
use cpe_core::view_data::{ComponentData, PageData, PageRequest};
use cpe_core::ConfigError;
use cpe_store::db::{DbError, ProjectStore};
use cpe_store::query::reads::ProjectReads;
use cpe_store::query::{self, QueryError};

#[derive(Debug)]
pub struct Project {
    id: String,
    config: ValidatedConfig,
    // The store owns the connection and takes every lock on it, so nothing
    // here can hold a guard across an `.await`. `ProjectStore` is `Sync`, which
    // is what lets `Arc<Project>` be shared across handlers.
    store: ProjectStore,
    narrative: NarrativeStore,
    /// Warning-severity diagnostics produced during `validate_with_data`.
    /// Empty when the project loaded without data-check warnings.
    /// Held here so the host can log them at boot; this crate does no I/O.
    load_warnings: Vec<Diagnostic>,
    /// Class→route map built from the validated config at load time.
    ///
    /// Maps a resource `class` string (e.g. `"Character"`) to the project-relative
    /// route template (e.g. `"/characters/:id"`) of the parameterised
    /// `resource-detail` page that renders that class. Used by the renderer to
    /// build clickable cross-resource link hrefs.
    ///
    /// Only pages with exactly one `:param` in their route and a `resource-detail`
    /// with a declared `class` attribute contribute. Ambiguous bindings (two pages,
    /// same class) are a boot hard-fail (`AmbiguousClassRoute`) and never reach
    /// this map. Empty for projects with no such pages.
    class_routes: std::collections::HashMap<String, String>,
    /// Class→list-route map built from the validated config at load time.
    ///
    /// Maps a resource `class` string (e.g. `"Character"`) to the project-relative
    /// route (e.g. `"/characters"`) of a parameter-less `resource-list` page
    /// browsing that class. Used by the resource-not-found renderer and its
    /// breadcrumb to build the "Browse all {class}" exit and the live class
    /// crumb. **Advisory, not routing** — first registration wins across pages
    /// (no `AmbiguousClassRoute`-style hard-fail; see `build_class_list_routes`'s
    /// doc comment). Empty for projects with no browse pages.
    class_list_routes: std::collections::HashMap<String, String>,
    /// Class→media-property map built from `resource-detail media="…"`
    /// declarations at load time.
    ///
    /// Maps a resource `class` string (e.g. `"Character"`) to the property
    /// name (e.g. `"image"`) of the host→image `link` property carrying that
    /// class's embedded-media set. Used to suppress the media property when
    /// it would otherwise be rendered as a plain property row (the plate
    /// column renders it separately). Only pages with a
    /// resolvable class (an explicit `class` attribute, or the data-looked-up
    /// class of a literal `id`) contribute; unresolvable bindings are a boot
    /// hard-fail (`UnresolvableMediaClass`) and never reach this map. Empty
    /// for projects that declare no `media` attributes.
    media_properties: std::collections::HashMap<String, String>,
    /// Every stored data ARK by id (`resources.ark`, `annotations.ark`), read
    /// once at load after `MalformedDataArk` passed. Empty for a project that
    /// stores none.
    data_arks: DataArks,
}

impl Project {
    /// Opens the DB (schema + data), parses the config, runs the structural
    /// validator, and stores the narrative content supplied by the host.
    /// Returns `Ok(Project)` only when validation passes.
    ///
    /// The project id is available through `ctx.project_id`.
    ///
    /// `narrative` is a slice of `(filename, content)` pairs, the
    /// project's narrative files. Pass `&[]` when the
    /// project has no narrative content (e.g. in tests).
    pub fn load(
        ctx: &ValidationContext<'_>,
        config_src: &str,
        data_sql: &str,
        narrative: &[(String, String)],
    ) -> Result<Self, LoadError> {
        let store = ProjectStore::open(data_sql)?;
        let facts = store.facts();
        let (config, load_warnings) =
            validate::load_config(config_src, ctx, &facts, narrative).map_err(|e| match e {
                ConfigLoadError::Parse(e) => LoadError::Config(e),
                ConfigLoadError::Validation(r) => LoadError::Validation(r),
            })?;
        // Build the class→route map after validation passes (unambiguous by invariant:
        // `check_class_routes` inside validate() catches and hard-fails on duplicates).
        let class_routes = build_class_routes(&config.pages);
        // Class→list-route map — same "already validated, safe to
        // build unconditionally" reasoning as `class_routes` above, but this map
        // has no corresponding hard-fail check (it's advisory; see the doc comment).
        let class_list_routes = build_class_list_routes(&config.pages);
        // Class→media-property map: built the
        // same way as `class_routes` above, but data-dependent (a literal-id
        // `resource-detail`'s class is looked up via `facts`). Safe to build
        // unconditionally after validation passed — `check_resource_detail_media`
        // already hard-failed any unresolvable/conflicting binding.
        let media_properties = build_media_properties(&config.pages, &facts);
        // Build every search-bar's FTS5 index after validation (so we can
        // enumerate validated `config.pages` search-bars + their resolved
        // target classes).
        store.build_search_indexes(&config).map_err(LoadError::Search)?;
        // Read here, so a render never queries the ARKs; the full page and the
        // `_compound` fragment read one map.
        let data_arks = store.data_arks().map_err(LoadError::DataArks)?;
        Ok(Self {
            id: ctx.project_id.to_owned(),
            config,
            store,
            narrative: NarrativeStore::from_pairs(narrative),
            load_warnings,
            class_routes,
            class_list_routes,
            media_properties,
            data_arks,
        })
    }

    /// The store's composite reads over this project's connection.
    fn reads(&self) -> ProjectReads<'_> {
        self.store.reads()
    }

    /// Fetch a `Resource` by id from this project's in-memory database.
    ///
    /// Returns `Ok(None)` for a missing resource (caller maps to 404).
    ///
    /// The contract and the lock protocol are `query::reads::ProjectReads::get_resource`'s.
    ///
    /// `pub` only for the host router's tests: a page's view data comes from
    /// [`Project::page_data`], so no production caller reads through here.
    #[doc(hidden)]
    pub fn get_resource(&self, id: &str, lang: &Lang) -> Result<Option<Resource>, QueryError> {
        self.reads().get_resource(id, lang)
    }

    /// Fetch a paginated page of resources of `class`, filtered by `wheres`,
    /// sorted by `sorts`, for the requested `page` (1-based).
    ///
    /// `Limit(n)` ignores `page`; `PerPage(n)` clamps `page` to the last page.
    ///
    /// The contract and the lock protocol are `query::reads::ProjectReads::list_resources`'s.
    #[cfg(test)]
    pub(crate) fn list_resources(
        &self,
        class: &str,
        wheres: &[WhereClause],
        sorts: &[Sort],
        pagination: Pagination,
        page: u32,
        lang: &Lang,
    ) -> Result<ResourceListPage, QueryError> {
        self.reads().list_resources(class, wheres, sorts, pagination, page, lang)
    }

    /// Count resources of `class` matching `wheres`, with no pagination or
    /// hydration.
    ///
    /// The contract and the lock protocol are `query::reads::ProjectReads::count_resources`'s.
    #[cfg(test)]
    pub(crate) fn count_resources(&self, class: &str, wheres: &[WhereClause]) -> Result<usize, QueryError> {
        self.reads().count_resources(class, wheres)
    }

    /// Fetch a single-member view of a compound Resource.
    ///
    /// A resolving `page` (the raw `?page=` value) beats the 1-based `index`;
    /// an out-of-range `index` is clamped.
    ///
    /// The contract and the lock protocol are `query::reads::ProjectReads::compound_parts`'s.
    #[cfg(test)]
    pub(crate) fn compound_parts(
        &self,
        parent_id: &str,
        index: u32,
        page: Option<&str>,
        media_property: Option<&str>,
        annotations: bool,
        lang: &Lang,
    ) -> Result<CompoundPage, QueryError> {
        self.reads()
            .compound_parts(parent_id, index, page, media_property, annotations, lang)
    }

    /// The annotation register of `resource_id`
    /// (`dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md`): its
    /// annotations as numbered entries (larger box first, ties by id), then one
    /// unnumbered entry per link that targets the resource itself, in
    /// link-id order.
    ///
    /// The contract and the lock protocol are `query::reads::ProjectReads::page_annotations`'s.
    #[cfg(test)]
    pub(crate) fn page_annotations(&self, resource_id: &str, lang: &Lang) -> Result<Vec<AnnotationEntry>, QueryError> {
        self.reads().page_annotations(resource_id, lang)
    }

    /// The members of compound `parent_id` that have an annotation register
    /// (at least one annotation, or a link targeting the member itself) —
    /// the set the reader's signposts mark.
    ///
    /// The contract and the lock protocol are `query::reads::ProjectReads::annotated_members`'s.
    #[cfg(test)]
    pub(crate) fn annotated_members(&self, parent_id: &str) -> Result<BTreeSet<String>, QueryError> {
        self.reads().annotated_members(parent_id)
    }

    /// The resource-level link partners of `resource_id`: the other resources
    /// that share an annotation link with it, ordered by link id then target
    /// order (see [`ResourceLink`]).
    ///
    /// The contract and the lock protocol are `query::reads::ProjectReads::resource_links`'s.
    #[cfg(test)]
    pub(crate) fn resource_links(&self, resource_id: &str, lang: &Lang) -> Result<Vec<ResourceLink>, QueryError> {
        self.reads().resource_links(resource_id, lang)
    }

    /// Compute facet option groups for a single facet on a given class, the
    /// group label resolved through this project's `site.property_labels`.
    ///
    /// `active_ids` only marks options selected; counts never depend on it.
    ///
    /// The contract and the lock protocol are `query::reads::ProjectReads::facet_options`'s.
    #[cfg(test)]
    pub(crate) fn facet_options(
        &self,
        class: &str,
        facet: &Facet,
        base_wheres: &[WhereClause],
        active_ids: &[String],
        lang: &Lang,
    ) -> Result<FacetGroup, QueryError> {
        self.reads()
            .facet_options(class, facet, base_wheres, active_ids, &self.config.site.property_labels, lang)
    }

    /// Build the view data for one page request: each `page.body` slot's
    /// data, the page's status and the side data `request.scope` asks for
    /// (see `PageScope`).
    ///
    /// The store builds it (`query::page::page_data`) over this project's
    /// reads; every lock is taken there, one read at a time.
    pub fn page_data(&self, page: &Page, request: &PageRequest<'_>, lang: &Lang) -> PageData {
        query::page::page_data(
            &self.reads(),
            &self.media_properties,
            &self.config.site.property_labels,
            page,
            request,
            lang,
        )
    }

    /// The `incoming-links` rows of `page`'s `resource-detail` resource, for a
    /// caller that already holds the page's `components`: the lookup
    /// [`Project::page_data`] runs for its `incoming_links`.
    ///
    /// `None` when the page's `resource-detail` declares no `incoming-links`,
    /// its resource did not resolve, or the lookup failed (logged).
    ///
    /// `pub` only for the host router's tests (its `#[cfg(test)]`
    /// `resolve_incoming_links` wrapper); production reads `page_data`.
    #[doc(hidden)]
    pub fn page_incoming_links(&self, page: &Page, components: &[ComponentData], lang: &Lang) -> Option<IncomingLinks> {
        query::page::incoming_links_for_page(&self.reads(), page, components, lang)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn config(&self) -> &ValidatedConfig {
        &self.config
    }

    /// The narrative content store for this project.
    ///
    /// Callers use `narrative().get(filename)` to look up content, or pass
    /// the store to `component::narrative::render_narrative` for
    /// variant-resolved rendering.
    pub fn narrative(&self) -> &NarrativeStore {
        &self.narrative
    }

    /// Warning-severity diagnostics produced during boot-time data validation.
    ///
    /// Empty when the project loaded without any data-check warnings (the
    /// common case for well-configured projects). Non-empty when `SparseProperty`
    /// or similar advisory diagnostics fired. The project is fully operational
    /// even when warnings are present — they do not block load.
    ///
    /// The host logs these at boot (via its own tracing setup) so the operator
    /// can act on them without this crate performing I/O.
    pub fn warnings(&self) -> &[Diagnostic] {
        &self.load_warnings
    }

    /// The class→route map built from the validated config at load time.
    ///
    /// Maps a resource `class` name (e.g. `"Character"`) to the project-relative
    /// route template of its parameterised `resource-detail` page (e.g.
    /// `"/characters/:id"`). Used by the renderer to build clickable cross-resource
    /// link hrefs.
    ///
    /// Only pages with exactly one `:param` in their route and a `resource-detail`
    /// with a declared `class` attribute contribute. Projects with no such pages
    /// return an empty map. Ambiguous bindings (two pages, same class) are a boot
    /// hard-fail and never appear here.
    pub fn class_routes(&self) -> &std::collections::HashMap<String, String> {
        &self.class_routes
    }

    /// The project's stored data ARKs by resource and annotation id, read once
    /// at load. Every value passed the `MalformedDataArk` boot check, so it is
    /// a plain DSP data ARK URL. Empty for a project that stores none.
    pub fn data_arks(&self) -> &DataArks {
        &self.data_arks
    }

    /// The class→list-route map built from the validated config at load time.
    ///
    /// Maps a resource `class` name (e.g. `"Character"`) to the project-relative
    /// route of a parameter-less `resource-list` page browsing that class (e.g.
    /// `"/characters"`). Used by the resource-not-found renderer and its breadcrumb
    /// to build the "Browse all {class}" exit and the live class crumb.
    /// **Advisory** — first registration wins across pages; unlike
    /// `class_routes`, two browse pages over the same class is not a boot
    /// error (see `build_class_list_routes`'s doc comment). Projects with no
    /// browse pages for a class simply omit it here.
    pub fn class_list_routes(&self) -> &std::collections::HashMap<String, String> {
        &self.class_list_routes
    }

    /// The class→media-property map built from `resource-detail media="…"` and
    /// `compound-viewer media="…"` declarations at load time: `resource-detail`
    /// registrations win first, `compound-viewer` registrations fill in
    /// any class not already covered.
    ///
    /// Maps a resource `class` name (e.g. `"Character"`) to the property
    /// name (e.g. `"image"`) of its host→image `link` property. Used by the
    /// renderer to suppress the media property from plain property-row
    /// rendering (the record viewer renders it separately). Projects with no
    /// `media` declarations return an empty map.
    pub fn media_properties(&self) -> &std::collections::HashMap<String, String> {
        &self.media_properties
    }

    /// Fetch the ordered, hydrated Image resources a host resource points at
    /// via its `link`-typed `property`.
    ///
    /// The contract and the lock protocol are `query::reads::ProjectReads::media_items`'s.
    #[cfg(test)]
    pub(crate) fn media_items(
        &self,
        resource_id: &str,
        property: &str,
        lang: &Lang,
    ) -> Result<Vec<Resource>, QueryError> {
        self.reads().media_items(resource_id, property, lang)
    }

    /// The project's `site.title`, resolved chrome-id-or-literal for `lang`.
    ///
    /// Mirrors [`Project::description`] (see `labels::site_title`, which
    /// every chrome surface uses).
    pub fn title(&self, lang: &Lang) -> &str {
        cpe_core::labels::site_title(&self.config.site, lang)
    }

    /// The project's `site.description`, resolved chrome-id-or-literal for `lang`.
    ///
    /// Mirrors nav `label`'s "chrome string id OR literal" convention (see
    /// `labels::chrome_lookup`/`labels::nav_label`): if `site.description`
    /// matches a declared chrome-string id and that id has a translation for
    /// `lang`, the translation is returned; otherwise the literal value is
    /// returned as-is. Returns `None` when `site.description` is unset.
    pub fn description(&self, lang: &Lang) -> Option<&str> {
        self.config
            .site
            .description
            .as_deref()
            .map(|d| cpe_core::labels::chrome_lookup(&self.config.site, d, lang).unwrap_or(d))
    }

    /// The project's `site.eyebrow`, resolved chrome-id-or-literal for `lang`.
    ///
    /// Mirrors [`Project::description`]'s "chrome string id OR literal"
    /// convention (see `labels::chrome_lookup`/`labels::nav_label`): if
    /// `site.eyebrow` matches a declared chrome-string id and that id has a
    /// translation for `lang`, the translation is returned; otherwise the
    /// literal value is returned as-is. Returns `None` when `site.eyebrow` is
    /// unset.
    pub fn eyebrow(&self, lang: &Lang) -> Option<&str> {
        self.config
            .site
            .eyebrow
            .as_deref()
            .map(|e| cpe_core::labels::chrome_lookup(&self.config.site, e, lang).unwrap_or(e))
    }

    /// The project's `site.image_credit`, resolved chrome-id-or-literal for `lang`.
    ///
    /// Mirrors [`Project::description`]'s "chrome string id OR literal"
    /// convention. Returns `None` when `site.image_credit` is unset.
    pub fn image_credit(&self, lang: &Lang) -> Option<&str> {
        self.config
            .site
            .image_credit
            .as_deref()
            .map(|c| cpe_core::labels::chrome_lookup(&self.config.site, c, lang).unwrap_or(c))
    }

    /// The project's `site.cover_image` path, as declared in config.
    ///
    /// Unlike `description`/`eyebrow`/`image_credit`, this is a plain
    /// (non-localized) accessor — a file path is not translatable text.
    pub fn cover_image(&self) -> Option<&str> {
        self.config.site.cover_image.as_deref()
    }

    /// The project as the overview shows it to a reader of `lang`, which is
    /// one of the built-in languages and need not be one of the project's.
    ///
    /// The texts are resolved for `lang` when the project declares it, else
    /// for `en`, else for the project's default language. A language the
    /// project does not declare is never resolved: validation requires a
    /// chrome string's translations for the declared languages only, so the
    /// lookup would fall through to the string's id.
    pub fn teaser(&self, lang: &Lang) -> crate::page::ProjectTeaser<'_> {
        let site = &self.config.site;
        let shown = &cpe_core::lang::resolve(&[lang.as_str().to_owned(), "en".to_owned()], site);
        crate::page::ProjectTeaser {
            id: &self.id,
            title: self.title(shown),
            description: self.description(shown),
            eyebrow: self.eyebrow(shown),
            image_credit: self.image_credit(shown),
            accent: site.theme.accent.as_data_attr(),
            cover_image: self.cover_image(),
        }
    }
}

/// What can go wrong loading a project. The foreign-crate sources live as
/// `#[source]` variant fields inside `ConfigError`/`DbError`, with no `From`
/// conversion from them. A caller matching `ConfigError`'s variants can reach
/// the kdl error; `DbError` and `QueryError` carry no driver type.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Db(#[from] DbError),
    /// One or more structural validation errors were found in `config.kdl`.
    /// The `ValidationReport` accumulates all diagnostics; call
    /// `report.render(config_src, file_label)` for located output.
    #[error(transparent)]
    Validation(ValidationReport),
    /// Building a `search-bar`'s FTS5 index failed. A dedicated variant
    /// (not folded into `Db`) per the "errors carry a stable kind" convention
    /// — this is a distinct boot step (`ProjectStore::build_search_indexes`), not a
    /// generic database-open failure.
    #[error("failed to build search index")]
    Search(#[source] QueryError),
    /// Reading the stored data ARKs into `Project::data_arks` failed. Its own
    /// variant, like `Search`: a distinct boot step (`ProjectStore::data_arks`).
    #[error("failed to read the data ARKs")]
    DataArks(#[source] QueryError),
}

#[cfg(test)]
mod tests {
    use cpe_core::diagnostic::DiagnosticKind;
    use cpe_core::validate::context::ValidationContext;
    use cpe_store::query::reads::{last_page_for, resolve_page_param};

    use super::*;

    #[test]
    fn load_succeeds_on_valid_sources() {
        let config_src = r#"
site {
    title "T"
    languages "en" "de"
}
page id="home" route="/"
"#;
        let data_sql = "INSERT INTO resources (id, class) VALUES ('r1', 'Thing');";
        let ctx = ValidationContext::for_tests("my-project");
        let project = Project::load(&ctx, config_src, data_sql, &[]).expect("valid sources should load");
        assert_eq!(project.id(), "my-project");
        assert_eq!(project.config().site.title, "T");
        assert_eq!(project.data_arks(), &DataArks::default());
    }

    #[test]
    fn load_reads_the_stored_data_arks() {
        let config_src = r#"
site {
    title "T"
    languages "en" "de"
}
page id="home" route="/"
"#;
        let data_sql = "INSERT INTO resources (id, class, ark) VALUES
            ('b1', 'Book', 'https://ark.dasch.swiss/ark:/72163/1/0803/cpQ3=JfqVZOkd7hUQ26kTg7'),
            ('i1', 'Image', NULL),
            ('p1', 'Page', NULL);
            INSERT INTO annotations (id, resource_id, shape, x, y, w, h, ark) VALUES
            ('a1', 'p1', 'rectangle', 0.1, 0.1, 0.2, 0.2,
             'https://ark.dasch.swiss/ark:/72163/1/0803/bsX4cDKkW3GMC8_Ms2tOpw5');";
        let ctx = ValidationContext::for_tests("my-project");
        let project = Project::load(&ctx, config_src, data_sql, &[]).expect("valid sources should load");
        let arks = project.data_arks();
        assert_eq!(
            arks.resource("b1"),
            Some("https://ark.dasch.swiss/ark:/72163/1/0803/cpQ3=JfqVZOkd7hUQ26kTg7")
        );
        assert_eq!(arks.resource("i1"), None);
        assert_eq!(
            arks.annotation("a1"),
            Some("https://ark.dasch.swiss/ark:/72163/1/0803/bsX4cDKkW3GMC8_Ms2tOpw5")
        );
    }

    #[test]
    fn load_surfaces_config_parse_error() {
        let malformed_kdl = "node {\n  unclosed {";
        let ctx = ValidationContext::for_tests("p");
        let err = Project::load(&ctx, malformed_kdl, "", &[]).expect_err("malformed KDL should fail");
        assert!(matches!(err, LoadError::Config(_)));
    }

    #[test]
    fn load_surfaces_data_error() {
        let config_src = r#"
site {
    title "T"
    languages "en" "de"
}
page id="home" route="/"
"#;
        // References a nonexistent resource (FK violation).
        let bad_data = "INSERT INTO properties (resource_id, name, seq, value_type, value_text) \
             VALUES ('ghost', 'title', 0, 'text', 'x');";
        let ctx = ValidationContext::for_tests("p");
        let err = Project::load(&ctx, config_src, bad_data, &[]).expect_err("FK violation should fail");
        assert!(matches!(err, LoadError::Db(_)));
    }

    #[test]
    fn load_surfaces_validation_error() {
        // Missing site title → validation fails.
        let config_src = r#"
site {
    languages "en" "de"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err = Project::load(&ctx, config_src, "", &[]).expect_err("missing title should fail");
        assert!(
            matches!(err, LoadError::Validation(_)),
            "expected Validation error, got: {:?}",
            err
        );
    }

    #[test]
    fn load_stores_narrative_and_accessor_returns_content() {
        let config_src = r#"
site {
    title "T"
    languages "en" "de"
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let pairs = [
            ("home.md".to_owned(), "# Hi".to_owned()),
            ("home.de.md".to_owned(), "# Hallo".to_owned()),
        ];
        let project =
            Project::load(&ctx, config_src, "", &pairs).expect("valid sources with narrative pairs should load");
        assert_eq!(
            project.narrative().get("home.md"),
            Some("# Hi"),
            "narrative accessor must return en content"
        );
        assert_eq!(
            project.narrative().get("home.de.md"),
            Some("# Hallo"),
            "narrative accessor must return de content"
        );
        assert_eq!(project.narrative().get("absent.md"), None, "absent file returns None");
    }

    #[test]
    fn build_media_properties_includes_compound_member_class() {
        // A compound-viewer media="image" over Page members, with NO Page
        // detail page in the config at all — build_media_properties must still
        // register the class via the compound-viewer's own resolved member classes.
        let config_src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="book" route="/book" {
    compound-viewer source="B1" layout="contents" index="rail" media="image"
}
"#;
        let data_sql = "\
            INSERT INTO resources (id, class) VALUES ('B1', 'Book'), ('P1', 'Page'), ('img1', 'Image');\
            INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('B1', 'P1', 0);\
            INSERT INTO properties (resource_id, name, seq, value_type, target_id) \
                VALUES ('P1', 'image', 0, 'link', 'img1');\
            INSERT INTO representations (resource_id, kind, uri, seq) \
                VALUES ('img1', 'image', 'https://example.org/full/max/0/default.jpg', 0);";
        let ctx = ValidationContext::for_tests("my-project");
        let project = Project::load(&ctx, config_src, data_sql, &[])
            .expect("a compound-viewer media= over a clean Page member must load without errors");
        assert_eq!(
            project.media_properties().get("Page").map(String::as_str),
            Some("image"),
            "compound-viewer media=\"image\" must register 'Page' in the class->media map \
             even with no resource-detail page for it"
        );
    }

    // ── `narrative-for` resource existence (both validate_with_data arms) ──────

    const NARRATIVE_FOR_DATA: &str = "INSERT INTO resources (id, class) VALUES ('b1', 'Book');";

    #[test]
    fn narrative_for_unknown_resource_is_a_warning() {
        let config_src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="book" route="/books/:id" {
    resource-detail id="$id" class="Book" {
        narrative-for "b1" file="book-b1.md"
        narrative-for "nope" file="book-nope.md"
    }
}
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let project = Project::load(&ctx, config_src, NARRATIVE_FOR_DATA, &[])
            .expect("an unknown narrative-for id is advisory, not a hard-fail");
        let unknown: Vec<&DiagnosticKind> = project
            .warnings()
            .iter()
            .map(|d| &d.kind)
            .filter(|k| matches!(k, DiagnosticKind::UnknownNarrativeForResource { .. }))
            .collect();
        assert_eq!(
            unknown,
            vec![&DiagnosticKind::UnknownNarrativeForResource {
                resource_id: "nope".to_owned(),
                page: "/books/:id".to_owned(),
            }],
            "only the missing id warns — the existing 'b1' does not"
        );
    }

    #[test]
    fn narrative_for_unknown_resource_still_reported_when_lowering_failed() {
        // Dual failure: a narrative-for with no `file=` fails lowering (so the
        // raw arm runs) AND another entry names an unknown resource. Both
        // families must appear in one report.
        let config_src = r#"
site {
    title "T"
    languages "de" "en"
}
page id="book" route="/books/:id" {
    resource-detail id="$id" class="Book" {
        narrative-for "nope" file="book-nope.md"
        narrative-for "b1"
    }
}
"#;
        let ctx = ValidationContext::for_tests("my-project");
        let err =
            Project::load(&ctx, config_src, NARRATIVE_FOR_DATA, &[]).expect_err("the missing file= must fail boot");
        let LoadError::Validation(report) = err else {
            panic!("expected a validation error, got {err:?}");
        };
        let kinds: Vec<&DiagnosticKind> = report.iter().map(|d| &d.kind).collect();
        assert!(
            kinds.contains(&&DiagnosticKind::MissingRequiredAttribute),
            "lowering's error must be present: {kinds:?}"
        );
        assert!(
            kinds.contains(&&DiagnosticKind::UnknownNarrativeForResource {
                resource_id: "nope".to_owned(),
                page: "/books/:id".to_owned(),
            }),
            "the raw arm must still run the resource-existence check: {kinds:?}"
        );
        assert!(
            !kinds.iter().any(|k| matches!(
                k,
                DiagnosticKind::UnknownNarrativeForResource { resource_id, .. } if resource_id == "b1"
            )),
            "an existing id never warns: {kinds:?}"
        );
    }

    // ── Pagination helper unit tests ──────────────────────────────────────────

    #[test]
    fn last_page_for_zero_total_is_one() {
        assert_eq!(last_page_for(0, 20), 1, "zero total → page 1");
    }

    #[test]
    fn last_page_for_exact_multiple() {
        assert_eq!(last_page_for(40, 20), 2, "40 / 20 = 2");
        assert_eq!(last_page_for(20, 20), 1, "20 / 20 = 1");
    }

    #[test]
    fn last_page_for_remainder_rounds_up() {
        assert_eq!(last_page_for(21, 20), 2, "21 / 20 = 2 (ceiling)");
        assert_eq!(last_page_for(1, 20), 1, "1 / 20 = 1 (ceiling)");
    }

    // ── list_resources integration tests ─────────────────────────────────────
    //
    // Build a minimal Project via `Project::load` (the only public constructor)
    // and exercise `list_resources` against the in-memory SQLite.

    /// Minimal valid config — no pages, just a site block — used by the
    /// list_resources tests. Validation does not check for empty page lists.
    const MINIMAL_CONFIG: &str = r#"
site {
    title "T"
    languages "en" "de"
}
page id="home" route="/"
"#;

    /// Five Characters + two Locations in deterministic id order.
    const LIST_FIXTURE_SQL: &str = "
        INSERT INTO resources (id, class) VALUES
            ('c1', 'Character'),
            ('c2', 'Character'),
            ('c3', 'Character'),
            ('c4', 'Character'),
            ('c5', 'Character'),
            ('l1', 'Location'),
            ('l2', 'Location');
        INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
            ('c1', 'name', 0, 'text', 'Alice',    'en'),
            ('c2', 'name', 0, 'text', 'Bob',      'en'),
            ('c3', 'name', 0, 'text', 'Charlie',  'en'),
            ('c4', 'name', 0, 'text', 'Diana',    'en'),
            ('c5', 'name', 0, 'text', 'Eve',      'en'),
            ('l1', 'name', 0, 'text', 'London',   'en'),
            ('l2', 'name', 0, 'text', 'Paris',    'en');
    ";

    /// `p1` carries two annotations (`b` the larger box, though `a` sorts
    /// first by id) and is the target of two links: `L9`, which also joins
    /// `p2`, and the one-target `L8`.
    const ANNOTATION_FIXTURE_SQL: &str = "
        INSERT INTO resources (id, class) VALUES ('bk', 'Book'), ('p1', 'Page'), ('p2', 'Page');
        INSERT INTO compound_parts (parent_id, child_id, seq) VALUES ('bk', 'p1', 1), ('bk', 'p2', 2);
        INSERT INTO annotations (id, resource_id, shape, x, y, w, h) VALUES
            ('a', 'p1', 'rectangle', 0.1, 0.1, 0.1, 0.1),
            ('b', 'p1', 'rectangle', 0.0, 0.0, 0.5, 0.5);
        INSERT INTO annotation_links (id, name) VALUES ('L9', 'pair'), ('L8', 'alone');
        INSERT INTO annotation_link_targets (link_id, seq, annotation_id, resource_id) VALUES
            ('L9', 0, NULL, 'p1'), ('L9', 1, NULL, 'p2'), ('L8', 0, NULL, 'p1');
    ";

    #[test]
    fn page_annotations_numbers_annotations_then_lists_page_level_links() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, ANNOTATION_FIXTURE_SQL, &[]).expect("should load");
        let entries = project.page_annotations("p1", &make_lang_en()).expect("query should succeed");
        let got: Vec<(String, Option<u32>, Vec<String>)> = entries
            .iter()
            .map(|e| match e {
                AnnotationEntry::Annotation { number, annotation, partners } => (
                    annotation.id.clone(),
                    Some(*number),
                    partners.iter().map(|p| p.resource_id.clone()).collect(),
                ),
                AnnotationEntry::Link { link, partners } => {
                    (link.id.clone(), None, partners.iter().map(|p| p.resource_id.clone()).collect())
                }
            })
            .collect();
        assert_eq!(
            got,
            [
                ("b".to_owned(), Some(1), vec![]),
                ("a".to_owned(), Some(2), vec![]),
                ("L8".to_owned(), None, vec![]),
                ("L9".to_owned(), None, vec!["p2".to_owned()]),
            ]
        );
        assert!(project
            .page_annotations("p2", &make_lang_en())
            .expect("ok")
            .iter()
            .all(|e| matches!(e, AnnotationEntry::Link { .. })));
        assert_eq!(
            project.annotated_members("bk").expect("ok"),
            ["p1".to_owned(), "p2".to_owned()].into()
        );
        let links = project.resource_links("p1", &make_lang_en()).expect("ok");
        assert_eq!(links.len(), 1);
        assert_eq!((links[0].link_id.as_str(), links[0].resource_id.as_str()), ("L9", "p2"));
    }

    #[test]
    fn compound_parts_carries_the_register_only_when_asked() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, ANNOTATION_FIXTURE_SQL, &[]).expect("should load");
        let lang = make_lang_en();
        let on = project.compound_parts("bk", 1, None, None, true, &lang).expect("ok");
        assert_eq!(on.annotations, project.page_annotations("p1", &lang).expect("ok"));
        assert_eq!(on.annotations.len(), 4);
        assert_eq!(on.annotated_members, ["p1".to_owned(), "p2".to_owned()].into());
        // The register follows the shown member, not the first one.
        let second = project.compound_parts("bk", 2, None, None, true, &lang).expect("ok");
        assert_eq!(second.annotations, project.page_annotations("p2", &lang).expect("ok"));
        let off = project.compound_parts("bk", 1, None, None, false, &lang).expect("ok");
        assert!(off.annotations.is_empty() && off.annotated_members.is_empty());
    }

    fn make_lang_en() -> cpe_core::lang::Lang {
        use cpe_core::ir::SiteConfig;
        use cpe_core::lang::resolve;
        let site = SiteConfig {
            title: "T".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned()],
            default_language: "en".to_owned(),
            nav: vec![],
            chrome: vec![],
            property_labels: vec![],
            class_labels: vec![],
            theme: cpe_core::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: vec![],
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        };
        resolve(&["en".to_owned()], &site)
    }

    #[test]
    fn list_resources_limit_returns_at_most_n_items() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, LIST_FIXTURE_SQL, &[]).expect("should load");
        let lang = make_lang_en();
        let page = project
            .list_resources("Character", &[], &[], Pagination::Limit(3), 1, &lang)
            .expect("query should succeed");
        assert_eq!(page.items.len(), 3, "Limit(3) should return 3 items");
        assert_eq!(page.total, 3, "Limit: total == items.len()");
        assert_eq!(page.page, 1, "Limit always returns page 1");
        assert!(matches!(page.pagination, Pagination::Limit(3)), "pagination carried through");
    }

    #[test]
    fn list_resources_limit_ignores_page_arg() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, LIST_FIXTURE_SQL, &[]).expect("should load");
        let lang = make_lang_en();
        // Passing page=99 with Limit should be ignored; page in result is always 1.
        let page = project
            .list_resources("Character", &[], &[], Pagination::Limit(3), 99, &lang)
            .expect("query should succeed");
        assert_eq!(page.page, 1, "Limit ignores page arg, returns 1");
    }

    #[test]
    fn list_resources_per_page_first_page() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, LIST_FIXTURE_SQL, &[]).expect("should load");
        let lang = make_lang_en();
        let page = project
            .list_resources("Character", &[], &[], Pagination::PerPage(2), 1, &lang)
            .expect("query should succeed");
        assert_eq!(page.items.len(), 2, "page 1 with per_page=2 → 2 items");
        assert_eq!(page.total, 5, "total should be all 5 Characters");
        assert_eq!(page.page, 1);
    }

    #[test]
    fn list_resources_per_page_last_page_shorter() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, LIST_FIXTURE_SQL, &[]).expect("should load");
        let lang = make_lang_en();
        // 5 Characters, per_page=2 → pages 1,2 have 2 each, page 3 has 1.
        let page = project
            .list_resources("Character", &[], &[], Pagination::PerPage(2), 3, &lang)
            .expect("query should succeed");
        assert_eq!(page.items.len(), 1, "last page has 1 item (5 mod 2)");
        assert_eq!(page.total, 5);
        assert_eq!(page.page, 3);
    }

    #[test]
    fn list_resources_per_page_clamps_past_end() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, LIST_FIXTURE_SQL, &[]).expect("should load");
        let lang = make_lang_en();
        // 5 Characters, per_page=2 → last_page=3. Requesting page 99 clamps to 3.
        let page = project
            .list_resources("Character", &[], &[], Pagination::PerPage(2), 99, &lang)
            .expect("query should succeed");
        assert_eq!(page.page, 3, "page > last_page should be clamped to last_page");
        assert_eq!(page.items.len(), 1, "last page has 1 item");
    }

    #[test]
    fn list_resources_per_page_zero_total_returns_empty_page_one() {
        let ctx = ValidationContext::for_tests("p");
        // No Location data.
        let project = Project::load(&ctx, MINIMAL_CONFIG, "", &[]).expect("should load");
        let lang = make_lang_en();
        let page = project
            .list_resources("Location", &[], &[], Pagination::PerPage(10), 1, &lang)
            .expect("query should succeed");
        assert_eq!(page.items.len(), 0, "empty class returns 0 items");
        assert_eq!(page.total, 0);
        assert_eq!(page.page, 1, "page clamped to 1 when total=0");
    }

    #[test]
    fn list_resources_filters_by_class() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, LIST_FIXTURE_SQL, &[]).expect("should load");
        let lang = make_lang_en();
        let page = project
            .list_resources("Location", &[], &[], Pagination::PerPage(20), 1, &lang)
            .expect("query should succeed");
        assert_eq!(page.total, 2, "only Locations are returned");
        assert!(
            page.items.iter().all(|r| r.class == "Location"),
            "all returned resources should be Location class"
        );
    }

    // ── count_resources integration tests ────────────────────────────────────

    #[test]
    fn count_resources_no_wheres_counts_all_of_class() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, LIST_FIXTURE_SQL, &[]).expect("should load");
        let count = project.count_resources("Character", &[]).expect("count query should succeed");
        assert_eq!(count, 5, "all 5 Characters, no filter");
    }

    #[test]
    fn count_resources_with_where_counts_filtered_subset() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, LIST_FIXTURE_SQL, &[]).expect("should load");
        let wheres = vec![WhereClause::Equals {
            property: "name".to_owned(),
            value: cpe_core::ir::Value::String("Alice".to_owned()),
        }];
        let count = project
            .count_resources("Character", &wheres)
            .expect("count query should succeed");
        assert_eq!(count, 1, "only 'Alice' matches the where clause");
    }

    #[test]
    fn count_resources_filters_by_class() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, LIST_FIXTURE_SQL, &[]).expect("should load");
        let count = project.count_resources("Location", &[]).expect("count query should succeed");
        assert_eq!(count, 2, "only the 2 Locations, not Characters");
    }

    // ── search-bar FTS5 boot indexing integration tests ───────────────────────
    //
    // Uses the `good-search` fixture (`web/tests/projects/good-search/`) —
    // a `Thing` class with `name` (text) and `description` (richtext)
    // properties, and a page with `resource-list id="things"` + `search-bar
    // target="things" searches="name" "description"`. `Project::load` builds
    // the FTS5 index at boot (`ProjectStore::build_search_indexes`); these tests
    // exercise it through the public query entry points with an injected
    // `WhereClause::Search`, as a host injects one at request time. This
    // integration test lives here, beside the
    // `Project` it drives and with the crate-internal test delegations
    // (`#[cfg(test)] pub(crate)`) in reach, rather than under `web/tests/`,
    // which only sees the public surface.

    const GOOD_SEARCH_CONFIG: &str = include_str!("../tests/projects/good-search/config.kdl");
    const GOOD_SEARCH_DATA: &str = include_str!("../tests/projects/good-search/data.sql");

    fn good_search_project() -> Project {
        let ctx = ValidationContext::for_tests("good-search");
        Project::load(&ctx, GOOD_SEARCH_CONFIG, GOOD_SEARCH_DATA, &[])
            .expect("good-search fixture should load and boot-index cleanly")
    }

    /// Build a `WhereClause::Search` for the `good-search` fixture's `Thing`
    /// search-bar (`searches="name" "description"`), using the SAME
    /// `search_index_name` derivation `build_search_indexes` used at boot —
    /// the two must never disagree, exactly as `search_index_name`'s own doc
    /// comment requires.
    fn search_where(query: &str) -> WhereClause {
        let table = query::search_index_name("Thing", &["name".to_owned(), "description".to_owned()]);
        WhereClause::Search { table, query: query.to_owned() }
    }

    #[test]
    fn search_bar_prefix_match_is_case_insensitive() {
        let project = good_search_project();
        let lang = make_lang_en();
        for q in ["rab", "RAB"] {
            let page = project
                .list_resources("Thing", &[search_where(q)], &[], Pagination::PerPage(20), 1, &lang)
                .unwrap_or_else(|e| panic!("query {q:?} should not error: {e}"));
            let ids: Vec<&str> = page.items.iter().map(|r| r.id.as_str()).collect();
            assert_eq!(ids, vec!["t1"], "query {q:?} should match only RabbIT (t1), got: {ids:?}");
        }
    }

    #[test]
    fn search_bar_matches_richtext_only_content() {
        let project = good_search_project();
        let lang = make_lang_en();
        let page = project
            .list_resources("Thing", &[search_where("caterpillar")], &[], Pagination::PerPage(20), 1, &lang)
            .expect("query should not error");
        let ids: Vec<&str> = page.items.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["t3"],
            "'caterpillar' only appears in t3's richtext description, got: {ids:?}"
        );
    }

    #[test]
    fn search_bar_no_match_returns_empty_not_error() {
        let project = good_search_project();
        let lang = make_lang_en();
        let page = project
            .list_resources("Thing", &[search_where("xyzzy")], &[], Pagination::PerPage(20), 1, &lang)
            .expect("query should not error");
        assert!(page.items.is_empty(), "xyzzy should match nothing, got: {:?}", page.items);
        assert_eq!(page.total, 0);
    }

    // ── Live hostile-input robustness (executes against the real FTS index,
    // not just the string) ─────────────────────────────────────────────────
    //
    // Distinct from the pure-string `fts_match_expr` unit test in `util.rs`:
    // this runs the SAME empty-after-tokenize corpus (plus the bare word
    // "AND") through the actual query path against a booted fixture and
    // asserts no error/panic for any of them — a generated string that merely
    // *looks* safe could still raise a runtime FTS5 syntax error.
    #[test]
    fn search_bar_hostile_input_corpus_never_errors() {
        let project = good_search_project();
        let lang = make_lang_en();
        let corpus = ["", " ", "\"", "(", ":", "-", "*", "AND"];

        for q in corpus {
            let result = project.list_resources("Thing", &[search_where(q)], &[], Pagination::PerPage(20), 1, &lang);
            assert!(
                result.is_ok(),
                "list_resources with hostile input {q:?} must not error, got: {:?}",
                result.err()
            );
        }

        for q in corpus {
            let result = project.count_resources("Thing", &[search_where(q)]);
            assert!(
                result.is_ok(),
                "count_resources with hostile input {q:?} must not error, got: {:?}",
                result.err()
            );
        }
    }

    // ── compound_parts integration tests ──────────────────────────────────────
    //
    // Build a minimal Project via `Project::load` and exercise
    // `Project::compound_parts` directly. Through its delegation this
    // exercises `ProjectReads::compound_parts`'s lock-once-drop-before-hydrate
    // path at the unit layer (a regression that
    // re-locked the non-reentrant Mutex would hang here rather than only in
    // a host's HTTP tests). Mirrors the query-layer fixture in `query.rs`'s
    // `setup_compound_headers_db` (see the `compound_part_headers` tests).

    /// A Story parent ("ST_001") with three StoryChapter members at
    /// `compound_parts.seq` 0, 1, 2, each with an EN `name` property.
    const COMPOUND_FIXTURE_SQL: &str = "
        INSERT INTO resources (id, class) VALUES
            ('ST_001', 'Story'),
            ('SCH_001', 'StoryChapter'),
            ('SCH_002', 'StoryChapter'),
            ('SCH_003', 'StoryChapter');
        INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
            ('ST_001', 'SCH_001', 0),
            ('ST_001', 'SCH_002', 1),
            ('ST_001', 'SCH_003', 2);
        INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
            ('SCH_001', 'name', 0, 'text', 'Down the Rabbit Hole', 'en'),
            ('SCH_002', 'name', 0, 'text', 'The Pool of Tears', 'en'),
            ('SCH_003', 'name', 0, 'text', 'A Caucus-Race and a Long Tale', 'en');
    ";

    #[test]
    fn compound_parts_returns_members_in_order_and_hydrates_member_at_index() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, COMPOUND_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();

        let page = project
            .compound_parts("ST_001", 2, None, None, false, &lang)
            .expect("compound_parts should not error");

        // members are returned in compound_parts.seq, child_id order.
        let ids: Vec<&str> = page.members.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["SCH_001", "SCH_002", "SCH_003"],
            "members must be in compound_parts.seq order, got: {page:?}"
        );
        let names: Vec<&str> = page.members.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "Down the Rabbit Hole",
                "The Pool of Tears",
                "A Caucus-Race and a Long Tale",
            ],
            "member names must resolve from the 'name' property, got: {page:?}"
        );

        assert_eq!(page.total, page.members.len(), "total must equal members.len()");
        assert_eq!(page.total, 3, "three members in the fixture");

        // index=2 (1-based) → the hydrated member is members[1] (SCH_002).
        assert_eq!(page.index, 2, "effective index should echo the requested 2");
        let member = page.member.as_ref().expect("member must be Some for a non-empty compound");
        assert_eq!(
            member.id, page.members[1].id,
            "hydrated member at index 2 must match members[1]"
        );
        assert_eq!(member.id, "SCH_002");
    }

    #[test]
    fn compound_parts_index_zero_clamps_to_first_position() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, COMPOUND_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();

        let page = project
            .compound_parts("ST_001", 0, None, None, false, &lang)
            .expect("compound_parts should not error");

        assert_eq!(page.index, 1, "index=0 must clamp to the nearest valid 1-based position (1)");
        let member = page.member.as_ref().expect("member must be Some");
        assert_eq!(member.id, "SCH_001", "clamped index=1 must hydrate the first member");
    }

    #[test]
    fn compound_parts_index_past_end_clamps_to_last_position() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, COMPOUND_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();

        let page = project
            .compound_parts("ST_001", 99, None, None, false, &lang)
            .expect("compound_parts should not error");

        assert_eq!(
            page.index, 3,
            "index > total must clamp to the nearest valid position (total=3)"
        );
        let member = page.member.as_ref().expect("member must be Some");
        assert_eq!(member.id, "SCH_003", "clamped index=3 must hydrate the last member");
    }

    #[test]
    fn compound_parts_empty_or_non_compound_parent_returns_empty_page() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, COMPOUND_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();

        // SCH_001 exists as a resource but has no compound_parts rows of its
        // own (it is a member, not a parent) — a non-compound source.
        let page = project
            .compound_parts("SCH_001", 1, None, None, false, &lang)
            .expect("compound_parts should not error");

        assert!(
            page.member.is_none(),
            "non-compound parent must yield member == None, got: {page:?}"
        );
        assert_eq!(page.total, 0, "non-compound parent must yield total == 0");
        assert!(page.members.is_empty(), "non-compound parent must yield empty members");
        assert_eq!(page.index, 1, "empty compound still reports index == 1 (documented invariant)");
    }

    // ── resolve_page_param ────────────────────────────────────────────────────
    //
    // Headers modelled on the real postilla book (Incunabula's data): a
    // lower-case first quire (`a1r; Titelblatt`, `a2v`) and a capital second
    // quire (`A1r` → `postilla-a1r`, member 207 of 340; `A2v`, whose id
    // carries the case-fold `-2` suffix because `a2v` took `postilla-a2v`), a
    // pagenum repeated three times (`D5v (doppelt gescannt???)`), and the two
    // `Nachsatzblatt` leaves.

    fn header(id: &str, name: &str) -> MemberHeader {
        MemberHeader {
            id: id.to_owned(),
            class: "Page".to_owned(),
            name: name.to_owned(),
        }
    }

    fn postilla_like_headers() -> Vec<MemberHeader> {
        vec![
            header("postilla-a1r-titelblatt", "a1r; Titelblatt"), // 0
            header("postilla-a2v", "a2v"),                        // 1
            header("postilla-a1r", "A1r"),                        // 2
            header("postilla-a2v-2", "A2v"),                      // 3
            header("postilla-d5v-doppelt-gescannt", "D5v (doppelt gescannt???)"), // 4
            header("postilla-d5v-doppelt-gescannt-2", "D5v (doppelt gescannt???)"), // 5
            header("postilla-nachsatzblatt-recto", "Nachsatzblatt, recto"), // 6
            header("postilla-nachsatzblatt-verso", "Nachsatzblatt, verso"), // 7
        ]
    }

    #[test]
    fn resolve_page_tier1_exact_id() {
        let h = postilla_like_headers();
        assert_eq!(resolve_page_param(&h, "postilla-a1r"), Some(2));
        assert_eq!(resolve_page_param(&h, "postilla-a2v-2"), Some(3));
    }

    #[test]
    fn resolve_page_tier2a_case_sensitive_name_beats_case_insensitive() {
        let h = postilla_like_headers();
        // `a2v` and `A2v` are distinct members; each typed form reaches its own.
        assert_eq!(resolve_page_param(&h, "a2v"), Some(1));
        assert_eq!(resolve_page_param(&h, "A2v"), Some(3));
    }

    #[test]
    fn resolve_page_tier2b_case_insensitive_name() {
        let h = postilla_like_headers();
        // No member is named exactly `a1r` (member 0 is `a1r; Titelblatt`),
        // so tier 2b folds to `A1r` — the same member `?page=postilla-a1r`
        // reaches (the step-7 acceptance pair).
        assert_eq!(resolve_page_param(&h, "a1r"), Some(2));
        assert_eq!(resolve_page_param(&h, "NACHSATZBLATT, RECTO"), Some(6));
    }

    #[test]
    fn resolve_page_tier2_duplicate_name_first_in_seq_wins() {
        let h = postilla_like_headers();
        assert_eq!(
            resolve_page_param(&h, "D5v (doppelt gescannt???)"),
            Some(4),
            "a repeated pagenum resolves to the first member in seq order"
        );
    }

    #[test]
    fn resolve_page_tier3_unique_prefix() {
        let h = postilla_like_headers();
        assert_eq!(resolve_page_param(&h, "nachsatzblatt, r"), Some(6));
        assert_eq!(resolve_page_param(&h, "a1r;"), Some(0));
        // Every hit shares one name → tier 2b spelled short → first in seq.
        assert_eq!(resolve_page_param(&h, "d5v (d"), Some(4));
    }

    #[test]
    fn resolve_page_ambiguous_prefix_is_none() {
        let h = postilla_like_headers();
        // `a1` spans `a1r; Titelblatt` and `A1r` (two distinct names).
        assert_eq!(resolve_page_param(&h, "a1"), None);
        assert_eq!(resolve_page_param(&h, "Nachsatz"), None);
    }

    #[test]
    fn resolve_page_decodes_percent_and_plus() {
        let h = postilla_like_headers();
        assert_eq!(resolve_page_param(&h, "a1r%3B+Titelblatt"), Some(0));
        assert_eq!(resolve_page_param(&h, "Nachsatzblatt%2C%20verso"), Some(7));
        assert_eq!(resolve_page_param(&h, "nachsatzblatt%2C+r"), Some(6));
    }

    #[test]
    fn resolve_page_empty_or_whitespace_is_absent() {
        let h = postilla_like_headers();
        assert_eq!(resolve_page_param(&h, ""), None);
        assert_eq!(resolve_page_param(&h, "+++"), None);
        assert_eq!(resolve_page_param(&h, "%20%09"), None);
        // Surrounding whitespace is trimmed, not part of the match.
        assert_eq!(resolve_page_param(&h, "+a2v+"), Some(1));
    }

    #[test]
    fn resolve_page_no_match_is_none() {
        let h = postilla_like_headers();
        assert_eq!(resolve_page_param(&h, "zzz"), None);
    }

    #[test]
    fn compound_parts_page_beats_member() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, COMPOUND_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();

        let page = project
            .compound_parts("ST_001", 1, Some("SCH_003"), None, false, &lang)
            .expect("compound_parts should not error");
        assert_eq!(page.index, 3, "?page= wins over ?member=");
        assert_eq!(page.member.as_ref().map(|m| m.id.as_str()), Some("SCH_003"));

        let by_name = project
            .compound_parts("ST_001", 3, Some("the+pool"), None, false, &lang)
            .expect("compound_parts should not error");
        assert_eq!(by_name.index, 2, "a decoded unique prefix resolves");
    }

    #[test]
    fn compound_parts_unresolvable_page_keeps_member() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, COMPOUND_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();

        let kept = project
            .compound_parts("ST_001", 2, Some("zzz"), None, false, &lang)
            .expect("compound_parts should not error");
        assert_eq!(kept.index, 2, "unresolvable page falls back to member");

        let first = project
            .compound_parts("ST_001", 1, Some("zzz"), None, false, &lang)
            .expect("compound_parts should not error");
        assert_eq!(first.index, 1, "no member either → member 1");
    }

    /// Book-page Choice H: `stale_page` carries the decoded `?page=` value
    /// exactly when it was non-empty and named no page — unknown or
    /// ambiguous — and is `None` for an absent or empty value and for every
    /// resolving tier. The page shown is unchanged by it.
    #[test]
    fn compound_parts_marks_an_unresolved_page_stale() {
        let sql = "
            INSERT INTO resources (id, class) VALUES
                ('B1', 'Book'), ('P1', 'Page'), ('P2', 'Page'), ('B0', 'Book');
            INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
                ('B1', 'P1', 0), ('B1', 'P2', 1);
            INSERT INTO properties (resource_id, name, seq, value_type, value_text) VALUES
                ('P1', 'name', 0, 'text', 'a1r'),
                ('P2', 'name', 0, 'text', 'a1v');
        ";
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, sql, &[]).expect("fixture should load");
        let lang = make_lang_en();
        let stale = |page: Option<&str>| {
            let got = project
                .compound_parts("B1", 1, page, None, false, &lang)
                .expect("compound_parts should not error");
            (got.index, got.stale_page)
        };
        assert_eq!(stale(Some("zzz")), (1, Some("zzz".to_owned())), "unknown");
        assert_eq!(stale(Some("a1")), (1, Some("a1".to_owned())), "ambiguous prefix");
        assert_eq!(stale(Some("a%3Cb%3E+c")), (1, Some("a<b> c".to_owned())), "kept decoded");
        for resolving in ["P2", "a1v", "A1V"] {
            assert_eq!(stale(Some(resolving)), (2, None), "{resolving} resolves");
        }
        assert_eq!(stale(None), (1, None), "absent");
        assert_eq!(stale(Some("")), (1, None), "empty");
        assert_eq!(stale(Some("+%20")), (1, None), "whitespace only");
        // A zero-page compound has no first page to name: no status.
        let empty = project
            .compound_parts("B0", 1, Some("zzz"), None, false, &lang)
            .expect("compound_parts should not error");
        assert_eq!((empty.total, empty.stale_page), (0, None));
    }

    // ── media_items / media_thumbs integration tests ─────────────────────────
    //
    // A Character host resource ("c1") pointing at three Image resources via
    // its `image` link property, at seq 2, 0, 1 (deliberately out of id
    // order, mirroring the query-layer fixture in `query.rs`'s
    // `setup_media_db`) — exercises the seqnum-ordering contract end to end
    // through `Project::media_items`, which delegates to
    // `ProjectReads::media_items`'s lock-once-drop-then-hydrate path.

    const MEDIA_FIXTURE_SQL: &str = "
        INSERT INTO resources (id, class) VALUES
            ('c1', 'Character'),
            ('I_001', 'Image'),
            ('I_002', 'Image'),
            ('I_003', 'Image');
        INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
            ('c1', 'image', 2, 'link', 'I_001'),
            ('c1', 'image', 0, 'link', 'I_002'),
            ('c1', 'image', 1, 'link', 'I_003');
        INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
            ('I_001', 'name', 0, 'text', 'First', 'en'),
            ('I_002', 'name', 0, 'text', 'Second', 'en'),
            ('I_003', 'name', 0, 'text', 'Third', 'en');
    ";

    #[test]
    fn media_items_returns_hydrated_resources_in_seqnum_order() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, MEDIA_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();

        let items = project.media_items("c1", "image", &lang).expect("media_items should not error");

        let ids: Vec<&str> = items.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["I_002", "I_003", "I_001"],
            "must be hydrated in properties.seq order (0, 1, 2), got: {ids:?}"
        );
    }

    /// `compound_parts(…, Some(property), …)` fills `member_media` with the
    /// CURRENT member's linked Images (Seqnum order); `None` leaves it empty
    /// (the member scan's data path).
    #[test]
    fn compound_parts_fills_member_media_only_when_a_property_is_given() {
        let sql = "
            INSERT INTO resources (id, class) VALUES
                ('B1', 'Book'), ('P1', 'Page'), ('P2', 'Page'),
                ('I_1', 'Image'), ('I_2', 'Image');
            INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
                ('B1', 'P1', 0), ('B1', 'P2', 1);
            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('P1', 'image', 0, 'link', 'I_1'),
                ('P2', 'image', 0, 'link', 'I_2');
        ";
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, sql, &[]).expect("fixture should load");
        let lang = make_lang_en();

        let with = project
            .compound_parts("B1", 2, None, Some("image"), false, &lang)
            .expect("compound_parts should not error");
        let ids: Vec<&str> = with.member_media.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["I_2"], "member 2's own image, not member 1's");

        let without = project
            .compound_parts("B1", 2, None, None, false, &lang)
            .expect("compound_parts should not error");
        assert!(without.member_media.is_empty(), "no media property ⇒ no member_media");
    }

    #[test]
    fn media_items_empty_for_resource_with_no_media_values() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, MEDIA_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();

        let items = project
            .media_items("I_001", "image", &lang)
            .expect("media_items should not error");
        assert!(
            items.is_empty(),
            "a resource with no 'image' link values must return [], got: {items:?}"
        );
    }

    // ── facet_options integration tests ──────────────────────────────────────
    //
    // Use a fixture with two Character resources, each with list_value `role`
    // properties pointing at nodes n1 (count=2) and n2 (count=1, only c2).
    // List node labels are provided for n1 (EN "Protagonist") and n2 (EN "Antagonist").

    /// SQL fixture for facet_options tests.
    /// Two Characters: c1 with role n1, c2 with roles n1 and n2.
    /// Node n1 has EN label "Protagonist"; n2 has EN label "Antagonist".
    const FACET_FIXTURE_SQL: &str = "
        INSERT INTO lists (id, name) VALUES ('roles', 'roles');
        INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
            ('n1', 'roles', NULL, 0),
            ('n2', 'roles', NULL, 1);
        INSERT INTO list_node_labels (node_id, lang, label) VALUES
            ('n1', 'en', 'Protagonist'),
            ('n2', 'en', 'Antagonist');
        INSERT INTO resources (id, class) VALUES
            ('c1', 'Character'),
            ('c2', 'Character');
        INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
            ('c1', 'role', 0, 'list_value', 'n1'),
            ('c2', 'role', 0, 'list_value', 'n1'),
            ('c2', 'role', 1, 'list_value', 'n2');
    ";

    #[test]
    fn facet_options_returns_correct_groups_and_counts() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, FACET_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();
        let facet = cpe_core::ir::Facet {
            property: "role".to_owned(),
            kind: cpe_core::ir::FacetKind::List,
        };
        let group = project
            .facet_options("Character", &facet, &[], &[], &lang)
            .expect("facet_options should not error");

        assert_eq!(group.property, "role", "property passthrough");
        assert_eq!(group.label, "Role", "humanized label: 'role' → 'Role'");
        assert!(matches!(group.kind, cpe_core::ir::FacetKind::List), "kind passthrough");
        assert_eq!(group.options.len(), 2, "two distinct nodes: {group:?}");

        // n1 count=2 → first (ORDER BY cnt DESC)
        let opt0 = &group.options[0];
        assert_eq!(opt0.value, "n1");
        assert_eq!(opt0.label, "Protagonist");
        assert_eq!(opt0.count, 2);
        assert!(!opt0.selected, "no active_ids → not selected");

        let opt1 = &group.options[1];
        assert_eq!(opt1.value, "n2");
        assert_eq!(opt1.label, "Antagonist");
        assert_eq!(opt1.count, 1);
        assert!(!opt1.selected);
    }

    #[test]
    fn facet_options_selected_from_active_ids() {
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, FACET_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();
        let facet = cpe_core::ir::Facet {
            property: "role".to_owned(),
            kind: cpe_core::ir::FacetKind::List,
        };
        let active_ids = vec!["n1".to_owned()];
        let group = project
            .facet_options("Character", &facet, &[], &active_ids, &lang)
            .expect("facet_options should not error");

        // n1 is active → selected=true; n2 is not → selected=false
        let opt_n1 = group.options.iter().find(|o| o.value == "n1").expect("n1 must be in options");
        let opt_n2 = group.options.iter().find(|o| o.value == "n2").expect("n2 must be in options");
        assert!(opt_n1.selected, "n1 should be selected (active_ids contains n1)");
        assert!(!opt_n2.selected, "n2 should not be selected");
    }

    #[test]
    fn facet_options_counts_independent_of_active_ids() {
        // active_ids must NOT affect counts — counts are computed against base_wheres only.
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, FACET_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();
        let facet = cpe_core::ir::Facet {
            property: "role".to_owned(),
            kind: cpe_core::ir::FacetKind::List,
        };

        // With no active_ids:
        let group_unfiltered = project
            .facet_options("Character", &facet, &[], &[], &lang)
            .expect("should not error");

        // With n1 active:
        let group_with_active = project
            .facet_options("Character", &facet, &[], &["n1".to_owned()], &lang)
            .expect("should not error");

        // Counts must be identical — active_ids only flips `selected`
        let counts_unfiltered: Vec<usize> = group_unfiltered.options.iter().map(|o| o.count).collect();
        let counts_with_active: Vec<usize> = group_with_active.options.iter().map(|o| o.count).collect();
        assert_eq!(
            counts_unfiltered, counts_with_active,
            "active_ids must not affect counts (static counts design)"
        );
    }

    #[test]
    fn facet_counts_are_static_regardless_of_active_selection() {
        // Facet option
        // counts are computed against the class + config `where`s only —
        // they are STATIC and must never be narrowed by the active facet
        // selection. `active_ids` may only ever set `FacetOption::selected`;
        // it must never change any `FacetOption::count`.
        //
        // Unlike `facet_options_counts_independent_of_active_ids` above,
        // this test also asserts that `selected` DID flip for the toggled
        // value — proving `active_ids` is actually wired at all, so an
        // accidental "counts never change because active_ids is a no-op"
        // regression couldn't make this test pass vacuously.
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, MINIMAL_CONFIG, FACET_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();
        let facet = cpe_core::ir::Facet {
            property: "role".to_owned(),
            kind: cpe_core::ir::FacetKind::List,
        };

        // Before: no active selection.
        let before = project
            .facet_options("Character", &facet, &[], &[], &lang)
            .expect("facet_options should not error");

        // After: toggle "n1" on, simulating a visitor clicking that facet option.
        let after = project
            .facet_options("Character", &facet, &[], &["n1".to_owned()], &lang)
            .expect("facet_options should not error");

        // 1) Counts must be identical, compared as a value → count map so the check is independent
        //    of option ordering.
        let counts_before: std::collections::HashMap<&str, usize> =
            before.options.iter().map(|o| (o.value.as_str(), o.count)).collect();
        let counts_after: std::collections::HashMap<&str, usize> =
            after.options.iter().map(|o| (o.value.as_str(), o.count)).collect();
        assert_eq!(
            counts_before, counts_after,
            "toggling a filter must not change any option's count \
             (AC4: counts are class+config-where-only, static)"
        );

        // 2) `selected` must actually flip for "n1" — otherwise (1) could pass vacuously if
        //    active_ids were accidentally left unwired.
        let n1_before = before
            .options
            .iter()
            .find(|o| o.value == "n1")
            .expect("n1 must be present in the unfiltered result");
        let n1_after = after
            .options
            .iter()
            .find(|o| o.value == "n1")
            .expect("n1 must be present in the toggled result");
        assert!(!n1_before.selected, "n1 must not be selected with empty active_ids");
        assert!(n1_after.selected, "n1 must be selected once active_ids contains it");
    }

    #[test]
    fn facet_options_orphan_node_fallback_uses_node_id() {
        // n3 has no label rows → label fallback = node id "n3"
        let ctx = ValidationContext::for_tests("p");
        let orphan_sql = "
            INSERT INTO lists (id, name) VALUES ('roles', 'roles');
            INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
                ('n3', 'roles', NULL, 0);
            -- no list_node_labels row for n3 → orphan node
            INSERT INTO resources (id, class) VALUES ('c1', 'Character');
            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('c1', 'role', 0, 'list_value', 'n3');
        ";
        let project = Project::load(&ctx, MINIMAL_CONFIG, orphan_sql, &[]).expect("fixture should load");
        let lang = make_lang_en();
        let facet = cpe_core::ir::Facet {
            property: "role".to_owned(),
            kind: cpe_core::ir::FacetKind::List,
        };
        let group = project
            .facet_options("Character", &facet, &[], &[], &lang)
            .expect("facet_options should not error");

        assert_eq!(group.options.len(), 1, "one option for the orphan node");
        assert_eq!(group.options[0].label, "n3", "orphan node: label must fall back to node id");
        assert_eq!(group.options[0].value, "n3");
    }

    #[test]
    fn facet_options_respects_base_wheres() {
        // With a base_where that only matches c1 (name=Alice), n2 should not appear
        // because n2 only exists on c2 (which has no name property matching "Alice").
        let ctx = ValidationContext::for_tests("p");
        let lang = make_lang_en();
        let facet = cpe_core::ir::Facet {
            property: "role".to_owned(),
            kind: cpe_core::ir::FacetKind::List,
        };
        // Build a separate fixture with a name property on c1 so we can filter to it.
        let sql_with_names = "
            INSERT INTO lists (id, name) VALUES ('roles', 'roles');
            INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
                ('n1', 'roles', NULL, 0),
                ('n2', 'roles', NULL, 1);
            INSERT INTO list_node_labels (node_id, lang, label) VALUES
                ('n1', 'en', 'Protagonist'),
                ('n2', 'en', 'Antagonist');
            INSERT INTO resources (id, class) VALUES ('c1', 'Character'), ('c2', 'Character');
            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('c1', 'role', 0, 'list_value', 'n1'),
                ('c2', 'role', 0, 'list_value', 'n1'),
                ('c2', 'role', 1, 'list_value', 'n2');
            INSERT INTO properties (resource_id, name, seq, value_type, value_text, value_lang) VALUES
                ('c1', 'name', 0, 'text', 'Alice', 'en');
        ";
        let project2 = Project::load(&ctx, MINIMAL_CONFIG, sql_with_names, &[]).expect("should load");
        let base_wheres = vec![WhereClause::Equals {
            property: "name".to_owned(),
            value: cpe_core::ir::Value::String("Alice".to_owned()),
        }];
        let group = project2
            .facet_options("Character", &facet, &base_wheres, &[], &lang)
            .expect("facet_options with base_wheres should not error");

        // Only c1 is in the base (name=Alice). c1 only has n1, so n2 shouldn't appear.
        assert_eq!(
            group.options.len(),
            1,
            "base_where filters to c1 → only n1 should appear: {group:?}"
        );
        assert_eq!(group.options[0].value, "n1");
        assert_eq!(group.options[0].count, 1);
    }

    #[test]
    fn facet_options_humanizes_hyphenated_property_name() {
        // "location-type" → "Location Type"
        let ctx = ValidationContext::for_tests("p");
        let sql = "
            INSERT INTO lists (id, name) VALUES ('location-types', 'location-types');
            INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
                ('lt1', 'location-types', NULL, 0);
            INSERT INTO list_node_labels (node_id, lang, label) VALUES
                ('lt1', 'en', 'Wonderland');
            INSERT INTO resources (id, class) VALUES ('loc1', 'Location');
            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('loc1', 'location-type', 0, 'list_value', 'lt1');
        ";
        let project = Project::load(&ctx, MINIMAL_CONFIG, sql, &[]).expect("fixture should load");
        let lang = make_lang_en();
        let facet = cpe_core::ir::Facet {
            property: "location-type".to_owned(),
            kind: cpe_core::ir::FacetKind::List,
        };
        let group = project
            .facet_options("Location", &facet, &[], &[], &lang)
            .expect("facet_options should not error");

        assert_eq!(group.label, "Location Type", "hyphenated → title-cased words");
    }

    /// When the project's `labels {
    /// property "…" { … } }` block declares a translation for the facet's
    /// property, `facet_options` must return the DECLARED translation as
    /// `FacetGroup.label`, not the humanized property name (mirrors
    /// `facet_options_humanizes_hyphenated_property_name` above, which locks
    /// the fallback path with no labels declared).
    #[test]
    fn facet_options_resolves_declared_property_label() {
        const LABELED_CONFIG: &str = r#"
site {
    title "T"
    languages "en" "de"
}
labels {
    property "role" {
        en "Character Role"
        de "Rolle der Figur"
    }
}
page id="home" route="/"
"#;
        let ctx = ValidationContext::for_tests("p");
        let project = Project::load(&ctx, LABELED_CONFIG, FACET_FIXTURE_SQL, &[]).expect("fixture should load");
        let lang = make_lang_en();
        let facet = cpe_core::ir::Facet {
            property: "role".to_owned(),
            kind: cpe_core::ir::FacetKind::List,
        };
        let group = project
            .facet_options("Character", &facet, &[], &[], &lang)
            .expect("facet_options should not error");

        assert_eq!(
            group.label, "Character Role",
            "declared label must win over the humanized fallback ('Role'), got: {group:?}"
        );
    }

    #[test]
    fn facet_options_lang_fallback_uses_first_present_label() {
        // When the requested language lacks a label for a node, `facet_options`
        // must surface the fallback label (the first available translation) —
        // not the node-id and not empty string.
        //
        // Fixture: two nodes.
        //   n1: has both EN ("Protagonist") and DE ("Protagonistin") labels.
        //   n2: has EN ("Antagonist") only — no DE label.
        //
        // Requesting "de" (a language declared by MINIMAL_CONFIG) must:
        //   - n1 → "Protagonistin" (DE label present → resolved directly)
        //   - n2 → "Antagonist"    (DE absent → fallback to EN, the only row)
        //
        // This mirrors the query-layer test
        // `list_facet_values_lang_fallback_for_missing_translation` but exercises
        // the `Project::facet_options` integration path.
        let ctx = ValidationContext::for_tests("p");

        // Build a DE lang against the declared EN+DE site.
        // MINIMAL_CONFIG declares "en" and "de", so "de" is a valid declared lang.
        use cpe_core::ir::SiteConfig;
        use cpe_core::lang::resolve;
        let site_en_de = SiteConfig {
            title: "T".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned(), "de".to_owned()],
            default_language: "en".to_owned(),
            nav: vec![],
            chrome: vec![],
            property_labels: vec![],
            class_labels: vec![],
            theme: cpe_core::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: vec![],
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        };
        let lang_de = resolve(&["de".to_owned()], &site_en_de);

        let facet = cpe_core::ir::Facet {
            property: "role".to_owned(),
            kind: cpe_core::ir::FacetKind::List,
        };

        // FACET_FIXTURE_SQL only has EN labels; build a separate fixture that
        // adds a DE label for n1 (but NOT for n2) to exercise the fallback path.
        let sql_with_de = "
            INSERT INTO lists (id, name) VALUES ('roles', 'roles');
            INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
                ('n1', 'roles', NULL, 0),
                ('n2', 'roles', NULL, 1);
            INSERT INTO list_node_labels (node_id, lang, label) VALUES
                ('n1', 'en', 'Protagonist'),
                ('n1', 'de', 'Protagonistin'),
                ('n2', 'en', 'Antagonist');
                -- n2 intentionally has no DE label → triggers the fallback path
            INSERT INTO resources (id, class) VALUES
                ('c1', 'Character'),
                ('c2', 'Character');
            INSERT INTO properties (resource_id, name, seq, value_type, target_id) VALUES
                ('c1', 'role', 0, 'list_value', 'n1'),
                ('c2', 'role', 0, 'list_value', 'n1'),
                ('c2', 'role', 1, 'list_value', 'n2');
        ";
        let project_de = Project::load(&ctx, MINIMAL_CONFIG, sql_with_de, &[]).expect("fixture should load");

        let group = project_de
            .facet_options("Character", &facet, &[], &[], &lang_de)
            .expect("facet_options should not error");

        assert_eq!(group.options.len(), 2, "two nodes must appear: {group:?}");

        // n1 has count=2 → first option (ORDER BY cnt DESC).
        let opt_n1 = group.options.iter().find(|o| o.value == "n1").expect("n1 must be in options");
        assert_eq!(
            opt_n1.label, "Protagonistin",
            "n1 must use DE label when DE is present (requested lang = DE)"
        );

        // n2 has no DE label → must fall back to the EN label, NOT the node-id.
        let opt_n2 = group.options.iter().find(|o| o.value == "n2").expect("n2 must be in options");
        assert_eq!(
            opt_n2.label, "Antagonist",
            "n2 must fall back to EN label when DE is absent — must not be node-id 'n2'"
        );
        // Verify the fallback is not the node-id (explicit guard against regressing
        // to the orphan-node fallback).
        assert_ne!(
            opt_n2.label, "n2",
            "fallback must be a real label (EN), not the node-id — node-id fallback \
             is only for nodes with no label rows at all"
        );
    }

    // ── page_data: which side data each scope fills ───────────────────────────

    const GOOD_MEDIA_CONFIG: &str = include_str!("../tests/projects/good-media/config.kdl");
    const GOOD_MEDIA_DATA: &str = include_str!("../tests/projects/good-media/data.sql");

    /// A request with no query string: every value at its default.
    struct NoQuery;

    impl cpe_core::view_data::PageParams for NoQuery {
        fn list_page(&self) -> u32 {
            1
        }
        fn member(&self) -> u32 {
            1
        }
        fn reader_page(&self) -> Option<&str> {
            None
        }
        fn sorts(
            &self,
            config: &[Sort],
            _options: &[cpe_core::ir::OrderOption],
            _name_property: Option<&str>,
        ) -> Vec<Sort> {
            config.to_vec()
        }
        fn facet_values(&self, _property: &str) -> Vec<String> {
            Vec::new()
        }
        fn search(&self) -> String {
            String::new()
        }
    }

    /// The page data for `page_id` under each scope, in the order `Page`,
    /// `CompoundFragment`, `ListFragment`.
    fn page_data_by_scope(
        project: &Project,
        page_id: &str,
        route_params: &std::collections::HashMap<String, String>,
    ) -> [PageData; 3] {
        use cpe_core::view_data::PageScope;
        let page = project
            .config()
            .pages
            .iter()
            .find(|p| p.id == page_id)
            .expect("good-media declares the page");
        let lang = cpe_core::lang::resolve(&[], &project.config().site);
        [PageScope::Page, PageScope::CompoundFragment, PageScope::ListFragment].map(|scope| {
            let request = PageRequest { route_params, params: &NoQuery, scope };
            project.page_data(page, &request, &lang)
        })
    }

    #[test]
    fn page_data_fills_the_side_data_each_scope_asks_for() {
        let ctx = ValidationContext::for_tests("good-media");
        let project =
            Project::load(&ctx, GOOD_MEDIA_CONFIG, GOOD_MEDIA_DATA, &[]).expect("good-media fixture should load");

        let route_params = std::collections::HashMap::from([("id".to_owned(), "r1".to_owned())]);
        let [page, compound, list] = page_data_by_scope(&project, "character", &route_params);
        let media_ids = |data: &PageData| -> Vec<String> { data.media_items.iter().map(|r| r.id.clone()).collect() };
        assert_eq!(media_ids(&page), ["img1"], "Page fills media_items");
        assert_eq!(media_ids(&compound), ["img1"], "CompoundFragment fills media_items");
        assert!(list.media_items.is_empty(), "ListFragment leaves media_items empty");
        for other in [&compound, &list] {
            assert_eq!(other.components, page.components);
            assert_eq!(other.status, page.status);
        }
        assert_eq!(page.status, cpe_core::view_data::PageStatus::Ok);

        let no_params = std::collections::HashMap::new();
        let [page, compound, list] = page_data_by_scope(&project, "characters", &no_params);
        for data in [&page, &compound, &list] {
            assert!(data.media_thumbs.contains_key("r1"), "every scope fills media_thumbs");
        }
        for other in [&compound, &list] {
            assert_eq!(other.components, page.components);
            assert_eq!(other.status, page.status);
        }
        assert_eq!(page.status, cpe_core::view_data::PageStatus::Ok);
    }
}

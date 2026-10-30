//! The **built-in engine chrome message catalog** — the single seam for every
//! language-varying UI string the engine renders (skip-link, nav aria-labels,
//! not-found/empty-state copy, facet/compound/pager chrome, …).
//!
//! This is cross-cutting data shared by every renderer (not a per-surface
//! renderer itself), so it is a top-level module of `cpe-components`, beside
//! `component`. It is `pub` because `cpe-web`'s pages without a project (the overview of the
//! projects, CPE's not-found) read it. It belongs to the components crate, so `cpe-core` (the
//! domain crate) cannot import it.
//!
//! Two message shapes:
//!
//! - **Fixed messages** — [`ChromeMsg`], an enum where every variant carries an `L4` (all four
//!   built-in languages, no exceptions). Call sites read `ChromeMsg::ClearAll.text(lang)`. A
//!   handful of thin `pub(crate)` wrapper functions group related fixed messages back into the
//!   tuples call sites already expect (e.g. `bool_labels` → `(yes, no)`), without introducing a
//!   second message shape for what is still, per-element, a fixed message.
//! - **Parameterized / pluralized messages** — kept as their own `pub(crate) fn`s, each fusing the
//!   language `match` with a literal `format!` (the template must be a compile-time string literal,
//!   so a runtime-`pick`ed template could never be passed to `format!` — the language branch and
//!   interpolation must stay fused).
//!
//! Every string in this module carries provenance in its doc comment:
//! **"pre-existing draft"** (migrated verbatim from a per-module
//! `match lang.as_str()` helper that predates this catalog — do not silently
//! reword it here) or **"new"** (authored for this catalog's gap-fill). Both
//! are still unreviewed drafts pending native-speaker review.

use std::borrow::Cow;

use cpe_core::lang::Lang;
pub use cpe_core::lang::BUILTIN_LANGUAGES;
use maud::{html, Markup};

/// A message translated into all four built-in engine languages.
///
/// All four fields are **required** — this is the completeness guarantee for
/// engine UI chrome: it is structurally impossible to add a [`ChromeMsg`]
/// variant without supplying every language.
///
/// **Do not derive `Default` on this struct.** A `..Default::default()`
/// literal would silently fill unspecified languages with `""`, defeating the
/// completeness guarantee this type exists to provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct L4 {
    pub(crate) en: &'static str,
    pub(crate) de: &'static str,
    pub(crate) fr: &'static str,
    pub(crate) it: &'static str,
}

impl L4 {
    /// Pick this message's string for `lang`. Any language outside
    /// [`BUILTIN_LANGUAGES`] (a project-declared 5th+ language) falls back to
    /// English — this is the one arm every message shares, so completeness
    /// only has to be proven across four fields, not against an open set of
    /// project-declared languages.
    fn pick(&self, lang: &Lang) -> &'static str {
        match lang.as_str() {
            "de" => self.de,
            "fr" => self.fr,
            "it" => self.it,
            _ => self.en,
        }
    }
}

/// Every fixed (non-parameterized, non-pluralized) engine UI chrome string,
/// grouped by the module it renders in. Call sites read
/// `ChromeMsg::Variant.text(lang)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeMsg {
    // `dsp-incubator cpe/web`'s `page` module — shell chrome
    /// The skip-to-content link's visible/accessible text. Shared verbatim by
    /// `render_page`, the unknown-path page, and `cpe-web`'s `render_site_index`
    /// (independent full-page renderers — see `dsp-incubator cpe/CLAUDE.md`'s Non-obvious
    /// constraints). Pre-existing draft (EN/DE only; fr/it are new).
    SkipToContent,
    /// Appearance-toggle accessible name when the toggle currently offers
    /// "switch to light" (i.e. the page is in dark mode). Shared verbatim by
    /// `shell_header` and `cpe-web`'s `site_page`. New.
    ThemeToggleToLight,
    /// Appearance-toggle accessible name when the toggle offers "switch to
    /// dark". See [`ChromeMsg::ThemeToggleToLight`].
    ThemeToggleToDark,
    /// `aria-label` for the primary navigation `<nav>`. New.
    NavPrimaryLabel,
    /// The language row's label in the header menu panel ("Sprache"), and the
    /// prefix of [`language_current_text`].
    LanguageSwitcherLabel,
    /// The header burger's label ("Menü"): visible from 641px, visually
    /// hidden on a phone, and the button's accessible name at every width.
    MenuLabel,
    /// The appearance row's label in the header menu panel ("Darstellung").
    AppearanceLabel,
    /// The panel form's visible appearance label in light mode: it names the
    /// mode the toggle switches TO ("Dunkler Modus").
    ThemeDarkModeLabel,
    /// The panel form's visible appearance label in dark mode ("Heller Modus").
    ThemeLightModeLabel,
    /// The visually hidden end of the panel form's appearance name, after
    /// " – " ("umschalten"), so the name starts with the visible label.
    ThemeSwitchSuffix,
    /// The per-project footer attribution sentence (`p.footer__attr`). New. A
    /// single sentence with "DaSCH" as a plain proper noun (no `<strong>`): the
    /// position of an emphasis would vary by language, e.g. German puts
    /// "des DaSCH" at the end.
    FooterAttribution,
    /// `aria-label` of the DaSCH lockup in the per-project footer: the org's
    /// long name, unlike `DaschAriaLabel`'s bare "DaSCH". fr/it keep the English
    /// official name (drafts).
    FooterDaschLabel,

    // `component/breadcrumb.rs`
    /// `aria-label` for the resource-detail breadcrumb `<nav>`. New.
    BreadcrumbLabel,
    /// The phone back link's visually hidden prefix ("Back:"), before the
    /// parent crumb's label. It ends in a colon so that the label needs no
    /// declension. fr/it are unreviewed drafts.
    CrumbBackPrefix,

    // `component/facet_panel.rs`
    /// "Filters" — both the facet panel's `aria-label` and the filterbar's
    /// visible label (identical wording in all four languages, so one message
    /// serves both call sites). Pre-existing draft (was `filters_word`).
    FacetFiltersWord,
    /// Empty-state message for a facet panel with no options. Pre-existing
    /// draft (was `no_filters_message`).
    FacetNoFiltersMessage,
    /// Filterbar "Clear all" action label. Pre-existing draft (was
    /// `clear_all_label`).
    FacetClearAllLabel,
    /// Per-group "clear" action label. Pre-existing draft (was
    /// `group_clear_label`).
    FacetGroupClearLabel,

    // `component/compound_viewer.rs`
    /// Generic (non-class-aware) empty-state message for a compound with no
    /// members. Pre-existing draft. The class-aware
    /// `compound_no_members_text` is used whenever the compound's parent
    /// Resource resolves; this variant is reserved for the degrade path
    /// where the parent's source id itself doesn't resolve (no class known
    /// to interpolate).
    CompoundEmptyState,
    /// Prev-control accessible name ("Previous"). Pre-existing draft (was one
    /// half of `compound_step_labels`).
    CompoundPrevLabel,
    /// Next-control accessible name ("Next"). See [`ChromeMsg::CompoundPrevLabel`].
    CompoundNextLabel,
    /// Static `aria-label` for the compound viewer's contents-rail `<nav>` ("Contents").
    /// Pre-existing draft (was `compound_contents_label`).
    CompoundContentsLabel,
    /// Static `aria-label` of the `pages` reader's pager row
    /// (`nav.pg.pg--reader`, "Turn pages"). The `contents` layout keeps
    /// `compound_nav_label`'s count-bearing label. FR/IT drafts.
    CompoundPagerLabel,
    /// The `pages` reader head's eyebrow suffix for a member with no label
    /// ("no label", as in `PAGE · NO LABEL`). FR/IT drafts — see
    /// [`ChromeMsg::CompoundPagerLabel`].
    CompoundMemberUnlabelled,
    /// Summary of the `pages` reader head's closed aside disclosure
    /// ("more"): reveals the full page description in place. FR/IT drafts —
    /// see [`ChromeMsg::CompoundPagerLabel`].
    CompoundAsideMore,
    /// The same summary while open ("less"). See
    /// [`ChromeMsg::CompoundAsideMore`].
    CompoundAsideLess,
    /// The `pages` reader's page-picker summary ("Choose page"). FR/IT
    /// drafts — see [`ChromeMsg::CompoundPagerLabel`].
    PickerSummary,
    /// The picker's jump-mark band label in signatures mode ("Gatherings").
    PickerIndexGatherings,
    /// The band label in numbered mode ("Pages").
    PickerIndexPages,
    /// The front-flyleaves group's visible mark ("Front").
    PickerFrontMark,
    /// The front-flyleaves group's full (visually hidden) name ("Front
    /// flyleaves").
    PickerFrontName,
    /// The back-flyleaves group's visible mark ("Back").
    PickerBackMark,
    /// The back-flyleaves group's full name ("Back flyleaves").
    PickerBackName,
    /// An unparsed run between two gatherings ("Other"): its mark and name.
    PickerOther,
    /// The `pages` reader's stale-address line, before the `?page=` value
    /// ("The address names no page of this book:"). FR/IT
    /// drafts — see [`ChromeMsg::CompoundPagerLabel`].
    CompoundStaleAddress,

    // `component/resource_detail.rs`
    /// Boolean-value "Yes" label. Pre-existing draft (was one half of the
    /// inline `(yes, no)` match in the property-value renderer).
    BoolYes,
    /// Boolean-value "No" label. See [`ChromeMsg::BoolYes`].
    BoolNo,
    /// Date precision word for a day-precise single date (`span.prec`, only
    /// when `site { date-precision-word #true }`). New; fr/it drafts.
    DatePrecDay,
    /// Date precision word for a month-precise single date. See
    /// [`ChromeMsg::DatePrecDay`].
    DatePrecMonth,
    /// Date precision word for a year-precise single date. See
    /// [`ChromeMsg::DatePrecDay`].
    DatePrecYear,
    /// Date precision word for a range (`ir::date_is_range` — there is no
    /// stored `range` precision). See [`ChromeMsg::DatePrecDay`].
    DatePrecRange,
    /// Calendar word appended (` · gregorianisch`) after the precision word
    /// when a date's `calendar` is Gregorian and its precision is not year
    /// (`component::values::calendar_word`). Julian is the unmarked case and
    /// has no word. See [`ChromeMsg::DatePrecDay`].
    CalendarGregorian,
    /// Carousel prev-control accessible name ("Previous image"). Pre-existing
    /// draft (was one half of `carousel_control_labels`).
    CarouselPrevImageLabel,
    /// Carousel next-control accessible name ("Next image"). See
    /// [`ChromeMsg::CarouselPrevImageLabel`].
    CarouselNextImageLabel,
    /// `aria-label` for the carousel `<nav>` element. Pre-existing draft (was
    /// `carousel_nav_label`).
    CarouselNavLabel,
    /// `aria-label` for the media plate's `.pg` pager `<nav>`. fr/it are fresh
    /// drafts pending native-speaker review.
    PlatePagerNavLabel,
    /// Prev-control accessible name on the media plate's `.pg` pager
    /// ("Previous image"). See [`ChromeMsg::PlatePagerNavLabel`].
    PlatePagerPrevLabel,
    /// Next-control accessible name on the media plate's `.pg` pager
    /// ("Next image"). See [`ChromeMsg::PlatePagerNavLabel`].
    PlatePagerNextLabel,
    /// Thumb-strip toggle label when the strip is expanded ("Hide
    /// thumbnails"). See [`ChromeMsg::PlatePagerNavLabel`].
    PlateStripCollapseLabel,
    /// Thumb-strip toggle label when the strip is collapsed, count-aware
    /// ("Show N more" — the count is spliced in by
    /// `crate::component::media::strip_toggle_text`). See
    /// [`ChromeMsg::PlatePagerNavLabel`].
    PlateStripExpandLabel,
    /// The mono eyebrow above the media plate ("Illustrations"). See
    /// [`ChromeMsg::PlatePagerNavLabel`].
    PlateIllustrationsLabel,
    /// Placeholder text for an unresolved list value. Pre-existing draft (was
    /// `unresolved_list_placeholder`).
    UnresolvedListPlaceholder,
    /// Image-placeholder `<figure aria-label>` on the absent-plate stage
    /// ("Image unavailable"). Pre-existing draft.
    ImagePlaceholderAriaLabel,

    // `component/resource_list.rs`
    /// `aria-label` for the pager `<nav>` element. Pre-existing draft (was
    /// `pager_nav_label`).
    PagerNavLabel,

    // `cpe-web`'s `page` module — the overview of the projects (site index)
    /// `aria-label` on the site-index brand link ("CPE — site index"). New.
    SiteIndexAriaLabel,
    /// `aria-label`/`alt` for the DaSCH logo lockup in the site-index footer.
    /// New — the value is the org's proper name and does not change across
    /// languages, but is still routed through the catalog (rather than an
    /// inline literal) so the site-index's chrome has no orphan strings.
    DaschAriaLabel,
    /// Empty-state message when no projects are configured. New.
    SiteIndexNoProjects,
    /// "Projects" list heading. New.
    SiteIndexProjectsHeading,
    /// Per-project "Enter" call-to-action. New.
    SiteIndexEnterCta,
    /// Site-index footer tagline — the expansion of the "CPE" acronym
    /// ("Configurable Presentation Environment"). New.
    SiteIndexTagline,
    /// Site-index footer PoC description paragraph. New.
    SiteIndexPocDesc,
    /// "DaSCH website" external link label in the site-index footer. New.
    SiteIndexDaschLinkLabel,

    // `component/search_bar.rs`
    /// The search row's visible `<label>` text ("Search"). New — fr/it are
    /// unreviewed drafts.
    SearchLabel,
    /// The search submit control's `title`/`aria-label` ("Search"/"Suchen" —
    /// distinct from [`ChromeMsg::SearchLabel`] in German, hence a separate
    /// variant rather than reuse). New — fr/it are unreviewed drafts.
    SearchSubmit,
    /// The search clear (`×`) control's `title`/`aria-label` ("Clear
    /// search"). New — fr/it are unreviewed drafts.
    SearchClear,
    /// The `.search-empty` state's mono hint line ("Matching is on the
    /// beginning of words — try a shorter term."). New — fr/it are
    /// unreviewed drafts.
    SearchHint,

    // `component/media.rs` — the plate access line
    /// The plate's ACCESS line for a restricted (`limited-view`) image
    /// (`.em-access`). One source of truth for
    /// the restricted-view sentence.
    RfAccessRestricted,

    // `component/media.rs` — the record viewer's zoom chrome
    /// `title`/`aria-label` on the viewer bar's "zoom in" chip
    /// (`data-viewer-zoom="in"`). New — fr/it are
    /// unreviewed drafts pending native-speaker review, per this catalog's
    /// usual provenance discipline (see the module doc comment above).
    RvZoomInLabel,
    /// `title`/`aria-label` on the viewer bar's "zoom out" chip
    /// (`data-viewer-zoom="out"`). New — fr/it
    /// are unreviewed drafts pending native-speaker review.
    RvZoomOutLabel,
    /// `title`/`aria-label` on the viewer bar's "fit to view" chip
    /// (`data-viewer-zoom="home"`). New — fr/it
    /// are unreviewed drafts pending native-speaker review.
    RvZoomHomeLabel,
    /// The quiet mono line shown in `.rv__bar` when the crate's
    /// `viewer-error` event fires ("not a dialog"). Deliberately a
    /// translated, generic sentence, not the raw
    /// JS error text — see `VIEWER_ERROR_EXPR`'s doc comment in
    /// `component/media.rs`. New — fr/it are unreviewed drafts pending
    /// native-speaker review.
    RvViewerErrorLabel,

    // `component/media.rs` — the record viewer's drawers
    /// The bar's "Details" drawer toggle — mono label, deliberately no icon.
    /// Also doubles as the
    /// Details rail's own head label. New — fr/it are unreviewed drafts
    /// pending native-speaker review.
    RvDetailsToggleLabel,
    /// The bar's "Adjust" drawer toggle (paired with `Icon::Sliders`).
    /// Also doubles as the Adjust rail's own
    /// head label. New — fr/it are unreviewed drafts pending native-speaker
    /// review.
    RvAdjustToggleLabel,
    /// `title`/`aria-label` on the bar's WIDER toggle (icon-only, paired with
    /// `Icon::Maximize`; `aria-pressed` conveys the pressed/expanded state
    /// so the label itself stays constant).
    /// New — fr/it are unreviewed drafts pending native-speaker review.
    RvWiderToggleLabel,
    /// `title`/`aria-label` on each rail's `x` close control.
    /// New — fr/it are unreviewed drafts pending
    /// native-speaker review.
    RvDrawerCloseLabel,
    /// The Details drawer's DESCRIPTION row label. New — fr/it are
    /// unreviewed drafts pending native-speaker review.
    RvFieldDescription,
    /// The Details drawer's ACCESS row label (restricted images only). See
    /// [`ChromeMsg::RvFieldDescription`].
    RvFieldAccess,
    /// The Details drawer's IDENTIFIER row label. See
    /// [`ChromeMsg::RvFieldDescription`].
    RvFieldIdentifier,
    /// The Details drawer's DIMENSIONS row label. See
    /// [`ChromeMsg::RvFieldDescription`].
    RvFieldDimensions,
    /// The Details drawer's IIIF SOURCE row label. See
    /// [`ChromeMsg::RvFieldDescription`].
    RvFieldIiifSource,
    /// The Details drawer's ACCESS row prose — a longer restricted-view
    /// explanation than the bar's rank-line [`ChromeMsg::RfAccessRestricted`].
    /// ACCESS is prose, and only in the Details drawer.
    /// New — fr/it are unreviewed drafts pending native-speaker review.
    RvAccessRestrictedLong,
    /// The Adjust drawer's brightness row label.
    /// New — fr/it are unreviewed drafts
    /// pending native-speaker review.
    RvAdjustBrightness,
    /// The Adjust drawer's contrast row label. See
    /// [`ChromeMsg::RvAdjustBrightness`].
    RvAdjustContrast,
    /// The Adjust drawer's saturation row label. See
    /// [`ChromeMsg::RvAdjustBrightness`].
    RvAdjustSaturation,
    /// The Adjust drawer's exposure row label. See
    /// [`ChromeMsg::RvAdjustBrightness`].
    RvAdjustExposure,
    /// The Adjust drawer's gamma row label. See
    /// [`ChromeMsg::RvAdjustBrightness`].
    RvAdjustGamma,
    /// The Adjust drawer's hue row label. See
    /// [`ChromeMsg::RvAdjustBrightness`].
    RvAdjustHue,
    /// The Adjust drawer's invert row label AND its chip's own text (kept as
    /// one string — `aria-pressed` already conveys the on/off state, so the
    /// chip doesn't need a second, different label). See
    /// [`ChromeMsg::RvAdjustBrightness`].
    RvAdjustInvert,
    /// The Adjust drawer's `RESET ALL` foot control: one `RESET ALL`, never a
    /// per-slider reset. New — fr/it are unreviewed drafts pending native-speaker
    /// review.
    RvAdjustResetLabel,
    /// The Adjust drawer's transient-looking note, read inside the scrolling
    /// body after the seven rows it explains. New — fr/it are
    /// unreviewed drafts pending native-speaker review.
    RvAdjustNote,

    // `component/resource_detail.rs` — `.pgroup`, the host's own rights
    /// `.pgroup__head`'s title. CSS applies
    /// `text-transform: uppercase`, so the source string stays normal-case
    /// (matches every other `.rf`/label string in this catalog). New — fr/it
    /// are unreviewed drafts pending native-speaker review.
    RvHostRightsTitle,
    /// The `.pgroup` rights-holder row label. See [`ChromeMsg::RvHostRightsTitle`].
    RvFieldRightsHolder,
    /// The `.pgroup` edition row label. See [`ChromeMsg::RvHostRightsTitle`].
    RvFieldEdition,
    /// The `.pgroup` licence row label. See [`ChromeMsg::RvHostRightsTitle`].
    RvFieldLicense,
    /// The fallback `.pgroup__title` cap of a config-declared property group
    /// (`resource-detail { group label="…" … }`),
    /// used when the group's `label=` chrome id does not resolve for the
    /// current language (boot-flagged as `UnknownGroupLabel`). CSS applies
    /// `text-transform: uppercase`. New — fr/it are unreviewed drafts pending
    /// native-speaker review.
    PropertyGroupTitle,
    /// The empty sentence of a `resource-detail`'s `incoming-links` list
    /// — the list resolved and nothing links to
    /// this record. Engine voice: names neither the source nor the target
    /// class. New — fr/it are unreviewed drafts pending native-speaker
    /// review.
    IncomingLinksEmpty,

    // `component/project/incunabula/timeline.rs` — the timeline
    /// The key line below the timeline's lanes variant, explaining the bar
    /// grammar. Engine-owned (it describes the component's own visual
    /// grammar, so a second project reusing the timeline inherits it). fr/it are
    /// unreviewed drafts.
    TimelineKey,

    // `component/project/incunabula/showcase.rs` — crop figures
    // fr/it are unreviewed drafts.
    /// The hidden end of a crop link's accessible name, after the book and
    /// page ("— Seite öffnen").
    CaseOpenPage,
    /// The label a crop's bed shows when its image fails ("Ausschnitt nicht
    /// geladen"); CSS sets it uppercase.
    CaseCropNotLoaded,
    /// The hidden lead before a page case's row ("Zwei Seiten, auf
    /// Seitenebene verknüpft:"); a page case always joins two pages.
    CaseSamePages,
    /// The eyebrow above a whole page's caption in a page case ("Ganze
    /// Seite").
    CaseWholePage,
    /// The note under a page case ("Verknüpfung auf Seitenebene — ohne
    /// markierte Region").
    CasePageNote,
    /// The detail crop's eyebrow word, before its page ("Detail").
    CaseDetail,
    /// An edition card's call to read the book ("Lesen"); the arrow after it
    /// is markup, hidden from assistive technology.
    EditionRead,

    // `component/editorial.rs` — the page rail
    /// The page rail's visible caption ("Auf dieser Seite"), which also
    /// names its `nav`. fr/it are unreviewed drafts.
    NarrativeRailLabel,

    // `component/resource_list.rs` — grid-mode view toggle
    /// The view-toggle's visible/accessible label ("View"). New; fr/it are unreviewed
    /// drafts.
    ViewLabel,
    /// The view-toggle's "List" option label. See [`ChromeMsg::ViewLabel`].
    ViewOptionList,
    /// The view-toggle's "Grid" option label. See [`ChromeMsg::ViewLabel`].
    ViewOptionGrid,
    /// The grid tile's decorative "no illustration" label, for a text-only
    /// tile with no representation to show. See [`ChromeMsg::ViewLabel`].
    GridNoImage,

    // empty states and not-found pages
    /// Resource-list empty-class message ("This class contains no
    /// resources."). New; italic `--ink-faint`.
    ListEmptyClass,
    /// Facet-group empty-state fallback for a language that cannot
    /// grammatically inflect the `{group}` slot (German compounds — see
    /// `facet_empty_group_text`). New.
    FacetEmptyGroupFallback,
    /// Absent-image "No image" mono mark, all three plate scales. Rendered uppercase
    /// by CSS — keep the string lower-case. New.
    ImageAbsentLabel,
    /// Figure-scale absent-image explanation ("This resource has no
    /// representation in the archive."). New.
    ImageAbsentRecord,
    /// Dark-plate-scale absent-image explanation ("This plate could not
    /// be resolved."). New.
    ImageAbsentPlate,
    /// Absent-image plate legal-band caption substitute when no attribution exists
    /// ("IIIF · not available"). New.
    ImageCaptionAbsent,
    /// Narrowed-to-zero "Clear all filters" action label. Distinct text
    /// from [`ChromeMsg::FacetClearAllLabel`] ("Clear all") — the empty-state
    /// action has no adjacent "FILTERS" label to supply the missing noun, so
    /// it spells "filters" out. New.
    EmptyClearFilters,
    /// The unresolved-link mark's mono text ("unresolved"). Rendered
    /// uppercase by CSS — keep the string lower-case. New.
    ValueUnresolved,
    /// `title` attribute on the unresolved-link mark. New.
    ValueUnresolvedTitle,
    /// The mono "404 — not found" mark of every not-found page. Rendered uppercase by
    /// CSS; em dash. New.
    NfCode,
    /// The unknown-path page's `<h1>` — "This page does not exist." New.
    NfPathTitle,
    /// The unknown-path page's body message. New.
    NfPathMsg,
    /// The unknown-path page's secondary line — "Or browse:", introducing at most three nav pages.
    /// New.
    NfOrBrowse,
    /// The unknown-project page's `<h1>` — "This address does not name a project." New.
    NfSiteTitle,
    /// The unknown-project page's body message. New.
    NfSiteMsg,
    /// The unknown-project page's only invitation row — "All projects", linking to `/`. New.
    NfAllProjects,
    /// The unknown-record page's `<h1>` fallback, used where a language has no comfortable singular
    /// for a configured class label, or the page's class is unset. New.
    NfIdTitleFallback,
    /// The unknown-record page's exit — "Project home". New.
    NfProjectHome,
    /// The unknown-record page's breadcrumb current-item label — "Not found". New.
    NfCrumb,

    // share view — copy-link control
    /// The copy-link control's resting visible/accessible label ("Copy
    /// link"). New — fr/it are unreviewed drafts pending
    /// native-speaker review.
    CopyLinkLabel,
    /// The copy-link control's 2.4s "copied" receipt label, replacing
    /// [`ChromeMsg::CopyLinkLabel`] in place.
    /// New — fr/it are unreviewed drafts pending native-speaker review.
    CopyLinkCopied,
    /// The `aria-live` announcement fired on a successful copy — distinct
    /// from the visible [`ChromeMsg::CopyLinkCopied`] receipt. New — fr/it are unreviewed drafts,
    /// drafted (not migrated) in the same register as the EN/DE source
    /// sentences.
    CopyLinkLive,
    /// The `title`/`aria-label` accessible-name **prefix** ("Copy link to
    /// this view") — the caller appends ` — <state in words>` on browse
    /// surfaces; this
    /// variant is only the prefix string itself. New — fr/it are unreviewed
    /// drafts, drafted in the same register as the EN/DE source sentences.
    CopyLinkLongPrefix,
    /// The phone crumb row's short resting label ("Copy"). It must stay a
    /// substring of [`ChromeMsg::CopyLinkLongPrefix`], the control's
    /// accessible name (WCAG 2.5.3). fr/it are unreviewed drafts.
    CopyLinkShort,

    // browse controls — ordering control + pager
    /// The ordering control's visible/`aria-label` prefix ("Order:
    /// `<current>`"). New — fr/it are unreviewed
    /// drafts pending native-speaker review.
    OrderLabel,
    /// "As curated" row label — the page's configured order, wire value
    /// `default`. New — fr/it unreviewed drafts.
    OrderCurated,
    /// "Name A–Z" row label, wire value `name`.
    /// New — de is identical to en; fr/it may differ and are unreviewed
    /// drafts.
    OrderNameAsc,
    /// "Name Z–A" row label, wire value `name-desc`. New — de is identical to en; fr/it may differ
    /// and are unreviewed drafts.
    OrderNameDesc,
    /// "Identifier" row label, wire value `id`. New —
    /// fr/it unreviewed drafts.
    OrderId,
    /// Pager inert/active "Previous" end label.
    /// New — fr/it unreviewed drafts.
    PagerPrev,
    /// Pager inert/active "Next" end label. New —
    /// fr/it unreviewed drafts.
    PagerNext,

    // `component/annotation.rs` — the annotation register
    // fr/it are unreviewed drafts.
    /// A rectangle annotation's shape word ("Rectangle").
    AnnoShapeRectangle,
    /// A polygon annotation's shape word ("Polygon").
    AnnoShapePolygon,
    /// A circle annotation's shape word ("Circle").
    AnnoShapeCircle,
    /// A page-level entry's shape word ("Whole page").
    AnnoWholePage,
    /// The register's toggle that shows or hides the overlay on the scan
    /// ("Show on scan"); its state is `aria-pressed` alone.
    AnnoShowOnScan,
    /// The signposts' word for a page with a register ("annotated"): the
    /// picker's legend, and the suffix of an annotated pager step's or
    /// picker entry's accessible name.
    AnnoMarked,
    /// An annotation link's partner title when the partner page is in the shown
    /// book ("this book"), in place of that book's own title.
    AnnoThisBook,
    /// A `group links-label=` row's caption when its chrome id is not
    /// declared (boot-flagged `UnknownGroupLinksLabel`): "Related".
    AnnoRelated,
    /// The ten colour terms `component::annotation::colour_term` names an
    /// annotation's colour with (this and the nine variants after it).
    ColourRed,
    ColourOrange,
    ColourYellow,
    ColourGreen,
    ColourBlue,
    ColourViolet,
    ColourPink,
    ColourGrey,
    ColourBlack,
    ColourWhite,

    // `component/data_links.rs` — the crumb line's dataset panel
    // fr/it are unreviewed drafts.
    /// An annotation's role word in the panel ("Region").
    DataRegion,
    /// The panel's summary label ("Show in dataset").
    DataSummary,
    /// The summary's short label on the phone crumb row ("Dataset"). It must
    /// stay a substring of [`ChromeMsg::DataSummary`], which starts the
    /// summary's accessible name (WCAG 2.5.3). fr/it are unreviewed drafts.
    DataSummaryShort,
    /// The open panel's group name ("In the dataset · DSP").
    DataCaption,
    /// The lede beside the lockup in the panel head.
    DataLede,
    /// An entry's link to its ARK ("Open in DSP").
    DataOpen,
    /// The caption over an entry's PID ("ARK URL (PID)").
    DataPidLabel,
    /// The copy button's tooltip and name prefix ("Copy PID").
    DataCopyPid,
    /// The Open link's name suffix ("opens DSP, leaves CPE").
    DataLeavesCpe,
    /// A dataset listing's group label for a case joined at page level, not
    /// by regions ("Seitenebene · h8r / c6v · 2").
    DataPageLevel,
}

impl ChromeMsg {
    fn l4(self) -> L4 {
        use ChromeMsg::*;
        match self {
            SkipToContent => L4 {
                en: "Skip to content",
                de: "Zum Inhalt springen",
                fr: "Aller au contenu",
                it: "Vai al contenuto",
            },
            ThemeToggleToLight => L4 {
                en: "Switch to light mode",
                de: "Zum hellen Modus wechseln",
                fr: "Passer en mode clair",
                it: "Passa alla modalità chiara",
            },
            ThemeToggleToDark => L4 {
                en: "Switch to dark mode",
                de: "Zum dunklen Modus wechseln",
                fr: "Passer en mode sombre",
                it: "Passa alla modalità scura",
            },
            NavPrimaryLabel => L4 {
                en: "Primary",
                de: "Hauptnavigation",
                fr: "Navigation principale",
                it: "Navigazione principale",
            },
            LanguageSwitcherLabel => L4 {
                en: "Language",
                de: "Sprache",
                fr: "Langue",
                it: "Lingua",
            },
            MenuLabel => L4 {
                en: "Menu",
                de: "Menü",
                fr: "Menu",
                it: "Menu",
            },
            AppearanceLabel => L4 {
                en: "Appearance",
                de: "Darstellung",
                fr: "Apparence",
                it: "Aspetto",
            },
            ThemeDarkModeLabel => L4 {
                en: "Dark mode",
                de: "Dunkler Modus",
                fr: "Mode sombre",
                it: "Modalità scura",
            },
            ThemeLightModeLabel => L4 {
                en: "Light mode",
                de: "Heller Modus",
                fr: "Mode clair",
                it: "Modalità chiara",
            },
            ThemeSwitchSuffix => L4 {
                en: "switch",
                de: "umschalten",
                fr: "basculer",
                it: "cambia",
            },
            FooterAttribution => L4 {
                en: "Part of the DaSCH Configurable Presentation Environment — \
                     a DaSCH research infrastructure service.",
                de: "Teil der Configurable Presentation Environment — ein \
                     Forschungsinfrastruktur-Angebot des DaSCH.",
                fr: "Fait partie du Configurable Presentation Environment — un \
                     service d'infrastructure de recherche de DaSCH.",
                it: "Fa parte del Configurable Presentation Environment — un \
                     servizio di infrastruttura di ricerca di DaSCH.",
            },
            FooterDaschLabel => L4 {
                en: "DaSCH — Swiss National Data and Service Center for the \
                     Humanities (dasch.swiss)",
                de: "DaSCH — Schweizerisches Daten- und Dienstleistungszentrum \
                     für die Geisteswissenschaften (dasch.swiss)",
                fr: "DaSCH — Swiss National Data and Service Center for the \
                     Humanities (dasch.swiss)",
                it: "DaSCH — Swiss National Data and Service Center for the \
                     Humanities (dasch.swiss)",
            },

            BreadcrumbLabel => L4 {
                en: "Breadcrumb",
                de: "Brotkrümelnavigation",
                fr: "Fil d'Ariane",
                it: "Percorso di navigazione",
            },
            CrumbBackPrefix => L4 {
                en: "Back:",
                de: "Zurück:",
                fr: "Retour :",
                it: "Indietro:",
            },

            FacetFiltersWord => L4 {
                en: "Filters",
                de: "Filter",
                fr: "Filtres",
                it: "Filtri",
            },
            FacetNoFiltersMessage => L4 {
                en: "No filters available.",
                de: "Keine Filter verfügbar.",
                fr: "Aucun filtre disponible.",
                it: "Nessun filtro disponibile.",
            },
            FacetClearAllLabel => L4 {
                en: "Clear all",
                de: "Alle entfernen",
                fr: "Tout effacer",
                it: "Cancella tutto",
            },
            FacetGroupClearLabel => L4 {
                en: "clear",
                de: "Löschen",
                fr: "Effacer",
                it: "Cancella",
            },

            CompoundEmptyState => L4 {
                en: "This compound has no members.",
                de: "Dieses Compound hat keine Elemente.",
                fr: "Cet objet composé n'a aucun élément.",
                it: "Questo oggetto composto non ha elementi.",
            },
            CompoundPrevLabel => L4 {
                en: "Previous",
                de: "Zurück",
                fr: "Précédent",
                it: "Precedente",
            },
            CompoundNextLabel => L4 {
                en: "Next",
                de: "Weiter",
                fr: "Suivant",
                it: "Successivo",
            },
            CompoundContentsLabel => L4 {
                en: "Contents",
                de: "Inhalt",
                fr: "Sommaire",
                it: "Indice",
            },
            CompoundPagerLabel => L4 {
                en: "Turn pages",
                de: "Blättern",
                fr: "Feuilleter",
                it: "Sfogliare",
            },
            CompoundMemberUnlabelled => L4 {
                en: "no label",
                de: "ohne Bezeichnung",
                fr: "sans libellé",
                it: "senza etichetta",
            },
            CompoundAsideMore => L4 {
                en: "more",
                de: "mehr",
                fr: "plus",
                it: "altro",
            },
            CompoundAsideLess => L4 {
                en: "less",
                de: "weniger",
                fr: "moins",
                it: "meno",
            },
            PickerSummary => L4 {
                en: "Choose page",
                de: "Seite wählen",
                fr: "Choisir la page",
                it: "Scegli la pagina",
            },
            PickerIndexGatherings => L4 {
                en: "Gatherings",
                de: "Lagen",
                fr: "Cahiers",
                it: "Fascicoli",
            },
            PickerIndexPages => L4 {
                en: "Pages",
                de: "Seiten",
                fr: "Pages",
                it: "Pagine",
            },
            PickerFrontMark => L4 {
                en: "Front",
                de: "Vorsatz",
                fr: "Début",
                it: "Inizio",
            },
            PickerFrontName => L4 {
                en: "Front flyleaves",
                de: "Vorsatz",
                fr: "Gardes antérieures",
                it: "Carte di guardia anteriori",
            },
            PickerBackMark => L4 {
                en: "Back",
                de: "Nachsatz",
                fr: "Fin",
                it: "Fine",
            },
            PickerBackName => L4 {
                en: "Back flyleaves",
                de: "Nachsatz",
                fr: "Gardes postérieures",
                it: "Carte di guardia posteriori",
            },
            PickerOther => L4 {
                en: "Other",
                de: "Weitere",
                fr: "Autres",
                it: "Altre",
            },
            CompoundStaleAddress => L4 {
                en: "The address names no page of this book:",
                de: "Die Adresse nennt keine Seite dieses Buchs:",
                fr: "L’adresse ne désigne aucune page de ce livre :",
                it: "L’indirizzo non indica alcuna pagina di questo libro:",
            },

            BoolYes => L4 {
                en: "Yes",
                de: "Ja",
                fr: "Oui",
                it: "Sì",
            },
            BoolNo => L4 {
                en: "No",
                de: "Nein",
                fr: "Non",
                it: "No",
            },
            DatePrecDay => L4 {
                en: "day",
                de: "Tag",
                fr: "jour",
                it: "giorno",
            },
            DatePrecMonth => L4 {
                en: "month",
                de: "Monat",
                fr: "mois",
                it: "mese",
            },
            DatePrecYear => L4 {
                en: "year",
                de: "Jahr",
                fr: "année",
                it: "anno",
            },
            DatePrecRange => L4 {
                en: "range",
                de: "Zeitraum",
                fr: "période",
                it: "periodo",
            },
            CalendarGregorian => L4 {
                en: "Gregorian",
                de: "gregorianisch",
                fr: "grégorien",
                it: "gregoriano",
            },
            CarouselPrevImageLabel => L4 {
                en: "Previous image",
                de: "Vorheriges Bild",
                fr: "Image précédente",
                it: "Immagine precedente",
            },
            CarouselNextImageLabel => L4 {
                en: "Next image",
                de: "Nächstes Bild",
                fr: "Image suivante",
                it: "Immagine successiva",
            },
            CarouselNavLabel => L4 {
                en: "Images of this resource",
                de: "Bilder dieser Ressource",
                fr: "Images de cette ressource",
                it: "Immagini di questa risorsa",
            },
            PlatePagerNavLabel => L4 {
                en: "Images of this resource",
                de: "Bilder dieser Ressource",
                fr: "Images de cette ressource",
                it: "Immagini di questa risorsa",
            },
            PlatePagerPrevLabel => L4 {
                en: "Previous image",
                de: "Vorheriges Bild",
                fr: "Image précédente",
                it: "Immagine precedente",
            },
            PlatePagerNextLabel => L4 {
                en: "Next image",
                de: "Nächstes Bild",
                fr: "Image suivante",
                it: "Immagine successiva",
            },
            PlateStripCollapseLabel => L4 {
                en: "Hide thumbnails",
                de: "Miniaturansichten ausblenden",
                fr: "Masquer les vignettes",
                it: "Nascondi le miniature",
            },
            PlateStripExpandLabel => L4 {
                en: "Show thumbnails",
                de: "Miniaturansichten anzeigen",
                fr: "Afficher les vignettes",
                it: "Mostra le miniature",
            },
            PlateIllustrationsLabel => L4 {
                en: "Illustrations",
                de: "Illustrationen",
                fr: "Illustrations",
                it: "Illustrazioni",
            },
            UnresolvedListPlaceholder => L4 {
                en: "Unresolved list value",
                de: "Nicht aufgelöster Listenwert",
                fr: "Valeur de liste non résolue",
                it: "Valore di lista non risolto",
            },
            ImagePlaceholderAriaLabel => L4 {
                en: "Image unavailable",
                de: "Bild nicht verfügbar",
                fr: "Image non disponible",
                it: "Immagine non disponibile",
            },
            PagerNavLabel => L4 {
                en: "Page navigation",
                de: "Seitennavigation",
                fr: "Navigation par page",
                it: "Navigazione pagina",
            },
            SiteIndexAriaLabel => L4 {
                en: "CPE — site index",
                de: "CPE — Seitenindex",
                fr: "CPE — index du site",
                it: "CPE — indice del sito",
            },
            DaschAriaLabel => L4 {
                en: "DaSCH",
                de: "DaSCH",
                fr: "DaSCH",
                it: "DaSCH",
            },
            SiteIndexNoProjects => L4 {
                en: "No projects are being presented yet.",
                de: "Noch werden keine Projekte präsentiert.",
                fr: "Aucun projet n'est encore présenté.",
                it: "Nessun progetto è ancora presentato.",
            },
            SiteIndexProjectsHeading => L4 {
                en: "Projects",
                de: "Projekte",
                fr: "Projets",
                it: "Progetti",
            },
            SiteIndexEnterCta => L4 {
                en: "Enter",
                de: "Öffnen",
                fr: "Ouvrir",
                it: "Apri",
            },
            SiteIndexTagline => L4 {
                en: "Configurable Presentation Environment",
                de: "Konfigurierbare Präsentationsumgebung",
                fr: "Environnement de présentation configurable",
                it: "Ambiente di presentazione configurabile",
            },
            SiteIndexPocDesc => L4 {
                en: "A PoC that presents archived DaSCH research data through \
                     project-specific lenses — narrative pages, curated glimpses, \
                     and browsable record collections.",
                de: "Ein PoC, der archivierte Forschungsdaten des DaSCH durch \
                     projektspezifische Perspektiven präsentiert — narrative \
                     Seiten, kuratierte Einblicke und durchstöberbare Sammlungen \
                     von Einträgen.",
                fr: "Un PoC qui présente des données de recherche archivées du \
                     DaSCH à travers des perspectives propres à chaque projet — \
                     pages narratives, aperçus choisis et collections \
                     d'enregistrements consultables.",
                it: "Un PoC che presenta dati di ricerca archiviati del DaSCH \
                     attraverso prospettive specifiche per progetto — pagine \
                     narrative, scorci curati e collezioni di record consultabili.",
            },
            SiteIndexDaschLinkLabel => L4 {
                en: "DaSCH website",
                de: "DaSCH-Website",
                fr: "Site web de DaSCH",
                it: "Sito web di DaSCH",
            },

            SearchLabel => L4 {
                en: "Search",
                de: "Suche",
                fr: "Recherche",
                it: "Cerca",
            },
            SearchSubmit => L4 {
                en: "Search",
                de: "Suchen",
                fr: "Rechercher",
                it: "Cerca",
            },
            SearchClear => L4 {
                en: "Clear search",
                de: "Suche löschen",
                fr: "Effacer la recherche",
                it: "Cancella ricerca",
            },
            SearchHint => L4 {
                en: "Matching is on the beginning of words — try a shorter term.",
                de: "Gesucht wird am Wortanfang — versuchen Sie einen kürzeren Begriff.",
                fr: "La recherche porte sur le début des mots — essayez un terme plus court.",
                it: "La ricerca corrisponde all'inizio delle parole — prova un termine più breve.",
            },

            RfAccessRestricted => L4 {
                en: "Restricted view — reduced resolution",
                de: "Eingeschränkte Ansicht — reduzierte Auflösung",
                fr: "Vue restreinte — résolution réduite",
                it: "Vista limitata — risoluzione ridotta",
            },

            RvZoomInLabel => L4 {
                en: "Zoom in",
                de: "Vergrössern",
                fr: "Zoom avant",
                it: "Aumenta zoom",
            },

            RvZoomOutLabel => L4 {
                en: "Zoom out",
                de: "Verkleinern",
                fr: "Zoom arrière",
                it: "Riduci zoom",
            },
            RvZoomHomeLabel => L4 {
                en: "Fit to view",
                de: "An Ansicht anpassen",
                fr: "Ajuster à la vue",
                it: "Adatta alla vista",
            },
            RvViewerErrorLabel => L4 {
                en: "The zoomable image couldn't load.",
                de: "Das zoombare Bild konnte nicht geladen werden.",
                fr: "L'image zoomable n'a pas pu être chargée.",
                it: "Impossibile caricare l'immagine zoomabile.",
            },

            RvDetailsToggleLabel => L4 {
                en: "Details",
                de: "Details",
                fr: "Détails",
                it: "Dettagli",
            },
            RvAdjustToggleLabel => L4 {
                en: "Adjust",
                de: "Anpassen",
                fr: "Ajuster",
                it: "Regola",
            },
            RvWiderToggleLabel => L4 {
                en: "Wider view",
                de: "Breitere Ansicht",
                fr: "Vue élargie",
                it: "Vista più ampia",
            },
            RvDrawerCloseLabel => L4 {
                en: "Close",
                de: "Schliessen",
                fr: "Fermer",
                it: "Chiudi",
            },
            RvFieldDescription => L4 {
                en: "Description",
                de: "Beschreibung",
                fr: "Description",
                it: "Descrizione",
            },
            RvFieldAccess => L4 {
                en: "Access",
                de: "Zugang",
                fr: "Accès",
                it: "Accesso",
            },
            RvFieldIdentifier => L4 {
                en: "Identifier",
                de: "Kennung",
                fr: "Identifiant",
                it: "Identificativo",
            },
            RvFieldDimensions => L4 {
                en: "Dimensions",
                de: "Abmessungen",
                fr: "Dimensions",
                it: "Dimensioni",
            },
            RvFieldIiifSource => L4 {
                en: "IIIF source",
                de: "IIIF-Quelle",
                fr: "Source IIIF",
                it: "Fonte IIIF",
            },
            RvAccessRestrictedLong => L4 {
                en: "This image is shown at reduced resolution to protect a living subject or restricted collection.",
                de: "Dieses Bild wird in reduzierter Auflösung gezeigt, um eine lebende Person oder eine eingeschränkte Sammlung zu schützen.",
                fr: "Cette image est présentée en résolution réduite afin de protéger une personne vivante ou une collection soumise à restriction.",
                it: "Questa immagine viene mostrata a risoluzione ridotta per proteggere una persona vivente o una collezione soggetta a restrizioni.",
            },
            RvAdjustBrightness => L4 {
                en: "Brightness",
                de: "Helligkeit",
                fr: "Luminosité",
                it: "Luminosità",
            },
            RvAdjustContrast => L4 {
                en: "Contrast",
                de: "Kontrast",
                fr: "Contraste",
                it: "Contrasto",
            },
            RvAdjustSaturation => L4 {
                en: "Saturation",
                de: "Sättigung",
                fr: "Saturation",
                it: "Saturazione",
            },
            RvAdjustExposure => L4 {
                en: "Exposure",
                de: "Belichtung",
                fr: "Exposition",
                it: "Esposizione",
            },
            RvAdjustGamma => L4 {
                en: "Gamma",
                de: "Gamma",
                fr: "Gamma",
                it: "Gamma",
            },
            RvAdjustHue => L4 {
                en: "Hue",
                de: "Farbton",
                fr: "Teinte",
                it: "Tonalità",
            },
            RvAdjustInvert => L4 {
                en: "Invert",
                de: "Invertieren",
                fr: "Inverser",
                it: "Inverti",
            },
            RvAdjustResetLabel => L4 {
                en: "Reset all",
                de: "Alles zurücksetzen",
                fr: "Tout réinitialiser",
                it: "Ripristina tutto",
            },
            RvAdjustNote => L4 {
                en: "Adjustments are a way of looking, not a setting: they carry across plates and are forgotten when you leave.",
                de: "Anpassungen sind eine Art zu betrachten, keine Einstellung: Sie bleiben beim Blättern erhalten und werden beim Verlassen vergessen.",
                fr: "Les réglages sont une manière de regarder, non un paramètre : ils se conservent d'une planche à l'autre et sont oubliés en quittant la page.",
                it: "Le regolazioni sono un modo di guardare, non un'impostazione: rimangono attive tra le tavole e vengono dimenticate quando si esce.",
            },
            RvHostRightsTitle => L4 {
                en: "Rights & attribution",
                de: "Rechte & Namensnennung",
                fr: "Droits et attribution",
                it: "Diritti e attribuzione",
            },
            RvFieldRightsHolder => L4 {
                en: "Rights holder",
                de: "Rechteinhaber",
                fr: "Titulaire des droits",
                it: "Titolare dei diritti",
            },
            RvFieldEdition => L4 {
                en: "Edition",
                de: "Ausgabe",
                fr: "Édition",
                it: "Edizione",
            },
            RvFieldLicense => L4 {
                en: "Licence",
                de: "Lizenz",
                fr: "Licence",
                it: "Licenza",
            },
            PropertyGroupTitle => L4 {
                en: "Details",
                de: "Angaben",
                fr: "Informations",
                it: "Dettagli",
            },
            IncomingLinksEmpty => L4 {
                en: "Nothing links here.",
                de: "Nichts verweist hierher.",
                fr: "Rien ne renvoie ici.",
                it: "Nulla rimanda qui.",
            },

            TimelineKey => L4 {
                en: "bar = one book · wide bar = a date range · title, date and extent on hover · grid lines every three years",
                de: "Balken = ein Buch · breiter Balken = Zeitraum · Titel, Datum und Umfang beim Zeigen · Rasterlinien alle drei Jahre",
                fr: "barre = un livre · barre large = une période · titre, date et étendue au survol · lignes de grille tous les trois ans",
                it: "barra = un libro · barra larga = un periodo · titolo, data e consistenza al passaggio del mouse · linee della griglia ogni tre anni",
            },

            CaseOpenPage => L4 {
                en: "— open the page",
                de: "— Seite öffnen",
                fr: "— ouvrir la page",
                it: "— apri la pagina",
            },
            CaseCropNotLoaded => L4 {
                en: "Crop not loaded",
                de: "Ausschnitt nicht geladen",
                fr: "Détail non chargé",
                it: "Dettaglio non caricato",
            },
            CaseSamePages => L4 {
                en: "Two pages, linked at page level:",
                de: "Zwei Seiten, auf Seitenebene verknüpft:",
                fr: "Deux pages, liées au niveau de la page :",
                it: "Due pagine, collegate a livello di pagina:",
            },
            CaseWholePage => L4 {
                en: "Whole page",
                de: "Ganze Seite",
                fr: "Page entière",
                it: "Pagina intera",
            },
            CasePageNote => L4 {
                en: "Page-level link — no marked region",
                de: "Verknüpfung auf Seitenebene — ohne markierte Region",
                fr: "Lien au niveau de la page — sans région marquée",
                it: "Collegamento a livello di pagina — senza regione marcata",
            },
            CaseDetail => L4 {
                en: "Detail",
                de: "Detail",
                fr: "Détail",
                it: "Dettaglio",
            },
            EditionRead => L4 {
                en: "Read",
                de: "Lesen",
                fr: "Lire",
                it: "Leggi",
            },

            NarrativeRailLabel => L4 {
                en: "On this page",
                de: "Auf dieser Seite",
                fr: "Sur cette page",
                it: "In questa pagina",
            },

            ViewLabel => L4 {
                en: "View",
                de: "Ansicht",
                fr: "Affichage",
                it: "Vista",
            },
            ViewOptionList => L4 {
                en: "List",
                de: "Liste",
                fr: "Liste",
                it: "Elenco",
            },
            ViewOptionGrid => L4 {
                en: "Grid",
                de: "Raster",
                fr: "Grille",
                it: "Griglia",
            },
            GridNoImage => L4 {
                en: "No illustration",
                de: "Keine Abbildung",
                fr: "Sans illustration",
                it: "Senza illustrazione",
            },

            ListEmptyClass => L4 {
                en: "This class contains no resources.",
                de: "Diese Klasse enthält keine Ressourcen.",
                fr: "Cette classe ne contient aucune ressource.",
                it: "Questa classe non contiene risorse.",
            },
            FacetEmptyGroupFallback => L4 {
                en: "No values are recorded for this class.",
                de: "Für diese Klasse sind keine Werte erfasst.",
                fr: "Aucune valeur n'est enregistrée pour cette classe.",
                it: "Nessun valore è registrato per questa classe.",
            },
            ImageAbsentLabel => L4 {
                en: "No image",
                de: "Kein Bild",
                fr: "Aucune image",
                it: "Nessuna immagine",
            },
            ImageAbsentRecord => L4 {
                en: "This resource has no representation in the archive.",
                de: "Für diese Ressource ist im Archiv keine Repräsentation vorhanden.",
                fr: "Cette ressource n'a aucune représentation dans les archives.",
                it: "Questa risorsa non ha alcuna rappresentazione nell'archivio.",
            },
            ImageAbsentPlate => L4 {
                en: "This plate could not be resolved.",
                de: "Diese Tafel konnte nicht aufgelöst werden.",
                fr: "Cette planche n'a pas pu être résolue.",
                it: "Questa tavola non è stata risolta.",
            },
            ImageCaptionAbsent => L4 {
                en: "IIIF · not available",
                de: "IIIF · nicht verfügbar",
                fr: "IIIF · non disponible",
                it: "IIIF · non disponibile",
            },
            EmptyClearFilters => L4 {
                en: "Clear all filters",
                de: "Alle Filter entfernen",
                fr: "Effacer tous les filtres",
                it: "Cancella tutti i filtri",
            },
            ValueUnresolved => L4 {
                en: "unresolved",
                de: "nicht aufgelöst",
                fr: "non résolu",
                it: "non risolto",
            },
            ValueUnresolvedTitle => L4 {
                en: "This reference names a resource that is not in the archive.",
                de: "Dieser Verweis benennt eine Ressource, die nicht im Archiv vorhanden ist.",
                fr: "Cette référence désigne une ressource absente des archives.",
                it: "Questo riferimento indica una risorsa non presente nell'archivio.",
            },
            NfCode => L4 {
                en: "404 — not found",
                de: "404 — nicht gefunden",
                fr: "404 — introuvable",
                it: "404 — non trovato",
            },
            NfPathTitle => L4 {
                en: "This page does not exist.",
                de: "Diese Seite existiert nicht.",
                fr: "Cette page n'existe pas.",
                it: "Questa pagina non esiste.",
            },
            NfPathMsg => L4 {
                en: "The address you followed does not name anything in this project. \
                     It may have been re-modelled since the link was made, or the address \
                     may be incomplete.",
                de: "Die Adresse, der Sie gefolgt sind, bezeichnet nichts in diesem Projekt. \
                     Sie wurde möglicherweise seit der Erstellung des Links umstrukturiert, \
                     oder die Adresse ist unvollständig.",
                fr: "L'adresse que vous avez suivie ne désigne rien dans ce projet. Elle a \
                     peut-être été remodelée depuis la création du lien, ou l'adresse est \
                     incomplète.",
                it: "L'indirizzo che hai seguito non indica nulla in questo progetto. \
                     Potrebbe essere stato rimodellato dopo la creazione del link, oppure \
                     l'indirizzo è incompleto.",
            },
            NfOrBrowse => L4 {
                en: "Or browse:",
                de: "Oder durchsuchen:",
                fr: "Ou parcourez :",
                it: "Oppure sfoglia:",
            },
            NfSiteTitle => L4 {
                en: "This address does not name a project.",
                de: "Diese Adresse bezeichnet kein Projekt.",
                fr: "Cette adresse ne désigne aucun projet.",
                it: "Questo indirizzo non indica alcun progetto.",
            },
            NfSiteMsg => L4 {
                en: "CPE presents each archived project under its own address. Nothing is \
                     presented at this one — it may name a project that has not been \
                     published, or the address may be incomplete.",
                de: "CPE präsentiert jedes archivierte Projekt unter einer eigenen Adresse. \
                     Unter dieser Adresse wird nichts präsentiert — sie bezeichnet \
                     möglicherweise ein noch nicht veröffentlichtes Projekt, oder die \
                     Adresse ist unvollständig.",
                fr: "CPE présente chaque projet archivé sous sa propre adresse. Rien n'est \
                     présenté à celle-ci — elle désigne peut-être un projet qui n'a pas été \
                     publié, ou l'adresse est incomplète.",
                it: "CPE presenta ogni progetto archiviato con un proprio indirizzo. A \
                     questo indirizzo non è presentato nulla — potrebbe indicare un \
                     progetto non ancora pubblicato, oppure l'indirizzo è incompleto.",
            },
            NfAllProjects => L4 {
                en: "All projects",
                de: "Alle Projekte",
                fr: "Tous les projets",
                it: "Tutti i progetti",
            },
            NfIdTitleFallback => L4 {
                en: "There is nothing at this address.",
                de: "Unter dieser Adresse gibt es nichts.",
                fr: "Il n'y a rien à cette adresse.",
                it: "Non c'è nulla a questo indirizzo.",
            },
            NfProjectHome => L4 {
                en: "Project home",
                de: "Projektstartseite",
                fr: "Accueil du projet",
                it: "Home del progetto",
            },
            NfCrumb => L4 {
                en: "Not found",
                de: "Nicht gefunden",
                fr: "Introuvable",
                it: "Non trovato",
            },

            CopyLinkLabel => L4 {
                en: "Copy link",
                de: "Link kopieren",
                fr: "Copier le lien",
                it: "Copia il link",
            },
            CopyLinkCopied => L4 {
                en: "Copied",
                de: "Kopiert",
                fr: "Copié",
                it: "Copiato",
            },
            CopyLinkLive => L4 {
                en: "Link to this view copied to the clipboard",
                de: "Link zu dieser Ansicht in die Zwischenablage kopiert",
                fr: "Lien vers cette vue copié dans le presse-papiers",
                it: "Link a questa vista copiato negli appunti",
            },
            CopyLinkLongPrefix => L4 {
                en: "Copy link to this view",
                de: "Link zu dieser Ansicht kopieren",
                fr: "Copier le lien vers cette vue",
                it: "Copia il link a questa vista",
            },
            CopyLinkShort => L4 {
                en: "Copy",
                de: "Link",
                fr: "Lien",
                it: "Link",
            },

            OrderLabel => L4 {
                en: "Order",
                de: "Reihenfolge",
                fr: "Ordre",
                it: "Ordine",
            },
            OrderCurated => L4 {
                en: "As curated",
                de: "Wie kuratiert",
                fr: "Ordre éditorial",
                it: "Ordine curato",
            },
            OrderNameAsc => L4 {
                en: "Name A–Z",
                de: "Name A–Z",
                fr: "Nom A–Z",
                it: "Nome A–Z",
            },
            OrderNameDesc => L4 {
                en: "Name Z–A",
                de: "Name Z–A",
                fr: "Nom Z–A",
                it: "Nome Z–A",
            },
            OrderId => L4 {
                en: "Identifier",
                de: "Kennung",
                fr: "Identifiant",
                it: "Identificatore",
            },
            PagerPrev => L4 {
                en: "Previous",
                de: "Zurück",
                fr: "Précédent",
                it: "Precedente",
            },
            PagerNext => L4 {
                en: "Next",
                de: "Weiter",
                fr: "Suivant",
                it: "Successivo",
            },

            AnnoShapeRectangle => L4 {
                en: "Rectangle",
                de: "Rechteck",
                fr: "Rectangle",
                it: "Rettangolo",
            },
            AnnoShapePolygon => L4 {
                en: "Polygon",
                de: "Polygon",
                fr: "Polygone",
                it: "Poligono",
            },
            AnnoShapeCircle => L4 {
                en: "Circle",
                de: "Kreis",
                fr: "Cercle",
                it: "Cerchio",
            },
            AnnoWholePage => L4 {
                en: "Whole page",
                de: "Ganze Seite",
                fr: "Page entière",
                it: "Pagina intera",
            },
            AnnoShowOnScan => L4 {
                en: "Show on scan",
                de: "Auf dem Scan zeigen",
                fr: "Afficher sur le scan",
                it: "Mostra sulla scansione",
            },
            AnnoMarked => L4 {
                en: "annotated",
                de: "mit Annotation",
                fr: "avec annotation",
                it: "con annotazione",
            },
            AnnoThisBook => L4 {
                en: "this book",
                de: "diese Ausgabe",
                fr: "cette édition",
                it: "questa edizione",
            },
            AnnoRelated => L4 {
                en: "Related",
                de: "Verwandt",
                fr: "Lié",
                it: "Correlato",
            },
            ColourRed => L4 {
                en: "Red",
                de: "Rot",
                fr: "Rouge",
                it: "Rosso",
            },
            ColourOrange => L4 {
                en: "Orange",
                de: "Orange",
                fr: "Orange",
                it: "Arancione",
            },
            ColourYellow => L4 {
                en: "Yellow",
                de: "Gelb",
                fr: "Jaune",
                it: "Giallo",
            },
            ColourGreen => L4 {
                en: "Green",
                de: "Grün",
                fr: "Vert",
                it: "Verde",
            },
            ColourBlue => L4 {
                en: "Blue",
                de: "Blau",
                fr: "Bleu",
                it: "Blu",
            },
            ColourViolet => L4 {
                en: "Violet",
                de: "Violett",
                fr: "Violet",
                it: "Viola",
            },
            ColourPink => L4 {
                en: "Pink",
                de: "Rosa",
                fr: "Rose",
                it: "Rosa",
            },
            ColourGrey => L4 {
                en: "Grey",
                de: "Grau",
                fr: "Gris",
                it: "Grigio",
            },
            ColourBlack => L4 {
                en: "Black",
                de: "Schwarz",
                fr: "Noir",
                it: "Nero",
            },
            ColourWhite => L4 {
                en: "White",
                de: "Weiss",
                fr: "Blanc",
                it: "Bianco",
            },

            DataRegion => L4 {
                en: "Region",
                de: "Region",
                fr: "Région",
                it: "Regione",
            },
            DataSummary => L4 {
                en: "Show in dataset",
                de: "Im Datensatz zeigen",
                fr: "Afficher dans le jeu de données",
                it: "Mostra nel dataset",
            },
            DataSummaryShort => L4 {
                en: "Dataset",
                de: "Datensatz",
                fr: "Données",
                it: "Dataset",
            },
            DataCaption => L4 {
                en: "In the dataset · DSP",
                de: "Im Datensatz · DSP",
                fr: "Dans le jeu de données · DSP",
                it: "Nel dataset · DSP",
            },
            DataLede => L4 {
                en: "Source data in the DaSCH Service Platform (DSP)",
                de: "Quelldaten in der DaSCH Service Platform (DSP)",
                fr: "Données sources dans la DaSCH Service Platform (DSP)",
                it: "Dati di origine nella DaSCH Service Platform (DSP)",
            },
            DataOpen => L4 {
                en: "Open in DSP",
                de: "In DSP öffnen",
                fr: "Ouvrir dans DSP",
                it: "Apri in DSP",
            },
            DataPidLabel => L4 {
                en: "ARK URL (PID)",
                de: "ARK-URL (PID)",
                fr: "URL ARK (PID)",
                it: "URL ARK (PID)",
            },
            DataCopyPid => L4 {
                en: "Copy PID",
                de: "PID kopieren",
                fr: "Copier le PID",
                it: "Copia PID",
            },
            DataLeavesCpe => L4 {
                en: "opens DSP, leaves CPE",
                de: "öffnet DSP, verlässt CPE",
                fr: "ouvre DSP, quitte CPE",
                it: "apre DSP, lascia CPE",
            },
            DataPageLevel => L4 {
                en: "Page level",
                de: "Seitenebene",
                fr: "Niveau de la page",
                it: "Livello di pagina",
            },
        }
    }

    /// Resolve this message's string for `lang`.
    pub fn text(self, lang: &Lang) -> &'static str {
        self.l4().pick(lang)
    }
}

// Tuple wrapper helpers
//
// A handful of call sites want a `(prev, next)` / `(yes, no)` pair rather than
// two separate lookups. Each element is still an independent fixed
// [`ChromeMsg`] variant — these are thin convenience wrappers, not a second
// message shape.

/// `(yes, no)` boolean-value labels.
pub(crate) fn bool_labels(lang: &Lang) -> (&'static str, &'static str) {
    (ChromeMsg::BoolYes.text(lang), ChromeMsg::BoolNo.text(lang))
}

// Dates
//
// The long form a date value composes from its ISO bounds when its curated
// display string is written in ANOTHER language than the request's. New; fr/it are
// drafts.

/// The localized name of month `n` (1 = January … 12 = December), in the
/// running-text case each language uses inside a date (`Februar`, `February`,
/// `février`, `febbraio`). `None` for `n` outside `1..=12` — a malformed ISO
/// month the caller falls back from, never a panic.
pub(crate) fn month_name(n: u32, lang: &Lang) -> Option<&'static str> {
    const EN: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const DE: [&str; 12] = [
        "Januar",
        "Februar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktober",
        "November",
        "Dezember",
    ];
    const FR: [&str; 12] = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    const IT: [&str; 12] = [
        "gennaio",
        "febbraio",
        "marzo",
        "aprile",
        "maggio",
        "giugno",
        "luglio",
        "agosto",
        "settembre",
        "ottobre",
        "novembre",
        "dicembre",
    ];
    let names = match lang.as_str() {
        "de" => &DE,
        "fr" => &FR,
        "it" => &IT,
        _ => &EN,
    };
    let idx = usize::try_from(n.checked_sub(1)?).ok()?;
    names.get(idx).copied()
}

/// Compose one date bound in the long form: `19 February 1491` (en),
/// `19. Februar 1491` (de), `19 février 1491` / `1er février 1491` (fr),
/// `19 febbraio 1491` (it); with `day == None`, `February 1491` etc. The
/// caller has already resolved `month_name`.
pub(crate) fn date_long_text(year: &str, month: &str, day: Option<u32>, lang: &Lang) -> String {
    match (day, lang.as_str()) {
        (None, _) => format!("{month} {year}"),
        (Some(d), "de") => format!("{d}. {month} {year}"),
        (Some(1), "fr") => format!("1er {month} {year}"),
        (Some(d), _) => format!("{d} {month} {year}"),
    }
}

/// `(prev, next)` media-plate `.pg` pager control accessible names.
pub(crate) fn plate_pager_labels(lang: &Lang) -> (&'static str, &'static str) {
    (
        ChromeMsg::PlatePagerPrevLabel.text(lang),
        ChromeMsg::PlatePagerNextLabel.text(lang),
    )
}

/// The record-viewer Details "plate i of n" identifier suffix,
/// appended after an image's own id when its host
/// carries more than one image. `i` and `n` are 1-based / total. New — fr/it
/// are unreviewed drafts pending native-speaker review.
pub(crate) fn rv_plate_of(i: usize, n: usize, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("Tafel {i} von {n}"),
        "fr" => format!("planche {i} sur {n}"),
        "it" => format!("tavola {i} di {n}"),
        _ => format!("plate {i} of {n}"),
    }
}

/// The browse pager's "Page {n} of {N}" position text,
/// used both in the nav `aria-label` (always) and, at `last >=
/// 9`, as the visible `.pager__of` replacement for numbered links. fr/it are
/// unreviewed drafts pending native-speaker review.
pub(crate) fn pager_page_of(page: usize, last: usize, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("Seite {page} von {last}"),
        "fr" => format!("Page {page} sur {last}"),
        "it" => format!("Pagina {page} di {last}"),
        _ => format!("Page {page} of {last}"),
    }
}

/// The browse pager's per-number `aria-label` ("Page {n}") for each
/// `.pager__num` link. fr/it are
/// unreviewed drafts pending native-speaker review.
pub(crate) fn pager_page_n(page: usize, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("Seite {page}"),
        "fr" => format!("Page {page}"),
        "it" => format!("Pagina {page}"),
        _ => format!("Page {page}"),
    }
}

/// `(prev, next)` compound-viewer step accessible names.
pub(crate) fn compound_step_labels(lang: &Lang) -> (&'static str, &'static str) {
    (ChromeMsg::CompoundPrevLabel.text(lang), ChromeMsg::CompoundNextLabel.text(lang))
}

// Pluralization

/// A CLDR-style plural category. Only two categories are needed for the
/// built-in languages' count-inflected engine strings — no external
/// CLDR/ICU crate is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluralCategory {
    One,
    Other,
}

/// The shared count → category rule for all four built-in languages.
///
/// `n == 1` is *One* for en/de/it. **French uses *One* for `n == 0 || n ==
/// 1`** (French convention — "0 partie", not "0 parties"). Everything else is
/// *Other*. Any language outside [`BUILTIN_LANGUAGES`] falls back to the
/// en/de/it rule (`n == 1` → *One*), mirroring [`L4::pick`]'s English
/// fallback.
pub(crate) fn plural_category(n: usize, lang: &Lang) -> PluralCategory {
    if lang.as_str() == "fr" {
        if n == 0 || n == 1 {
            PluralCategory::One
        } else {
            PluralCategory::Other
        }
    } else if n == 1 {
        PluralCategory::One
    } else {
        PluralCategory::Other
    }
}

/// Group `n`'s decimal digits into thousands with the locale's own group
/// separator: `de`/`it` use the **Swiss apostrophe** U+2019 (`8\u{2019}942`),
/// `fr` uses CLDR's **narrow no-break space** U+202F (`8\u{202F}942`), and
/// `en` — plus every language outside [`BUILTIN_LANGUAGES`] — uses `,`
/// (`8,942`). Written as escapes on purpose: a hand-typed `'` would land on
/// ASCII U+0027 and a hand-typed space on U+0020, both of which render as the
/// wrong glyph while still reading "right" in source.
///
/// The `de`/`it` choice is the **Swiss** convention, a deliberate
/// choice over `8.942`.
///
/// No CLDR/ICU dependency, mirroring [`plural_category`]'s same deliberate
/// choice.
///
/// **Scope boundary — the browse count line ONLY.** This helper is applied by
/// [`count_resources_text`] / [`count_of_text`] and nowhere else. It is
/// deliberately NOT applied to [`pager_page_of`] / [`pager_page_n`] ("Seite 1
/// von 373"), [`rv_plate_of`], [`index_count_text`], or the facet-panel's
/// `span.facet-option__count`: grouping belongs to the count line
/// and nowhere else, and no shipped project reaches four digits in those
/// places. Do not "finish the job" by spreading it.
pub(crate) fn group_thousands(n: usize, lang: &Lang) -> String {
    let sep = match lang.as_str() {
        "de" | "it" => "\u{2019}",
        "fr" => "\u{202F}",
        _ => ",",
    };
    let digits = n.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + len / 3);
    // `digits` is ASCII by construction (`usize::to_string`), so byte indices
    // are char indices.
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push_str(sep);
        }
        out.push(ch);
    }
    out
}

/// Lowercase `noun_one`'s first character to agree with `noun_other`'s
/// casing, derived from the data rather than hardcoded per language.
///
/// `noun_one` (the class's capitalised display label, e.g. "Work") and
/// `noun_other` (the class's declared plural, lowercase in the EN/FR/IT
/// convention — see `config.kdl`'s `labels { class { plural } }` comment)
/// are declared independently and can disagree in casing even though they
/// name the same noun, producing a count line that flips case between its
/// singular and plural arms on the SAME element ("1 Work" / "2 works").
/// German is unaffected in practice (German nouns are always capitalised,
/// and the declared DE plurals are capitalised too), so this only ever
/// fires for the other three languages' lowercase-plural convention.
///
/// Guarded against acronyms: when `noun_one`'s SECOND character is also
/// uppercase (e.g. "IIIF Manifest"), the label is assumed to carry an
/// acronym and is left untouched, even if the plural is lowercase.
fn agree_singular_casing<'a>(noun_one: &'a str, noun_other: &str) -> Cow<'a, str> {
    let mut chars = noun_one.chars();
    let Some(first) = chars.next() else {
        return Cow::Borrowed(noun_one);
    };
    if !first.is_uppercase() {
        return Cow::Borrowed(noun_one);
    }
    // Peek the second character WITHOUT consuming it from `chars` — `chars`
    // (specifically its `.as_str()` remainder) is reused below to build the
    // lowered string, so consuming here would silently drop that character
    // from the output.
    if chars.clone().next().is_some_and(char::is_uppercase) {
        return Cow::Borrowed(noun_one);
    }
    let other_starts_lowercase = noun_other.chars().next().is_some_and(|c| c.is_lowercase());
    if !other_starts_lowercase {
        return Cow::Borrowed(noun_one);
    }
    let mut lowered = String::with_capacity(noun_one.len());
    lowered.extend(first.to_lowercase());
    lowered.push_str(chars.as_str());
    Cow::Owned(lowered)
}

/// Localized "N {noun}" count text for the resource-list surf-head
/// (unnarrowed case), e.g. `"8\u{2019}942 Aufführungen"`.
///
/// Both nouns come from the CALLER — the class's own declared labels
/// (`labels::resolve_class_label` for `noun_one`,
/// `labels::resolve_class_plural` for `noun_other`), never a generic
/// engine noun. The former hardcoded per-language `Einträge` /
/// `enregistrements` / `record` / `resources` are gone, and deliberately NOT
/// kept as a fallback: `resolve_class_plural`'s chain already ends in
/// `util::humanize_property_name`, so the noun is never empty, and a silent
/// generic fallback would hide a missing `labels { class { plural } }` block
/// instead of surfacing it (`validate::check_class_plurals` warns
/// `MissingClassPlural` for exactly that).
///
/// `noun_one` is case-agreed to `noun_other` via [`agree_singular_casing`]
/// before use — this is the ONE place the two nouns meet, so it is the
/// right (and only) place to fix a casing mismatch between them (e.g. "Work"
/// singular vs. declared-lowercase "works" plural).
///
/// Count-inflected via [`plural_category`] — keep that call: it is what makes
/// French singularize `n == 0` ("0 enregistrement"). The count itself is
/// grouped by [`group_thousands`].
#[doc(hidden)]
pub fn count_resources_text(total: usize, noun_one: &str, noun_other: &str, lang: &Lang) -> String {
    let n = group_thousands(total, lang);
    match plural_category(total, lang) {
        PluralCategory::One => {
            let noun = agree_singular_casing(noun_one, noun_other);
            format!("{n} {noun}")
        }
        PluralCategory::Other => format!("{n} {noun_other}"),
    }
}

/// German dative-plural inflection of a project-declared nominative plural
/// noun, for the `"von"` connective in [`count_of_text`]'s `"de"` arm only.
///
/// `von` governs the dative case. The regular textbook rule for a German
/// plural noun's dative form is: append `-n`, unless the noun already ends
/// in `-n` or `-s` (case-insensitive on the final character) — those forms
/// are unchanged in the dative. This is deliberately a RULE, not a
/// per-class declared form: no config surface was added for it, so
/// `resolve_class_plural`'s declared nominative plural is inflected
/// mechanically at render time.
///
/// **Known limit**: a project declaring an irregular or foreign plural that
/// happens to need no `-n` in the dative (there is no such case in any
/// shipped project's declared plurals today) would get it wrong here. The
/// fix for that case is a declared dative form, not a change to this rule.
fn dative_plural_de(noun: &str) -> Cow<'_, str> {
    match noun.chars().next_back() {
        // An empty noun stays empty — `""` must never become `"n"`. Symmetric
        // with `agree_singular_casing`'s own empty-label arm; unreachable in
        // the shipped projects, but nothing rejects `plural { de "" }`.
        None => Cow::Borrowed(noun),
        Some(c) if c.eq_ignore_ascii_case(&'n') || c.eq_ignore_ascii_case(&'s') => Cow::Borrowed(noun),
        _ => Cow::Owned(format!("{noun}n")),
    }
}

/// Localized "N of M {noun}" narrowed-count text for the resource-list
/// surf-head, e.g. `"54 von 8\u{2019}942 Aufführungen"`.
///
/// The connective (of / von / sur / di) never inflects — but the trailing noun
/// **does**, and it agrees with `m` (the UNFILTERED total), not with `total`
/// (the narrowed count), in all four languages: "1 von 8’942 Aufführungen",
/// never "1 von 8’942 Aufführung". That is why this fn takes only
/// `noun_other`; `m >= 2` whenever this form renders in practice (the caller
/// only reaches it when `m != total`).
///
/// The `"de"` arm additionally case-inflects `noun_other` to the dative via
/// [`dative_plural_de`] — `von` governs the dative, which German's regular
/// plural-dative rule turns into a trailing `-n` for most nouns (`Werke` →
/// `Werken`, `Bücher` → `Büchern`); nouns already ending in `-n`/`-s`
/// (`Aufführungen`, `Personen`) are unaffected. The other three language
/// arms are untouched.
///
/// **The dative rule fires only on a DECLARED plural**, which is what
/// `plural_declared` carries (`labels::class_plural_declared`, computed at
/// the call site). `resolve_class_plural` degrades to the singular label — and
/// then to the humanized class name — when a class carries no `plural { … }`
/// block; that degraded noun is already the wrong copy in German ("2 Character"),
/// flagged at boot by `MissingClassPlural`. Inflecting it would turn merely
/// untranslated copy into apparent nonsense ("2 von 8 Charactern"), so the rule
/// is skipped and the degraded noun passes through verbatim. A *declared* plural
/// identical to its singular (German has many — `Kapitel`, `Fenster`) still
/// inflects correctly (`Kapiteln`), which a string comparison could not tell
/// apart from the degraded case — hence the explicit flag rather than
/// `noun_other == noun_one`.
///
/// Both counts are grouped by [`group_thousands`].
#[doc(hidden)]
pub fn count_of_text(total: usize, m: usize, noun_other: &str, plural_declared: bool, lang: &Lang) -> String {
    let t = group_thousands(total, lang);
    let m = group_thousands(m, lang);
    match lang.as_str() {
        "de" if !plural_declared => format!("{t} von {m} {noun_other}"),
        "de" => {
            let noun = dative_plural_de(noun_other);
            format!("{t} von {m} {noun}")
        }
        "fr" => format!("{t} sur {m} {noun_other}"),
        "it" => format!("{t} di {m} {noun_other}"),
        _ => format!("{t} of {m} {noun_other}"),
    }
}

/// The `incoming-links` capped-list count line:
/// `render_incoming_links` appends this after the `ul` only when
/// the spec's `limit` actually cut rows (`rows.len() < total`). Noun-free
/// (no article/plural agreement needed — `shown`/`total` are bare numbers),
/// so unlike [`count_of_text`] there is no `plural_declared`/noun parameter.
/// Both counts are grouped by [`group_thousands`]. FR colon spacing matches
/// the rest of this catalog's FR chrome (see e.g. [`remove_filter_label`]'s
/// "Retirer le filtre : …" — a space before the colon).
pub(crate) fn incoming_links_shown_text(shown: usize, total: usize, lang: &Lang) -> String {
    let n = group_thousands(shown, lang);
    let m = group_thousands(total, lang);
    match lang.as_str() {
        "de" => format!("Angezeigt: {n} von {m}"),
        "fr" => format!("Affichage : {n} sur {m}"),
        "it" => format!("Visualizzazione: {n} di {m}"),
        _ => format!("Showing {n} of {m}"),
    }
}

/// Wrap `q` in the language's own quote-mark pair, for the "Copy link"
/// share control's state-bearing accessible-name suffix on browse surfaces
/// (for example `"3 of 54 · Supporting ·
/// Fantasy · “hat”"`). Plain text, never [`Markup`] — this string is
/// embedded verbatim into an HTML `title`/`aria-label` attribute (Maud
/// escapes the whole attribute value at the call site), not into the page
/// body, so it intentionally uses the *typographic* quote pair for every
/// language (EN “ ” included) rather than
/// [`narrowed_to_zero_text`]'s DE „…" quirk (that one is fixed body copy and is not a
/// general DE-quoting convention to reuse here).
#[doc(hidden)]
pub fn localized_quote(q: &str, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("\u{201E}{q}\u{201C}"),
        "fr" => format!("\u{00AB}\u{00A0}{q}\u{00A0}\u{00BB}"),
        "it" => format!("\u{00AB}{q}\u{00BB}"),
        _ => format!("\u{201C}{q}\u{201D}"),
    }
}

/// The language menu summary's hidden name, "Sprache: Deutsch": the
/// [`ChromeMsg::LanguageSwitcherLabel`] and the current language's endonym.
/// French sets a no-break space before the colon.
#[doc(hidden)]
pub fn language_current_text(endonym: &str, lang: &Lang) -> String {
    let label = ChromeMsg::LanguageSwitcherLabel.text(lang);
    match lang.as_str() {
        "fr" => format!("{label}\u{00A0}: {endonym}"),
        _ => format!("{label}: {endonym}"),
    }
}

/// Localized `aria-label` for the facet filter chip's remove control:
/// "Remove filter: {label}". Parameterized (not plural). Pre-existing draft,
/// migrated verbatim.
pub(crate) fn remove_filter_label(lang: &Lang, label: &str) -> String {
    match lang.as_str() {
        "de" => format!("Filter entfernen: {label}"),
        "fr" => format!("Retirer le filtre : {label}"),
        "it" => format!("Rimuovi filtro: {label}"),
        _ => format!("Remove filter: {label}"),
    }
}

/// Localized `aria-label` for the compound `<nav>` element, count-inflecting
/// "parts". Pre-existing draft for the `Other` arms (was `compound_nav_label`,
/// singular-blind); the `One` arms are new.
pub(crate) fn compound_nav_label(total: usize, lang: &Lang) -> String {
    match (lang.as_str(), plural_category(total, lang)) {
        ("de", PluralCategory::One) => format!("Navigation, {total} Teil"),
        ("de", PluralCategory::Other) => format!("Navigation, {total} Teile"),
        ("fr", PluralCategory::One) => format!("Navigation, {total} partie"),
        ("fr", PluralCategory::Other) => format!("Navigation, {total} parties"),
        ("it", PluralCategory::One) => format!("Navigazione, {total} parte"),
        ("it", PluralCategory::Other) => format!("Navigazione, {total} parti"),
        (_, PluralCategory::One) => format!("Navigation, {total} part"),
        (_, PluralCategory::Other) => format!("Navigation, {total} parts"),
    }
}

/// The stale-address line's meta, naming the page actually shown: the first
/// page ("Showing the first page, Vorsatzblatt, recto") or, when a valid
/// `?member=` also applied, another ("Showing page a2r"). FR/IT drafts.
pub(crate) fn compound_stale_shown(label: &str, first: bool, lang: &Lang) -> String {
    match (lang.as_str(), first) {
        ("de", true) => format!("Gezeigt wird die erste Seite, {label}"),
        ("de", false) => format!("Gezeigt wird Seite {label}"),
        ("fr", true) => format!("Première page affichée : {label}"),
        ("fr", false) => format!("Page affichée : {label}"),
        ("it", true) => format!("Viene mostrata la prima pagina, {label}"),
        ("it", false) => format!("Viene mostrata la pagina {label}"),
        (_, true) => format!("Showing the first page, {label}"),
        (_, false) => format!("Showing page {label}"),
    }
}

/// The annotation register's heading, naming the page ("Annotations on
/// c1v"). FR/IT drafts.
pub(crate) fn anno_register_heading(page: &str, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("Annotationen auf {page}"),
        "fr" => format!("Annotations sur {page}"),
        "it" => format!("Annotazioni su {page}"),
        _ => format!("Annotations on {page}"),
    }
}

/// An annotation's role line in the dataset panel ("Region · Annotation 1"),
/// `number` its register number. FR/IT drafts.
pub(crate) fn data_region_role(number: u32, lang: &Lang) -> String {
    let region = ChromeMsg::DataRegion.text(lang);
    match lang.as_str() {
        "it" => format!("{region} · Annotazione {number}"),
        _ => format!("{region} · Annotation {number}"),
    }
}

/// A panel entry named by its role and quoted title ("Page “c1v”"): the
/// subject of the entry's Open and Copy PID names and of its copy receipt.
pub(crate) fn data_entry_name(role: &str, title: &str, lang: &Lang) -> String {
    format!("{role} {}", localized_quote(title, lang))
}

/// The summary's accessible name: its visible label, then the listed entries
/// ("Show in dataset — Book …, Page c1v, Region 1").
pub(crate) fn data_summary_name(items: &[String], lang: &Lang) -> String {
    format!("{} — {}", ChromeMsg::DataSummary.text(lang), items.join(", "))
}

/// An entry's Open link name, its visible label first ("Open in DSP: Page
/// “c1v” (opens DSP, leaves CPE)").
pub(crate) fn data_open_name(entry: &str, lang: &Lang) -> String {
    format!(
        "{}: {entry} ({})",
        ChromeMsg::DataOpen.text(lang),
        ChromeMsg::DataLeavesCpe.text(lang)
    )
}

/// An entry's copy button name, its visible tooltip first ("Copy PID: Page
/// “c1v”").
pub(crate) fn data_copy_name(entry: &str, lang: &Lang) -> String {
    format!("{}: {entry}", ChromeMsg::DataCopyPid.text(lang))
}

const DATA_REGIONS_ONE: L4 = L4 {
    en: "{n} Region",
    de: "{n} Region",
    fr: "{n} région",
    it: "{n} regione",
};

const DATA_REGIONS_OTHER: L4 = L4 {
    en: "{n} Regions",
    de: "{n} Regionen",
    fr: "{n} régions",
    it: "{n} regioni",
};

/// A listing's Region count: DE `32 Regionen`, EN `32 Regions`. FR/IT drafts.
pub(crate) fn data_regions_text(n: usize, lang: &Lang) -> String {
    let template = match plural_category(n, lang) {
        PluralCategory::One => DATA_REGIONS_ONE,
        PluralCategory::Other => DATA_REGIONS_OTHER,
    };
    template.pick(lang).replace("{n}", &group_thousands(n, lang))
}

const DATA_ENTRIES_ONE: L4 = L4 {
    en: "{n} entry",
    de: "{n} Eintrag",
    fr: "{n} entrée",
    it: "{n} voce",
};

const DATA_ENTRIES_OTHER: L4 = L4 {
    en: "{n} entries",
    de: "{n} Einträge",
    fr: "{n} entrées",
    it: "{n} voci",
};

/// A listing fold's entry count for screen readers (the visible digit has
/// no noun): DE `16 Einträge`. FR/IT drafts.
pub(crate) fn data_entries_text(n: usize, lang: &Lang) -> String {
    let template = match plural_category(n, lang) {
        PluralCategory::One => DATA_ENTRIES_ONE,
        PluralCategory::Other => DATA_ENTRIES_OTHER,
    };
    template.pick(lang).replace("{n}", &group_thousands(n, lang))
}

const DATA_AND: L4 = L4 { en: "and", de: "und", fr: "et", it: "e" };

const DATA_IN_SECTIONS: L4 = L4 {
    en: "in {n} sections",
    de: "in {n} Abschnitten",
    fr: "dans {n} sections",
    it: "in {n} sezioni",
};

/// A listing's count line from its count parts (each already counted, e.g.
/// [`data_regions_text`]) and its number of page sections: DE `32 Regionen
/// und 6 Seiten · in 4 Abschnitten`. The parts are joined `a, b und c`; the
/// sections part is left out for one section, which is never folded. FR/IT
/// drafts.
pub(crate) fn data_count_line(parts: &[String], sections: usize, lang: &Lang) -> String {
    let joined = match parts {
        [] => String::new(),
        [only] => only.clone(),
        [init @ .., last] => format!("{} {} {last}", init.join(", "), DATA_AND.pick(lang)),
    };
    if sections > 1 {
        let in_sections = DATA_IN_SECTIONS.pick(lang).replace("{n}", &sections.to_string());
        format!("{joined} · {in_sections}")
    } else {
        joined
    }
}

/// A listed entry's role line, the kind of entry before the page it is on:
/// `Region · c1v`, `Seite · h8r`.
pub(crate) fn data_listed_role(kind: &str, page: &str) -> String {
    format!("{kind} · {page}")
}

/// A listing group's head, its label then the count of its listed entries:
/// `Schramm 1124 · 2`, `Seitenebene · h8r / c6v · 2`.
pub(crate) fn data_group_head(label: &str, n: usize) -> String {
    format!("{label} · {n}")
}

/// The copy receipt the panel's live region announces. FR/IT drafts.
pub(crate) fn data_copied_live(entry: &str, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("PID von {entry} in die Zwischenablage kopiert"),
        "fr" => format!("PID de {entry} copié dans le presse-papiers"),
        "it" => format!("PID di {entry} copiato negli appunti"),
        _ => format!("PID of {entry} copied to the clipboard"),
    }
}

/// A register chip's accessible name ("Annotation 1, Rectangle – highlight
/// on scan"). FR/IT drafts.
pub(crate) fn anno_chip_name(number: u32, shape: &str, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("Annotation {number}, {shape} – auf dem Scan hervorheben"),
        "fr" => format!("Annotation {number}, {shape} – mettre en évidence sur le scan"),
        "it" => format!("Annotazione {number}, {shape} – evidenzia sulla scansione"),
        _ => format!("Annotation {number}, {shape} – highlight on scan"),
    }
}

/// The register's arrival note, naming the partner link's origin page: "From
/// {book} · {page} – annotation {n} highlighted." for a numbered entry, "From
/// {book} · {page}." for a page-level one (`number` `None`). FR/IT drafts.
pub(crate) fn anno_arrival_text(book: &str, page: &str, number: Option<u32>, lang: &Lang) -> String {
    match (lang.as_str(), number) {
        ("de", Some(n)) => format!("Von {book} · {page} – Annotation {n} hervorgehoben."),
        ("de", None) => format!("Von {book} · {page}."),
        ("fr", Some(n)) => format!("Depuis {book} · {page} – annotation {n} mise en évidence."),
        ("fr", None) => format!("Depuis {book} · {page}."),
        ("it", Some(n)) => format!("Da {book} · {page} – annotazione {n} evidenziata."),
        ("it", None) => format!("Da {book} · {page}."),
        (_, Some(n)) => format!("From {book} · {page} – annotation {n} highlighted."),
        (_, None) => format!("From {book} · {page}."),
    }
}

/// The reader eyebrow's register count ("2 annotations"), count-inflected;
/// `n` counts numbered and page-level entries alike. FR/IT drafts.
pub(crate) fn anno_count_text(n: usize, lang: &Lang) -> String {
    match (lang.as_str(), plural_category(n, lang)) {
        ("de", PluralCategory::One) => format!("{n} Annotation"),
        ("de", PluralCategory::Other) => format!("{n} Annotationen"),
        ("it", PluralCategory::One) => format!("{n} annotazione"),
        ("it", PluralCategory::Other) => format!("{n} annotazioni"),
        (_, PluralCategory::One) => format!("{n} annotation"),
        (_, PluralCategory::Other) => format!("{n} annotations"),
    }
}

/// The accessible name of a picker group holding a page with a register:
/// "Lage c – enthält annotierte Seiten". FR/IT drafts.
pub(crate) fn anno_group_marked_name(name: &str, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("{name} – enthält annotierte Seiten"),
        "fr" => format!("{name} – contient des pages annotées"),
        "it" => format!("{name} – contiene pagine annotate"),
        _ => format!("{name} – contains annotated pages"),
    }
}

/// The visually hidden end of an annotated neighbour's pager-step name
/// (", mit Annotation"): it follows the step's visible word.
pub(crate) fn anno_marked_suffix(lang: &Lang) -> String {
    format!(", {}", ChromeMsg::AnnoMarked.text(lang))
}

/// The accessible name of an annotated picker entry or group: "c1v – mit
/// Annotation".
pub(crate) fn anno_marked_name(name: &str, lang: &Lang) -> String {
    format!("{name} – {}", ChromeMsg::AnnoMarked.text(lang))
}

/// "{n} pages", count-inflected: the picker's flat-group meta and the
/// summary name's page count.
pub(crate) fn picker_pages_meta(n: usize, lang: &Lang) -> String {
    match (lang.as_str(), plural_category(n, lang)) {
        ("de", PluralCategory::One) => format!("{n} Seite"),
        ("de", PluralCategory::Other) => format!("{n} Seiten"),
        ("fr", PluralCategory::One) => format!("{n} page"),
        ("fr", PluralCategory::Other) => format!("{n} pages"),
        ("it", PluralCategory::One) => format!("{n} pagina"),
        ("it", PluralCategory::Other) => format!("{n} pagine"),
        (_, PluralCategory::One) => format!("{n} page"),
        (_, PluralCategory::Other) => format!("{n} pages"),
    }
}

/// The page picker's summary accessible name: its visible text plus the
/// current page and the book's extent (WCAG 2.5.3 — the name contains the
/// visible "Choose page"). `current` is the page label, or
/// [`picker_page_at`] when it has none. FR/IT drafts.
pub(crate) fn picker_summary_name(current: &str, total: usize, lang: &Lang) -> String {
    let summary = ChromeMsg::PickerSummary.text(lang);
    let pages = picker_pages_meta(total, lang);
    match lang.as_str() {
        "de" => format!("{summary} (aktuell {current}, {pages})"),
        "fr" => format!("{summary} (actuelle {current}, {pages})"),
        "it" => format!("{summary} (attuale {current}, {pages})"),
        _ => format!("{summary} (current {current}, {pages})"),
    }
}

/// "Seite {n}" / "page {n}" — the position-based name of a page, for a page
/// without a usable label and to tell stacked pages apart. FR/IT drafts.
pub(crate) fn picker_page_at(n: usize, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("Seite {n}"),
        "it" => format!("pagina {n}"),
        _ => format!("page {n}"),
    }
}

/// A gathering's full (visually hidden) name: "Gathering b". FR/IT drafts.
pub(crate) fn picker_gathering_name(key: &str, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("Lage {key}"),
        "fr" => format!("Cahier {key}"),
        "it" => format!("Fascicolo {key}"),
        _ => format!("Gathering {key}"),
    }
}

/// A gathering's meta, its leaf count: "8 leaves" (German "8 Bl." for any
/// number). FR/IT drafts.
pub(crate) fn picker_leaves_meta(n: usize, lang: &Lang) -> String {
    match (lang.as_str(), plural_category(n, lang)) {
        ("de", _) => format!("{n} Bl."),
        ("fr", PluralCategory::One) => format!("{n} feuillet"),
        ("fr", PluralCategory::Other) => format!("{n} feuillets"),
        ("it", PluralCategory::One) => format!("{n} carta"),
        ("it", PluralCategory::Other) => format!("{n} carte"),
        (_, PluralCategory::One) => format!("{n} leaf"),
        (_, PluralCategory::Other) => format!("{n} leaves"),
    }
}

/// A numbered run's full name: "Pages 1–20". FR/IT drafts.
pub(crate) fn picker_range_name(first: usize, last: usize, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("Seiten {first}–{last}"),
        "it" => format!("Pagine {first}–{last}"),
        _ => format!("Pages {first}–{last}"),
    }
}

/// The accessible name of a dash entry with no label: "Page 41, no label".
pub(crate) fn picker_unlabelled_name(n: usize, lang: &Lang) -> String {
    let unlabelled = ChromeMsg::CompoundMemberUnlabelled.text(lang);
    match lang.as_str() {
        "de" => format!("Seite {n}, {unlabelled}"),
        "it" => format!("Pagina {n}, {unlabelled}"),
        _ => format!("Page {n}, {unlabelled}"),
    }
}

/// The accessible name of one of several pages stacked in one picker cell,
/// told apart by position: "D5v (page 267)".
pub(crate) fn picker_collision_name(label: &str, n: usize, lang: &Lang) -> String {
    format!("{label} ({})", picker_page_at(n, lang))
}

/// The pager row's visually hidden live count once the picker takes the
/// centre: "Page a1r, 3 of 328". FR/IT drafts.
pub(crate) fn picker_live_count(label: &str, n: u32, total: usize, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("Seite {label}, {n} von {total}"),
        "fr" => format!("Page {label}, {n} sur {total}"),
        "it" => format!("Pagina {label}, {n} di {total}"),
        _ => format!("Page {label}, {n} of {total}"),
    }
}

/// Localized contents-rail caption interpolating the member `total`
/// ("Contents · N parts"), count-inflecting "parts". Pre-existing draft for
/// the `Other` arms (was `compound_contents_cap`, singular-blind); the `One`
/// arms are new.
pub(crate) fn compound_contents_cap(total: usize, lang: &Lang) -> String {
    match (lang.as_str(), plural_category(total, lang)) {
        ("de", PluralCategory::One) => format!("Inhalt · {total} Teil"),
        ("de", PluralCategory::Other) => format!("Inhalt · {total} Teile"),
        ("fr", PluralCategory::One) => format!("Sommaire · {total} partie"),
        ("fr", PluralCategory::Other) => format!("Sommaire · {total} parties"),
        ("it", PluralCategory::One) => format!("Indice · {total} parte"),
        ("it", PluralCategory::Other) => format!("Indice · {total} parti"),
        (_, PluralCategory::One) => format!("Contents · {total} part"),
        (_, PluralCategory::Other) => format!("Contents · {total} parts"),
    }
}

/// Localized summary text for a facet group's per-group "show the rest"
/// disclosure: "Show 82 more options" / "82 weitere Optionen anzeigen"
/// (`render_facet_panel`'s `FACET_GROUP_COLLAPSE_THRESHOLD` split). `n` is the
/// **remainder** count — the options hidden behind the `<details>`, never the
/// group total. Callers must not invoke this with `n == 0`: at a remainder of
/// zero the disclosure is not rendered at all (never "Show 0 more").
///
/// Count-inflected in all four builtin languages, deliberately. A wording
/// without a noun ("N weitere anzeigen" / "Show N more") would make the
/// `One` and `Other` arms byte-identical apart from the numeral, and a
/// singular test could not fail. Naming the thing counted ("option(s)" /
/// "Option(en)" / "opzione"/"opzioni") makes the singular arm genuinely
/// observable, at the cost of one extra word. The noun is "option", not
/// [`count_resources_text`]'s "entry"/"Eintrag" — what is hidden here is facet
/// options, not records.
///
/// ⚠️ All four `BUILTIN_LANGUAGES` arms are written out explicitly. This is a
/// standalone `fn` matching on `lang.as_str()` with a `_` catch-all (same shape
/// as [`count_resources_text`] / [`compound_nav_label`]), so a missing `fr`/`it`
/// arm would compile silently and fall back to English — there is no compile
/// error and no boot diagnostic to catch it, only review.
pub(crate) fn facet_group_remainder_text(n: usize, lang: &Lang) -> String {
    match (lang.as_str(), plural_category(n, lang)) {
        ("de", PluralCategory::One) => format!("{n} weitere Option anzeigen"),
        ("de", PluralCategory::Other) => format!("{n} weitere Optionen anzeigen"),
        ("fr", PluralCategory::One) => format!("Afficher {n} option de plus"),
        ("fr", PluralCategory::Other) => format!("Afficher {n} options de plus"),
        ("it", PluralCategory::One) => format!("Mostra {n} altra opzione"),
        ("it", PluralCategory::Other) => format!("Mostra {n} altre opzioni"),
        (_, PluralCategory::One) => format!("Show {n} more option"),
        (_, PluralCategory::Other) => format!("Show {n} more options"),
    }
}

/// Localized "narrowed to zero" message for a resource-list's empty state, as
/// ready-to-embed [`Markup`]. Its third
/// (`query.is_none() && filters_active`) branch covers
/// facet-only narrowing. `noun` is a pre-resolved, already
/// localized **plural** class noun (e.g. "characters"/"Figuren"; passed
/// through verbatim, NOT cased or pluralized here). `query` is `Some(q)` when
/// a search query is active (`q` is the literal raw search string) and `None`
/// otherwise; `filters_active` is `true` when at least one facet is
/// currently selected. Callers must only invoke this when `query.is_some() ||
/// filters_active` — the empty-class case (neither active) has its own
/// message ([`ChromeMsg::ListEmptyClass`]).
///
/// Three templates:
/// - `query.is_some() && !filters_active` → ("No {noun} match {q}.")
/// - `query.is_none() && filters_active` → ("No {noun} match these filters.") — no `<em>`, no query
///   to escape.
/// - `query.is_some() && filters_active` → ("No {noun} match {q} with these filters.")
///
/// **The query renders inside `<em>`, quote marks and all** (the
/// `.search-empty__msg em` rule styles it). `noun` and
/// `query` are interpolated via Maud's own `(expr)` auto-escaping inside the
/// `html!` call below — never `PreEscaped`/`format!` — so a
/// `"`/`<script>`-bearing query still renders HTML-escaped, safely nested
/// inside the `<em>`.
///
/// The DE query templates intentionally use the German low-quote `„` …
/// straight `"` pair (not curly `“ … ”`); do not
/// "fix" it. The filters-only branch has no query to quote. New — fr/it
/// are unreviewed drafts.
#[doc(hidden)]
pub fn narrowed_to_zero_text(noun: &str, query: Option<&str>, filters_active: bool, lang: &Lang) -> Markup {
    match (lang.as_str(), query, filters_active) {
        ("de", Some(q), false) => html! {
            "Keine "
            (noun)
            " stimmen mit "
            em { "\u{201E}" (q) "\"" }
            " überein."
        },
        ("de", None, true) => html! {
            "Keine "
            (noun)
            " stimmen mit diesen Filtern überein."
        },
        ("de", Some(q), true) => html! {
            "Keine "
            (noun)
            " stimmen mit "
            em { "\u{201E}" (q) "\"" }
            " und diesen Filtern überein."
        },
        ("fr", Some(q), false) => html! {
            "Aucun "
            (noun)
            " ne correspond à "
            em { "« " (q) " »" }
            "."
        },
        ("fr", None, true) => html! {
            "Aucun "
            (noun)
            " ne correspond à ces filtres."
        },
        ("fr", Some(q), true) => html! {
            "Aucun "
            (noun)
            " ne correspond à "
            em { "« " (q) " »" }
            " avec ces filtres."
        },
        ("it", Some(q), false) => html! {
            "Nessun "
            (noun)
            " corrisponde a "
            em { "«" (q) "»" }
            "."
        },
        ("it", None, true) => html! {
            "Nessun "
            (noun)
            " corrisponde a questi filtri."
        },
        ("it", Some(q), true) => html! {
            "Nessun "
            (noun)
            " corrisponde a "
            em { "«" (q) "»" }
            " con questi filtri."
        },
        (_, Some(q), false) => html! {
            "No "
            (noun)
            " match "
            em { "\"" (q) "\"" }
            "."
        },
        (_, None, true) => html! {
            "No "
            (noun)
            " match these filters."
        },
        (_, Some(q), true) => html! {
            "No "
            (noun)
            " match "
            em { "\"" (q) "\"" }
            " with these filters."
        },
        // Neither active — not a valid call per the doc contract above.
        // Per this codebase's convention (panic on a named-invariant
        // violation, see `dsp-incubator cpe/docs/dev/coding-conventions.md`), callers must
        // gate on query/filters activity before calling this function;
        // silently degrading to a filters-only template would mask a caller
        // bug instead of surfacing it.
        (_, None, false) => {
            unreachable!(
                "narrowed_to_zero_text called with no active narrowing — \
                 callers gate on query/filters activity"
            )
        }
    }
}

/// Localized "no {group} recorded" empty message for a facet group
/// with no options, as plain text (interpolated by the caller's `(expr)` —
/// no inline markup needed). `group` is the pre-resolved, already localized
/// facet-group label, lower-cased (e.g. "themes").
///
/// **German uses the fallback template instead of inflecting `{group}`** —
/// German compounds (e.g. "Themenklasse") don't fit this sentence shape
/// grammatically; see [`ChromeMsg::FacetEmptyGroupFallback`]
/// ("Für diese Klasse sind keine Werte erfasst."). New — fr/it are drafts.
/// Consumed by `facet_panel::render_facet_panel`'s per-group empty
/// branch, which lower-cases the
/// group's resolved label before passing it here.
pub(crate) fn facet_empty_group_text(group: &str, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => ChromeMsg::FacetEmptyGroupFallback.text(lang).to_owned(),
        "fr" => format!("Aucun {group} n'est enregistré pour cette classe."),
        "it" => format!("Nessun {group} è registrato per questa classe."),
        _ => format!("No {group} are recorded for this class."),
    }
}

/// Localized "no parts recorded" empty message for a compound with no
/// members, as plain text. `class_singular` is the pre-resolved, already
/// localized **singular** class label (the sentence shape, "for this
/// {class}", is grammatically singular). New — fr/it are drafts.
///
/// Consumed by `compound_viewer::render_compound_viewer`'s empty branch
/// when the compound's parent Resource
/// resolves; `class_singular` is `resolve_class_label`'s output for the
/// parent's class. When the parent itself doesn't resolve, the renderer
/// falls back to the non-class-aware [`ChromeMsg::CompoundEmptyState`]
/// instead (no class known to interpolate).
pub(crate) fn compound_no_members_text(class_singular: &str, lang: &Lang) -> String {
    match lang.as_str() {
        // DE, FR and IT anchor the article on a fixed noun (`Eintrag` m. /
        // `enregistrement` m. / `record` m.) and carry the class noun after
        // "der Kategorie" / "de la catégorie" / "della categoria", so no
        // sentence has to agree in gender with an interpolated class label it
        // knows nothing about ("dieses {X}" would be hardcoded neuter, "pas de
        // {X}" would need elision). EN needs no such anchor.
        "de" => format!("Für diesen Eintrag der Kategorie {class_singular} sind keine Teile erfasst."),
        "fr" => format!("Aucune partie n'est répertoriée pour cet enregistrement de la catégorie {class_singular}."),
        "it" => format!("Nessuna parte è registrata per questo record della categoria {class_singular}."),
        _ => format!("No parts are recorded for this {class_singular}."),
    }
}

/// Count-aware "N projects" text for the site-index head device.
/// Mirrors [`count_resources_text`]'s shape (see `plural_category`) but
/// with the "projects" noun; it has no "· M available" segment
/// (no data source). New.
pub fn index_count_text(n: usize, lang: &Lang) -> String {
    match (lang.as_str(), plural_category(n, lang)) {
        ("de", PluralCategory::One) => format!("{n} Projekt"),
        ("de", PluralCategory::Other) => format!("{n} Projekte"),
        ("fr", PluralCategory::One) => format!("{n} projet"),
        ("fr", PluralCategory::Other) => format!("{n} projets"),
        ("it", PluralCategory::One) => format!("{n} progetto"),
        ("it", PluralCategory::Other) => format!("{n} progetti"),
        (_, PluralCategory::One) => format!("{n} project"),
        (_, PluralCategory::Other) => format!("{n} projects"),
    }
}

/// The unknown-path page's "{project} — project home" invitation-row text. `project` is the
/// project's title, as the header renders it. New.
pub(crate) fn nf_home_text(project: &str, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("{project} — Projektstartseite"),
        "fr" => format!("{project} — accueil du projet"),
        "it" => format!("{project} — home del progetto"),
        _ => format!("{project} — project home"),
    }
}

/// The unknown-record page's `<h1>` — "There is no {class_singular} at this address."
/// `class_singular` is the pre-resolved, already localized singular class label. Callers
/// unable to supply a comfortable singular for the language (or with no
/// class at all) use [`ChromeMsg::NfIdTitleFallback`] instead. New.
pub(crate) fn nf_id_title_text(class_singular: &str, lang: &Lang) -> String {
    match lang.as_str() {
        // DE, FR and IT anchor the article on a fixed noun (`Eintrag` m. /
        // `enregistrement` m. / `record` m.) and carry the class noun after
        // "der Kategorie" / "de la catégorie" / "della categoria", so neither
        // sentence has to agree in gender with an interpolated class label it
        // knows nothing about (French "pas de {X}" would
        // need elision before a vowel-initial label, e.g. "pas de
        // Image"). EN needs no such anchor.
        "de" => {
            format!("Unter dieser Adresse gibt es keinen Eintrag der Kategorie {class_singular}.")
        }
        "fr" => {
            format!("Il n'y a aucun enregistrement de la catégorie {class_singular} à cette adresse.")
        }
        "it" => {
            format!("A questo indirizzo non esiste alcun record della categoria {class_singular}.")
        }
        _ => format!("There is no {class_singular} at this address."),
    }
}

/// The unknown-record page's body message, count-neutral, parameterized by the singular class
/// label. New.
pub(crate) fn nf_id_msg_text(class_singular: &str, lang: &Lang) -> String {
    match lang.as_str() {
        // Same genderless anchoring as `nf_id_title_text` — see its comment.
        "de" => format!(
            "Der Bezeichner in dieser Adresse benennt keinen Eintrag der Kategorie \
             {class_singular} in diesem Projekt. Einträge werden manchmal umstrukturiert, \
             nachdem ein Link oder ein Zitat erstellt wurde."
        ),
        "fr" => format!(
            "L'identifiant de cette adresse ne désigne aucun enregistrement de la \
             catégorie {class_singular} dans ce projet. Les enregistrements sont parfois \
             remodelés après la création d'un lien ou d'une citation."
        ),
        "it" => format!(
            "L'identificatore in questo indirizzo non indica alcun record della categoria \
             {class_singular} in questo progetto. I record vengono talvolta rimodellati \
             dopo la creazione di un link o di una citazione."
        ),
        _ => format!(
            "The identifier in this address does not name a {class_singular} in this \
             project. Records are sometimes re-modelled after a link or citation is made."
        ),
    }
}

/// The unknown-record page's exit — "Browse all {class}", `class` being the pre-resolved, already
/// localized **plural** class label (never singular — "Browse all
/// characters", not "Browse all character"). New.
pub(crate) fn nf_browse_all_text(class_plural: &str, lang: &Lang) -> String {
    match lang.as_str() {
        "de" => format!("Alle {class_plural} durchsuchen"),
        "fr" => format!("Parcourir tous les {class_plural}"),
        "it" => format!("Sfoglia tutti i {class_plural}"),
        _ => format!("Browse all {class_plural}"),
    }
}

// Timeline
//
// Built from `L4` TEMPLATES, not a `match lang.as_str()` with a `_` catch-all:
// the struct literal makes a missing language a compile error, which a string
// match can never do. Each template is picked with `L4::pick` and its
// placeholder (`{n}` / `{year}`) substituted. fr/it are unreviewed drafts.

/// The singular (`PluralCategory::One`) arm of [`timeline_extent_text`].
const TIMELINE_EXTENT_ONE: L4 = L4 {
    en: "{n} page",
    de: "{n} Seite",
    fr: "{n} page",
    it: "{n} pagina",
};

/// The plural (`PluralCategory::Other`) arm of [`timeline_extent_text`].
const TIMELINE_EXTENT_OTHER: L4 = L4 {
    en: "{n} pages",
    de: "{n} Seiten",
    fr: "{n} pages",
    it: "{n} pagine",
};

/// The template of [`timeline_decade_label`].
const TIMELINE_DECADE_LABEL: L4 = L4 {
    en: "{year}s",
    de: "{year}er",
    fr: "années {year}",
    it: "anni {year}",
};

/// The extent segment of a timeline marker's accessible name (`title` and
/// `aria-label`): DE `1 Seite` / `N Seiten`, EN `1 page` / `N pages`.
///
/// Count-inflected via [`plural_category`] (so French singularizes `n == 0`).
/// `n` is the `extent` property's `PropertyValue::Integer` (`i64`); a
/// negative value — nonsensical for a page count, but not something this
/// helper may panic on — has no `usize` form and takes the `Other` arm. The
/// number is **not** grouped into thousands: a page count is small, and
/// [`group_thousands`] is scoped to the browse count line.
pub(crate) fn timeline_extent_text(n: i64, lang: &Lang) -> String {
    let category = usize::try_from(n).map_or(PluralCategory::Other, |u| plural_category(u, lang));
    let template = match category {
        PluralCategory::One => TIMELINE_EXTENT_ONE,
        PluralCategory::Other => TIMELINE_EXTENT_OTHER,
    };
    template.pick(lang).replace("{n}", &n.to_string())
}

/// The decades variant's decade label: DE `1470er`, EN `1470s`.
///
/// Both languages get their own form; EN does not reuse the German
/// `"1470er"`. Localizing follows HdM's `decade` list nodes
/// (DE `1760er` / EN `1760s`) and the rule that an EN visitor never reads
/// German.
pub(crate) fn timeline_decade_label(start_year: i32, lang: &Lang) -> String {
    TIMELINE_DECADE_LABEL.pick(lang).replace("{year}", &start_year.to_string())
}

// Showcase crop figures
//
// `L4` templates like the timeline's above. fr/it are unreviewed drafts.

const CASE_IMPRESSIONS_ONE: L4 = L4 {
    en: "{n} impression",
    de: "{n} Abdruck",
    fr: "{n} tirage",
    it: "{n} impressione",
};
const CASE_IMPRESSIONS_OTHER: L4 = L4 {
    en: "{n} impressions",
    de: "{n} Abdrucke",
    fr: "{n} tirages",
    it: "{n} impressioni",
};
const CASE_SAME_BLOCK_ONE: L4 = L4 {
    en: "The same woodblock, {n} impression:",
    de: "Derselbe Holzstock, {n} Abdruck:",
    fr: "Le même bois, {n} tirage :",
    it: "Lo stesso legno, {n} impressione:",
};
const CASE_SAME_BLOCK_OTHER: L4 = L4 {
    en: "The same woodblock, {n} impressions:",
    de: "Derselbe Holzstock, {n} Abdrucke:",
    fr: "Le même bois, {n} tirages :",
    it: "Lo stesso legno, {n} impressioni:",
};
/// The catalogue name is a proper noun, the same in every language.
const CASE_SCHRAMM: L4 = L4 {
    en: "Schramm {n}",
    de: "Schramm {n}",
    fr: "Schramm {n}",
    it: "Schramm {n}",
};
const CASE_ALT_FALLBACK: L4 = L4 {
    en: "Woodcut, {book}, {page}",
    de: "Holzschnitt, {book}, {page}",
    fr: "Gravure sur bois, {book}, {page}",
    it: "Xilografia, {book}, {page}",
};

const CASE_PAGES_ONE: L4 = L4 {
    en: "{n} page",
    de: "{n} Seite",
    fr: "{n} page",
    it: "{n} pagina",
};
const CASE_PAGES_OTHER: L4 = L4 {
    en: "{n} pages",
    de: "{n} Seiten",
    fr: "{n} pages",
    it: "{n} pagine",
};
const CASE_PAGE_ALT: L4 = L4 {
    en: "Whole page {page}, {book}",
    de: "Ganze Seite {page}, {book}",
    fr: "Page entière {page}, {book}",
    it: "Pagina intera {page}, {book}",
};

/// A page case's page count in its id line: DE `2 Seiten`, EN `2 pages`.
pub(crate) fn case_pages_text(n: usize, lang: &Lang) -> String {
    let template = match plural_category(n, lang) {
        PluralCategory::One => CASE_PAGES_ONE,
        PluralCategory::Other => CASE_PAGES_OTHER,
    };
    template.pick(lang).replace("{n}", &n.to_string())
}

/// A whole page's alt in a page case: DE `Ganze Seite {page}, {book}`. The
/// page is filled before the book is joined on, so a `{page}` inside the
/// book's title is never substituted.
pub(crate) fn case_page_alt(book: &str, page: &str, lang: &Lang) -> String {
    let template = CASE_PAGE_ALT.pick(lang);
    let (head, rest) = template
        .split_once("{book}")
        .expect("every CASE_PAGE_ALT arm has a {book} placeholder");
    format!("{}{book}{rest}", head.replace("{page}", page))
}

/// The detail crop's eyebrow: DE `Detail · b3v`.
pub(crate) fn case_detail_label(page: &str, lang: &Lang) -> String {
    format!("{} · {page}", ChromeMsg::CaseDetail.text(lang))
}

const STRIP_TILE_NAME: L4 = L4 {
    en: "Border strip {id}",
    de: "Randleiste {id}",
    fr: "Bordure {id}",
    it: "Fregio {id}",
};

/// A strip-row tile link's accessible name, around the strip's visible id:
/// DE `Randleiste l_6`.
pub(crate) fn strip_tile_name(id: &str, lang: &Lang) -> String {
    STRIP_TILE_NAME.pick(lang).replace("{id}", id)
}

/// A case's impression count in its id line: DE `2 Abdrucke`, EN
/// `2 impressions`.
pub(crate) fn case_impressions_text(n: usize, lang: &Lang) -> String {
    let template = match plural_category(n, lang) {
        PluralCategory::One => CASE_IMPRESSIONS_ONE,
        PluralCategory::Other => CASE_IMPRESSIONS_OTHER,
    };
    template.pick(lang).replace("{n}", &n.to_string())
}

/// A case's hidden lead before its row, saying the crops are one block: DE
/// `Derselbe Holzstock, 2 Abdrucke:`.
pub(crate) fn case_same_block_text(n: usize, lang: &Lang) -> String {
    let template = match plural_category(n, lang) {
        PluralCategory::One => CASE_SAME_BLOCK_ONE,
        PluralCategory::Other => CASE_SAME_BLOCK_OTHER,
    };
    template.pick(lang).replace("{n}", &n.to_string())
}

/// A case's Schramm reference in its id line: `Schramm 1124`.
pub(crate) fn case_schramm_text(n: u32, lang: &Lang) -> String {
    CASE_SCHRAMM.pick(lang).replace("{n}", &n.to_string())
}

/// A crop's alt when its config names none: DE `Holzschnitt, {book},
/// {page}`. One pass over the template, so a `{page}` inside the book's
/// title is never substituted.
pub(crate) fn case_alt_fallback(book: &str, page: &str, lang: &Lang) -> String {
    let template = CASE_ALT_FALLBACK.pick(lang);
    let (head, rest) = template
        .split_once("{book}")
        .expect("every CASE_ALT_FALLBACK arm has a {book} placeholder");
    format!("{head}{book}{}", rest.replace("{page}", page))
}

// Tests

#[cfg(test)]
mod tests {
    use cpe_core::ir::SiteConfig;

    use super::*;

    fn lang(code: &str) -> Lang {
        let site = SiteConfig {
            title: "Test".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec![
                "en".to_owned(),
                "de".to_owned(),
                "fr".to_owned(),
                "it".to_owned(),
                "zz".to_owned(),
            ],
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
        cpe_core::lang::resolve(&[code.to_owned()], &site)
    }

    // month_name / date_long_text

    #[test]
    fn month_name_covers_twelve_months_in_four_languages() {
        assert_eq!(month_name(1, &lang("en")), Some("January"));
        assert_eq!(month_name(2, &lang("de")), Some("Februar"));
        assert_eq!(month_name(8, &lang("fr")), Some("août"));
        assert_eq!(month_name(12, &lang("it")), Some("dicembre"));
        // A 5th+ language falls back to English, like `L4::pick`.
        assert_eq!(month_name(3, &lang("zz")), Some("March"));
        for code in BUILTIN_LANGUAGES {
            for n in 1..=12 {
                assert!(month_name(n, &lang(code)).is_some(), "{code} {n}");
            }
            assert_eq!(month_name(0, &lang(code)), None);
            assert_eq!(month_name(13, &lang(code)), None);
        }
    }

    #[test]
    fn date_long_text_per_language_pattern() {
        assert_eq!(date_long_text("1491", "February", Some(19), &lang("en")), "19 February 1491");
        assert_eq!(date_long_text("1491", "Februar", Some(19), &lang("de")), "19. Februar 1491");
        assert_eq!(date_long_text("1491", "février", Some(19), &lang("fr")), "19 février 1491");
        assert_eq!(date_long_text("1491", "février", Some(1), &lang("fr")), "1er février 1491");
        assert_eq!(date_long_text("1491", "febbraio", Some(19), &lang("it")), "19 febbraio 1491");
        assert_eq!(date_long_text("1491", "Februar", None, &lang("de")), "Februar 1491");
    }

    // Timeline

    #[test]
    fn timeline_key_in_four_languages() {
        let msg = ChromeMsg::TimelineKey;
        assert_eq!(
            msg.text(&lang("de")),
            "Balken = ein Buch · breiter Balken = Zeitraum · Titel, Datum und Umfang beim Zeigen · Rasterlinien alle drei Jahre"
        );
        assert_eq!(
            msg.text(&lang("en")),
            "bar = one book · wide bar = a date range · title, date and extent on hover · grid lines every three years"
        );
        assert_eq!(
            msg.text(&lang("fr")),
            "barre = un livre · barre large = une période · titre, date et étendue au survol · lignes de grille tous les trois ans"
        );
        assert_eq!(
            msg.text(&lang("it")),
            "barra = un libro · barra larga = un periodo · titolo, data e consistenza al passaggio del mouse · linee della griglia ogni tre anni"
        );
    }

    #[test]
    fn case_texts_inflect_and_fill_their_placeholders() {
        assert_eq!(case_impressions_text(2, &lang("de")), "2 Abdrucke");
        assert_eq!(case_impressions_text(1, &lang("de")), "1 Abdruck");
        assert_eq!(case_impressions_text(4, &lang("en")), "4 impressions");
        assert_eq!(case_same_block_text(3, &lang("de")), "Derselbe Holzstock, 3 Abdrucke:");
        assert_eq!(case_same_block_text(2, &lang("en")), "The same woodblock, 2 impressions:");
        assert_eq!(case_schramm_text(1124, &lang("en")), "Schramm 1124");
        assert_eq!(
            case_alt_fallback("Reise {page}", "46r", &lang("de")),
            "Holzschnitt, Reise {page}, 46r",
            "a placeholder inside the title stays literal"
        );
        assert_eq!(case_alt_fallback("Postilla", "e3r", &lang("en")), "Woodcut, Postilla, e3r");
        // Every arm carries `{book}` (the `expect`) and `{page}`.
        for code in BUILTIN_LANGUAGES {
            let alt = case_alt_fallback("BOOK", "PAGE", &lang(code));
            assert!(alt.contains("BOOK") && alt.contains("PAGE"), "{code}: {alt}");
            assert!(!alt.contains('{'), "{code}: {alt}");
        }
    }

    #[test]
    fn page_case_texts_inflect_and_fill_their_placeholders() {
        assert_eq!(case_pages_text(2, &lang("de")), "2 Seiten");
        assert_eq!(case_pages_text(1, &lang("de")), "1 Seite");
        assert_eq!(case_pages_text(2, &lang("en")), "2 pages");
        assert_eq!(
            case_page_alt("Walfart {book}", "h8r", &lang("de")),
            "Ganze Seite h8r, Walfart {book}",
            "a placeholder inside the title stays literal"
        );
        assert_eq!(
            case_page_alt("Postilla {page}", "e3r", &lang("en")),
            "Whole page e3r, Postilla {page}"
        );
        assert_eq!(case_detail_label("b3v", &lang("en")), "Detail · b3v");
        // Every arm carries `{book}` (the `expect`) and `{page}`.
        for code in BUILTIN_LANGUAGES {
            let alt = case_page_alt("BOOK", "PAGE", &lang(code));
            assert!(alt.contains("BOOK") && alt.contains("PAGE"), "{code}: {alt}");
            assert!(!alt.contains('{'), "{code}: {alt}");
        }
    }

    #[test]
    fn timeline_extent_text_plural_arms_in_four_languages() {
        let cases: [(&str, [&str; 3]); 4] = [
            ("en", ["0 pages", "1 page", "2 pages"]),
            ("de", ["0 Seiten", "1 Seite", "2 Seiten"]),
            // French singularizes n == 0.
            ("fr", ["0 page", "1 page", "2 pages"]),
            ("it", ["0 pagine", "1 pagina", "2 pagine"]),
        ];
        for (code, expected) in cases {
            for (n, want) in [0_i64, 1, 2].into_iter().zip(expected) {
                assert_eq!(timeline_extent_text(n, &lang(code)), want, "{code} n={n}");
            }
        }
        // Not grouped into thousands.
        assert_eq!(timeline_extent_text(1234, &lang("de")), "1234 Seiten");
        // A negative value has no usize form: the Other arm, no panic.
        assert_eq!(timeline_extent_text(-1, &lang("fr")), "-1 pages");
    }

    #[test]
    fn timeline_decade_label_in_four_languages() {
        assert_eq!(timeline_decade_label(1470, &lang("de")), "1470er");
        assert_eq!(timeline_decade_label(1470, &lang("en")), "1470s");
        assert_eq!(timeline_decade_label(1470, &lang("fr")), "années 1470");
        assert_eq!(timeline_decade_label(1470, &lang("it")), "anni 1470");
    }

    // L4::pick

    #[test]
    fn l4_pick_selects_each_declared_language() {
        let msg = ChromeMsg::OrderLabel;
        assert_eq!(msg.text(&lang("en")), "Order");
        assert_eq!(msg.text(&lang("de")), "Reihenfolge");
        assert_eq!(msg.text(&lang("fr")), "Ordre");
        assert_eq!(msg.text(&lang("it")), "Ordine");
    }

    #[test]
    fn l4_pick_falls_back_to_english_for_unknown_code() {
        // "zz" is declared on the test site (so `resolve` doesn't itself fall
        // back to "en"), but is not one of the four builtins `pick` knows —
        // it must still resolve through `pick`'s `_ => en` arm.
        let msg = ChromeMsg::OrderLabel;
        assert_eq!(msg.text(&lang("zz")), "Order");
    }

    // BUILTIN_LANGUAGES

    #[test]
    fn builtin_languages_is_en_de_fr_it_in_order() {
        assert_eq!(BUILTIN_LANGUAGES, ["en", "de", "fr", "it"]);
    }

    // general chrome: the header menu strings

    #[test]
    fn language_current_text_names_the_current_language() {
        assert_eq!(language_current_text("Deutsch", &lang("de")), "Sprache: Deutsch");
        assert_eq!(language_current_text("English", &lang("en")), "Language: English");
        assert_eq!(language_current_text("Italiano", &lang("it")), "Lingua: Italiano");
        // French: a no-break space before the colon.
        assert_eq!(language_current_text("Français", &lang("fr")), "Langue\u{00A0}: Français");
    }

    /// Each header-menu string in every built-in language, in
    /// `BUILTIN_LANGUAGES` order (en, de, fr, it).
    #[test]
    fn header_menu_strings_cover_every_builtin_language() {
        let table: [(ChromeMsg, [&str; 4]); 5] = [
            (ChromeMsg::MenuLabel, ["Menu", "Menü", "Menu", "Menu"]),
            (
                ChromeMsg::AppearanceLabel,
                ["Appearance", "Darstellung", "Apparence", "Aspetto"],
            ),
            (
                ChromeMsg::ThemeDarkModeLabel,
                ["Dark mode", "Dunkler Modus", "Mode sombre", "Modalità scura"],
            ),
            (
                ChromeMsg::ThemeLightModeLabel,
                ["Light mode", "Heller Modus", "Mode clair", "Modalità chiara"],
            ),
            (ChromeMsg::ThemeSwitchSuffix, ["switch", "umschalten", "basculer", "cambia"]),
        ];
        for (msg, expected) in table {
            for (code, want) in BUILTIN_LANGUAGES.iter().zip(expected) {
                assert_eq!(msg.text(&lang(code)), want, "{msg:?} {code}");
            }
        }
    }

    #[test]
    fn narrative_rail_label_covers_every_builtin_language() {
        let expected = ["On this page", "Auf dieser Seite", "Sur cette page", "In questa pagina"];
        for (code, want) in BUILTIN_LANGUAGES.iter().zip(expected) {
            assert_eq!(ChromeMsg::NarrativeRailLabel.text(&lang(code)), want);
        }
    }

    // plural_category

    #[test]
    fn plural_category_en_de_it_only_one_is_singular() {
        for code in ["en", "de", "it"] {
            assert_eq!(plural_category(0, &lang(code)), PluralCategory::Other);
            assert_eq!(plural_category(1, &lang(code)), PluralCategory::One);
            assert_eq!(plural_category(2, &lang(code)), PluralCategory::Other);
        }
    }

    #[test]
    fn data_count_line_joins_its_parts_and_names_more_than_one_section() {
        let de = lang("de");
        let parts = |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).to_owned()).collect() };
        assert_eq!(
            data_count_line(&parts(&["32 Regionen", "6 Seiten"]), 4, &de),
            "32 Regionen und 6 Seiten · in 4 Abschnitten"
        );
        assert_eq!(data_count_line(&parts(&["5 Regionen"]), 1, &de), "5 Regionen");
        assert_eq!(
            data_count_line(&parts(&["1 Region", "2 Seiten", "3 Bücher"]), 1, &de),
            "1 Region, 2 Seiten und 3 Bücher"
        );
        assert_eq!(
            data_count_line(&parts(&["32 Regions", "6 pages"]), 4, &lang("en")),
            "32 Regions and 6 pages · in 4 sections"
        );
        for code in BUILTIN_LANGUAGES {
            let l = lang(code);
            let line = data_count_line(&parts(&["a", "b"]), 2, &l);
            assert!(line.starts_with("a ") && line.contains(" b · "), "{code}: {line}");
            assert!(line.contains(" 2 "), "{code}: {line}");
        }
    }

    #[test]
    fn data_regions_text_is_plural_aware_and_grouped() {
        assert_eq!(data_regions_text(1, &lang("de")), "1 Region");
        assert_eq!(data_regions_text(32, &lang("de")), "32 Regionen");
        assert_eq!(data_regions_text(1, &lang("en")), "1 Region");
        assert_eq!(data_regions_text(1200, &lang("en")), "1,200 Regions");
        assert_eq!(data_regions_text(0, &lang("fr")), "0 région");
        assert_eq!(data_regions_text(2, &lang("it")), "2 regioni");
    }

    #[test]
    fn plural_category_fr_zero_and_one_are_singular() {
        assert_eq!(plural_category(0, &lang("fr")), PluralCategory::One);
        assert_eq!(plural_category(1, &lang("fr")), PluralCategory::One);
        assert_eq!(plural_category(2, &lang("fr")), PluralCategory::Other);
    }

    // group_thousands

    /// Every separator is asserted **by codepoint escape**, never by a
    /// hand-typed glyph: an ASCII `'` (U+0027) and a plain space (U+0020) look
    /// identical enough in source to the Swiss apostrophe (U+2019) and the
    /// narrow no-break space (U+202F) that a typed expectation could pass
    /// while the wrong character ships.
    #[test]
    fn group_thousands_uses_the_locale_separator_by_codepoint() {
        for code in ["de", "en", "fr", "it"] {
            assert_eq!(group_thousands(0, &lang(code)), "0", "lang={code}");
            assert_eq!(group_thousands(1, &lang(code)), "1", "lang={code}");
            assert_eq!(group_thousands(999, &lang(code)), "999", "lang={code}");
        }

        // de/it — Swiss apostrophe U+2019 (NOT ASCII U+0027).
        for code in ["de", "it"] {
            assert_eq!(group_thousands(1000, &lang(code)), "1\u{2019}000");
            assert_eq!(group_thousands(8942, &lang(code)), "8\u{2019}942");
            assert_eq!(group_thousands(1_000_000, &lang(code)), "1\u{2019}000\u{2019}000");
        }

        // en (and the non-builtin fallback) — ASCII comma.
        assert_eq!(group_thousands(1000, &lang("en")), "1,000");
        assert_eq!(group_thousands(8942, &lang("en")), "8,942");
        assert_eq!(group_thousands(1_000_000, &lang("en")), "1,000,000");
        assert_eq!(group_thousands(8942, &lang("es")), "8,942");

        // fr — narrow no-break space U+202F (NOT U+0020, NOT U+00A0).
        assert_eq!(group_thousands(1000, &lang("fr")), "1\u{202F}000");
        assert_eq!(group_thousands(8942, &lang("fr")), "8\u{202F}942");
        assert_eq!(group_thousands(1_000_000, &lang("fr")), "1\u{202F}000\u{202F}000");
    }

    // agree_singular_casing

    #[test]
    fn agree_singular_casing_lowercases_when_plural_is_lowercase() {
        assert_eq!(agree_singular_casing("Work", "works"), "work");
    }

    #[test]
    fn agree_singular_casing_leaves_capitalized_plural_unchanged() {
        assert_eq!(agree_singular_casing("Werk", "Werke"), "Werk");
    }

    #[test]
    fn agree_singular_casing_guards_acronyms() {
        assert_eq!(agree_singular_casing("IIIF Manifest", "IIIF manifests"), "IIIF Manifest");
    }

    #[test]
    fn agree_singular_casing_handles_single_char_label() {
        assert_eq!(agree_singular_casing("A", "as"), "a");
        assert_eq!(agree_singular_casing("A", "As"), "A");
    }

    #[test]
    fn agree_singular_casing_handles_empty_label() {
        assert_eq!(agree_singular_casing("", "works"), "");
    }

    // count_resources_text

    /// The noun is the caller's (the class's own declared labels) — there is no
    /// generic `Einträge`/`resources` arm left to fall back to.
    #[test]
    fn count_resources_text_pluralizes_all_langs() {
        assert_eq!(count_resources_text(1, "resource", "resources", &lang("en")), "1 resource");
        assert_eq!(count_resources_text(0, "resource", "resources", &lang("en")), "0 resources");
        assert_eq!(count_resources_text(2, "resource", "resources", &lang("en")), "2 resources");

        assert_eq!(
            count_resources_text(0, "Aufführung", "Aufführungen", &lang("de")),
            "0 Aufführungen"
        );
        assert_eq!(
            count_resources_text(1, "Aufführung", "Aufführungen", &lang("de")),
            "1 Aufführung"
        );
        assert_eq!(
            count_resources_text(2, "Aufführung", "Aufführungen", &lang("de")),
            "2 Aufführungen"
        );

        // French singularizes `n == 0` — `plural_category`'s fr arm must still
        // fire through the class-noun signature.
        assert_eq!(
            count_resources_text(0, "représentation", "représentations", &lang("fr")),
            "0 représentation"
        );
        assert_eq!(
            count_resources_text(1, "représentation", "représentations", &lang("fr")),
            "1 représentation"
        );
        assert_eq!(
            count_resources_text(2, "représentation", "représentations", &lang("fr")),
            "2 représentations"
        );
    }

    /// An invariable noun (Italian's borrowed "record") is simply declared
    /// identically in both slots — the engine does no pluralization of its own.
    #[test]
    fn count_resources_text_italian_record_is_invariant() {
        assert_eq!(count_resources_text(0, "record", "record", &lang("it")), "0 record");
        assert_eq!(count_resources_text(1, "record", "record", &lang("it")), "1 record");
        assert_eq!(count_resources_text(2, "record", "record", &lang("it")), "2 record");
    }

    /// The count itself is grouped.
    #[test]
    fn count_resources_text_groups_thousands() {
        assert_eq!(
            count_resources_text(8942, "Aufführung", "Aufführungen", &lang("de")),
            "8\u{2019}942 Aufführungen"
        );
        assert_eq!(
            count_resources_text(8942, "performance", "performances", &lang("en")),
            "8,942 performances"
        );
    }

    // compound_nav_label / compound_contents_cap

    #[test]
    fn compound_nav_label_pluralizes_parts_across_languages() {
        assert_eq!(compound_nav_label(0, &lang("en")), "Navigation, 0 parts");
        assert_eq!(compound_nav_label(1, &lang("en")), "Navigation, 1 part");
        assert_eq!(compound_nav_label(2, &lang("en")), "Navigation, 2 parts");
        assert_eq!(compound_nav_label(0, &lang("de")), "Navigation, 0 Teile");
        assert_eq!(compound_nav_label(1, &lang("de")), "Navigation, 1 Teil");
        assert_eq!(compound_nav_label(2, &lang("de")), "Navigation, 2 Teile");
        assert_eq!(compound_nav_label(0, &lang("fr")), "Navigation, 0 partie");
        assert_eq!(compound_nav_label(1, &lang("fr")), "Navigation, 1 partie");
        assert_eq!(compound_nav_label(2, &lang("fr")), "Navigation, 2 parties");
        assert_eq!(compound_nav_label(0, &lang("it")), "Navigazione, 0 parti");
        assert_eq!(compound_nav_label(1, &lang("it")), "Navigazione, 1 parte");
        assert_eq!(compound_nav_label(2, &lang("it")), "Navigazione, 2 parti");
    }

    #[test]
    fn compound_contents_cap_pluralizes_parts_across_languages() {
        assert_eq!(compound_contents_cap(0, &lang("en")), "Contents · 0 parts");
        assert_eq!(compound_contents_cap(1, &lang("en")), "Contents · 1 part");
        assert_eq!(compound_contents_cap(2, &lang("en")), "Contents · 2 parts");
        assert_eq!(compound_contents_cap(0, &lang("de")), "Inhalt · 0 Teile");
        assert_eq!(compound_contents_cap(1, &lang("de")), "Inhalt · 1 Teil");
        assert_eq!(compound_contents_cap(2, &lang("de")), "Inhalt · 2 Teile");
        assert_eq!(compound_contents_cap(0, &lang("fr")), "Sommaire · 0 partie");
        assert_eq!(compound_contents_cap(1, &lang("fr")), "Sommaire · 1 partie");
        assert_eq!(compound_contents_cap(2, &lang("fr")), "Sommaire · 2 parties");
        assert_eq!(compound_contents_cap(0, &lang("it")), "Indice · 0 parti");
        assert_eq!(compound_contents_cap(1, &lang("it")), "Indice · 1 parte");
        assert_eq!(compound_contents_cap(2, &lang("it")), "Indice · 2 parti");
    }

    #[test]
    fn anno_count_text_pluralizes_across_languages() {
        assert_eq!(anno_count_text(1, &lang("de")), "1 Annotation");
        assert_eq!(anno_count_text(2, &lang("de")), "2 Annotationen");
        assert_eq!(anno_count_text(1, &lang("en")), "1 annotation");
        assert_eq!(anno_count_text(2, &lang("en")), "2 annotations");
        assert_eq!(anno_count_text(1, &lang("fr")), "1 annotation");
        assert_eq!(anno_count_text(2, &lang("fr")), "2 annotations");
        assert_eq!(anno_count_text(1, &lang("it")), "1 annotazione");
        assert_eq!(anno_count_text(2, &lang("it")), "2 annotazioni");
    }

    #[test]
    fn anno_marked_names_in_every_language() {
        for (l, page, suffix, group) in [
            (
                "de",
                "c1v – mit Annotation",
                ", mit Annotation",
                "Lage c – enthält annotierte Seiten",
            ),
            ("en", "c1v – annotated", ", annotated", "Lage c – contains annotated pages"),
            (
                "fr",
                "c1v – avec annotation",
                ", avec annotation",
                "Lage c – contient des pages annotées",
            ),
            (
                "it",
                "c1v – con annotazione",
                ", con annotazione",
                "Lage c – contiene pagine annotate",
            ),
        ] {
            assert_eq!(anno_marked_name("c1v", &lang(l)), page);
            assert_eq!(anno_marked_suffix(&lang(l)), suffix);
            assert_eq!(anno_group_marked_name("Lage c", &lang(l)), group);
        }
    }

    #[test]
    fn anno_arrival_text_with_and_without_a_number() {
        let dt = "Narrenschiff (dt.)";
        assert_eq!(
            anno_arrival_text(dt, "c1v", Some(1), &lang("de")),
            "Von Narrenschiff (dt.) · c1v – Annotation 1 hervorgehoben."
        );
        assert_eq!(anno_arrival_text(dt, "c1v", None, &lang("de")), "Von Narrenschiff (dt.) · c1v.");
        assert_eq!(
            anno_arrival_text(dt, "c1v", Some(2), &lang("en")),
            "From Narrenschiff (dt.) · c1v – annotation 2 highlighted."
        );
        assert_eq!(
            anno_arrival_text(dt, "c1v", None, &lang("en")),
            "From Narrenschiff (dt.) · c1v."
        );
        for l in ["fr", "it"] {
            let with = anno_arrival_text(dt, "c1v", Some(3), &lang(l));
            assert!(with.contains(dt) && with.contains("c1v") && with.contains('3'), "{with}");
        }
    }

    // dative_plural_de

    #[test]
    fn dative_plural_de_appends_n_for_regular_plurals() {
        assert_eq!(dative_plural_de("Werke"), "Werken");
        assert_eq!(dative_plural_de("Bücher"), "Büchern");
        assert_eq!(dative_plural_de("Orte"), "Orten");
        assert_eq!(dative_plural_de("Kapitel"), "Kapiteln");
    }

    #[test]
    fn dative_plural_de_leaves_n_and_s_endings_unchanged() {
        for noun in ["Aufführungen", "Personen", "Quellen", "Figuren", "Geschichten"] {
            assert_eq!(dative_plural_de(noun), noun);
        }
        // An `-s` plural (e.g. a borrowed/foreign form) is also unchanged.
        assert_eq!(dative_plural_de("Kondominiums"), "Kondominiums");
    }

    // count_of_text

    /// The narrowed "N von M {noun}" form dative-inflects the noun — a
    /// project-declared nominative plural like "Werke" must read "Werken"
    /// after `von`.
    #[test]
    fn count_of_text_de_dative_inflects_the_noun() {
        assert_eq!(
            count_of_text(145, 2449, "Werke", true, &lang("de")),
            "145 von 2\u{2019}449 Werken"
        );
    }

    /// The connective never inflects; the trailing noun is the caller's plural
    /// in every case.
    #[test]
    fn count_of_text_connective_never_inflects() {
        assert_eq!(count_of_text(0, 54, "personnages", true, &lang("fr")), "0 sur 54 personnages");
        assert_eq!(count_of_text(1, 54, "personnages", true, &lang("fr")), "1 sur 54 personnages");
        assert_eq!(count_of_text(8, 54, "characters", true, &lang("en")), "8 of 54 characters");
        assert_eq!(count_of_text(8, 54, "Figuren", true, &lang("de")), "8 von 54 Figuren");
        assert_eq!(count_of_text(8, 54, "personaggi", true, &lang("it")), "8 di 54 personaggi");
    }

    /// `""` must never inflect to `"n"` — symmetric with
    /// `agree_singular_casing`'s own empty-label arm.
    #[test]
    fn dative_plural_de_leaves_an_empty_noun_empty() {
        assert_eq!(dative_plural_de(""), "");
    }

    /// A class with no declared `plural { … }` block degrades to its singular
    /// label, and the German dative rule must NOT fire on that — "2 von 8
    /// Charactern" reads as a bug, where the degraded "Character" merely reads
    /// as untranslated (and is already flagged at boot by `MissingClassPlural`).
    #[test]
    fn count_of_text_de_skips_the_dative_on_an_undeclared_plural() {
        assert_eq!(count_of_text(2, 8, "Character", false, &lang("de")), "2 von 8 Character");
        // A DECLARED plural identical to its singular (Alice's `Kapitel`) is a
        // different case and does inflect — which is why the flag exists rather
        // than a `noun_other == noun_one` comparison.
        assert_eq!(count_of_text(2, 8, "Kapitel", true, &lang("de")), "2 von 8 Kapiteln");
    }

    /// The noun follows `m` (the unfiltered total), NOT `total` — a browse
    /// narrowed to exactly one row still reads "1 von 8’942 Aufführungen".
    #[test]
    fn count_of_text_noun_follows_the_unfiltered_total() {
        assert_eq!(
            count_of_text(54, 8942, "Aufführungen", true, &lang("de")),
            "54 von 8\u{2019}942 Aufführungen"
        );
        let one = count_of_text(1, 8942, "Aufführungen", true, &lang("de"));
        assert_eq!(one, "1 von 8\u{2019}942 Aufführungen");
        assert!(
            !one.ends_with("Aufführung"),
            "the noun must agree with m, not with total, got: {one}"
        );
    }

    // incoming_links_shown_text

    #[test]
    fn incoming_links_shown_text_all_langs() {
        assert_eq!(incoming_links_shown_text(2, 5, &lang("en")), "Showing 2 of 5");
        assert_eq!(incoming_links_shown_text(2, 5, &lang("de")), "Angezeigt: 2 von 5");
        assert_eq!(incoming_links_shown_text(2, 5, &lang("fr")), "Affichage : 2 sur 5");
        assert_eq!(incoming_links_shown_text(2, 5, &lang("it")), "Visualizzazione: 2 di 5");
        // Both counts through `group_thousands` — the de/it Swiss apostrophe.
        assert_eq!(
            incoming_links_shown_text(50, 1200, &lang("de")),
            "Angezeigt: 50 von 1\u{2019}200"
        );
    }

    // remove_filter_label

    #[test]
    fn remove_filter_label_interpolates_across_languages() {
        assert_eq!(remove_filter_label(&lang("en"), "Alice"), "Remove filter: Alice");
        assert_eq!(remove_filter_label(&lang("fr"), "Alice"), "Retirer le filtre : Alice");
    }

    // narrowed_to_zero_text

    #[test]
    fn narrowed_to_zero_text_query_only_across_langs() {
        // The query (with its quote marks) renders inside `<em>` — the
        // `.search-empty__msg em` rule styles it.
        assert_eq!(
            narrowed_to_zero_text("characters", Some("xyzzy"), false, &lang("en")).into_string(),
            "No characters match <em>&quot;xyzzy&quot;</em>."
        );
        assert_eq!(
            narrowed_to_zero_text("Figuren", Some("xyzzy"), false, &lang("de")).into_string(),
            "Keine Figuren stimmen mit <em>„xyzzy&quot;</em> überein."
        );
        assert_eq!(
            narrowed_to_zero_text("personnages", Some("xyzzy"), false, &lang("fr")).into_string(),
            "Aucun personnages ne correspond à <em>« xyzzy »</em>."
        );
        assert_eq!(
            narrowed_to_zero_text("personaggi", Some("xyzzy"), false, &lang("it")).into_string(),
            "Nessun personaggi corrisponde a <em>«xyzzy»</em>."
        );
    }

    #[test]
    fn narrowed_to_zero_text_query_and_filters_across_langs() {
        assert_eq!(
            narrowed_to_zero_text("characters", Some("rab"), true, &lang("en")).into_string(),
            "No characters match <em>&quot;rab&quot;</em> with these filters."
        );
        assert_eq!(
            narrowed_to_zero_text("Figuren", Some("rab"), true, &lang("de")).into_string(),
            "Keine Figuren stimmen mit <em>„rab&quot;</em> und diesen Filtern überein."
        );
        assert_eq!(
            narrowed_to_zero_text("personnages", Some("rab"), true, &lang("fr")).into_string(),
            "Aucun personnages ne correspond à <em>« rab »</em> avec ces filtres."
        );
        assert_eq!(
            narrowed_to_zero_text("personaggi", Some("rab"), true, &lang("it")).into_string(),
            "Nessun personaggi corrisponde a <em>«rab»</em> con questi filtri."
        );
    }

    /// Facet-only narrowing (no active
    /// query, no `<em>` to escape).
    #[test]
    fn narrowed_to_zero_text_filters_only_across_langs() {
        assert_eq!(
            narrowed_to_zero_text("characters", None, true, &lang("en")).into_string(),
            "No characters match these filters."
        );
        assert_eq!(
            narrowed_to_zero_text("Figuren", None, true, &lang("de")).into_string(),
            "Keine Figuren stimmen mit diesen Filtern überein."
        );
        assert_eq!(
            narrowed_to_zero_text("personnages", None, true, &lang("fr")).into_string(),
            "Aucun personnages ne correspond à ces filtres."
        );
        assert_eq!(
            narrowed_to_zero_text("personaggi", None, true, &lang("it")).into_string(),
            "Nessun personaggi corrisponde a questi filtri."
        );
    }

    /// A `"`/`<script>`-bearing query must still render escaped, nested
    /// safely inside the `<em>` — reflected-input safety (never `PreEscaped`)
    /// — for both branches that carry a query.
    #[test]
    fn narrowed_to_zero_text_escapes_hostile_query_in_every_branch_with_a_query() {
        let hostile = "<script>alert(1)</script>";
        let expected_escaped = "&quot;&lt;script&gt;alert(1)&lt;/script&gt;&quot;";

        let query_only = narrowed_to_zero_text("characters", Some(hostile), false, &lang("en")).into_string();
        assert!(
            !query_only.contains("<script>alert(1)</script>"),
            "a hostile query must never render as raw, unescaped markup, got: {query_only}"
        );
        assert!(
            query_only.contains(&format!("<em>{expected_escaped}</em>")),
            "the hostile query must render HTML-escaped inside the <em>, got: {query_only}"
        );

        let query_and_filters = narrowed_to_zero_text("characters", Some(hostile), true, &lang("en")).into_string();
        assert!(
            !query_and_filters.contains("<script>alert(1)</script>"),
            "a hostile query must never render as raw, unescaped markup, got: {query_and_filters}"
        );
        assert!(
            query_and_filters.contains(&format!("<em>{expected_escaped}</em>")),
            "the hostile query must render HTML-escaped inside the <em>, got: {query_and_filters}"
        );
    }

    // facet_empty_group_text

    #[test]
    fn facet_empty_group_text_inflects_group_except_german() {
        assert_eq!(
            facet_empty_group_text("themes", &lang("en")),
            "No themes are recorded for this class."
        );
        assert_eq!(
            facet_empty_group_text("thèmes", &lang("fr")),
            "Aucun thèmes n'est enregistré pour cette classe."
        );
        assert_eq!(
            facet_empty_group_text("temi", &lang("it")),
            "Nessun temi è registrato per questa classe."
        );
        // German uses the fallback template regardless of `group` — see the
        // fn's doc comment.
        assert_eq!(
            facet_empty_group_text("Themen", &lang("de")),
            "Für diese Klasse sind keine Werte erfasst."
        );
    }

    // compound_no_members_text

    #[test]
    fn compound_no_members_text_interpolates_singular_class_per_lang() {
        // DE/FR/IT anchor the article on a fixed noun (`Eintrag`/
        // `enregistrement`/`record`), so the sentence agrees with that
        // anchor noun's own gender, never with the interpolated class label.
        assert_eq!(
            compound_no_members_text("story", &lang("en")),
            "No parts are recorded for this story."
        );
        assert_eq!(
            compound_no_members_text("Geschichte", &lang("de")),
            "Für diesen Eintrag der Kategorie Geschichte sind keine Teile erfasst."
        );
        assert_eq!(
            compound_no_members_text("histoire", &lang("fr")),
            "Aucune partie n'est répertoriée pour cet enregistrement de la catégorie histoire."
        );
        assert_eq!(
            compound_no_members_text("storia", &lang("it")),
            "Nessuna parte è registrata per questo record della categoria storia."
        );
    }

    // index_count_text

    #[test]
    fn index_count_text_pluralizes_all_langs() {
        assert_eq!(index_count_text(0, &lang("en")), "0 projects");
        assert_eq!(index_count_text(1, &lang("en")), "1 project");
        assert_eq!(index_count_text(2, &lang("en")), "2 projects");

        assert_eq!(index_count_text(0, &lang("de")), "0 Projekte");
        assert_eq!(index_count_text(1, &lang("de")), "1 Projekt");
        assert_eq!(index_count_text(2, &lang("de")), "2 Projekte");

        assert_eq!(index_count_text(0, &lang("fr")), "0 projet");
        assert_eq!(index_count_text(1, &lang("fr")), "1 projet");
        assert_eq!(index_count_text(2, &lang("fr")), "2 projets");

        assert_eq!(index_count_text(0, &lang("it")), "0 progetti");
        assert_eq!(index_count_text(1, &lang("it")), "1 progetto");
        assert_eq!(index_count_text(2, &lang("it")), "2 progetti");
    }

    // not-found page parameterized helpers

    #[test]
    fn nf_home_text_interpolates_project_per_lang() {
        assert_eq!(
            nf_home_text("Alice in DaSCHland", &lang("en")),
            "Alice in DaSCHland — project home"
        );
        assert_eq!(
            nf_home_text("Alice in DaSCHland", &lang("de")),
            "Alice in DaSCHland — Projektstartseite"
        );
        assert_eq!(
            nf_home_text("Alice in DaSCHland", &lang("fr")),
            "Alice in DaSCHland — accueil du projet"
        );
        assert_eq!(
            nf_home_text("Alice in DaSCHland", &lang("it")),
            "Alice in DaSCHland — home del progetto"
        );
    }

    #[test]
    fn nf_id_title_text_interpolates_class_singular_per_lang() {
        assert_eq!(
            nf_id_title_text("character", &lang("en")),
            "There is no character at this address."
        );
        assert_eq!(
            nf_id_title_text("Figur", &lang("de")),
            // Genderless: the article agrees with `Eintrag`, not with the
            // interpolated class noun (not "kein Figur").
            "Unter dieser Adresse gibt es keinen Eintrag der Kategorie Figur."
        );
        assert_eq!(
            nf_id_title_text("personnage", &lang("fr")),
            "Il n'y a aucun enregistrement de la catégorie personnage à cette adresse."
        );
        assert_eq!(
            nf_id_title_text("personaggio", &lang("it")),
            "A questo indirizzo non esiste alcun record della categoria personaggio."
        );
        // Guards against "pas de Image" recurring: with the anchor noun
        // taking the article, a vowel-initial class label needs no
        // elision.
        assert!(nf_id_title_text("image", &lang("fr")).contains("de la catégorie image"));
    }

    #[test]
    fn nf_id_msg_text_interpolates_class_singular_per_lang() {
        assert!(nf_id_msg_text("character", &lang("en")).contains("character"));
        assert!(nf_id_msg_text("Figur", &lang("de")).contains("Figur"));
        assert!(nf_id_msg_text("personnage", &lang("fr")).contains("personnage"));
        assert!(nf_id_msg_text("personaggio", &lang("it")).contains("personaggio"));
        assert!(nf_id_msg_text("personnage", &lang("fr")).contains(
            "L'identifiant de cette adresse ne désigne aucun enregistrement de la \
             catégorie personnage dans ce projet."
        ));
    }

    #[test]
    fn nf_browse_all_text_interpolates_class_plural_per_lang() {
        assert_eq!(nf_browse_all_text("characters", &lang("en")), "Browse all characters");
        assert_eq!(nf_browse_all_text("Figuren", &lang("de")), "Alle Figuren durchsuchen");
        assert_eq!(nf_browse_all_text("personnages", &lang("fr")), "Parcourir tous les personnages");
        assert_eq!(nf_browse_all_text("personaggi", &lang("it")), "Sfoglia tutti i personaggi");
    }

    // New fixed ChromeMsg spot-checks

    #[test]
    fn list_empty_class_text_per_lang() {
        let msg = ChromeMsg::ListEmptyClass;
        assert_eq!(msg.text(&lang("en")), "This class contains no resources.");
        assert_eq!(msg.text(&lang("de")), "Diese Klasse enthält keine Ressourcen.");
        assert_eq!(msg.text(&lang("fr")), "Cette classe ne contient aucune ressource.");
        assert_eq!(msg.text(&lang("it")), "Questa classe non contiene risorse.");
    }

    #[test]
    fn value_unresolved_is_lowercase_in_every_lang() {
        let msg = ChromeMsg::ValueUnresolved;
        for code in ["en", "de", "fr", "it"] {
            let text = msg.text(&lang(code));
            assert_eq!(text, text.to_lowercase(), "must stay lower-case for {code}");
        }
    }

    #[test]
    fn nf_code_has_em_dash_per_lang() {
        let msg = ChromeMsg::NfCode;
        for code in ["en", "de", "fr", "it"] {
            assert!(
                msg.text(&lang(code)).contains('\u{2014}'),
                "nf.code must use an em dash for {code}"
            );
        }
    }

    // ChromeMsg::SearchLabel

    #[test]
    fn search_label_text_per_lang() {
        let msg = ChromeMsg::SearchLabel;
        assert_eq!(msg.text(&lang("en")), "Search");
        assert_eq!(msg.text(&lang("de")), "Suche");
        assert_eq!(msg.text(&lang("fr")), "Recherche");
        assert_eq!(msg.text(&lang("it")), "Cerca");
    }

    // share-view copy-link chrome

    #[test]
    fn copy_link_strings_are_non_empty_in_every_lang() {
        for msg in [
            ChromeMsg::CopyLinkLabel,
            ChromeMsg::CopyLinkCopied,
            ChromeMsg::CopyLinkLive,
            ChromeMsg::CopyLinkLongPrefix,
        ] {
            for code in BUILTIN_LANGUAGES {
                assert!(!msg.text(&lang(code)).is_empty(), "{msg:?} must not be empty for {code}");
            }
        }
    }

    #[test]
    fn copy_link_label_and_copied_text_per_lang() {
        assert_eq!(ChromeMsg::CopyLinkLabel.text(&lang("en")), "Copy link");
        assert_eq!(ChromeMsg::CopyLinkLabel.text(&lang("de")), "Link kopieren");
        assert_eq!(ChromeMsg::CopyLinkLabel.text(&lang("fr")), "Copier le lien");
        assert_eq!(ChromeMsg::CopyLinkLabel.text(&lang("it")), "Copia il link");

        assert_eq!(ChromeMsg::CopyLinkCopied.text(&lang("en")), "Copied");
        assert_eq!(ChromeMsg::CopyLinkCopied.text(&lang("de")), "Kopiert");
        assert_eq!(ChromeMsg::CopyLinkCopied.text(&lang("fr")), "Copié");
        assert_eq!(ChromeMsg::CopyLinkCopied.text(&lang("it")), "Copiato");
    }

    #[test]
    fn copy_link_live_status_matches_design_doc_en_de() {
        assert_eq!(
            ChromeMsg::CopyLinkLive.text(&lang("en")),
            "Link to this view copied to the clipboard"
        );
        assert_eq!(
            ChromeMsg::CopyLinkLive.text(&lang("de")),
            "Link zu dieser Ansicht in die Zwischenablage kopiert"
        );
    }

    #[test]
    fn copy_link_long_prefix_matches_design_doc_en_de() {
        assert_eq!(ChromeMsg::CopyLinkLongPrefix.text(&lang("en")), "Copy link to this view");
        assert_eq!(
            ChromeMsg::CopyLinkLongPrefix.text(&lang("de")),
            "Link zu dieser Ansicht kopieren"
        );
    }

    // the phone crumb row's short strings

    #[test]
    fn phone_crumb_strings_cover_every_builtin_language() {
        let table = [
            (ChromeMsg::CopyLinkShort, ["Copy", "Link", "Lien", "Link"]),
            (ChromeMsg::DataSummaryShort, ["Dataset", "Datensatz", "Données", "Dataset"]),
            (ChromeMsg::CrumbBackPrefix, ["Back:", "Zurück:", "Retour :", "Indietro:"]),
        ];
        for (msg, expected) in table {
            for (code, want) in BUILTIN_LANGUAGES.iter().zip(expected) {
                assert_eq!(msg.text(&lang(code)), want, "{msg:?} {code}");
            }
        }
    }

    /// WCAG 2.5.3: a control's visible label is part of its accessible name.
    #[test]
    fn each_short_crumb_label_is_part_of_its_controls_name() {
        for code in BUILTIN_LANGUAGES {
            let l = lang(code);
            let share_name = ChromeMsg::CopyLinkLongPrefix.text(&l).to_lowercase();
            let short = ChromeMsg::CopyLinkShort.text(&l).to_lowercase();
            assert!(share_name.contains(&short), "{code}: \"{short}\" is not in \"{share_name}\"");
            let data_name = data_summary_name(&["Book x".to_owned()], &l).to_lowercase();
            let short = ChromeMsg::DataSummaryShort.text(&l).to_lowercase();
            assert!(data_name.contains(&short), "{code}: \"{short}\" is not in \"{data_name}\"");
        }
    }

    /// The phone crumb row is one line at 320px: the two short labels and the
    /// back link's prefix each keep to a character budget.
    #[test]
    fn short_crumb_strings_keep_their_length_budget() {
        let budgets = [
            (ChromeMsg::CopyLinkShort, 10),
            (ChromeMsg::DataSummaryShort, 10),
            (ChromeMsg::CrumbBackPrefix, 9),
        ];
        for (msg, max) in budgets {
            for code in BUILTIN_LANGUAGES {
                let n = msg.text(&lang(code)).chars().count();
                assert!(n <= max, "{msg:?} {code}: {n} characters, budget {max}");
            }
        }
    }

    // browse controls — ordering control + pager strings

    #[test]
    fn order_label_text_per_lang() {
        let msg = ChromeMsg::OrderLabel;
        assert_eq!(msg.text(&lang("en")), "Order");
        assert_eq!(msg.text(&lang("de")), "Reihenfolge");
        assert_eq!(msg.text(&lang("fr")), "Ordre");
        assert_eq!(msg.text(&lang("it")), "Ordine");
    }

    #[test]
    fn order_curated_text_per_lang() {
        let msg = ChromeMsg::OrderCurated;
        assert_eq!(msg.text(&lang("en")), "As curated");
        assert_eq!(msg.text(&lang("de")), "Wie kuratiert");
        assert_eq!(msg.text(&lang("fr")), "Ordre éditorial");
        assert_eq!(msg.text(&lang("it")), "Ordine curato");
    }

    #[test]
    fn order_name_asc_text_per_lang() {
        let msg = ChromeMsg::OrderNameAsc;
        assert_eq!(msg.text(&lang("en")), "Name A–Z");
        assert_eq!(msg.text(&lang("de")), "Name A–Z");
        assert_eq!(msg.text(&lang("fr")), "Nom A–Z");
        assert_eq!(msg.text(&lang("it")), "Nome A–Z");
    }

    #[test]
    fn order_name_desc_text_per_lang() {
        let msg = ChromeMsg::OrderNameDesc;
        assert_eq!(msg.text(&lang("en")), "Name Z–A");
        assert_eq!(msg.text(&lang("de")), "Name Z–A");
        assert_eq!(msg.text(&lang("fr")), "Nom Z–A");
        assert_eq!(msg.text(&lang("it")), "Nome Z–A");
    }

    #[test]
    fn order_id_text_per_lang() {
        let msg = ChromeMsg::OrderId;
        assert_eq!(msg.text(&lang("en")), "Identifier");
        assert_eq!(msg.text(&lang("de")), "Kennung");
        assert_eq!(msg.text(&lang("fr")), "Identifiant");
        assert_eq!(msg.text(&lang("it")), "Identificatore");
    }

    #[test]
    fn pager_prev_text_per_lang() {
        let msg = ChromeMsg::PagerPrev;
        assert_eq!(msg.text(&lang("en")), "Previous");
        assert_eq!(msg.text(&lang("de")), "Zurück");
        assert_eq!(msg.text(&lang("fr")), "Précédent");
        assert_eq!(msg.text(&lang("it")), "Precedente");
    }

    #[test]
    fn pager_next_text_per_lang() {
        let msg = ChromeMsg::PagerNext;
        assert_eq!(msg.text(&lang("en")), "Next");
        assert_eq!(msg.text(&lang("de")), "Weiter");
        assert_eq!(msg.text(&lang("fr")), "Suivant");
        assert_eq!(msg.text(&lang("it")), "Successivo");
    }

    #[test]
    fn pager_page_of_text_per_lang() {
        assert_eq!(pager_page_of(2, 3, &lang("en")), "Page 2 of 3");
        assert_eq!(pager_page_of(2, 3, &lang("de")), "Seite 2 von 3");
        assert_eq!(pager_page_of(2, 3, &lang("fr")), "Page 2 sur 3");
        assert_eq!(pager_page_of(2, 3, &lang("it")), "Pagina 2 di 3");
    }

    #[test]
    fn pager_page_n_text_per_lang() {
        assert_eq!(pager_page_n(2, &lang("en")), "Page 2");
        assert_eq!(pager_page_n(2, &lang("de")), "Seite 2");
        assert_eq!(pager_page_n(2, &lang("fr")), "Page 2");
        assert_eq!(pager_page_n(2, &lang("it")), "Pagina 2");
    }

    #[test]
    fn browse_controls_new_variants_fall_back_to_english_for_unknown_code() {
        for msg in [
            ChromeMsg::OrderLabel,
            ChromeMsg::OrderCurated,
            ChromeMsg::OrderNameAsc,
            ChromeMsg::OrderNameDesc,
            ChromeMsg::OrderId,
            ChromeMsg::PagerPrev,
            ChromeMsg::PagerNext,
        ] {
            assert_eq!(msg.text(&lang("zz")), msg.text(&lang("en")));
        }
    }
}

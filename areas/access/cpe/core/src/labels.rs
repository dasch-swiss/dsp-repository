//! Label resolution for the request language: declared chrome strings, the
//! site title, nav labels, property and class labels (singular and plural),
//! list glosses and ordering rows.
//!
//! A domain leaf in `cpe-core`: it reads the validated IR (`ir`), a resolved
//! `Lang` and `util`'s humanizer, never a renderer, so the validator and the
//! components can both call it.
//!
//! These are the project-declared strings (`chrome {}`, `labels {}`, nav
//! labels, glosses, order rows); the engine's own built-in strings are
//! the `ChromeMsg` catalog in `cpe_components::chrome`. A new resolver over the
//! IR's label types goes here, not in a component.

use crate::ir::{ChromeString, ClassLabel, PropertyLabel, SiteConfig};
use crate::lang::Lang;

/// Look up a chrome string by `id` and return the translation for `lang`.
///
/// Returns `Some(&str)` when the `id` exists and has a translation for the
/// given language; `None` when the id is not declared or the language has no
/// translation (the caller falls back to the literal label in that case).
pub fn chrome_lookup<'a>(site: &'a SiteConfig, id: &str, lang: &Lang) -> Option<&'a str> {
    chrome_text(&site.chrome, id, lang)
}

/// [`chrome_lookup`] over the declared chrome strings alone, for a renderer
/// that holds them without the rest of the site.
pub fn chrome_text<'a>(chrome: &'a [ChromeString], id: &str, lang: &Lang) -> Option<&'a str> {
    let chrome = chrome.iter().find(|cs| cs.id == id)?;
    chrome
        .translations
        .iter()
        .find(|(l, _)| l == lang.as_str())
        .map(|(_, text)| text.as_str())
}

/// The project's `site.title`, resolved chrome-id-or-literal for `lang` —
/// the `site.description` rule (`Project::description`): a declared
/// chrome-string id resolves to its translation, anything else renders as the
/// literal. Every chrome surface that names the project (`<title>`, brand
/// lockup, breadcrumb project crumb, the 404s) reads it through
/// here, never `site.title` directly. A declared id is guaranteed a
/// translation for every declared language by the chrome-completeness
/// hard-fail; a literal title that happens to equal a declared chrome id is
/// translated (the same accepted risk as `description`).
pub fn site_title<'a>(site: &'a SiteConfig, lang: &Lang) -> &'a str {
    chrome_lookup(site, &site.title, lang).unwrap_or(&site.title)
}

/// Resolve the A12.4 `order-name-label`/`order-name-desc-label` overrides and
/// the `order-default-label` override, returned in that order —
/// chrome-id attribute values, id-only, never literal text — through
/// `chrome_lookup`. `render_list_component` (a fresh full-page/list render)
/// calls this same helper, so the full-page and fragment paths are provably
/// identical (Incunabula Slice 8 decision 11b).
///
/// Exposed as `pub` (re-exported by `cpe-engine`) so the bin's three
/// `_facets`/`_sort`/`_search` fragment handlers — which read the raw
/// `Component::ResourceList` fields directly rather than going through
/// `render_component` — can resolve the same overrides for a swapped-in list,
/// without `cpe-engine` having to expose `chrome_lookup` itself. An absent
/// attribute, or an id that fails to resolve, both yield `None` — never an
/// empty label — leaving `order_label_for_key`'s `ChromeMsg` default in
/// place.
pub fn resolve_order_name_labels<'a>(
    site: &'a SiteConfig,
    order_name_label: Option<&str>,
    order_name_desc_label: Option<&str>,
    order_default_label: Option<&str>,
    lang: &Lang,
) -> (Option<&'a str>, Option<&'a str>, Option<&'a str>) {
    (
        order_name_label.and_then(|id| chrome_lookup(site, id, lang)),
        order_name_desc_label.and_then(|id| chrome_lookup(site, id, lang)),
        order_default_label.and_then(|id| chrome_lookup(site, id, lang)),
    )
}

/// A `resource-list` row gloss with its unit chrome ids already resolved for
/// one request language — what `ResourceListCfg` renders from.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedGloss<'a> {
    /// [`crate::ir::Gloss::Property`]: the property name.
    Property(&'a str),
    /// [`crate::ir::Gloss::Composed`], parts in config order.
    Composed { parts: Vec<ResolvedGlossPart<'a>>, teaser: Option<&'a str> },
}

/// One resolved composed-gloss part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedGlossPart<'a> {
    pub property: &'a str,
    /// The unit template for the request language (`"{n} Seiten"`). `None`
    /// when the part has no `unit=`, or when the id has no translation for
    /// that language — the value then renders as bare digits, never as the
    /// raw id.
    pub unit: Option<&'a str>,
}

/// Resolve a list's gloss for `lang`. `render_list_component` and the bin's
/// `_facets`/`_sort`/`_search` fragment handlers all call this one helper, so
/// a full-page render and a swapped-in list render the same gloss.
pub fn resolve_gloss<'a>(site: &'a SiteConfig, gloss: &'a crate::ir::Gloss, lang: &Lang) -> ResolvedGloss<'a> {
    match gloss {
        crate::ir::Gloss::Property(property) => ResolvedGloss::Property(property),
        crate::ir::Gloss::Composed { parts, teaser } => ResolvedGloss::Composed {
            parts: resolve_parts(site, parts, lang),
            teaser: teaser.as_deref(),
        },
    }
}

/// Resolve composed parts' `unit=` chrome ids for `lang` — shared by a
/// list's composed gloss and a record's imprint.
pub fn resolve_parts<'a>(
    site: &'a SiteConfig,
    parts: &'a [crate::ir::GlossPart],
    lang: &Lang,
) -> Vec<ResolvedGlossPart<'a>> {
    parts
        .iter()
        .map(|part| ResolvedGlossPart {
            property: &part.property,
            unit: part.unit.as_deref().and_then(|id| chrome_lookup(site, id, lang)),
        })
        .collect()
}

/// Resolve a list's declared `order-by` options (Incunabula Slice 8, G9)
/// into the `(key, label)` rows the ordering panel renders, in config order:
/// each option's `label=` chrome-string id runs through `chrome_lookup` for
/// `lang`, falling back to the option's **key** when the id has no
/// translation (never an empty label, never a panic — boot validation's
/// `UnknownOrderLabel` already rejects an undeclared id, so the fallback is
/// a pure safety net).
///
/// The one resolver for declared rows, `pub` for the same reason as
/// [`resolve_order_name_labels`]: `render_list_component` and the bin's
/// `_facets`/`_sort`/`_search` fragment handlers all call it, so a full-page
/// render and a swapped-in list render the same panel.
pub fn resolve_order_options<'a>(
    site: &'a SiteConfig,
    options: &'a [crate::ir::OrderOption],
    lang: &Lang,
) -> Vec<OrderRow<'a>> {
    options
        .iter()
        .map(|o| OrderRow {
            key: o.key.as_str(),
            label: chrome_lookup(site, &o.label, lang).unwrap_or(o.key.as_str()),
        })
        .collect()
}

/// Resolve a property's display label: the declared translation for `lang` if
/// a `labels { property "…" { … } }` entry exists and has one, else the
/// humanized property name (the pre-existing fallback — unchanged behaviour
/// for projects that declare no labels).
///
/// Called by `ProjectReads::facet_options` (facet-group headings) and
/// `resource_detail::render_property_block` (`div.prop__label`).
pub fn resolve_property_label(labels: &[PropertyLabel], property: &str, lang: &Lang) -> String {
    labels
        .iter()
        .find(|l| l.property == property)
        .and_then(|l| l.translations.iter().find(|(lg, _)| lg == lang.as_str()))
        .map(|(_, t)| t.clone())
        .unwrap_or_else(|| crate::util::humanize_property_name(property))
}

/// Same contract as `resolve_property_label`, for class labels.
///
/// Called by `resource_detail::render_resource_detail` (the header eyebrow)
/// and `resource_list::render_surf_head`, among others.
pub fn resolve_class_label(labels: &[ClassLabel], class: &str, lang: &Lang) -> String {
    labels
        .iter()
        .find(|l| l.class == class)
        .and_then(|l| l.translations.iter().find(|(lg, _)| lg == lang.as_str()))
        .map(|(_, t)| t.clone())
        .unwrap_or_else(|| crate::util::humanize_property_name(class))
}

/// Resolve a class's display-plural noun (search-bar plan step 5b, #178
/// reconciliation — see `ir::ClassLabel`'s doc comment). This is what feeds
/// the no-match `{noun}` slot in the search-bar template
/// (`chrome::search_no_match_text`), e.g. `"characters"` (EN) / `"Figuren"`
/// (DE) for a `?q=` search on `/characters` that matched nothing.
///
/// Resolution order:
/// 1. Declared `plural` for the exact request `lang`.
/// 2. Declared `plural` for "default/any lang" — this fn's signature deliberately mirrors
///    `resolve_class_label`'s (no `default_lang` parameter is threaded in), so there is no
///    default-lang value available to prefer specifically; "any" therefore collapses to the FIRST
///    entry in the `plural` Vec, in declared order, when the exact- lang lookup in rung 1 misses.
/// 3. Singular `resolve_class_label(labels, class, lang)` — which ALREADY falls back to
///    `humanize_property_name` on a total miss, so that fallback is inherited for free here; it is
///    NOT re-implemented in this fn.
///
/// Never panics; always returns a usable `String`.
///
/// The declared plural string is used **VERBATIM** — this fn does **NO**
/// casing/pluralization of its own. The project author is responsible for
/// writing the correct case per language (e.g. lowercase `"characters"` for
/// EN vs. capitalized `"Figuren"` for DE per design-doc §4) — both are
/// stored as literal strings in `ClassLabel.plural` and returned as-is.
pub fn resolve_class_plural(labels: &[ClassLabel], class: &str, lang: &Lang) -> String {
    let hit = match labels.iter().find(|l| l.class == class) {
        Some(l) => l,
        None => return resolve_class_label(labels, class, lang),
    };
    if let Some((_, t)) = hit.plural.iter().find(|(lg, _)| lg == lang.as_str()) {
        return t.clone();
    }
    if let Some((_, t)) = hit.plural.first() {
        return t.clone();
    }
    resolve_class_label(labels, class, lang)
}

/// Whether `class` carries a `plural { … }` block at all — i.e. whether
/// [`resolve_class_plural`]'s return value is a real declared plural or its
/// degraded fallback to the singular label / humanized class name.
///
/// Class-level, not per-language, on purpose: `resolve_class_plural` falls back
/// to the block's FIRST entry when the requested language is missing, so a
/// declared block is a declared plural in every language.
///
/// The one consumer is `chrome::count_of_text`'s German dative rule, which must
/// not inflect a degraded noun ("2 von 8 Charactern"); see its doc comment.
/// `MissingClassPlural` (Warning) already reports the degraded case at boot.
pub fn class_plural_declared(labels: &[ClassLabel], class: &str) -> bool {
    labels.iter().find(|l| l.class == class).is_some_and(|l| !l.plural.is_empty())
}

/// Resolve a class's display-plural noun, capitalized for title/breadcrumb-copy
/// contexts — the breadcrumb class segment (both the browse crumb's own
/// current segment and the resource-detail crumb's linked class segment) reads
/// as `"Characters"`, not the lowercase sentence-copy `"characters"`
/// [`resolve_class_plural`] returns for its OTHER caller
/// (`chrome::search_no_match_text`'s no-match sentence, which stays
/// deliberately lowercase — see that caller and `config.kdl`'s `plural`
/// doc comment; do NOT change `resolve_class_plural` itself or its no-match
/// caller).
///
/// Fallback chain: capitalized declared plural (rung 1/2 of
/// `resolve_class_plural`) → capitalized singular `resolve_class_label`
/// (rung 3, i.e. `resolve_class_plural`'s own fallback) → humanized. Because
/// `resolve_class_plural` already folds rung 3 into its own return value,
/// this fn is simply `resolve_class_plural` + [`crate::util::capitalize_first`]
/// — capitalizing the SAME resolved string regardless of which rung produced
/// it, never a second independent fallback chain.
///
/// Supersedes an earlier choice (issues.md) to reuse the singular
/// `resolve_class_label` for both the browse crumb's current segment and the
/// resource-detail crumb's class segment "to match" each other — the owner's
/// expected copy is the capitalized PLURAL noun in both places (e.g.
/// `"Characters"`, not `"Character"`); the two surfaces still always agree
/// with each other, just on the plural form instead of the singular one.
pub fn resolve_class_plural_capitalized(labels: &[ClassLabel], class: &str, lang: &Lang) -> String {
    crate::util::capitalize_first(&resolve_class_plural(labels, class, lang))
}

/// Resolve a nav item's display label.
///
/// If the item's `label` matches a declared chrome-string id and the chrome
/// string has a translation for `lang`, the translation is returned.
/// Otherwise the literal `label` value is used as-is.
///
/// Per `CONFIG_LANGUAGE.md`: nav `label` is "chrome string id OR literal".
pub fn nav_label<'a>(site: &'a SiteConfig, item: &'a crate::ir::NavItem, lang: &Lang) -> &'a str {
    chrome_lookup(site, &item.label, lang).unwrap_or(&item.label)
}

/// One declared ordering option, resolved for rendering (Incunabula Slice 8,
/// G9): the `?sort=` wire `key` and the visible `label` — already run through
/// the chrome-string lookup by [`resolve_order_options`], falling back
/// to the key itself (never an empty label) when the id cannot be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderRow<'a> {
    /// The `?sort=` wire token (the option's `key=`).
    pub key: &'a str,
    /// The resolved row label.
    pub label: &'a str,
}

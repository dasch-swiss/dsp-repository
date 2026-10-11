//! KDL config parsing: turn a `config.kdl` source string into a typed
//! [`RawConfig`] IR. Parse only — KDL syntax errors are the *only* failure
//! mode. Semantic validation (required fields, uniqueness, component
//! typing, reference resolution, …) is Phase 2's job (`validate` module).

use crate::diagnostic::Span;
use crate::ir::Value;

/// A parsed `config.kdl`. Site-level metadata is flattened onto this
/// struct (no `Site` type — see CONTEXT.md on "site"); pages carry
/// generic component bodies. Presence/shape is NOT validated here.
///
/// `languages` and `theme` entries are captured with their raw [`Value`]
/// and source [`Span`] so the validator can emit `InvalidFieldType` when
/// an entry is not a string, rather than silently dropping it.
///
/// `unknown_top_level` preserves the names and spans of top-level nodes
/// that are none of `site`, `labels`, `page` or `remodel` (or a repeat of
/// `site`, `labels` or `remodel`), so the validator can emit
/// `UnexpectedTopLevelNode`.
#[derive(Debug)]
pub(crate) struct RawConfig {
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) cover_image: Option<String>,
    pub(crate) image_credit: Option<String>,
    pub(crate) eyebrow: Option<String>,
    /// Each entry is the raw `(Value, Span)` — type check happens in validate.
    pub(crate) languages: Vec<(Value, Span)>,
    pub(crate) default_language: Option<String>,
    pub(crate) nav: Vec<NavItem>,
    pub(crate) chrome: Vec<ChromeString>,
    /// Raw `labels { property "…" { … } class "…" { … } }` entries — a
    /// top-level block, a sibling of `site`/`page`, NOT a `site` child.
    pub(crate) labels: Vec<RawLabel>,
    /// Each theme var value is the raw `(Value, Span)` — type check in validate.
    pub(crate) theme: Vec<ThemeVar>,
    /// Raw first argument of `site { date-precision-word … }` — type check
    /// (KDL boolean) happens in validate. `None` when the child is absent.
    pub(crate) date_precision_word: Option<(Value, Span)>,
    /// Raw first argument of `site { footer-attribution … }` — type check
    /// happens in validate. `None` when the child is absent.
    pub(crate) footer_attribution: Option<(Value, Span)>,
    /// Raw first argument of `site { narrative-companion … }` — type and
    /// vocabulary checks happen in validate. `None` when the child is absent.
    pub(crate) narrative_companion: Option<(Value, Span)>,
    /// Raw first argument of `site { narrative-side … }`, as above.
    pub(crate) narrative_side: Option<(Value, Span)>,
    /// Raw first argument of `site { text-align … }`, as above.
    pub(crate) text_align: Option<(Value, Span)>,
    /// Span of the `site { not-found { … } }` node, `None` when absent — the
    /// location `TooManyNotFoundBrowseLinks` reports at.
    pub(crate) not_found_span: Option<Span>,
    /// The `browse` children of `site { not-found { … } }`, captured
    /// structurally (like page components) so validate can read `route=` /
    /// `label=` with the reporting attribute readers. Other children are
    /// ignored, like unknown `site {}` children.
    pub(crate) not_found_browse: Vec<Component>,
    pub(crate) pages: Vec<Page>,
    /// The first top-level `remodel { … }` block, captured structurally (like
    /// page components) so `validate::remodel` reads it with the reporting
    /// attribute readers. `None` when the config has no block.
    pub(crate) remodel: Option<Component>,
    /// Names + spans of top-level nodes that are not the first `site`, `labels`
    /// or `remodel`, nor a `page`.
    pub(crate) unknown_top_level: Vec<(String, Span)>,
}

#[derive(Debug)]
pub(crate) struct NavItem {
    pub(crate) label: Option<String>,
    pub(crate) route: Option<String>,
}

/// One `chrome` string: an id and its per-language translations.
#[derive(Debug)]
pub(crate) struct ChromeString {
    pub(crate) id: Option<String>,
    pub(crate) translations: Vec<(String, String)>, // (lang, text), order preserved
}

/// Which kind of thing a `RawLabel` names — `property` or `class`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawLabelTarget {
    Property,
    Class,
}

/// One raw `labels` entry: a `property "…" { … }` or `class "…" { … }`
/// child of the top-level `labels { }` block, with its key, per-language
/// (singular) translations, and an optional per-language `plural`
/// (search-bar plan step 5b, #178 reconciliation — see `ir::ClassLabel`).
#[derive(Debug)]
pub(crate) struct RawLabel {
    pub(crate) target: RawLabelTarget,
    pub(crate) key: Option<String>,
    pub(crate) translations: Vec<(String, String)>, // (lang, text), order preserved
    /// Parsed from an optional nested `plural { <lang> "…" … }` child.
    /// Empty when no `plural` child is present (today's shape, #178) — this
    /// is not an error, just "undeclared". Structurally parsed for both
    /// `property` and `class` targets (parsing is symmetric and simplest to
    /// implement once), but only `class` entries' `plural` is actually
    /// consumed downstream (validate.rs's lowering + completeness check); a
    /// `property` entry's `plural` child parses but is silently unused
    /// (out of scope — properties have no plural concept).
    pub(crate) plural: Vec<(String, String)>, // (lang, text), order preserved
}

/// One raw `theme` knob entry: a knob key (`accent` / `background`) and its
/// raw value `(Value, Span)`. The validator type-checks each key and value,
/// lowering the full set into a typed `ThemeKnob` and rejecting unknown keys
/// or values with `UnknownThemeValue`.
#[derive(Debug)]
pub(crate) struct ThemeVar {
    pub(crate) name: String,
    pub(crate) value: Option<(Value, Span)>,
}

#[derive(Debug)]
pub(crate) struct Page {
    pub(crate) id: Option<String>,
    pub(crate) route: Option<String>,
    pub(crate) body: Vec<Component>,
}

/// A generic component node inside a page body. Per-component typing
/// (resource-list, narrative, …) is Phase 2's job; here every component
/// is captured structurally so Phase 2 can type and validate it.
#[derive(Debug)]
pub(crate) struct Component {
    pub(crate) name: String,
    pub(crate) args: Vec<Value>,            // positional args, in order
    pub(crate) attrs: Vec<(String, Value)>, // named attributes, in order
    pub(crate) children: Vec<Component>,    // nested nodes (where/sort/facet/…)
    pub(crate) span: Span,                  // source location, for Phase 2 errors
}

/// What can go wrong while parsing `config.kdl`. A single variant by
/// design: the parser has exactly one failure mode (KDL syntax error).
/// Phase 2 adds validation variants in a sibling `validate` module.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to parse config.kdl")]
    Parse(#[source] kdl::KdlError),
}

/// Parse `source` as a `config.kdl` document and walk it into a typed
/// [`RawConfig`]. The only failure mode is a KDL syntax error; a
/// structurally incomplete-but-syntactically-valid config parses fine,
/// yielding `None`/empty fields for the Phase 2 validator to check.
///
/// Unknown top-level nodes are preserved in `unknown_top_level` with their
/// spans so the validator can emit `UnexpectedTopLevelNode` for each.
pub(crate) fn parse(source: &str) -> Result<RawConfig, ConfigError> {
    let doc: kdl::KdlDocument = source.parse().map_err(ConfigError::Parse)?;

    let mut config = RawConfig {
        title: None,
        description: None,
        cover_image: None,
        image_credit: None,
        eyebrow: None,
        languages: Vec::new(),
        default_language: None,
        nav: Vec::new(),
        chrome: Vec::new(),
        labels: Vec::new(),
        theme: Vec::new(),
        pages: Vec::new(),
        remodel: None,
        unknown_top_level: Vec::new(),
        date_precision_word: None,
        footer_attribution: None,
        narrative_companion: None,
        narrative_side: None,
        text_align: None,
        not_found_span: None,
        not_found_browse: Vec::new(),
    };

    let mut site_found = false;
    let mut labels_found = false;
    let mut remodel_found = false;

    for node in doc.nodes() {
        match node.name().value() {
            "site" if !site_found => {
                site_found = true;
                walk_site(node, &mut config);
            }
            "labels" if !labels_found => {
                labels_found = true;
                config.labels = walk_labels(node);
            }
            "remodel" if !remodel_found => {
                remodel_found = true;
                config.remodel = Some(walk_component(node));
            }
            "page" => {
                config.pages.push(walk_page(node));
            }
            // Unknown top-level nodes are preserved for validation (Phase 2).
            name => {
                config.unknown_top_level.push((name.to_owned(), span_of(node)));
            }
        }
    }

    Ok(config)
}

/// Populate flattened site fields from the `site` block's children.
fn walk_site(site: &kdl::KdlNode, config: &mut RawConfig) {
    let children = match site.children() {
        Some(c) => c,
        None => return,
    };

    for node in children.nodes() {
        match node.name().value() {
            "title" => {
                config.title = first_string_arg(node);
            }
            "description" => {
                config.description = first_string_arg(node);
            }
            "cover-image" => {
                config.cover_image = first_string_arg(node);
            }
            "image-credit" => {
                config.image_credit = first_string_arg(node);
            }
            "eyebrow" => {
                config.eyebrow = first_string_arg(node);
            }
            "default-language" => {
                config.default_language = first_string_arg(node);
            }
            "languages" => {
                // Capture every positional entry as raw (Value, Span) so the
                // validator can distinguish absent from wrong-type entries.
                config.languages = node
                    .entries()
                    .iter()
                    .filter(|e| e.name().is_none())
                    .map(|e| {
                        let span = entry_span(e);
                        (kdl_value_to_value(e.value()), span)
                    })
                    .collect();
            }
            "nav" => {
                config.nav = walk_nav(node);
            }
            "chrome" => {
                config.chrome = walk_chrome(node);
            }
            "theme" => {
                config.theme = walk_theme(node);
            }
            "date-precision-word" => {
                // Raw capture of the first positional argument so the
                // validator can reject a non-boolean with `InvalidFieldType`
                // (a bare `date-precision-word` with no argument lowers as
                // absent).
                config.date_precision_word = first_raw_arg(node);
            }
            // G11 copy override (Incunabula Slice 8, decision 9): raw
            // capture so validate can reject a non-string argument.
            "footer-attribution" => {
                config.footer_attribution = first_raw_arg(node);
            }
            "narrative-companion" => {
                config.narrative_companion = first_raw_arg(node);
            }
            "narrative-side" => {
                config.narrative_side = first_raw_arg(node);
            }
            "text-align" => {
                config.text_align = first_raw_arg(node);
            }
            "not-found" => {
                config.not_found_span = Some(span_of(node));
                config.not_found_browse = match node.children() {
                    Some(c) => c
                        .nodes()
                        .iter()
                        .filter(|n| n.name().value() == "browse")
                        .map(walk_component)
                        .collect(),
                    None => Vec::new(),
                };
            }
            // Unknown site children are ignored — Phase 2 validation.
            _ => {}
        }
    }
}

fn walk_nav(nav: &kdl::KdlNode) -> Vec<NavItem> {
    let children = match nav.children() {
        Some(c) => c,
        None => return Vec::new(),
    };

    children
        .nodes()
        .iter()
        .filter(|n| n.name().value() == "item")
        .map(|n| NavItem {
            label: string_attr(n, "label"),
            route: string_attr(n, "route"),
        })
        .collect()
}

fn walk_chrome(chrome: &kdl::KdlNode) -> Vec<ChromeString> {
    let children = match chrome.children() {
        Some(c) => c,
        None => return Vec::new(),
    };

    children
        .nodes()
        .iter()
        .filter(|n| n.name().value() == "string")
        .map(|n| {
            let id = first_string_arg(n);
            let translations = match n.children() {
                Some(lang_block) => lang_block
                    .nodes()
                    .iter()
                    .filter_map(|lang_node| {
                        let lang = lang_node.name().value().to_owned();
                        let text = first_string_arg(lang_node)?;
                        Some((lang, text))
                    })
                    .collect(),
                None => Vec::new(),
            };
            ChromeString { id, translations }
        })
        .collect()
}

/// Walk a top-level `labels { property "…" { … } class "…" { … } }` block.
/// Each child node is `property` or `class`; any other child name is
/// silently ignored here (Phase 2 validation is not this layer's job).
///
/// Each `property`/`class` node's own children are per-language `<lang>
/// "…"` singular-translation nodes, EXCEPT a child node literally named
/// `"plural"` — that one nests its OWN `<lang> "…"` children (a separate
/// per-language plural translation set) and is deliberately excluded from
/// the singular `translations` Vec (search-bar plan step 5b, #178
/// reconciliation). Back-compat: a `property`/`class` entry with no
/// `plural` child parses exactly as before this step — `plural` is simply
/// `Vec::new()`.
fn walk_labels(labels: &kdl::KdlNode) -> Vec<RawLabel> {
    let children = match labels.children() {
        Some(c) => c,
        None => return Vec::new(),
    };

    children
        .nodes()
        .iter()
        .filter_map(|n| {
            let target = match n.name().value() {
                "property" => RawLabelTarget::Property,
                "class" => RawLabelTarget::Class,
                _ => return None,
            };
            let key = first_string_arg(n);
            let mut translations: Vec<(String, String)> = Vec::new();
            let mut plural: Vec<(String, String)> = Vec::new();
            if let Some(lang_block) = n.children() {
                for lang_node in lang_block.nodes() {
                    if lang_node.name().value() == "plural" {
                        // Nested `plural { <lang> "…" … }` child: recurse into
                        // ITS children the same way, collecting into `plural`
                        // instead of `translations`. Explicit `continue` so
                        // the exclusion is deliberate, not an accidental
                        // side effect of `first_string_arg` returning `None`
                        // for a node with no direct positional string arg.
                        if let Some(plural_block) = lang_node.children() {
                            for grandchild in plural_block.nodes() {
                                let lang = grandchild.name().value().to_owned();
                                if let Some(text) = first_string_arg(grandchild) {
                                    plural.push((lang, text));
                                }
                            }
                        }
                        continue;
                    }
                    let lang = lang_node.name().value().to_owned();
                    if let Some(text) = first_string_arg(lang_node) {
                        translations.push((lang, text));
                    }
                }
            }
            Some(RawLabel { target, key, translations, plural })
        })
        .collect()
}

fn walk_theme(theme: &kdl::KdlNode) -> Vec<ThemeVar> {
    let children = match theme.children() {
        Some(c) => c,
        None => return Vec::new(),
    };

    children
        .nodes()
        .iter()
        .map(|n| {
            // Capture value as raw (Value, Span) for type checking in validate.
            let value = n
                .entries()
                .iter()
                .find(|e| e.name().is_none())
                .map(|e| (kdl_value_to_value(e.value()), entry_span(e)));
            ThemeVar { name: n.name().value().to_owned(), value }
        })
        .collect()
}

fn walk_page(page: &kdl::KdlNode) -> Page {
    let id = string_attr(page, "id");
    let route = string_attr(page, "route");
    let body = match page.children() {
        Some(c) => c.nodes().iter().map(walk_component).collect(),
        None => Vec::new(),
    };
    Page { id, route, body }
}

fn walk_component(node: &kdl::KdlNode) -> Component {
    let name = node.name().value().to_owned();
    let span = span_of(node);

    let mut args = Vec::new();
    let mut attrs = Vec::new();

    for entry in node.entries() {
        if let Some(key) = entry.name() {
            attrs.push((key.value().to_owned(), kdl_value_to_value(entry.value())));
        } else {
            args.push(kdl_value_to_value(entry.value()));
        }
    }

    let children = match node.children() {
        Some(c) => c.nodes().iter().map(walk_component).collect(),
        None => Vec::new(),
    };

    Component { name, args, attrs, children, span }
}

/// Copy a KDL node's source span into our decoupled [`Span`].
fn span_of(node: &kdl::KdlNode) -> Span {
    let s = node.span();
    Span { offset: s.offset(), len: s.len() }
}

/// Copy a KDL entry's source span into our decoupled [`Span`].
fn entry_span(entry: &kdl::KdlEntry) -> Span {
    let s = entry.span();
    Span { offset: s.offset(), len: s.len() }
}

/// Convert a `kdl::KdlValue` to our decoupled [`Value`].
fn kdl_value_to_value(v: &kdl::KdlValue) -> Value {
    match v {
        kdl::KdlValue::String(s) => Value::String(s.clone()),
        kdl::KdlValue::Integer(i) => Value::Integer(*i),
        kdl::KdlValue::Float(f) => Value::Float(*f),
        kdl::KdlValue::Bool(b) => Value::Bool(*b),
        kdl::KdlValue::Null => Value::Null,
    }
}

/// Project a KDL value to an owned `String`, discarding every non-string
/// variant. Shared by every string-typed field walk so the string-or-None
/// projection lives in one spot. (The exhaustive value mapper is
/// [`kdl_value_to_value`]; this is its string-only projection.)
fn as_string(value: &kdl::KdlValue) -> Option<String> {
    match value {
        kdl::KdlValue::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// Return the first positional (unnamed) arg of `node` as a raw
/// `(Value, Span)`, any type, so validate can type-check it. `None` for a
/// bare node with no positional argument.
fn first_raw_arg(node: &kdl::KdlNode) -> Option<(Value, Span)> {
    node.entries()
        .iter()
        .find(|e| e.name().is_none())
        .map(|e| (kdl_value_to_value(e.value()), entry_span(e)))
}

/// Return the first positional (unnamed) string arg of `node`, if any.
fn first_string_arg(node: &kdl::KdlNode) -> Option<String> {
    node.entries()
        .iter()
        .find(|e| e.name().is_none())
        .and_then(|e| as_string(e.value()))
}

/// Return the value of a named string attribute, if present.
fn string_attr(node: &kdl::KdlNode, key: &str) -> Option<String> {
    node.entries()
        .iter()
        .find(|e| e.name().map(|n| n.value()) == Some(key))
        .and_then(|e| as_string(e.value()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_site_metadata() {
        let src = r#"
site {
    title "My Project"
    languages "de" "en"
}
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert_eq!(cfg.title, Some("My Project".to_owned()));
        // languages are now raw (Value, Span) pairs
        assert_eq!(cfg.languages.len(), 2);
        assert_eq!(cfg.languages[0].0, Value::String("de".to_owned()));
        assert_eq!(cfg.languages[1].0, Value::String("en".to_owned()));
    }

    #[test]
    fn missing_optional_fields_are_none() {
        // No description or default-language; parse must still succeed —
        // absence is not an error (parse/validate boundary).
        let src = r#"
site {
    title "Minimal"
    languages "en"
}
"#;
        let cfg = parse(src).expect("valid KDL with missing optionals should parse");
        assert!(cfg.description.is_none());
        assert!(cfg.default_language.is_none());
        assert_eq!(cfg.languages.len(), 1);
        assert_eq!(cfg.languages[0].0, Value::String("en".to_owned()));
    }

    #[test]
    fn parses_site_index_card_fields() {
        let src = r#"
site {
    title "My Project"
    cover-image "cover.webp"
    image-credit "Photo by Someone"
    eyebrow "A Project"
}
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert_eq!(cfg.cover_image, Some("cover.webp".to_owned()));
        assert_eq!(cfg.image_credit, Some("Photo by Someone".to_owned()));
        assert_eq!(cfg.eyebrow, Some("A Project".to_owned()));
    }

    #[test]
    fn missing_site_index_card_fields_are_none() {
        let src = r#"
site {
    title "Minimal"
    languages "en"
}
"#;
        let cfg = parse(src).expect("valid KDL with missing optionals should parse");
        assert!(cfg.cover_image.is_none());
        assert!(cfg.image_credit.is_none());
        assert!(cfg.eyebrow.is_none());
    }

    #[test]
    fn parses_nav_items() {
        let src = r#"
site {
    nav {
        item label="Home" route="/home"
        item label="About" route="/about"
    }
}
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert_eq!(cfg.nav.len(), 2);
        assert_eq!(cfg.nav[0].label, Some("Home".to_owned()));
        assert_eq!(cfg.nav[0].route, Some("/home".to_owned()));
        assert_eq!(cfg.nav[1].label, Some("About".to_owned()));
        assert_eq!(cfg.nav[1].route, Some("/about".to_owned()));
    }

    #[test]
    fn parses_chrome_strings() {
        let src = r#"
site {
    chrome {
        string "search-placeholder" {
            de "Suchen…"
            en "Search…"
        }
    }
}
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert_eq!(cfg.chrome.len(), 1);
        let cs = &cfg.chrome[0];
        assert_eq!(cs.id, Some("search-placeholder".to_owned()));
        assert_eq!(cs.translations.len(), 2);
        assert_eq!(cs.translations[0], ("de".to_owned(), "Suchen\u{2026}".to_owned()));
        assert_eq!(cs.translations[1], ("en".to_owned(), "Search\u{2026}".to_owned()));
    }

    #[test]
    fn parses_top_level_labels_block() {
        // `labels` is a TOP-LEVEL block, a sibling of `site`/`page`, NOT a
        // `site` child.
        let src = r#"
labels {
    property "role" {
        en "Role"
        de "Rolle"
    }
    class "Character" {
        en "Character"
    }
}
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert_eq!(cfg.labels.len(), 2);

        let property = &cfg.labels[0];
        assert_eq!(property.target, RawLabelTarget::Property);
        assert_eq!(property.key, Some("role".to_owned()));
        assert_eq!(property.translations.len(), 2);
        assert_eq!(property.translations[0], ("en".to_owned(), "Role".to_owned()));
        assert_eq!(property.translations[1], ("de".to_owned(), "Rolle".to_owned()));

        let class = &cfg.labels[1];
        assert_eq!(class.target, RawLabelTarget::Class);
        assert_eq!(class.key, Some("Character".to_owned()));
        assert_eq!(class.translations.len(), 1);
        assert_eq!(class.translations[0], ("en".to_owned(), "Character".to_owned()));

        // Regression guard (search-bar plan step 5b, #178 back-compat): a
        // class entry with NO `plural` child must still parse with an empty
        // `plural` Vec, exactly as it did before this step existed.
        assert!(
            property.plural.is_empty(),
            "property entry with no `plural` child must have plural: vec![]"
        );
        assert!(
            class.plural.is_empty(),
            "class entry with no `plural` child must have plural: vec![]"
        );
    }

    #[test]
    fn parses_optional_plural_child_on_class_label() {
        // search-bar plan step 5b, #178 reconciliation: an optional nested
        // `plural { <lang> "…" … }` child under a `labels { class }` entry.
        // The bare `<lang> "…"` children remain the singular label,
        // unaffected by the `plural` child's presence.
        let src = r#"
labels {
    class "Character" {
        en "Character"
        de "Figur"
        plural {
            en "characters"
            de "Figuren"
        }
    }
}
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert_eq!(cfg.labels.len(), 1);

        let class = &cfg.labels[0];
        assert_eq!(class.target, RawLabelTarget::Class);
        assert_eq!(class.key, Some("Character".to_owned()));

        // Singular translations are unaffected by the `plural` child.
        assert_eq!(class.translations.len(), 2);
        assert_eq!(class.translations[0], ("en".to_owned(), "Character".to_owned()));
        assert_eq!(class.translations[1], ("de".to_owned(), "Figur".to_owned()));

        // The `plural` node itself must NOT leak into `translations`.
        assert!(
            !class.translations.iter().any(|(lang, _)| lang == "plural"),
            "the `plural` child node must not appear as a language in translations"
        );

        assert_eq!(class.plural.len(), 2);
        assert_eq!(class.plural[0], ("en".to_owned(), "characters".to_owned()));
        assert_eq!(class.plural[1], ("de".to_owned(), "Figuren".to_owned()));
    }

    #[test]
    fn absent_labels_block_yields_empty_vec() {
        let src = r#"
site {
    title "My Project"
    languages "de" "en"
}
page id="home" route="/"
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert!(cfg.labels.is_empty());
    }

    #[test]
    fn parses_theme_vars() {
        // Use a raw string with enough # delimiters so the color hex
        // value (#1a5490) does not accidentally close the literal.
        let src = r##"
site {
    theme {
        primary-color "#1a5490"
    }
}
"##;
        let cfg = parse(src).expect("valid KDL should parse");
        assert_eq!(cfg.theme.len(), 1);
        assert_eq!(cfg.theme[0].name, "primary-color");
        // theme values are now raw (Value, Span) pairs
        let (val, _span) = cfg.theme[0].value.as_ref().expect("theme var should have a value");
        assert_eq!(val, &Value::String("#1a5490".to_owned()));
    }

    #[test]
    fn parses_pages_in_order() {
        let src = r#"
page id="home" route="/"
page id="about" route="/about"
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert_eq!(cfg.pages.len(), 2);
        assert_eq!(cfg.pages[0].id, Some("home".to_owned()));
        assert_eq!(cfg.pages[0].route, Some("/".to_owned()));
        assert_eq!(cfg.pages[1].id, Some("about".to_owned()));
        assert_eq!(cfg.pages[1].route, Some("/about".to_owned()));
    }

    #[test]
    fn page_body_components_are_generic() {
        let src = r#"
page id="books" route="/books" {
    resource-list class="Book" {
        sort property="title"
    }
}
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert_eq!(cfg.pages.len(), 1);
        let body = &cfg.pages[0].body;
        assert_eq!(body.len(), 1);

        let comp = &body[0];
        assert_eq!(comp.name, "resource-list");
        assert_eq!(comp.attrs.len(), 1);
        assert_eq!(comp.attrs[0].0, "class");
        assert_eq!(comp.attrs[0].1, Value::String("Book".to_owned()));

        assert_eq!(comp.children.len(), 1);
        let sort = &comp.children[0];
        assert_eq!(sort.name, "sort");
        assert_eq!(sort.attrs.len(), 1);
        assert_eq!(sort.attrs[0], ("property".to_owned(), Value::String("title".to_owned())));
    }

    #[test]
    fn component_args_parse_value_types() {
        // KDL v6: booleans are #true/#false, null is #null.
        // 1.5 is used instead of 3.14 to avoid the approx_constant lint
        // (3.14 ≈ PI triggers clippy::approx_constant).
        let src = r#"
page id="p" route="/p" {
    demo-node "hello" 42 1.5 #true #null
}
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        let args = &cfg.pages[0].body[0].args;
        assert_eq!(args.len(), 5);
        assert_eq!(args[0], Value::String("hello".to_owned()));
        assert_eq!(args[1], Value::Integer(42));
        assert_eq!(args[2], Value::Float(1.5));
        assert_eq!(args[3], Value::Bool(true));
        assert_eq!(args[4], Value::Null);
    }

    #[test]
    fn component_carries_source_span() {
        let src = "page id=\"p\" route=\"/p\" {\n    widget {}\n}\n";
        let cfg = parse(src).expect("valid KDL should parse");
        let comp = &cfg.pages[0].body[0];
        assert!(comp.span.len > 0, "span length should be non-zero");
        // The span must actually point at the `widget` node, not just be a
        // plausible-looking offset/len (e.g. a 0..1 stub would still satisfy
        // the bounds check alone).
        let end = comp.span.offset + comp.span.len;
        assert!(end <= src.len(), "span must lie within the source");
        assert!(
            src[comp.span.offset..end].contains("widget"),
            "span should cover the `widget` node text, got {:?}",
            &src[comp.span.offset..end]
        );
    }

    #[test]
    fn unknown_top_level_node_is_preserved_for_validation() {
        let src = r#"
widget {}
page id="home" route="/"
"#;
        let cfg = parse(src).expect("unknown top-level node should not error");
        // The stray `widget` node is now preserved in `unknown_top_level`
        // so the validator can emit UnexpectedTopLevelNode.
        assert_eq!(cfg.pages.len(), 1);
        assert_eq!(cfg.pages[0].id, Some("home".to_owned()));
        assert!(cfg.title.is_none());
        assert!(cfg.languages.is_empty());
        assert!(cfg.nav.is_empty());
        assert!(cfg.chrome.is_empty());
        assert!(cfg.labels.is_empty());
        assert!(cfg.theme.is_empty());
        // Unknown top-level nodes are captured for Phase 2 validation.
        assert_eq!(cfg.unknown_top_level.len(), 1);
        assert_eq!(cfg.unknown_top_level[0].0, "widget");
    }

    #[test]
    fn first_site_block_wins() {
        // The walk takes the first `site` block and ignores later ones
        // (counting them is Phase 2 validation, not parsing).
        let src = r#"
site {
    title "First"
}
site {
    title "Second"
}
"#;
        let cfg = parse(src).expect("multiple site blocks should still parse");
        assert_eq!(cfg.title, Some("First".to_owned()));
    }

    #[test]
    fn parses_g11_copy_overrides_and_not_found_block() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
    footer-attribution "Scans: Universitätsbibliothek Basel"
    not-found {
        browse route="/books"
        browse route="/strips" label="nf-strips"
        other-node "ignored"
    }
}
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert_eq!(
            cfg.footer_attribution.as_ref().map(|(v, _)| v),
            Some(&Value::String("Scans: Universitätsbibliothek Basel".to_owned()))
        );
        assert!(cfg.not_found_span.is_some());
        // Only `browse` children are captured; order preserved.
        assert_eq!(cfg.not_found_browse.len(), 2);
        assert_eq!(cfg.not_found_browse[0].name, "browse");
        assert_eq!(
            cfg.not_found_browse[1].attrs,
            vec![
                ("route".to_owned(), Value::String("/strips".to_owned())),
                ("label".to_owned(), Value::String("nf-strips".to_owned())),
            ]
        );
    }

    #[test]
    fn g11_fields_absent_by_default() {
        let src = r#"
site {
    title "T"
    languages "de" "en"
}
"#;
        let cfg = parse(src).expect("valid KDL should parse");
        assert!(cfg.footer_attribution.is_none());
        assert!(cfg.not_found_span.is_none());
        assert!(cfg.not_found_browse.is_empty());
    }

    #[test]
    fn syntax_error_returns_parse_error() {
        let src = "node {\n  unclosed {";
        let err = parse(src).expect_err("malformed KDL should fail");
        assert!(matches!(err, ConfigError::Parse(_)));
    }
}

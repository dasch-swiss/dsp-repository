# Config Language

How a project's `config.kdl` is structured, how it composes
components, and how the engine validates it. Reads on top of
`dsp-incubator cpe/docs/COMPONENT_LIBRARY.md` (the components
this file invokes) and `dsp-incubator cpe/docs/adr/0006-config-language-kdl.md`
(why KDL).

The language is implemented in `cpe-core`, at `areas/access/cpe/core`. A change
to the language changes this document in the same PR.

**"Boot", "hard-fail" and "the web binary" in this document.** The text was written for a binary
that validated every project when it started and exited on an error. Here a project's configuration is
validated at the project's **Activation** (`areas/access/cpe/CONTEXT.md`). So "at boot" reads "at the
project's Activation", and a "hard-fail", or "boot fails", reads: the project is not served and its
paths answer 503; the process and the other projects are unaffected. "The web binary" reads "the
page assembly and its host crate" (`cpe-web`, `cpe-server`). The `--check` flag and the `cpe` command
are the incubator binary's; neither exists here.

## The KDL primer (just enough)

KDL is "JSON for documents". One node per line; positional
arguments separated by whitespace; named attributes as `key=value`;
children in braces. Comments with `//` (line) or `/-` (node).

```kdl
node-name "positional-arg" attribute="value" {
    child-node "another-arg"
    /- commented-out-child
}
```

That's the whole syntax CPE needs. The full KDL spec has more
(slashdashes for arguments, raw strings, type annotations); the
engine ignores constructs it doesn't need.

**KDL v6 literals:** booleans and null are written `#true`, `#false`,
and `#null` — not bare `true`/`false`/`null`. CPE's v1 config schema
doesn't currently use boolean or null values directly, but the `where`
node (data-binding, see below) may accept `equals=#true`. Authors
copying examples from pre-v6 KDL docs will see a parse error if they
use the bare form.

Property names in KDL are unquoted identifiers in most cases. If a
property name contains hyphens (`per-page`, `show-properties`), KDL
allows it. If it has weird characters, use a string.

## Top-level structure

Every `config.kdl` has exactly one `site` block, an optional top-level
`labels` block, an optional top-level `remodel` block, plus one or more
`page` blocks:

```kdl
site {
    title "Incunabula"
    description "Early printed books from the 15th century"
    languages "de" "en"
    nav {
        item label="Home" route="/"
        item label="Books" route="/books"
    }
    chrome {
        // chrome translations — see "Translations" below
    }
    theme {
        // CSS variable overrides — see "Theming" below
    }
}

labels {
    // property/class display-label translations — see "`labels` block" below
}

remodel {
    // how the archive's snapshot becomes this project's store — see
    // "`remodel` block" below
}

page id="home" route="/" {
    // page body
}

page id="books" route="/books" {
    // page body
}
```

`labels` is optional and top-level — a sibling of `site` and `page`, **not**
nested inside `site {}` (unlike `chrome`, which genuinely is a `site` child).
A project that declares no `labels` block gets identical behaviour to today:
every property/class display label falls back to `humanize_property_name`.

`remodel` is optional and top-level too, and a file holds at most one: a
second `remodel` node is `UnexpectedTopLevelNode`. It is the store builder's
input; the web binary validates it and does not use it. A project without
one (`hdm`, `alice-in-daschland`) validates as before.

## `site` block

Project-level metadata and chrome.

**Required:**

- `title` — project display name. Used in `<title>`, the brand lockup,
  the breadcrumb's project crumb, the 404s and the site index. **Chrome-id-or-literal** (like `description`): a value that
  matches a declared `chrome { string }` id renders that string's
  translation for the request language; any other value renders verbatim
  in every language (Alice, HdM). A literal that happens to equal a
  declared chrome id is translated — the same accepted risk as
  `description`. The site index resolves it per declared language and
  falls back to `en` for an undeclared one.
- `languages` — list of ISO 639-1 codes. Must include at least `de`
  and `en` for v1 (per `cpe-poc.md` §9.2). A project MAY declare
  additional languages beyond this floor (e.g. Alice declares
  `en`/`de`/`fr`/`it`) provided **chrome-string completeness** holds
  (every declared chrome string must carry every declared language —
  boot hard-fail otherwise). Per-language **narrative variant files are
  optional** (Phase 3.9) — a narrative `file` must resolve to either the
  base file or the `default-language` variant (boot hard-fail otherwise,
  `NarrativeFileNotFound`); non-default-language variants may be added
  incrementally without authoring every language up front (see
  Translations below for the render-time fallback chain). Record-page
  property values without a source translation for the additional
  language fall back gracefully to the first-present value via CPE's
  `pick_values` chain.

**Optional:**

- `description` — short prose for the site index and metadata. Like a nav
  item's `label` (see below), the value is chrome-string id OR literal — if
  it exactly matches a declared chrome-string `id`, the renderer resolves it
  to that id's translation for the current request language; otherwise it
  renders as-is. This lets a project supply a translated description via the
  `chrome {}` block (see Alice's `site-description` chrome string). Rendered
  in the per-project footer and, per-project, as the teaser text on the site
  index's project card.
- `cover-image` — a plain relative path to a cover asset **inside the
  project's own directory**, e.g. `"assets/cover.webp"`. Unlike
  `description`/`eyebrow`/`image-credit`, this is **not** chrome-id-or-literal
  — the value is a path, never translated, and used as-is for every language.
  **Validation:** must be a plain relative path with no `..` segment and no
  leading `/` (hard-fail, `DiagnosticKind::InvalidCoverImagePath`), and must
  resolve to a file that actually exists under the project's embedded
  `assets/` directory once the project is discovered (hard-fail,
  `DiagnosticKind::CoverImageNotFound`). A **missing `assets/` directory
  is not an exemption**: declaring a `cover-image` when the project has no
  `assets/` directory at all is the same hard-fail, since the truthful
  answer to "does this file exist?" is still no. Served at runtime via the
  engine-reserved `/{project-id}/_assets/{*path}` route under CPE's base (see
  "Route patterns" below) — a site index's project-card `<img src>` is
  `<base>/<project-id>/_assets/<cover-image path>`. Absent → the site-index
  card shows a same-height imageless ghost slot instead of a broken image.
- `image-credit` — credit/rights line shown under the cover image on the
  site index's project card (e.g. `"Foto: Alberto Venzago"`, or
  `"Authorship: AI-Generated."` for a synthetic asset). Like `description`,
  the value is chrome-id-or-literal — if it exactly matches a declared
  chrome-string id, the renderer resolves it to that id's translation for
  the current request language; otherwise it renders as-is. No existence
  validation beyond parsing. Absent → the card's reserved credit-caption
  row renders empty (the row is always reserved, so card titles still
  align across a grid row).
- `eyebrow` — a short scent line (material · place · span) shown above the
  title on the site-index's project card, e.g. `"Showcase · EN · DE"`.
  Chrome-id-or-literal, same idiom as `description`/`image-credit`. No
  existence validation beyond parsing. Absent → the card's reserved eyebrow
  line renders empty (the project's `/slug/` path in the card's footer row
  still identifies it, so no card loses its identity).
- `default-language` — one of `languages`. Default: first listed.
- `nav` — top navigation items (see below).
- `chrome` — translations for static UI strings (see below).
- `theme` — CSS variable overrides (see below).
- `footer-attribution`, `not-found` — the two project copy overrides (see
  "Project copy overrides" below).

`cover-image`/`image-credit`/`eyebrow` are all optional and purely additive —
they affect **only** the cross-project `/` site index's project-card
presentation, never a project's own pages. `cpe-server` has no site index:
the index is a page of the incubator's binary, and `Cpe::projects()` gives a
host a project's id, title and start URL and none of the three. Here they are
parsed, checked as said above and kept on the loaded project
(`cpe_web::project::Project`), the cover image is served with the project's
other assets, and no page shows them. Wherever this document says what the
site index does with a value, it describes that page. Alice's `config.kdl` is
the worked example:

```kdl
site {
    title "Alice in DaSCHland"
    cover-image "assets/cover.webp"
    image-credit "Authorship: AI-Generated."
    eyebrow "site-eyebrow"
    // ...
    chrome {
        string "site-eyebrow" {
            en "Showcase · EN · DE · FR · IT"
            de "Schaufenster · EN · DE · FR · IT"
            fr "Vitrine · EN · DE · FR · IT"
            it "Vetrina · EN · DE · FR · IT"
        }
    }
}
```

### Project copy overrides: `footer-attribution`, `not-found` (Incunabula Slice 8 / G11)

```kdl
site {
    footer-attribution "footer-credit"     // chrome-id-or-literal
    not-found {
        browse route="/books"
        browse route="/woodcuts" label="nf-woodcuts"   // label= optional
        browse route="/strips"
    }
}
```

**This is a narrow, deliberate exception** to the chrome-localization rule
that "engine UI strings live in the catalog, not per-project config"
(`docs/plans/chrome-localization/implementation-plan.md`). Exactly two pieces
of engine copy are project-overridable: the project 404's browse links (owner
decision O5) and the footer attribution (Slice 8 decision 9). Nothing else
is; a new override needs its own owner decision, not a precedent read off
this one. (A third, the settings page's lede, went with the settings page,
general chrome D2, 2026-10-08.)

- `footer-attribution` — replaces the footer's attribution line. The value
  is chrome-id-or-literal and renders as plain text. Absent: the engine's
  attribution string.
- `not-found { browse route="…" label="…" }` — the project 404 page's
  "Or browse:" links, in declaration order. Each `route` must match a
  declared page route **exactly**, the same rule as a nav item's route
  (hard-fail, `UnresolvedNotFoundBrowseRoute`). At most **three** `browse`
  entries are allowed, the design cap (hard-fail,
  `TooManyNotFoundBrowseLinks`). `label=` is optional and
  chrome-id-or-literal (a declared chrome-string id renders its translation;
  any other value renders verbatim — so there is no boot check on it).
  Absent, the link reads the label of the nav item with the same route, and
  failing that the route itself. With no `browse` entries at all, the 404
  lists the first three nav items, as before.

**Validation:** a non-string `footer-attribution` argument is
`InvalidFieldType`. A `browse` with no `route` is
`MissingRequiredAttribute`. A wrong-typed `route` or `label` is
`InvalidAttributeType`, never a silently dropped link. All of these are
config-internal. Absent config changes nothing (`dsp-incubator cpe/docs/adr/0002-deployment-topology.md`).

Both are rendered: `footer-attribution` in the shared footer
(`web/src/page.rs`), and `not-found { browse }` on the project 404 page
(`areas/access/cpe/components/src/component/resource_detail.rs`).

### `date-precision-word` (Incunabula Slice 8, decision 4)

```kdl
site {
    date-precision-word #true   // KDL bool; default #false
}
```

A single optional boolean `site {}` child, opt-in and off by default. When
`#true`, every rendered `Date` property value on a record/reader page (never
in a browse row's gloss) gets a trailing precision word appended after the
date text in a `span.prec` inside `span.v-date` — the localized word for the
value's `DatePrecision` (`Tag`/`Monat`/`Jahr`, DE; `day`/`month`/`year`, EN),
or `Zeitraum`/`range` when the value is a genuine range
(`ir::date_is_range`). When the value's stored calendar is
`Gregorian` and its precision is day or month, a calendar word is appended
after the precision word, separated by ` · ` (e.g. `Tag · gregorianisch`,
`month · Gregorian`; never at year precision, where Julian and Gregorian
share one year count) — Julian and an
absent calendar are the unmarked case and add nothing (owner decision 5:
Julian is Incunabula's default calendar, so marking it everywhere would be
noisier than marking the exception).

Incunabula's `config.kdl` sets `date-precision-word #true`; the word is
resolved by `format_date` in `areas/access/cpe/components/src/component/resource_detail.rs` from
the boot-lowered `SiteConfig.date_precision_word` (read from `site` in
`render_component` and passed via `ResourceDetailCfg.date_precision_word`,
`web/src/page.rs`, and `CompoundViewerCfg.date_precision_word` — NOT
threaded through `RenderContext`), using the
`ChromeMsg::DatePrec{Day,Month,Year,Range}` / `ChromeMsg::CalendarGregorian`
catalog entries.

**Validation:** a non-boolean argument (e.g. `date-precision-word "yes"`) is
`InvalidFieldType`, never a silent `false`. Absent, or a bare
`date-precision-word` node with no argument, lowers to `false` — the
project's dates render exactly as before this feature existed.

### `narrative-companion`, `narrative-side` and `text-align` (layout pass)

```kdl
site {
    narrative-companion "rail"   // "rail" (default) or "text"
    narrative-side "end"         // "end" (default) or "start"
    text-align "justify"         // "start" (default) or "justify"
}
```

- `narrative-companion` and `narrative-side` are the project's defaults for
  the narrative layout; a page's first `narrative` can override either (see
  "`narrative`: `companion=` and `side=`" below). `rail` shows the "On this
  page" list beside the intro from 80rem; `text` shows none. `side` is the
  edge the rail stands at. No shipped project sets either.
- `text-align "justify"` justifies and hyphenates the project's narrative
  prose and book introductions (a prose block's own paragraphs; the lede,
  lists, quotes, glosses and invite lists stay ragged), and only where the
  column is at least 30rem wide, so a phone stays ragged. It is emitted as
  `<html data-align="justify">`; `"start"` emits nothing. Incunabula sets it.
  **Justified text departs from WCAG 1.4.8** (level AAA, which asks for text
  that is not justified). CPE targets AA; this is an owner opt-in per
  project. Hyphenation follows the page's `<html lang>`, so a quotation in
  another language is divided by the page's rules.

**Validation:** a non-string argument is `InvalidFieldType`; a string
outside the key's two values is `UnknownSiteValue { key, value, allowed }`
(Error). All three are config-internal.

### `nav`

```kdl
nav {
    item label="Home" route="/"
    item label="Books" route="/books"
    item label="About" route="/about"
}
```

Each `item` has a `label` (translated chrome string id or literal)
and a `route` (must match a declared `page` route, leading slash
relative to the project root). Rendered as a `<nav>` on every page.

**Validation:** every nav `route` must match a declared page route
exactly (hard-fail, `DiagnosticKind::UnresolvedNavRoute`, Phase 3.8) —
exact string equality.

The renderer's `aria-current` active-page highlight (`nav_item_is_active`)
is a *different*, broader comparison: it marks a nav item current on an
exact route match, **and also** on any of that item's nested routes (the
current path starting with `item + "/"`), except for the home route `/`,
which never prefix-matches. This lets a `Books` nav item stay highlighted
on `/books/:id`, not just on `/books` itself.

Additionally: a nav item's `label` is chrome-string id OR literal (see
`chrome` below) — if it exactly matches a declared chrome-string `id`, the
renderer prefers the chrome translation over the literal text. That is the
intended bilingual-nav idiom, and it is **not** diagnosed: a
`NavLabelChromeIdCollision` Warning used to fire on it (14 across the three
shipped projects) but could not distinguish the idiom from an accidental
collision, because the only ids it compared against were ones the project
itself declared. It was removed as boot-log noise.

### `chrome`

Static UI strings (button labels, facet titles, system messages)
that need translation. Keyed strings, one per language:

```kdl
chrome {
    string "search-placeholder" {
        de "Suche..."
        en "Search..."
    }
    string "facet-publisher" {
        de "Verlag"
        en "Publisher"
    }
}
```

Components reference chrome strings by id (e.g.
`search-bar placeholder="search-placeholder"`).

**Validation:** every chrome string must have an entry for every
declared site language. Missing translation is a hard-fail at boot.

### `theme`

Enumerated per-project visual design knob, optional (defaults apply when
the block is absent):

```kdl
theme {
    accent "ochre"        // ochre | green | teal | red   (default: ochre)
    background "tint"     // neutral | tint               (default: tint)
    typeface "newsreader" // one of 11 faces (default: newsreader) — see below
}
```

The engine validates the declared values at boot (hard-fail on any
unrecognised value) and emits them as HTML attributes on `<html>`:

| Key | Allowed values | Emitted attribute |
|---|---|---|
| `accent` | `ochre`, `green`, `teal`, `red` | `data-accent="ochre"` |
| `background` | `neutral`, `tint` | `data-bg="tint"` |
| `typeface` | one of 11 faces (below) | `data-pick="newsreader"` |

The `typeface` value is the `data-pick` face name. The 11 faces, grouped
here as the design tool's Tweaks panel groups them (the grouping is not part
of the value):

- **Bookish** (serif-led): `newsreader`, `garamond`, `crimson`
- **Balanced** (incl. serif-title / sans-body pairings): `spectral`,
  `lora`, `literata`, `spectral-figtree`, `newsreader-figtree`
- **Casual** (sans-led): `figtree`, `hanken`, `schibsted`

⚠️ **A mockup's Tweaks `typeface` is a voice, not a face.** A Claude Design
mockup's Tweaks panel exposes typeface as one of the *voices* above (e.g.
`bookish`), while `theme { typeface }` accepts only the concrete face names
this table lists. Transcribing a mockup's Tweaks value verbatim is an
unrecognised value — a boot **hard-fail**, and per
`dsp-incubator cpe/docs/adr/0002-deployment-topology.md`'s single-binary topology one
project's invalid config takes the whole deployment down. Map the voice to
its default face instead (`bookish` → `newsreader`), and expect the same
voice-vs-face mismatch on any Tweaks value you transcribe into config. The
table above is the authority for the accepted faces.

The `data-direction` attribute is always `"edition"` (fixed, not a
project knob). The `data-appearance` attribute (`light` / `dark`) is
resolved from the user's cookie or `?appearance=` query parameter — it
is a **global** user/reading preference (toggled in the header), not a
per-project knob, so it is not part of `theme {}`.

The design-system `tokens.css` defines token overrides keyed on these
data attributes.

**No request-time override.** The `theme {}` block is the only source of
these attributes; no query key or cookie changes them. The settings page that
once previewed them live was removed (general chrome, D2, 2026-10-08). A
request still carrying one of its retired query keys (`accent`, `background`,
`typeface`, `reset`) is answered with a 303 to the same URL without them.

The `_`-prefixed route namespace is **reserved** for engine-registered routes
(`_static`, `_assets`, the `_<component>` fragment segments): a config page
must not declare a route whose first segment starts with `_` (boot
hard-fail, `DiagnosticKind::ReservedRoute`).

(An earlier `image` route reservation and `_zoom` fragment segment, for a
since-rejected-and-removed deep-zoom viewer-state surface, no longer exist
either — see
`dsp-incubator cpe/docs/adr/0008-images-are-embedded-only-resources.md`'s
post-implementation note. The embedded-media plan's plate column and its
`_plate` fragment endpoint are gone too — see the record-viewer entry in
`CLAUDE.md`'s Current state section.)

## `labels` block

A **top-level** block (a sibling of `site` and `page`, not nested inside
`site {}`) declaring per-language display labels for properties and classes —
a direct analog of the `chrome { string }` mechanism above, but keyed by
property/class **name** rather than an arbitrary id:

```kdl
labels {
    property "role" {
        en "Role"
        de "Rolle"
        fr "Rôle"
        it "Ruolo"
    }
    class "Character" {
        en "Character"
        de "Figur"
        fr "Personnage"
        it "Personaggio"
    }
}
```

Each `property`/`class` child names a property or Class (matching the same
strings used elsewhere in config — `show-properties`, `facet property`,
`resource-detail class="…"`) and carries one string per language, exactly
like `chrome { string }`.

**Fallback contract.** A declared label wins; where no label is declared for
a given property or class, the engine falls back to the pre-existing
`humanize_property_name` behaviour (the same title-casing/`-`-and-`_`-splitting
transform that was the *only* source of these labels before this mechanism
existed). This means a project can declare labels for some properties/classes
and rely on humanization for the rest — there is no all-or-nothing
requirement, and a project declaring no `labels` block at all behaves
identically to before this mechanism existed.

**Validation:**

- **Completeness is a hard-fail.** Every declared label must carry a
  translation for every declared site language — mirrors the chrome-string
  completeness rule (`DiagnosticKind::IncompleteLabelTranslation`).
- **Data-existence is advisory.** A label naming a property/class absent from
  the project's data produces a non-gating Warning
  (`DiagnosticKind::UnknownLabel`) — it catches typos in a label's key without
  blocking boot, mirroring the sparse-property warning family.

**Where it's used:** facet-group headings, the resource-detail eyebrow (the
class label above the title) and per-property `<dt>`-equivalent labels, and
the resource-list surf-head class title. See the "Alice demo" cross-reference
in `CLAUDE.md`'s "New public surface (config-translatable-labels)" entry for
the exact call sites.

**E1's empty-class title fallback (empty-states-not-found plan).** The
resource-list surf-head's class-label title used to read empty on a
genuinely-empty class — `render_surf_head` previously derived the label
from `list.items.first()`, which doesn't exist when there are no items. It
now resolves the label independently via `resolve_class_label` from the
`class` the `ir::Component::ResourceList` carries, so the title (and the
"0 resources" count line) render correctly even when the list is empty —
the same `labels { class }` declaration that resolves the populated-branch
eyebrow.

**Optional `plural` child on `class` entries.** A `class` entry may nest an
optional `plural { <lang> "…" }` child holding the class's localized
**plural** noun. This is a general class-noun declaration, **not** a
search-bar-only knob — several separate engine surfaces resolve it:

```kdl
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
```

The resolver is `resolve_class_plural` (`areas/access/cpe/core/src/labels.rs`) and its
capitalizing wrapper `resolve_class_plural_capitalized` (same module).
Its consumers:

1. the **narrowed-to-zero copy** (search and facet no-match) —
   `chrome::narrowed_to_zero_text`'s noun, via `resolve_class_plural` in
   `SearchFragmentSurface::resolve` (`web/src/page.rs`); used as declared.
2. the **browse breadcrumb / eyebrow** — `resolve_class_plural_capitalized`
   (`render_browse_breadcrumb_for_page`, `web/src/page.rs`).
3. the **`resource-detail` class crumb** —
   `resolve_class_plural_capitalized` (`render_record_breadcrumb_for_page`,
   `web/src/page.rs`).
4. the **404 / not-found (N3) "browse all" exit** —
   `chrome::nf_browse_all_text(class_plural, lang)`, which `render_not_found`
   (`areas/access/cpe/components/src/component/resource_detail.rs`) hands the resolved plural, used as
   declared.
5. the **browse count line** and its echo in the browse crumb's accessible
   name, and the dataset panel's count line — `resolve_class_plural` beside
   `resolve_class_label` (see the class-noun count line below); used as
   declared.
6. the **`incoming-links` caption** when its `label=` does not resolve —
   `resolve_class_plural_capitalized` (`web/src/page.rs`).

Two **nearby** surfaces stay **singular by engine design** — don't "fix"
either to the plural:

- the not-found page's **class crumb** uses `resolve_class_label`
  (singular, `render_record_breadcrumb_for_page`'s unresolved-id branch,
  `web/src/page.rs`); only the "browse all" exit beside it takes the plural.
- `render_surf_head`'s **browse title** uses `resolve_class_label`
  (`component/resource_list.rs`), so `/books` reads "Buch · 19 Bücher" —
  the TITLE is the singular; the **count line** beside it takes the plural
  (`resolve_class_plural`) and, at one result, the singular (see the
  class-noun count line below).

**Practical consequence for how you spell the declared value:** the crumbs
and the incoming-links caption capitalize the resolved noun, and the
sentence copy (no-match, browse-all, count lines) uses it as declared, so
declare **DE plurals capitalized** ("Bücher", "Figuren") and **EN plurals
lowercase** ("books", "characters") — the crumb capitalizes what it gets,
the sentences don't. (German common nouns are always
capitalized anyway; this is what makes the split work.)

**The trap.** An absent `plural` block is legal, so a declared class
*without* one degrades three live surfaces (browse eyebrow, detail class
crumb, 404 browse-all exit) to the **capitalized singular** — e.g. "Buch"
and "Alle Buch durchsuchen". Boot validation flags it advisorily:
`DiagnosticKind::MissingClassPlural` (Warning, `check_class_plurals`) fires
once per declared class label with no `plural` block — every one of them,
whether or not the class has a page bound to it. Declare `plural` for every
declared class.

The bare `<lang> "…"` children remain the **singular** label (the eyebrow/
surf-head text) — unaffected by `plural`'s presence. Resolution order
(never panics; always returns a usable noun): the declared plural for the
request language → the declared plural for the default/any language → the
class's own singular label (`resolve_class_label`, itself falling back to
`humanize_property_name`). This is a deliberate reuse of the EXISTING
`labels { class }` mechanism rather than a second, parallel class-noun
source (e.g. a `search-bar label="…"` chrome attr) — keep the singular and
plural forms declared together in one place. **Completeness applies to a
declared `plural` block exactly like the singular labels**: every site
language must have a plural translation once the block exists at all
(hard-fail, reusing `DiagnosticKind::IncompleteLabelTranslation`); an
ABSENT `plural` block is not a violation — it just falls through to the
singular/humanized fallback chain above.

## `remodel` block

The **remodel mapping**: the part of a project's re-modelling that is
declared and not coded. It says how the store builder turns the project's
archive snapshot into CPE's store: which archive classes and properties
become which CPE ones, how a resource gets its id and its image, which class
is a member of which, the list vocabularies, the shortcode, the text the
archive's editors wrote where a value was not known, how the images are
served, and which classes annotations may touch. What a project needs and
the block cannot say is code, the project's **hook**; the block says that
there is one and what it may do (the `hook` node, under "The hook"). The
block is the store builder's
(`areas/access/cpe/store-build/README.md`) input: the builder
lowers it through `cpe_core::validate::load_remodel` and builds the
project's store from the archive snapshot, the block and, where the block
declares one, the hook. Here the builder runs at the project's Activation, called by
`cpe-server`; the page assembly validates the block with the rest of the file and does
not use it. For where Incunabula's data comes from, see
`areas/access/cpe/projects/incunabula/PROVENANCE.md`.

The block is optional and top-level, at most once (see "Top-level
structure"). A node or attribute that the grammar below does not list is
refused as unknown. The worked example is Incunabula's block: the `remodel`
node stands in `projects/incunabula/config.kdl` as written here, line for
line, IRIs in full. A test of the store builder
(`areas/access/cpe/store-build/tests/it/worked_example.rs`) lowers the example and the file
and holds the two equal.

```kdl
site {
    title "Incunabula"
    languages "de" "en"
}

remodel {
    shortcode "0803"
    sentinel "[missing]"
    hook {
        key "sort_key" class="Book"
        key "catalogue_uri" class="Book"
        key "catalogue_uri_replaces" class="Book"
        provides "sort-key" class="Book"
        provides "side" class="BorderStrip"
        overrides "catalogue-uri" class="Book"
        drops class="Page"
    }

    list "office" {
        list-node "amerbach" { de "Johann Amerbach"; en "Johann Amerbach" }
        list-node "bergmann" { de "Johann Bergmann de Olpe"; en "Johann Bergmann de Olpe" }
        list-node "furter" { de "Michael Furter"; en "Michael Furter" }
        list-node "ysenhut" { de "Lienhart Ysenhut"; en "Lienhart Ysenhut" }
        list-node "other" { de "Richel · Wolff · ohne genannten Drucker"; en "Richel · Wolff · unnamed printer" }
    }
    list "side" {
        list-node "left" { de "links"; en "left" }
        list-node "right" { de "rechts"; en "right" }
    }

    class "Book" iri="http://www.knora.org/ontology/0803/incunabula#Book" no-addresses=#true {
        id curated-key="slug"
        property "name" from="label" curated-key="title_override"
        property "alt-title" iri="http://www.knora.org/ontology/0803/incunabula#hasTitle" type="text" exclude-label=#true
        property "creator" iri="http://www.knora.org/ontology/0803/incunabula#hasCreator" type="text"
        property "publisher" iri="http://www.knora.org/ontology/0803/incunabula#hasPublisher" type="text"
        property "pub-place" iri="http://www.knora.org/ontology/0803/incunabula#hasPubloc" type="text" max=1 sentinel="absent"
        property "shelfmark" iri="http://www.knora.org/ontology/0803/incunabula#hasLocation" type="text" max=1
        property "physical" iri="http://www.knora.org/ontology/0803/incunabula#hasPhysicalDesc" type="text" max=1
        property "catalogue-uri" iri="http://www.knora.org/ontology/0803/incunabula#hasUri" type="uri" max=1
        property "pub-date" iri="http://www.knora.org/ontology/0803/incunabula#hasPubdate" type="date" min=1 max=1 display-key="date_display"
        curated "office" key="office" type="list" list="office" min=1
        curated "teaser" key="teaser" type="per-language"
        member-count "extent" of="Page"
        cover curated-key="cover_page"
    }
    images class="Image" iiif-base="https://iiif.dasch.swiss/0803/" {
        authorship "Universitätsbibliothek Basel"
        license "CC BY 4.0"
    }
    class "Page" iri="http://www.knora.org/ontology/0803/incunabula#Page" image=#true {
        member-of "Book" seq="seq-number"
        id from="property" property="name" under-parent=#true
        property "name" "page-id" iri="http://www.knora.org/ontology/0803/incunabula#hasPagenum" type="text" min=1 max=1 empty="refuse" sentinel="keep"
        property "description" iri="http://www.knora.org/ontology/0803/incunabula#hasDescription" type="text" max=1
        property "comment" iri="http://www.knora.org/ontology/0803/incunabula#hasComment" type="text"
        property "strip-left" iri="http://www.knora.org/ontology/0803/incunabula#hasBandTypeL" type="link" max=1 target="BorderStrip"
        property "strip-right" iri="http://www.knora.org/ontology/0803/incunabula#hasBandTypeR" type="link" max=1 target="BorderStrip"
    }
    class "BorderStrip" iri="http://www.knora.org/ontology/0803/incunabula#Band" image=#true {
        id curated-key="slug" from="label"
        property "name" iri="http://www.knora.org/ontology/0803/incunabula#hasTitle" type="text" curated-key="name" min=1 max=1 empty="refuse" sentinel-within="keep"
        property "description" iri="http://www.knora.org/ontology/0803/incunabula#hasDescription" type="text" max=1 sentinel="absent" sentinel-within="keep"
        property "comment" iri="http://www.knora.org/ontology/0803/incunabula#hasComment" type="text" empty="refuse" sentinel-within="keep"
        incoming-count "page-uses" from="Page" {
            via "strip-left"
            via "strip-right"
        }
    }

    annotations {
        on "Page"
        link-targets "Book" "Page"
    }
}
```

### Grammar

Every node not listed, at any depth, is `UnknownChildNode`, except below a
`list-node`: there every child is read as a language label, and one in a
language the site does not declare is `UnknownRemodelReference` (a label with
an attribute or a child of its own is still `UnknownRemodelAttribute` or
`UnknownChildNode`). Every attribute not listed is `UnknownRemodelAttribute`. A required attribute that is
absent is `MissingRequiredAttribute` and a wrong-typed one is
`InvalidAttributeType`. A required node or argument that is absent is
`MissingRequiredAttribute` too, located at the parent (or, for an
argument, at the node). A node that occurs more often than its count
allows is `DuplicateRemodelEntry`, located at the extra node, and so is an
attribute written twice on one node, located at that node. A node takes
exactly the arguments the table lists: a further positional argument, or
any on a node listed with none, is `InvalidRemodelValue`. A text that must
name something (a class, property, curated, count or list name, a `list-node`
name, a slug, a label, an `iri`, a `curated-key`, `display-key` or `target`,
a `curated` node's `key=` or `list=`, an `id`'s `property=`, a `member-of`
parent or `seq=`, an `of=`, an `incoming-count`'s `from=`, a `via`, the
`images` `class=`, `authorship`, `license`, a class name of `on` or
`link-targets`, the argument and the `class=` of
a `hook` child) is `InvalidRemodelValue` when
empty. "n" is how often a node may occur in its parent. Every diagnostic is
an error.

| Node | n | Arguments and attributes (required unless a default is given) | Errors beyond the general ones |
|------|---|------------------------------------------|--------------------------------|
| `remodel` | 0..1, top level | none | a second block is `UnexpectedTopLevelNode` |
| `shortcode` | 1 | one string | `InvalidRemodelValue` unless four characters of `0-9A-F` |
| `sentinel` | 0..1 | one string | `InvalidRemodelValue` when empty, and when it starts or ends with whitespace (a text is compared with it stripped, so such a sentinel would match none) |
| `hook` | 0..1 | no argument; children `key`, `provides`, `overrides` and `drops`, each 0..n | `DuplicateRemodelEntry` for a second `hook`; `InvalidRemodelValue` for an argument |
| `hook` › `key` | 0..n | a curated key; `class=` | `UnknownRemodelReference` for a `class=` no `class` declares; `DuplicateRemodelEntry` for a key that class already declares |
| `hook` › `provides` | 0..n | a CPE property name; `class=` | `UnknownRemodelReference` for a `class=` no `class` declares; `DuplicateRemodelEntry` for a name that class already has; `ReservedRemodelName` for `image` in a block with an `images` node |
| `hook` › `overrides` | 0..n | a CPE property name; `class=` | `UnknownRemodelReference` for a `class=` no `class` declares, and for a name no `property` node of that class has; `DuplicateRemodelEntry` for a property two `overrides` nodes name, under one of its names or under two |
| `hook` › `drops` | 0..n | no argument; `class=` | `UnknownRemodelReference` for a `class=` no `class` declares; `DuplicateRemodelEntry` for a class two `drops` nodes name; `ConflictingRemodelAttributes` for a class that another class is `member-of` |
| `list` | 0..n | the list's name | `DuplicateRemodelEntry` for a name declared twice |
| `list` › `list-node` | 1..n | the node's slug; one child per site language, named by the language, holding the label | `DuplicateRemodelEntry` for a slug twice in one list and for a language twice in one node; `IncompleteListTranslation` for a site language without a label; `UnknownRemodelReference` for a label language the site does not declare |
| `class` | 1..n | the class's CPE name; `iri=`; `image=#true` (default `#false`); `no-addresses=#true` (default `#false`) | `DuplicateRemodelEntry` for a name or an `iri` declared twice; `ReservedRemodelName` for the name the `images` block gives its `class=`; `ConflictingRemodelAttributes` for `image` in a block without an `images` node |
| `id` | 1 per class | no argument; `curated-key=` (optional); `from="label"\|"property"\|"iri"` (optional); `property=` (with `from="property"`); `under-parent=#true` (default `#false`) | `ConflictingRemodelAttributes` for neither `curated-key` nor `from`, for `property` without `from="property"` and for `from="property"` without `property`, and for `under-parent` on a class without a `member-of` or beside `curated-key`; `UnknownRemodelReference` for a `property` that names no `text` property of the class with `max=1`; `InvalidRemodelValue` for a `from` outside its three words; `DuplicateRemodelEntry` for a curated key the class already declares |
| `member-of` | 0..1 per class | the parent class's name; `seq=` (optional, a CPE property name) | `UnknownRemodelReference` for a parent no `class` declares; `RemodelMembershipCycle` for the class itself or a longer cycle; `DuplicateRemodelEntry` and `ReservedRemodelName` for the `seq` name as for any property name |
| `property` | 0..n per class | one or more CPE names; a source, either `iri=` with `type="text"\|"uri"\|"date"\|"link"` or `from="label"`; `curated-key=`; `min=` (default 0); `max=` (default none); `empty="drop"\|"refuse"` (default `"drop"`); `sentinel="refuse"\|"absent"\|"keep"` (default `"refuse"`); `sentinel-within="refuse"\|"keep"` (default `"refuse"`); `exclude-label=#true`; `display-key=`; `target=` | `ConflictingRemodelAttributes` for both sources or none, for an `iri` without a `type`, for a `link` without a `target`, for an attribute the form does not take (the table below), for `exclude-label` beside `curated-key` and for `sentinel` or `sentinel-within` in a block without a `sentinel` node; `DuplicateRemodelEntry` for an `iri` another `property` of the class has, for a name the class already has and for a curated key the class already declares; `UnsupportedRemodelType` and `UnknownRemodelType` (below); `InvalidRemodelValue` for a `max` below the `min`, a `min` or `max` that is negative or above 4294967295, and a word outside the closed vocabulary of `from`, `empty`, `sentinel` or `sentinel-within`; `UnknownRemodelReference` for a `target` no `class` declares |
| `curated` | 0..n per class | the CPE name; `key=`; `type="text"\|"list"\|"per-language"`; `list=` (with `type="list"`); `min=` (0 or 1, default 0) | `MissingRequiredAttribute` for an absent `key=` or `type=`; `ConflictingRemodelAttributes` for `list=` beside another type and for `type="list"` without `list=`; `UnknownRemodelReference` for a `list=` no `list` declares; `DuplicateRemodelEntry` for a name the class already has and for a curated key the class already declares; `InvalidRemodelValue` for a `type` outside its three words and for a `min` other than 0 or 1 |
| `member-count` | 0..n per class | the CPE name; `of=` | `UnknownRemodelReference` when `of` names no class, or a class that is not `member-of` this one; `DuplicateRemodelEntry` for a name the class already has |
| `incoming-count` | 0..n per class | the CPE name; `from=`; children `via` 1..n, each one CPE property name of the `from` class | `UnknownRemodelReference` for a `from` no `class` declares and for a `via` that names no property of it; `ConflictingRemodelAttributes` for a `via` that names a property that is no `link`, or a link whose `target` is not this class; `DuplicateRemodelEntry` for a name the class already has and for a property two `via` nodes name |
| `cover` | 0..1 per class | no argument; `curated-key=` | `MissingRequiredAttribute` for an absent `curated-key=`; `ConflictingRemodelAttributes` in a block without an `images` node, on a class no class is `member-of`, and beside the class's `image`; `DuplicateRemodelEntry` for a curated key the class already declares |
| `images` | 0..1 | `class=` (the CPE name of the image class); `iiif-base=`; children `authorship` 1 and `license` 1, each one string | `InvalidRemodelValue` for a base that is not `https://`, a host and `/` |
| `annotations` | 0..1 | children `on` 0..1 and `link-targets` 0..1, each one or more class names | `UnknownRemodelReference` for a name no `class` declares; `DuplicateRemodelEntry` for a name repeated within one `on` or `link-targets` |

Which `property` attribute goes with which of its five forms. An attribute
that is present on a form that does not take it is
`ConflictingRemodelAttributes`: presence decides, so `exclude-label=#false`
on a `uri` is one too. `type` goes with the four `iri` forms and with
`from="label"` never. Such an attribute is one mistake and is reported
once: what it holds is not judged, so `empty="x"` on a `uri`,
`target="Ghost"` on a `text` and `sentinel="keep"` on a `date` in a block
without a `sentinel` node each give that one diagnostic. The same holds for
a `curated` node's `list=` beside a `type` other than `list`, and for an
`id`'s `property=` without `from="property"`.

| Attribute | `iri` text | `iri` uri | `iri` date | `iri` link | `from="label"` |
|-----------|-----------|----------|-----------|-----------|----------------|
| `curated-key` | yes | no | no | no | yes |
| `empty` | yes | no | no | no | no (an empty label is always refused) |
| `min`, `max` | yes | yes | yes | yes | no |
| `sentinel`, `sentinel-within` (need a `sentinel` node) | yes | yes | no | no | no |
| `exclude-label` (not with `curated-key`) | yes | no | no | no | no |
| `display-key` | no | no | yes | no | no |
| `target` | no | no | no | required | no |

Readings of the table, as built:

- **`type`.** `integer`, `decimal`, `boolean`, `list-node`, `geometry` and
  `color` are `UnsupportedRemodelType`, a located "not supported yet". The
  type word `list-node` names the archive port's value kind and has
  nothing to do with the `list-node` node. Any other word is
  `UnknownRemodelType`.
- **A `property`'s source.** A `property` has either an `iri` with a
  `type`, or `from="label"`. Both, or neither, is
  `ConflictingRemodelAttributes`, and so is an `iri` without a `type` and a
  `type` beside `from="label"`. `from` takes the one word `label`; any
  other is `InvalidRemodelValue`. A `property` with a faulty source is
  still held to the table above, in the same pass: an attribute is
  reported when neither the form its `type` names nor, with `from`
  present, the label form takes it. Only a node that names no form at all
  (no `from` and no readable `type`) is not checked against the table.
- **References.** A `link`'s `target`, a `member-of` parent, a
  `member-count`'s `of` and an `incoming-count`'s `from` name a declared
  `class`, and a `curated` node's `list=` a declared `list`, each anywhere in
  the block: a class declared further down resolves. An `id`'s `property`
  names a `property` of its own class and a `via` one of the `from` class,
  by any of the property's names, wherever in the class it stands. A name no
  node declares is `UnknownRemodelReference`. A `link` without a `target` is
  `ConflictingRemodelAttributes`.
- **Ids.** Every `class` has exactly one `id`; a class without one is
  `MissingRequiredAttribute`, located at the class. An `id` has a
  `curated-key`, a `from`, or both: the key alone requires a curated value,
  and with a `from` beside it the `from` is what the id is made of when no
  value is curated. Neither is `ConflictingRemodelAttributes`. `property`
  goes with `from="property"` and with no other `from`, and
  `from="property"` needs it; a mismatch either way is
  `ConflictingRemodelAttributes`. The `property` names a `text` property of
  the class that holds at most one value (`max=1`); a name of no property,
  of a property of another type or of one without `max=1` is
  `UnknownRemodelReference`. `under-parent` goes only on a class with a
  `member-of`, and not beside `curated-key`.
- **Membership.** `member-of` names the class whose resources this class's
  resources are members of. A class that names itself, or whose parents'
  own `member-of` nodes lead back to it, is `RemodelMembershipCycle`,
  reported at each `member-of` on the cycle; a class that only leads into
  a cycle is not reported. `seq=` is the name of the property the member's
  position is stored under.
- **Member and incoming counts.** A `member-count`'s `of` names a class
  that is `member-of` the class the count stands on; any other class, the
  class itself among them, is `UnknownRemodelReference`. Each `via` of an
  `incoming-count` names a `link` property of the `from` class whose
  `target` is the class the count stands on; a property that is no link,
  or a link to another class, is `ConflictingRemodelAttributes`, located
  at the `via`. An
  `incoming-count` needs at least one `via` and names no property twice,
  under one of its names or under two; the `from` class may be the class
  itself.
- **A faulty node is reported once.** A reference to a node that is itself
  refused is not reported a second time: an `id` or a `via` that names a
  property whose names, source (a `type` without an `iri` among them), `max`
  or `target` are faulty, and a `member-count`
  or `cover` on a class whose would-be member has a `member-of` that names
  no class, give no diagnostic of their own.
- **Images.** `image` on a `class` and `cover` both need an `images` node
  in the block. A `cover` stands on a class that some class is `member-of`,
  and not beside `image`: a class takes its image from its own file or from
  a member, never both. Each mismatch is `ConflictingRemodelAttributes`,
  located at the `class` for `image` and at the `cover` otherwise.
- **Presence decides.** As in the table above, a conflict is decided by an
  attribute being written, whatever its value: `image=#false` in a block
  without `images`, `image=#false` beside a `cover` and
  `under-parent=#false` on a class without a `member-of` are each
  `ConflictingRemodelAttributes`.
- **Counts.** `min` defaults to 0 and `max` to none. A negative `min` or
  `max`, one above 4294967295 (the largest count the mapping holds), and a
  `max` below the `min`, are `InvalidRemodelValue`. A
  `curated` node's `min` is 0 or 1, and `min=1` marks the value required;
  any other is `InvalidRemodelValue`.
- **Closed words.** `from`, `empty`, `sentinel`, `sentinel-within` and a
  `curated` node's `type` take the words the table lists and no other;
  another is `InvalidRemodelValue`. The two `type` attributes differ here:
  a `curated` node's `type` word outside its three is `InvalidRemodelValue`,
  while a `property`'s `type` word that is no value kind is
  `UnknownRemodelType`. `sentinel=` and `sentinel-within=` need
  a `sentinel` node in the block: without one they are
  `ConflictingRemodelAttributes`.
- **A `curated` node.** `key=` and `type=` are required; either absent is
  `MissingRequiredAttribute`. `list=` goes with `type="list"` and with no
  other type, and `type="list"` needs it; a mismatch either way is
  `ConflictingRemodelAttributes`.
- **Names are per class.** One namespace of CPE property names per class
  holds the name of every node that yields a property: its `property` nodes
  (each of their names), its `curated`, `member-count` and
  `incoming-count` nodes, its `member-of`'s `seq=`, and each `provides` the
  `hook` node declares on the class. A name taken twice,
  also once by each of two kinds, is `DuplicateRemodelEntry`. Within one
  class no two `property` nodes share an `iri`. Two classes may use the same
  `iri` or the same property name. One `property` node with several names
  is the form for one source under two names.
- **A reserved name.** In a block with an `images` node the property name
  `image` is taken: it names the link from a resource to its image, which
  is the mapping's own to write. Any node that takes it is
  `ReservedRemodelName`. A block without `images` may use the name.
- **Curated keys are per class.** A curated key is declared wherever it is
  read: `curated-key=` of an `id`, a `cover` or a `property`, `display-key=`
  of a `property`, `key=` of a `curated`, and a `key` of the `hook` node,
  which is a key the hook reads on the class it names. A key declared twice
  on one class, in any two of these places, is `DuplicateRemodelEntry`; the
  same key on two classes is fine.
- **The `hook` node.** Each child names a declared `class`, anywhere in the
  block, and the node itself may stand before or after the classes. A `key`
  and a `provides` are held against what their class declares, so a clash
  with a node of the class is reported at the hook's child. An `overrides`
  names a `property` node of its class by any of the property's names; a
  `curated` node or a count cannot be overridden, and a property is
  overridden once, whichever of its names the nodes use. A `drops` takes no
  argument, and a class is dropped from once.
- **`list` and `class`.** A `list` needs at least one `list-node`, and the
  block needs at least one `class`. The lowering keeps the declared order
  of classes and of list nodes.
- **No site language.** When the site declares no language, a `list-node`
  is not checked per language (no `IncompleteListTranslation`, no
  `UnknownRemodelReference` for a label) and its labels are kept as
  written. `validate::load_remodel`, the entry meant for the store builder,
  refuses such a site instead (`MissingSiteLanguages`).
- **What the store builder checks of the file.** `validate::load_remodel`
  lowers the block and refuses a second `remodel` block
  (`UnexpectedTopLevelNode`), as boot does; it checks nothing else of the
  config, so the builder builds from a file whose `site` or pages boot would
  refuse. A site language written twice (`languages "de" "en" "de"`) is
  accepted by the site and held once by the mapping, at its first place.

### The value steps

This is what the store builder does with one resource and one `property`
node. "Stripped" means with the whitespace around the text
cut, as Python's `str.strip()` does; it is used only to test, except where
a step names another way of cutting.

| Step | What happens |
|------|--------------|
| 1 | **Curated override.** With `curated-key`, the resource's curated texts under that key are read as every curated text is: an address or the sentinel in one of them refuses the key once (the first value in the order the port serves them decides which rule); each is stripped and an empty one is no value; more than one left is `curated-count`. A refused key ends the property: no later step runs. If one text remains it is the value, the source is not read, and steps 2 to 6 are skipped. |
| 2 | **Source and kind.** The `iri`'s values in the port's order, or the label as one untagged text. Each value of another kind than `type` is one `unexpected-value-kind`; the property then gives nothing and steps 3 to 7 are skipped. |
| 3 | **Count.** `min` and `max` over the source's values as served, before anything is left out. One `value-count` per property, and the property gives nothing. A curated value satisfies `min`; a value that is the sentinel alone counts. |
| 4 | **Exclusion.** With `exclude-label`, the first value that equals the label is left out. A value is compared after a trailing U+001E and the whitespace around it are cut, and only when that run holds a U+001E; the label is compared as it is. No such value is `label-not-a-title`. |
| 5 | **One value check per property.** The values are walked in the port's order; the first that offends gives the property's one refusal and the property gives nothing. Within a value the order is: empty once stripped with `empty="refuse"` (`empty-value`); holds `http` on a `no-addresses` class (`http-in-value`); the sentinel alone once stripped with `sentinel="refuse"`, or the sentinel inside a longer text with `sentinel-within="refuse"` (`missing-in-value`). A `uri` is exempt from the address check. |
| 6 | **Leaving out.** A text that is empty once stripped (`empty="drop"`) and a value that is the sentinel alone once stripped (`sentinel="absent"`) give no row. |
| 7 | **Writing.** What remains is written under each of the node's names, numbered from 0, as served: nothing is trimmed and a U+001E stays (`dsp-incubator cpe/docs/adr/0015-incunabulas-store-is-built-from-the-port.md`, N2). A text's language is carried. A curated value is written stripped. A `uri` is written as `text`. |

On a property the `hook` node lists under `overrides`, steps 5 and 6 run
after the hook, on the values the hook left: the hook's values where it set
any, the mapping's where it set none. Steps 1 to 4 run before the hook.

### The hook

A project's **hook** is a function of the store builder that runs once, over
what the mapping gave, for the rules of the project that this block cannot
say. Incunabula's block declares one (the worked example above). A block
without a `hook` node says that no code takes part in the project's store.
A block with one says so, and lists everything the hook may read or change
beyond the mapping's own result:

| Child | What it permits |
|-------|-----------------|
| `key "<key>" class="<Class>"` | the hook reads the curated texts under the key on a resource of the class. Where the hook reads the stripped texts, the key is checked like every curated key: a text that holds an address or the sentinel refuses it (`http-in-value`, `missing-in-value`), once per resource. A resource on which the hook reads the key only as served, or not at all, is not refused for it. How many texts there may be is the hook's rule. |
| `provides "<name>" class="<Class>"` | the hook writes the property, which no node of the class yields: texts, or nodes of a `list` the block declares (a node the list does not have is `unknown-list-node`). |
| `overrides "<name>" class="<Class>"` | the hook sets the values of that `property` node, under each of the node's names. A value is of the kind the node writes: a text for a `text`, a `uri` and a `from="label"` node, a link to a resource of the `target` class, and for a `date` only a date the mapping gave. |
| `drops class="<Class>"` | the hook drops resources of the class. |

The hook does not see the archive snapshot. It reads a resource as the
mapping left it: its id, whether it is kept, the values of its `property`
and `curated` nodes, and, where a step refused a property, why (a value of
another kind, the count, or a value). Of the archive's own data it reads
the resource's IRI, label and parents, and the values under the source of a
declared `property`, as served. A read or a
change the node does not declare is not made, and the build is refused
(`hook-exceeds-declaration`); so is a walk over the resources of a class
the block does not declare, which would otherwise find none and say
nothing.

The builder holds the block and its hooks to each other: a block with a
`hook` node in a project the builder has no hook for is
`hook-not-registered`, and a hook for a project whose block has no `hook`
node is `hook-not-declared`.

**A dropped resource** gets no row in any table: no values, no Image, no
place among its parent's members, and it is counted nowhere. It keeps its
id. The refusals of its property mapping are withdrawn with it (value steps
1 and 3 to 5, a link to a resource of another class, a curated node's
refusals); a value of another kind (step 2) is a defect of the archive's
data and stays, and so does a refusal of the resource's label, which is
reported where the label is checked. What points at it is refused, as
"Other rules" says for links, covers and annotations; a class that another
class is `member-of` cannot be dropped from. A resource that was refused
before the hook ran stays refused.

What the hook writes is held as follows. The values it sets on an
overridden property are counted against the node's `min` and `max` (a
`from="label"` node takes neither, and its values are exactly one, as
before the hook; another number is `value-count`) and
then checked and thinned by value steps 5 and 6. A property that makes its
class's id is checked where it is mapped, before the id is made, even when
the hook may override it. A provided property has no node and so no check:
what its texts may hold is the hook's rule.

### Other rules

The store builder's other rules. A name in backticks such as `page-parent`
is the rule a refusal is reported under.

- **A refused resource is mapped no further.** A resource refused for its
  class, parent, `seqnum`, file or id gets no value mapping, no cover and
  no counts, and nothing that points at it is refused for it.
- **The id's source is mapped first.** With `id from="property"`, that
  property's node is mapped before the id: when it is refused, or gives no
  single text, that refusal is the resource's one report, the resource has
  no id and is mapped no further.
- **The label is checked once per resource**, before anything reads it,
  when the class reads it at all (a `from="label"` property whose curated
  override is absent, an `id from="label"` that is used, `exclude-label`):
  no address on a `no-addresses` class (`http-in-value`), no sentinel
  (`missing-in-value`), and, when a `from="label"` property or
  `exclude-label` reads it, not empty once stripped (`empty-value`), in
  that order. An `id from="label"` alone does not ask for the emptiness
  check: a label that slugs to nothing is `empty-id-slug`. A refused label
  silences its readers: the nodes that read it give nothing and no refusal
  of their own, so `exclude-label` reports no `label-not-a-title`, and a
  hook reads the property as refused, not as a value.
- **A resource the hook drops reports only its kinds.** Every refusal of a
  resource's property mapping is held on the resource and withdrawn when a
  hook drops it; a value of another kind stays, and so does a refusal of
  its label. See "The hook".
- **Id policies.** With `curated-key`: the curated values under the key are
  counted as served. None: a class without `from` reports `slug-count`, or
  `book-without-curation` when the resource has no curated value at all; a
  class with `from` mints from that source. More than one: `slug-count`.
  One: an address or the sentinel in it refuses it; empty once stripped it
  is `empty-slug`, with no fallback; otherwise the stripped text is the id,
  and a second resource with it is `duplicate-slug`. The `from` source is
  read, and the label checked for it, only when no curated value is served:
  a BorderStrip labelled `[missing]` whose slug is curated is accepted
  (Incunabula has one, `strip-38`). `from="label"` and `from="property"`:
  the slug of the text (`empty-id-slug` when nothing is left).
  `from="iri"`: the IRI's last part, as it is. `under-parent=#true`: the id
  is `<parent id>-<slug>`, and a repeat among one parent's members gets
  `-2`, `-3`, … in positional order. Ids are made over all members before
  any drop, so a dropped member leaves a later one with the same number
  its suffix. Two resources with one id are `duplicate-id`, a dropped
  resource among them. Whatever the policy, an id that is empty or holds a
  character outside `A-Z a-z 0-9 - . _ ~` is `id-not-url-safe`: the check
  runs once the ids are made, on every kept resource, every Image id (the
  stem of the file), and every annotation and annotation link that gets a
  row (the last part of its IRI). Ids that differ only in case are two ids.
- **Membership.** `member-of` requires exactly one parent, of that class
  (`page-parent`), and a `seqnum` (`page-without-seqnum`).
  `compound_parts` lists a parent's kept members by (`seqnum`, IRI's last
  part), renumbered from 1. `seq` writes the `seqnum` as an integer
  property.
- **Counts.** `member-count` is the number of kept members.
  `incoming-count` is the number of links, not of linking resources: a
  resource that points here under two `via` properties counts twice. Both
  are taken after the hook's drops: a dropped resource is no member, and
  its links are not counted.
- **Dates.** A `date` is converted in its own calendar at the precision
  both bounds share (`mixed-date-precision`, `date-out-of-range`).
  On a resource that has a value under the property, `display-key`
  requires one curated text under that key (`curated-count`),
  tagged with one of the site's languages
  (`date-display-without-language`, `unknown-date-display-language`). On a
  resource without one (a `date` whose `min` is 0) the key is not read: no
  text is required, and one that is curated is not written and not
  reported.
- **Links.** A link is written with its target's id. A target of another
  class than `target` is `strip-link-target`. A target of the right class
  that was refused gives no row and no refusal. A target of the right class
  that a hook dropped is `link-target-without-row`.
- **Curated properties.** `type="text"`: at most one text. `type="list"`:
  a list value; a text that is no node of the list is `unknown-list-node`.
  `type="per-language"`: one text in every site language or none
  (`curated-languages`), numbered in the site's order. `min=1` requires
  the value, counted after empty texts are left out (`curated-count`).
- **Images.** Only a class with `image=#true` has Images: each of its
  kept resources gets the Image of its file (the id is the file's stem), and
  the resource's `image` property links to it. A class without it gets none,
  whatever files its resources have. The file must be a still image
  (`not-still-image`) with a stem (`empty-stem`), a name ending in `.jp2`
  (`not-jp2`) and a width and a height above zero (`zero-dimension`); a
  resource whose file is refused is refused with it, before the ids of its
  parent's members are made. Two resources of one class with one file are
  `shared-file`; a stem that is a resource's id, or the file of resources
  of two classes, is `image-id-taken`. Both checks run over the resources
  a hook drops too; a dropped resource gets no Image. The Image rows stand
  in `resources` where the `images` node stands among the `class` nodes, by
  id.
- **Cover.** `cover` writes the resource's `image` as the Image of the
  member its curated key names. The values are counted as served: exactly
  one (`cover-count`); it holds no sentinel (`missing-in-value`). Then, on
  the stripped value: a cover two resources name is `shared-cover`,
  reported once with the cover as its subject, and is checked no further
  for either; else it names a member of this resource
  (`cover-not-own-page`); else that member was not dropped by a hook
  (`cover-is-calibration-scan`).
- **Annotations.** An annotation sits on a resource of an `on` class
  (`region-target`); an annotation link points at annotations and at
  resources of the `link-targets` classes (`link-target`). A link's
  targets are written annotations first, then resources in the declared
  order of their classes, then by id. An annotation the editors keep on a
  resource that a hook dropped is `region-on-dropped-page`, and a link they
  keep to such a resource is `link-to-dropped-page`.

### What the mapping derives, to the byte

This is what the store builder writes from these declarations.

- A `list "<name>"` is the row `lists (id, name)` = (`list-<name>`,
  `<name>`). A `list-node "<slug>"` is the row `list_nodes` with id
  `<name>-<slug>`, no parent and its position as `seq`, and one
  `list_node_labels` row per language. Rows are written by id, labels by
  (node, language).
- An Image's `representations` row has `kind` `image`, `uri` the
  `iiif-base`, the asset and `/full/max/0/default.jpg`, the file's width
  and height, `alt` NULL, `attribution` `<authorship> · <license>` (with
  U+00B7 between two spaces) and `seq` 0. Its two properties are named
  `authorship` and `license`; its owner's link is named `image`, `seq` 0.

### What the block leaves out

What the store builder does with what the block does not declare.

- **An undeclared class.** A resource whose class the block does not
  declare is refused (`unmapped-class`).
- **An undeclared property.** A property the block does not declare on a
  class is dropped, and nothing reports it: a re-pin that brings a new
  property changes nothing in the store. Incunabula's tests list the ones
  it leaves out (`areas/access/cpe/store-build/README.md`, "What the port serves and the
  block does not map"). A declared property that the snapshot never serves
  is silent unless the node sets `min`, which makes it `value-count`:
  inside one project's build it cannot be told from a legitimate state.
  Nothing refuses two projects that declare one shortcode either.

### What a missing block means

What the store builder writes when the block has no node of a kind.

- **No `list`:** the three list tables are empty.
- **No `images`:** files are not modelled and `representations` is empty.
- **No `member-of` anywhere:** `compound_parts` is empty.
- **No `annotations`:** the three annotation tables are empty, and a
  snapshot that serves an annotation is refused (`unmapped-class`), as is
  any resource whose class the block does not declare.
- `resources` and `properties` are never empty. A table may be empty
  exactly when the block that feeds it is absent; an empty table whose
  block is present is refused.

## `page` block

A top-level route. Required attributes:

- `id` — string, unique within the project. Used for trace-id and
  internal references.
- `route` — route pattern, leading slash relative to the project
  root.

Children of a `page` are body components, rendered in order.

### Route patterns

- Plain: `route="/books"` matches the exact path.
- Parameterised: `route="/books/:id"` matches `/books/<anything>`
  and exposes `:id` as a route parameter.
- Multiple parameters supported: `route="/books/:bookId/pages/:pageNum"`.

Route params are accessed inside component attributes as
`$paramName`:

```kdl
page id="book-detail" route="/books/:id" {
    resource-detail id="$id"
    compound-viewer source="$id" layout="pages" index="none" media="image"
}
```

### `resource-detail` — `title-property` and `class` attributes

The optional `title-property` attribute names the property whose value
is used as the detail page's `<h1>` title. When omitted or absent from
the resource, the title falls back to the resource id.

The optional `class` attribute declares which Class this detail page
renders. When set, the engine includes this page in its **class→route
map** — allowing `link` property values whose target Resource belongs to
that Class to render as clickable `<a class="int-link">` anchors pointing
at this page. This mirrors the `class="…"` attribute on `resource-list`.

Rules:
- `class` is only meaningful on a parameterised page (route has exactly
  one `:param`). A literal-id page (e.g. a "featured" page) does not
  contribute a class→route entry even if `class` is set.
- Two pages binding the same Class to a parameterised route is a
  **boot hard-fail** (`DiagnosticKind::AmbiguousClassRoute`).
- Omitting `class` is always valid — the page works normally; `link`
  values targeting that Class fall back to plain-text labels.
- **`class`'s value must name a Class that actually exists in the data**
  (engine-hygiene-3 item 2) — a typo (e.g. `class="Charakter"`) is a boot
  **hard-fail** (`DiagnosticKind::UnknownResourceDetailClass`), checked for
  every `resource-detail` that declares `class` (literal `id`, `$param`, or
  class-only), mirroring `UnknownResourceListClass`/
  `UnknownIncomingLinksClass`/`UnknownTimelineClass`. An unknown class also
  suppresses that page's `media=` four-check and class→media-property map
  registration (one mistake, one diagnostic — boot fails anyway).

Example:

```kdl
page id="character" route="/characters/:id" {
    resource-detail id="$id" class="Character" title-property="name" {
        show-properties "name" "description" "quote" "role" "keyword"
    }
}

page id="chapter" route="/chapters/:id" {
    resource-detail id="$id" class="StoryChapter" title-property="name" {
        show-properties "name" "character" "location"
    }
}
```

With the above config, a `link` property whose target is a `Character`
renders as `<a class="int-link" href="/alice-in-daschland/characters/<id>">`.
A `link` targeting a Class that has no bound page falls back to a plain
`span.v-text` label.

A **literal-id** detail page (e.g. a featured resource page) works the
same way and additionally exercises the boot-time resource-existence
check:

```kdl
page id="featured" route="/featured" {
    resource-detail id="CH_001" title-property="name" {
        show-properties "name" "description"
    }
}
```

The engine attaches the project's path-prefix at request time (per
`dsp-incubator cpe/docs/adr/0002-deployment-topology.md`) — project configs
write routes relative to the project root, the engine mounts them
under `/<project-id>`.

**Validation:**

- Routes are unique within the project (hard-fail).
- Every `$param` reference matches a declared route param on the
  enclosing page (hard-fail).
- Routes don't collide with engine-reserved routes: under a project,
  `/incunabula/_static/*` (the shared static files) and
  `/incunabula/_assets/*` (the project's own files, the `cover-image` asset
  route, see "`site` block" above) are the engine's, which is why a page
  route's first segment must not start with `_`; `/incunabula/` always exists
  (the project home).

**`resource-detail`'s recognised child nodes** (engine-hygiene-3 item 1):
`show-properties`, `group`, `narrative-for`, `incoming-links`, `imprint` —
exactly the five documented in this section and the ones below. Any other child node
name (a typo like `gruop`, `narative-for`, or `incomming-links`) is a boot
**hard-fail** (`DiagnosticKind::UnknownChildNode { component, child }`) —
config-internal, so it is checked once, before the `validate_with_data`
fork, and reaches both validator arms. The rest of the component still
lowers and reports normally; only the one unrecognised child is flagged.

### `group` and `narrative-for` (property groups + per-book intros, Incunabula Slice 3)

Two child nodes on `resource-detail`. `group label="<chrome-id>" "<name>"
"<name>" ...` declares a captioned **property group** (see `CONTEXT.md`):
positional string args name the member properties, in the group's own
display order; `label=` is a **chrome-string id, never literal text** — the
same id-only convention `order-name-label`/`order-name-desc-label` and
`search-bar`'s `placeholder`/`scope` attributes follow (contrast the
chrome-id-**or**-literal convention `site.eyebrow`/`description` use). An undeclared `label` id falls back to `ChromeMsg::PropertyGroupTitle`
(the group still renders). `group` is repeatable; each declared group renders
as its own `section.pgroup` after the flat `.props`, in declaration order.

Optional `empty="<chrome-id>"` (HdM Slice 3, id-only) gives the group a
designed empty state: when EVERY member property is absent on the record, the
group renders its caption plus the chrome text as an italic sentence
(`p.pgroup__empty`) instead of nothing. It is plain text (no inline markup).
Without `empty=` an all-absent group renders nothing, as before. A group with
at least one present member never shows the sentence.

```kdl
group label="work-participants" empty="work-no-participants" "librettist" "composer"
```

Optional `links-label="<chrome-id>"` (Incunabula Slice 6, id-only) closes the
group with one more row: the record's **resource-level annotation links**
(`ProjectReads::resource_links`: the other resource targets of every annotation link
that targets the record itself as a resource, `dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md`; annotation targets are
left out). The row is the group's last `div.prop.prop--links`,
captioned by the chrome string, with one line per link name and each partner
under it, linked to its bound detail page (plain text without one). The
caption stays singular whatever the partner count. No partners ⇒ no row; a
group whose only content is the row still renders. The store fetches the links
only for a page whose `resource-detail` declares a `links-label=` group, and
only for a full-page request (`PageScope::Page`; the Apparat is outside every `_compound` patch
target).

```kdl
group label="group-apparatus" links-label="related-book" "shelfmark" "physical" "catalogue-uri"
```

```kdl
resource-detail id="$id" class="Book" title-property="name" {
    group label="group-apparatus" "alt-title" "creator" "publisher" \
        "date-display" "pub-date" "pub-place" "shelfmark" "physical" \
        "catalogue-uri" "extent" "office"
}
```

`narrative-for "<resource-id>" file="<file>"` declares a per-resource
introduction: a literal target resource id (no `$param` interpolation) plus a
narrative file. When the id matches the page's resolved resource, the file's
content becomes the record's intro. `narrative-for` is repeatable (for
different resources reached by the same parameterised page) and reuses the
existing 3-rung narrative fallback chain (`resolve_narrative_content`) — the
same requested-language-variant → default-language-variant → base-file chain
a page-level `narrative` node uses. There is no second narrative-resolution
mechanism.

```kdl
resource-detail id="$id" class="Book" title-property="name" {
    narrative-for "narrenschiff-dt"     file="book-narrenschiff-dt.md"
    narrative-for "postilla"            file="book-postilla.md"
}
```

**Validation:** see the validation table below. A property named by a
`group` is checked for existence exactly like `show-properties`/
`title-property` (the two-tier hard-fail/sparse-warn treatment doesn't apply
here — resource-detail's SparseProperty is warn-only regardless of source);
`narrative-for`'s `file=` reuses `InvalidNarrativePath`/`NarrativeFileNotFound`
via the same `check_narrative_file` helper `narrative` uses, and its
resource-id's absent-from-data case gets its own advisory diagnostic
(`UnknownNarrativeForResource`) in the same Warning-severity, data-dependent
spirit as a `where`/`sort` property that exists in config but is sparse in
the data.

### `alt-title-property`, `author-property` and `imprint` (record header slots, book-page redesign)

Three optional header slots on `resource-detail`, rendered in
`header.record__head` after the title, each only when it has text:

```kdl
resource-detail id="$id" class="Book" title-property="name" \
    alt-title-property="alt-title" author-property="creator" {
    imprint {
        part property="pub-place"
        part property="publisher"
        part property="pub-date"
        part property="extent" unit="gloss-extent"
    }
}
```

- `alt-title-property` → `p.record__alt` (uppercase mono), and
  `author-property` → `p.record__author` (display italic). Both name a
  property stored as `text`/`richtext`; several values join with ", ".
  Read like `title-property` (a wrong type is `InvalidAttributeType`).
- `imprint { part property= unit= }` → `p.record__imprint`: the composed
  gloss's `part` grammar (see "Composed gloss" below) — values join with
  ", ", parts with " · ", an empty part is skipped, `unit=` (a chrome id
  with `{n}`) is valid on an `integer`/`decimal` part only. A date part
  renders at its precision with no precision word, plus ` · gregorianisch`
  for a Gregorian date when `site { date-precision-word #true }` is on. A
  child other than `part` is `UnknownChildNode { component: "imprint" }`;
  an empty `imprint {}` is an error.
- Every slot property leaves the **implicit** flat property list, as the
  title property does. A `group` is an explicit list, so remove a slot
  property from it yourself (listing it there renders it twice, with no
  warning).
- A slot property missing from the class is the `SparseProperty` Warning; a
  wrong value type is `IncompatibleGlossPart` with the role `alt-title`,
  `author` or `imprint part` (both validation arms).

### `incoming-links` (reverse-link list, Incunabula Slice 5 / G7)

A third child node on `resource-detail`, alongside `group`/`narrative-for`:
`incoming-links label="<chrome-id>" class="<Class>"
order="name"|"id"|"compound-seq" "<property>" "<property>" ...` declares the
record's **reverse-resolved link list** — every resource of the named
source `class` that points AT the rendered resource through one of the
listed `link` properties (positional string args, same shape as
`show-properties`). At most one per `resource-detail`
(`DuplicateIncomingLinks`, hard-fail). `label=` is a chrome-string id,
**id-only**, same convention as `group label=`/`search-bar`'s
`placeholder`/`scope` — an undeclared id is the Warning
`UnknownIncomingLinksLabel`, falling back to the source class's capitalised
plural. `order=` is a closed vocabulary:

- `"name"` (default when `order=` is absent) — by the source's display
  name, then id.
- `"id"` — by the source's id.
- `"compound-seq"` — by the source's position in its compound parent
  (grouped by parent id, then by `compound_parts.seq`; sources with no
  compound parent sort last).

Every ordering is total and deterministic — ties always break on the source
id, then on the matching link property's name.

An optional `limit=<integer>` attribute (engine-hygiene-3 item 6) caps the
number of rendered rows; absent ⇒ `DEFAULT_INCOMING_LINKS_LIMIT = 50`. A
present value must be a positive integer (`n >= 1`) — zero, negative, or a
non-integer value is `InvalidAttributeType`. This is an **attribute** on
`incoming-links` itself, unlike `resource-list`'s `limit` **child node**
(`incoming-links` has no child block at all — its positional args are
already the property list). When the list is capped (fewer rows render than
the true total match count), a `p.incoming__count` line follows the `<ul>`:
"Showing N of M" (localized in all four languages, both counts
thousands-grouped). There is deliberately **no pager and no "show all"**
affordance yet — see `docs/BACKLOG.md`'s "`incoming-links` capped list has no
way to reach the rest" entry.

**Grouped mode and row anatomy (HdM Slice 3 / G3).** Six further optional
attributes, all generic (no project literal in Rust), all absent ⇒ today's
flat list, byte-identical:

- `group-by="<name>"` partitions the rows into headed groups. The reserved
  word `via` groups by the matching link property, in the declared property
  order, and the head is that property's label. Any other value names a
  `list_value` or `text` property of the **linking** resource: list values
  order by list-node `seq` (the head is the node label in the request
  language), text values alphabetically (case-insensitive; case variants fold
  onto the first spelling met). A multi-valued group property uses its first
  value only. Empty groups are omitted; rows with no group key land in a
  trailing group **with no head** (never dropped, no invented copy). One row
  per `(source, via)` stays, so a source linking through two properties shows
  in both role groups.
- `layout="cards"|"columns"` (default `columns`; closed vocabulary, anything
  else is `InvalidAttributeType`): `cards` is a grid of per-group cards, `columns`
  is per-group sections over a multi-column list. Ignored without `group-by`.
- `note-label="<chrome-id>"` puts a note on the section head; the text must
  contain `{n}`, replaced by the row count (locale thousands-grouped). Not
  plural-aware, so write it plural-neutral ("Aufführungen · {n}"). Ignored
  without `group-by`.
- `row-label="<property>"` makes the link text a `text` property of the source
  (fallback: its `name`). Also applies to an ungrouped list.
- `row-aside="<property>"` adds a faint right-aligned `text` or `list_value`
  label (list values resolve to their node label). Also applies to an
  ungrouped list.
- `sort-property="<property>"` orders rows by a `text` property of the source
  (case-insensitive, missing values last, then source id, then `via`); it wins
  over `order=`. An ISO sort key sorts lexicographically, so a year-only key
  (`1859-00-00`) sorts **before** the dated rows of the same year.

**The grouped-mode ceiling.** A grouped list with no explicit `limit=` uses
`GROUPED_INCOMING_LIMIT_CEILING = 1000` instead of the default 50, because a
cut before grouping would silently drop trailing groups and undercount later
ones. An explicit `limit=` (even `limit=50`) is honoured. Group counts are
exact only when no overall cut occurred: if a target ever exceeds the cap, the
"Showing N of M" line renders under the groups. No real HdM target is near it
(max fan-in 105 per Work, 84 per Person).

In grouped mode the `via` marker on rows is dropped when grouping by `via` or
when only one link property is declared; with several properties and a
property `group-by` it stays. A grouped list with no rows renders **nothing**
(no caption, no note); an ungrouped one keeps "Nothing links here.".

```kdl
incoming-links label="work-performances" class="Performance" "work" \
    group-by="decade" layout="cards" row-label="date" row-aside="source" \
    sort-property="sort-key" note-label="work-performances-note"
incoming-links label="person-works-by-role" class="Work" \
    "librettist" "composer" "principal" "actor" "interpreter" \
    group-by="via" layout="columns"
```

```kdl
resource-detail id="$id" class="BorderStrip" title-property="name" media="image" {
    show-properties "description" "side" "page-uses" "comment"
    incoming-links label="pages-using" class="Page" order="compound-seq" \
        "strip-left" "strip-right"
}
```

Renders as `section.pgroup.incoming` — the same `.pgroup` caption idiom
`group` uses (`header.pgroup__head > span.pgroup__title`) — followed by
`ul.incoming__list > li.incoming__item` rows, one per `(source, via)` pair
(a source linking through two different properties yields two rows). Each
row is the source's name plus a faint `span.incoming__via` marker naming the
matched property (its declared label, or humanized). A resolved-to-zero list
renders `p.resource-list__empty` / `ChromeMsg::IncomingLinksEmpty` instead of
the `<ul>`.

**The href — two rungs, then a plain label:**

1. The source's **own** bound detail page (`class="…"` on a parameterised
   `resource-detail`), exactly like any other internal `Link` value.
2. Else, if the source is a member of a compound whose **parent** has its
   own bound detail page: a link into that parent's reader at
   `?page=<member-id>` (no `#compound-viewer` fragment, owner decision
   2026-09-29) — a **fresh** query (the linking
   page's own `lang=`/`img=`/etc. never leak into it), built by
   `links::member_href_for`, the same builder the reader's own
   rail/pager and page picker use. This rung's
   assumption — that the parent's detail page actually hosts a reader — is
   not itself validated (`docs/BACKLOG.md`).
3. Else a plain `span.incoming__name` — no link at all.

**Validation:** see the validation table below.

### `timeline` (Incunabula Slice 4 — the first per-project custom component)

`timeline` is **not** one of the v1 components in this file's earlier
sections — it is the escape hatch's first tenant, registered
`(project = "incunabula", name = "timeline")` in
`areas/access/cpe/core/src/validate/project_components.rs` and
`areas/access/cpe/components/src/component/project/mod.rs`; see
`dsp-incubator cpe/docs/adr/0011-per-project-components.md` and
`COMPONENT_LIBRARY.md`'s "Custom components (escape hatch)" section and
`## \`timeline\`` entry for the full rendering contract. It is
documented here because its config attributes go through the same KDL
lowering and validation machinery as every v1 component.

```kdl
timeline class="Book" date-property="pub-date" lane-property="office" \
         extent-property="extent" axis-start=1470 axis-end=1506
```

- `class` (required, string) — the Class whose resources become
  markers.
- `date-property` (required, string) — the `date`-typed property
  positioning each marker on the axis.
- `lane-property` (required, string) — the `list_value`-typed property
  naming each marker's lane.
- `extent-property` (optional, string) — an `integer`-typed property
  appended to the marker's accessible label; absent ⇒ the segment is
  omitted.
- `axis-start` / `axis-end` (both required, integer) — the inclusive
  first/last year of the axis. Explicit config, never derived from the
  data; `axis-end` must exceed `axis-start` by at least three years.

**Validation surface (boot-time):**

| Diagnostic | Severity | Trigger |
| --- | --- | --- |
| `MissingRequiredAttribute` / `InvalidAttributeType` | Error | `class`/`date-property`/`lane-property` absent, or any attribute wrong-typed (config-internal, span-located — no `page` field) |
| `TimelineAxisOutOfRange` | Error | `axis-start`/`axis-end` doesn't fit in an `i32` (config-internal) |
| `InvalidTimelineAxis` | Error | `axis-end <= axis-start`, or the span is under three years (config-internal) |
| `UnknownTimelineClass` | Error | `class` names a Class absent from the data (data-dependent, both `validate_with_data` arms) |
| `UnknownTimelineProperty` | Error | `date-property`/`lane-property`/`extent-property` absent from the data entirely (data-dependent) |
| `MissingTimelineProperty` | Error | `date-property` or `lane-property` exists in the data but on no resource of `class` — a deliberate hard-fail, unlike the usual sparse-warn tier: without a date the marker is skipped, without a lane it falls to the `—` lane, so the component would render nothing useful (data-dependent) |
| `SparseTimelineProperty` | Warning | `extent-property` exists in the data but on no resource of `class` — the extent segment is just omitted (data-dependent) |
| `IncompatibleTimelineProperty` | Error | a named property is stored on `class` with the wrong `value_type` (`date-property` needs `date`, `lane-property` needs `list_value`, `extent-property` needs `integer`) (data-dependent) |

### `narrative`: `companion=` and `side=`, invite groups and heading ids (layout pass)

```kdl
page id="narrenschiff" route="/narrenschiff" {
    narrative file="narrenschiff.md" companion="rail" side="end"
}
```

- `companion="rail"|"text"` and `side="end"|"start"` are optional attributes
  on the page's **first** `narrative`; each overrides the `site {}` default
  for that page. On a later narrative of the same page either one is
  `NarrativeLayoutNotFirst` (it would be read and ignored). A wrong type or
  an unknown value (`companion="figures"` included: marginal figures are not
  built) is `InvalidAttributeType`.
- **Two combinations are accepted without a diagnostic.** `companion="rail"`
  on a page that ends up with fewer than two rail entries, or whose first
  narrative has neither a title nor an intro: the page renders as `text`
  (the count depends on the narrative's content per language). And `side=`
  together with `companion="text"`: the side has no effect until the rail is
  on.
- **Rail entries and generated ids.** The rail lists the page's top-level
  `##` headings, across all its narratives, in body order. Each gets an id:
  the authored `{#id}` (see the next section), else `sec-<n>`, where `n` is
  its position among those headings **in the language variant being
  served**. A generated id therefore changes when a variant has one section
  more or fewer, and when a section is inserted before it. **Write an
  authored `{#id}`, on the same `##` in every variant, for any section that
  must be linkable across languages or whose link must survive an edit.**
  `sec-` and `narr-rail-cap` are reserved (`InvalidHeadingId`).
- **Invite groups.** A `##`, an optional paragraph, then one or more groups,
  each a `###`, an optional paragraph and a link list of
  `[Name](/route) — gloss` items, renders as a section with a grid of
  groups (Incunabula's "Die Offizinen"). Every group's list must be a clean
  invite list; one that is not makes the whole section render as plain
  headings and lists, silently. A group `###` gets a rail entry never, and
  an id only when you write one.
- **Two narratives on one page must not share an authored id**:
  `DuplicateHeadingIdOnPage`. The check follows the language fallback, so a
  base-only `a.md` beside `b.de.md` collides on the German page; the same
  file named twice on one page collides with itself. The same id on two
  different pages is fine.

| Diagnostic | Severity | Trigger |
|---|---|---|
| `UnknownSiteValue` | Error | `site { narrative-companion \| narrative-side \| text-align }` holds a string outside its two values (config-internal) |
| `NarrativeLayoutNotFirst` | Error | `companion=` or `side=` on a `narrative` that is not its page's first (config-internal, before the validation fork) |
| `DuplicateHeadingIdOnPage` | Error | two narratives of one page carry the same authored heading id in the content one visitor is served (narrative content, both `validate_with_data` arms) |
| `TileRatioWithoutGrid` | Error | `resource-list tile-ratio=` without `grid=#true` (config-internal, before the fork) |
| `InvalidAttributeType` | Error | new triggers: `companion=` or `side=` of the wrong type or an unknown value; a `tile-ratio=` outside the `W:H` grammar |

### Figure blocks: heading ids and the slot attributes (Incunabula Slice 7)

A `##` heading in a narrative may carry an explicit id, `## Viermal gedruckt
{#four}` (only the id is kept; the editorial parser alone enables heading
attributes). A **per-project** component placed after a `narrative` on the same
page may then carry the **slot attributes**, read for any registered
component and lowered into the page's `figure_slots`, not the component's IR:

- `section="<id>"` — render the component after that `##` section of the
  nearest preceding `narrative` (figure mode; see `COMPONENT_LIBRARY.md`'s
  `narrative` entry). The id must be on a `##` in every present language
  variant.
- `count="<chrome-id>"` — a count line under the block.
- `more-label="<chrome-id>"` + `more-route="<route>"` — a more link under the
  block; the two come as a pair. A label may hold `{n}`, filled from the block
  when its registry row has a `slot_n` hook (`strip-row`: the class total).

`count=`, `more-label=` or `more-route=` without `section=`, and one of the
more pair without the other, are `MissingRequiredAttribute`.

```kdl
page id="woodcuts" route="/woodcuts" {
    narrative file="woodcuts.md"
    woodcut-cases section="within-book" layout="compact" media="image" count="showcase-count-within-book" { … }
}
```

Heading ids are lowercase ASCII words joined by `-`, unique within a variant,
and must not be an engine-owned id or prefix (`main`, `compound-viewer`,
`cv-title`, `crumb-data`, …; `annotation-`, `case-`, `crumb-`, `cv-pk-`, …):
the list lives in `areas/access/cpe/core/src/validate.rs` (`RESERVED_HEADING_IDS` /
`RESERVED_HEADING_ID_PREFIXES`).

### `woodcut-cases`, `edition-cards`, `strip-row` (Incunabula Slice 7)

Three more Incunabula components (`dsp-incubator cpe/docs/adr/0011-per-project-components.md` registry rows); each **requires**
`section=`, so none renders outside a slot. Full rendering contracts:
`COMPONENT_LIBRARY.md`.

```kdl
woodcut-cases section="editions" layout="edition" media="image" {
    case schramm=1124 title="Kap. 13 · Von der Buhlerei" remark-link="jwSBbhl4U_6dIifcGTn0Qg" {
        crop "bsX4cDKkW3GMC8_Ms2tOpw" alt="Holzschnitt zu Kap. 13: Von der Buhlerei."
        crop "gAfAiG_-U4itgN8Gdr9Ipw" alt="Holzschnitt zu Kap. 13: Von der Buhlerei."
    }
    case remark="showcase-remark-e2" { crop "…"; crop "…"; crop "…" }
    case remark-link="ggDWsmYKUd2RWy3w9PcD1w" { crop "…"; crop "…"; detail "…" alt="…" }
}
woodcut-cases section="between-books" layout="compact" media="image" {
    page-case link="voBOw9dGXw-f3h9Tn76ZBw" remark-excerpt="Holzschnitt: Anbetung des Kindes." { page "…"; page "…" }
}
edition-cards section="editions" media="image" {
    book "narrenschiff-dt" lang-label="edition-german"
    imprint { part property="pub-place"; part property="extent" unit="gloss-extent" }
}
strip-row section="strips" class="BorderStrip" media="image" \
    uses-property="page-uses" min-uses=9 limit=12 \
    more-label="strips-all" more-route="/strips"
```

- **`woodcut-cases`**: `layout` (required, `edition` | `four` | `compact`),
  `media` (required, the Page → Image link property). A `case` has 2–4 `crop
  "<annotation id>"` (optional `alt=`, verbatim from the page's description)
  and at most one `detail "<annotation id>"` (`alt=` = the annotation's own
  remark; not in `compact`); optional `schramm=` (integer) and `title=`. A
  `page-case link="<link id>"` names the link's two `page "<id>"` targets. A
  case takes at most one of `remark-link="<link id>"` (the link comment,
  verbatim), `remark="<chrome-id>"` (authored) and `remark-excerpt="…"` (a
  line of one of the case's page descriptions). Case keys (the DOM ids) are
  derived: the first crop's annotation id, or the link id.
- **`edition-cards`**: `media` (required); `book "<id>"` children (optional
  `lang-label="<chrome-id>"`) and at most one `imprint { part property="…"
  unit="<chrome-id>" }`, the record header's grammar.
- **`strip-row`**: `class`, `media`, `uses-property` (integer-typed),
  `min-uses` (≥ 0) and `limit` (1–100), all required.

Every attribute is read with the reporting readers, so a wrong-typed value is
`InvalidAttributeType`, never silently absent.

| Kind | Severity | Trigger |
|---|---|---|
| `FigureSectionWithoutNarrative` | Error | a `section=` on a component with no `narrative` before it in the page body (config-internal) |
| `UnknownFigureSection` | Error | the `section=` id is not the id of a `##` in every present variant of that narrative (an id on `#` or `###` does not count) (narrative content, both `validate_with_data` arms) |
| `FigureSectionHoldsInviteList` | Error | the named `##` is followed by an invite list or by invite groups; a figure block cannot attach to an invite section (the `<h2>` does carry its id since the layout pass) (narrative content, both arms) |
| `InvalidHeadingId` | Error | a heading id outside `[a-z0-9]+(-[a-z0-9]+)*`, used twice in one variant, or equal to (or prefixed by) an engine-owned id; over every file a `narrative` names (narrative content, both arms) |
| `UnresolvedFigureMoreRoute` | Error | `more-route` names no declared page route, or a parameterised one (config-internal, after pages lower) |
| `UnknownFigureLabel` | Error | `count` or `more-label` names no chrome string (config-internal, post-lowering pass) |
| `InvalidShowcaseCase` | Error | an unknown `layout`, a crop count outside 2–4, two remark sources, a second `detail`, a `detail` in a `compact` block, a page case without exactly two pages, a block with no case, a case key used twice in one block or across a page's blocks (config-internal) |
| `UnknownShowcaseLabel` | Error | `remark=` or `lang-label=` names no chrome string (config-internal, post-lowering pass) |
| `UnknownShowcaseAnnotation` | Error | a `crop`/`detail` id is not an annotation (data, both arms) |
| `UnknownShowcaseLink` | Error | a `remark-link` or `page-case link` is not an annotation link (data, both arms) |
| `ShowcaseLinkMismatch` | Error | a case's `remark-link` targets none of its crops; a page case's pages are not exactly its link's page targets, or its `remark-link` targets none of them (data, both arms) |
| `UnknownShowcasePage` | Error | a `page` id is not a Page resource (data, both arms) |
| `ShowcaseExcerptMismatch` | Error | a crop `alt` or a `remark-excerpt` is not part of one (`\n`-separated, trimmed) line of the page description; a `detail` `alt` is not the annotation's remark (data, both arms) |
| `ShowcaseSchrammMismatch` | Error | `schramm=N` while the first crop's page comment has no `Abb. N` (data, both arms) |
| `MissingCropDimensions` | Error | a crop's page image has no stored width or height, or a zero one (data, both arms) |
| `CropImageNotIiif` | Error | a crop's page image is sized, but its URI is not a safe URL (`is_safe_external_url`: http, https or root-relative) with the IIIF `/full/max/` segment the region URL is built from, so the crop would render as a failed bed (data, both arms) |
| `UnknownShowcaseBook` | Error | an `edition-cards` `book` is not a resource (data, both arms) |
| `UnknownShowcaseClass` | Error | `strip-row class` names a Class absent from the data (data, both arms) |
| `IncompatibleShowcaseProperty` | Error | `strip-row uses-property` has no value on `class`, or a value that is not `integer` (data, both arms) |

`media=` on `edition-cards` and `strip-row` goes through the shared media
checks (`validate::media_property::check_single_media_property`: `UnknownMediaProperty`,
`MediaPropertyTargetNotImagelike`, …), and an `edition-cards` imprint takes
the record header's checks: a part of the wrong type is
`IncompatibleGlossPart`, a `unit=` naming no chrome string `UnknownGlossUnit`,
a unit translation without `{n}` `GlossUnitMissingPlaceholder`.
`woodcut-cases media=` has no check of its own: a wrong name leaves every
crop's page without an image and surfaces as `MissingCropDimensions`.

## Component invocation

Components are KDL nodes. They take attributes (KDL attributes),
positional arguments (KDL positional args), and children (KDL child
block).

Conventions:

- **Single-value attributes**: KDL attributes — `id="book-list"`,
  `class="Book"`.
- **List-valued attributes** (like `search-bar`'s `searches` taking
  many property names): a named attribute followed by positional args —
  `searches="title" "description"`. The engine reconstructs the full
  list from the attribute value plus the trailing positional args.
- **Sub-configurations and list-valued child nodes** (where, sort,
  limit, facets, and `show-properties "title" "publisher" …`):
  child nodes — see examples. (`show-properties` is a child node of
  `resource-detail` whose positional string args name the properties,
  not an attribute on the component.)

The full example sketch (from `cpe-poc.md` §8.5, updated for Phase 5):

```kdl
page id="books" route="/books" {
    narrative file="books-intro.md"

    facet-panel target="book-list" {
        // Only kind="list" is implemented in this slice; other kinds
        // (date-decade, date-year, text, boolean, range) boot-hard-fail
        // with UnsupportedFacetKind and are deferred to a future slice.
        facet property="publisher" kind="list"
        facet property="language" kind="list"
    }

    search-bar target="book-list" searches="title" "description"

    // resource-list MUST have an id when it is a facet-panel target.
    resource-list id="book-list" class="Book" {
        sort property="title"
        per-page 20
    }
}
```

For each component's full attribute / child list, see
`dsp-incubator cpe/docs/COMPONENT_LIBRARY.md`.

## Data-binding primitives

These appear as **nested KDL nodes** inside components like
`resource-list`, not as their own top-level concept. Per
`cpe-poc.md` §8.4.3, this is deliberate — keep config in one
language, not config + a separate query DSL.

### `where`

Equality, IN, and range filters. Multiple `where` nodes
AND-combine:

```kdl
resource-list class="Book" {
    where property="featured" equals=true
    where property="publishedYear" range-start=1480 range-end=1500
}
```

The exact attribute set per `where` form (`equals=`, `in="..." "..."`,
`range-start=`, `range-end=`) is finalised at implementation.

### `sort`

```kdl
resource-list class="Book" {
    sort property="publishedYear" direction="desc"
    sort property="title"
}
```

Multiple `sort` nodes apply in order (primary, secondary, …).
`direction` defaults to `asc`. This config `sort` sets the list's
**default** order.

**Text sorts are case-insensitive** (since 2026-09-29): every text-valued
sort — config `sort`, a declared `order-by`, the built-in name rows, and
`incoming-links order="name"` — compares with SQLite's `COLLATE NOCASE`, so
`A`, `b`, `C` sort as `A, b, C` and values equal under case-folding fall back
to the resource-id tie-break. NOCASE folds **ASCII `A–Z` only**: an `Ä…`,
`É…` or other non-ASCII initial still files after `z…`. Integer, decimal and
ISO-date sort values are unaffected (SQLite applies a collation only between
two text values). **Facet-option ordering is not case-insensitive:** a
`facet-panel` group lists its options by count, then label in plain byte
order — an accepted inconsistency, since the label only breaks ties between
equal counts.

**Runtime `?sort=` override (Phase 5 browse redesign):** the resource-list's
sort control adds a `?sort=` query param on top of the config grammar
above. It is a fixed, closed vocabulary — `default` / `name` / `name-desc` /
`class` / `id` (parsed by `Sort::parse_wire`) — **not** a raw property name
like config `sort property="…"`. When present and recognised, it overrides
the page's configured `sort` nodes for that request only; when
absent/unknown (including `default`), the configured order applies
unchanged. See `CLAUDE.md`'s Non-obvious constraints and
`COMPONENT_LIBRARY.md`'s `resource-list` contract.

### `order-by` (declared ordering options, Incunabula Slice 8 / G9)

```kdl
resource-list id="book-list" class="Book" media="image" {
    sort property="pub-date"                                // the default order
    order-by key="extent" property="extent" direction="desc" label="order-book-extent"
}
```

A repeatable `resource-list` child that adds a named ordering option to
this list's `?sort=` vocabulary, on top of the built-ins. `?sort=` still
never takes a raw property name — it takes a **declared key**, so the
wire vocabulary stays closed per list and a typo still falls back to the
default row ("As curated", unless `order-default-label` relabels it).
Options keep their config order. Don't declare an option that repeats the
default order: relabel the default row instead (Incunabula's "Nach Datum"
is its relabelled default, not an `order-by key="date"`).

- `key="…"` — **required**, the `?sort=` wire token. Must be non-empty and
  use only `[a-z0-9-]` (`InvalidOrderKey`, hard-fail — the bin never
  percent-decodes `?sort=`, so any other character would be form-encoded
  on submit and never match). Must not be a built-in token — `default`,
  `name`, `name-desc`, `class`, `id` (`ReservedOrderKey`, hard-fail) — and
  must be unique within the list (`DuplicateOrderKey`, hard-fail; the same
  key on two different lists is fine).
- `property="…"` — **required**, the property to order by. Boot-validated
  exactly like `where`/`sort`: hard-fail (`UnknownResourceListProperty`)
  if it exists nowhere in the data, warn (`SparseResourceListProperty`) if
  no resource of the list's class carries it. It orders with the same
  language-aware sort key and NULLs-last rule as config `sort`.
- `direction="asc"|"desc"` — optional, defaults to `asc`. Any other value
  is `InvalidAttributeType`.
- `label="…"` — **required**, a `chrome { string }` **id** for the option's
  row in the ordering control — id-only, never literal text (the
  `order-name-label` convention). An id that is not a declared chrome
  string is `UnknownOrderLabel` (hard-fail — the engine has no default
  word for an arbitrary property).

Every attribute is read with a reporting reader, so a wrong-typed value
(`key=#true`, `property=1`) is `InvalidAttributeType`, never a
silently-dropped option.

`?sort=<key>` resolves against the declared options (on top of the four
built-ins), the ordering control renders a row per declared option in config
order, and the Copy-link URL carries the resolved key — see
`Project::resolve_order_options` and the amended Non-obvious constraint on
`?sort=`'s vocabulary in `dsp-incubator cpe/CLAUDE.md`.

### `limit` and `per-page`

```kdl
resource-list class="Book" {
    limit 6        // hard cap; not paginated
    per-page 20    // page size for paginated lists
}
```

`limit` and `per-page` are mutually exclusive: a list either
shows up to N items (no pagination) or is paginated at N items per
page.

### `gloss` and `chip` (annotated rows, RL-2)

The RL-2 "annotated rows" browse design lets each list row carry, under
the resource name, a one-line **gloss** and a set of **chips** built from
the resource's own properties:

```kdl
resource-list id="character-list" class="Character" {
    sort property="name"
    per-page 24
    gloss property="description"   // one-line summary under the name
    chip property="role"           // a chip group (repeatable)
    chip property="keyword"
}
```

- `gloss property="…"` — at most one per list. The property's **first**
  language-picked text/richtext/date value renders as `p.rl-annot__gloss` (a
  scannable summary, CSS-clamped to two lines; the full text still lives on
  the detail page). Richtext renders as **plain text**, not Markdown.
  Absent/empty ⇒ no gloss line (never the resource id).
- `gloss { part …; teaser … }` — the **composed** form, instead of
  `property=` (never both). See "Composed gloss" below.
- `chip property="…" kind="term"|"keyword"` — repeatable; each names a
  property whose values render as chips in `div.rl-annot__meta`. Absent
  `kind` ⇒ the shape follows value cardinality, exactly like
  `resource-detail`: a property with a **single** list value renders as a
  `.term` pill; a **multi-valued** list renders as a run of `.keyword`
  pills (path/leaf split). An explicit `kind` (A12.3) overrides that
  cardinality-derived shape for **every** label of that property — `kind=
  "term"` on a multi-valued property renders one `.term` per label (not
  "first only"); `kind="keyword"` on a single-valued property renders that
  one label as `.keyword`. Any other value is `InvalidAttributeType` (no
  new `DiagnosticKind`). No CSS change either way — `.term` and `.keyword`
  already differ only in background token + `.keyword`'s path/leaf split.
  Non-list and unresolved values contribute nothing regardless of `kind`; a
  row with no chip values emits no `.meta` box.
  Boot validation checks a chip property's **existence on the class, not its
  value type**, so `chip property="office"` pointing at a `text` property is
  a silently rendered nothing — no error, no warning. This has bitten for
  real: a config author and a data author independently chose `list_value`
  vs `text` for the same property, and the mismatch was invisible until both
  wrote it up. Verify the property's stored `value_type` is `list_value`
  before declaring it as a chip.

Both are optional and generic — there is no built-in "role"/"theme"
concept; the role-vs-theme look is purely the single-vs-multi split, unless
overridden by `kind`. The referenced properties are boot-validated for
existence on the class, same two-tier hard-fail / sparse-warn treatment as
`where`/`sort`. See `COMPONENT_LIBRARY.md`'s `resource-list` contract.

#### Composed gloss

```kdl
resource-list id="book-list" class="Book" media="image" {
    gloss {
        part property="publisher"
        part property="pub-place"
        part property="pub-date"
        part property="extent" unit="gloss-extent"   // chrome string "{n} Seiten"
        teaser property="teaser"
    }
}
```

- Each `part property="…"` renders **every** language-picked value of its
  property, joined with ", " (the legacy form shows only the first). Parts
  are joined with " · "; a part with no value is **skipped**, so no doubled
  separator and no placeholder dash. At least one `part`; a property may be
  named by only one part.
- A part must be `text`, `richtext`, `date`, `integer` or `decimal`. Text
  renders plain, a date through the same plain date text the detail page
  shows (no precision word), a number as bare digits.
- `unit="<chrome-id>"` — optional, only on an `integer`/`decimal` part: a
  `chrome { string }` id whose translations are templates containing
  `{n}` (`"{n} Seiten"`, `"{n} pages"`). The number is substituted as-is:
  there is **no plural handling** and no digit grouping, so a unit suits
  values that are never 1. A unit with no translation for the request
  language renders bare digits, never the id.
- `teaser property="…"` — optional, at most one, `text`/`richtext`: an
  authored one-sentence line appended after " — " in the **same**
  paragraph, or standing alone when no part rendered. A row whose teaser
  rendered is `p.rl-annot__gloss.rl-annot__gloss--teaser`, which clamps at
  **3 lines** instead of 2. Store it language-tagged (one value per
  language), or the language fallback will show one language to every
  visitor.
- A grid text tile (no image) shows the legacy `gloss property=` only, never
  a composed line.

Every part and teaser property is boot-checked for existence on the class
like `where`/`sort`. Boot errors: `UnknownGlossUnit` (the `unit=` id is not
a declared chrome string), `GlossUnitMissingPlaceholder` (a unit
translation lacks `{n}`), `IncompatibleGlossPart` (a part stored as
`list_value`/`link`/`boolean`, a non-text teaser, or `unit=` on a
non-numeric part), `UnknownChildNode` (a child other than `part`/`teaser`),
and `InvalidAttributeType` for `property=` plus children, no `part`, a
second `teaser` or a repeated part.

### `order-name-label` and `order-name-desc-label` (per-list ordering labels, A12.4)

Two optional single-value attributes on `resource-list`, each a
project-declared `chrome { string }` **id**, resolved exactly like
`search-bar`'s `placeholder`/`scope` — id-only, never literal text. An
absent id, or one that does not resolve to a translation for the current
language, falls back to the built-in `ChromeMsg::OrderNameAsc`/
`OrderNameDesc` default label, never to the literal attribute text (see
below).

```kdl
resource-list id="performance-list" class="Performance" \
    order-name-label="order-perf-name-asc" \
    order-name-desc-label="order-perf-name-desc" {
    sort property="sort-key"
}
```

They override the ordering control's "Name A–Z"/"Name Z–A" row labels
**only** — `?sort=name`/`?sort=name-desc`'s wire values, `BUILTIN_ORDER_KEYS`,
and `Sort::parse_wire` are all unaffected; this is a label-only override for a
class whose display `name` is not actually a name (HdM's Performance
`name` is a composed date+title, so "Name A–Z" would describe a sort it
does not perform). Absent ⇒ the `ChromeMsg::OrderNameAsc`/`OrderNameDesc`
defaults, unchanged — never an empty label, even when the id is declared
but does not resolve for the current language. The "As curated"/
"Identifier" rows are never overridable. *Amended 2026-09-29:* the
As-curated row is overridable via `order-default-label`; Identifier stays
fixed.

### `order-default-label` and `order-name-property` (default row and title order)

```kdl
resource-list id="book-list" class="Book" \
    order-default-label="order-book-date" \
    order-name-property="sort-key" \
    order-name-label="order-book-title-asc" \
    order-name-desc-label="order-book-title-desc" {
    sort property="pub-date"
}
```

- `order-default-label="<chrome-id>"` — relabels the built-in "As curated"
  (`default`) row, on the same silent-fallback tier as `order-name-label`:
  absent or unresolvable ⇒ `ChromeMsg::OrderCurated`; a wrong-typed value is
  `InvalidAttributeType`. **No boot check ties the label to the list's
  `sort`:** relabelling the default row makes the author responsible for the
  label describing what that `sort` actually orders by (Incunabula's default
  *is* the date order, so "Nach Datum" is true there).
- `order-name-property="<property>"` — the built-in `name`/`name-desc` rows
  (`?sort=name`, `?sort=name-desc`) order by this property instead of
  `name`. The wire tokens, `BUILTIN_ORDER_KEYS`, `normalize_sort_key` and
  `Sort::parse_wire` are unchanged; the config `sort` is never rewritten.
  Boot-validated for existence like `sort` (`UnknownResourceListProperty`/
  `SparseResourceListProperty`). Pair it with `order-name-label`/
  `order-name-desc-label` when the rows no longer order by the name.

### `media` (embedded-media axis)

A single-value attribute — like `class`, not a child node — on
`resource-detail` and `resource-list` (and, opt-in, `compound-viewer` —
see the last bullets below), naming the `link`
property that points at a resource's (or each row's) embedded image set:

```kdl
resource-detail id="$id" class="Character" title-property="name" media="image" {
    show-properties "description" "role" "keyword"
}

resource-list id="character-list" class="Character" media="image" {
    sort property="name"
}
```

- On `resource-detail`, `media` drives the **record viewer** — the
  designed replacement for the old per-resource image carousel (see
  `COMPONENT_LIBRARY.md`'s `resource-detail` entry). Requires a
  resolvable class (this component's own `class=`, or — for a literal
  `id` — the data-looked-up class of that one resource); a class-less
  `$param` page hard-fails (`UnresolvableMediaClass`).
- On `resource-list`, `media` drives each row's thumbnail (shown left of
  the text on desktop, right of the gloss on phones).
- **Validation tiers** — same two-tier treatment as `where`/`sort`: hard-fail
  (`UnknownMediaProperty`) if the named property doesn't exist anywhere on
  the class; sparse-warn (`SparseMediaProperty`) if it exists on some
  resources but not all. Additionally hard-fail if the property isn't
  `link`-typed (`IncompatibleMediaProperty`), or targets a class whose
  resources carry no `representations` rows
  (`MediaPropertyTargetNotImagelike`).
- **The class→media-property map.** The engine builds one project-wide
  `class → media-property` map, registered in a fixed order: `resource-detail
  media="…"` declarations first (parallel to the class→route map built from
  `resource-detail class="…"`), then `compound-viewer media="…"`
  declarations (see the last bullets below), then `resource-list
  media="…"` declarations. For each class, the **first** registration wins;
  a later declaration (from any of the three component kinds) naming a
  **different** property for a class that's already registered is a boot
  hard-fail (`ConflictingMediaProperty` — its two fields are
  `registered_property`, the class's already-registered property, and
  `declared_property`, this declaration's conflicting one; both are
  source-neutral, since a compound-viewer registration is neither a
  "detail" nor a "list"). A declaration for a class with no earlier entry
  is fine; it just doesn't suppress anything on a surface that never had
  the map entry to suppress against (suppression — excluding the media
  property from the plain `.props` list, or mounting a compound member's
  media fallback — only ever applies where the map has an entry).
- **On `compound-viewer` (opt-in for `layout="contents"`, REQUIRED for
  `layout="pages"`; Incunabula Slice 2 decision 11, tightened by Slice 3b
  D3).** `compound-viewer source="$id" layout="contents" index="rail"
  media="image"` names the **member**→image `link` property. When set, the
  reader shows the current member's first linked Image — a plain,
  non-interactive `<img>` in `layout="contents"`, or the record viewer's own
  `<image-viewer>` unit in `layout="pages"` (see `COMPONENT_LIBRARY.md`'s
  `compound-viewer` entry) — and that property is suppressed from the
  member's property rows. Absent ⇒ no member image at all in
  `layout="contents"` (today's behaviour; Alice's readers don't set it);
  **absent is a boot Error in `layout="pages"`**
  (`DiagnosticKind::PagesLayoutWithoutMedia` — the pages layout is scan-first
  and has nothing to show without it). Validated with the same four checks
  as `resource-detail`'s `media`, run against the member class(es) taken
  **from the data**: for a literal `source`, the distinct classes of its
  `compound_parts` children; for a `$param` source, the distinct classes of
  **every** `compound_parts` child in the project. An empty class set (no
  members yet) skips the check — it is never `UnresolvableMediaClass`. **It
  feeds the class→media-property map above** (registered right after
  `resource-detail media=`, before `resource-list media=` — see that
  bullet for the fixed registration order and `ConflictingMediaProperty`).
  For a literal `source`, the parts' classes register; for a `$param`
  source, every compound member class in the project registers (today's
  resolution — latent if that reader turns out to be a member of *other*
  compounds too, accepted and recorded here). **A `resource-detail`
  on the same resource must not also set `media`**
  (`DiagnosticKind::CompoundMediaOwnership`, `dsp-incubator cpe/docs/adr/0012-one-member-navigator-declared-layout.md` — the navigator owns
  the compound's images), and a `layout="pages"` reader paired with a
  `resource-detail media=` on the same page is `DiagnosticKind::
  MultipleImageViewers` (the record-viewer unit's signals are page-global —
  one viewer per page).
- The reserved query key `strip` (the record viewer's thumb-strip fold
  state) is
  added to `RESERVED_FACET_PROPERTIES` alongside the existing reserved
  keys — a facet property named `strip` hard-fails at boot, matching the
  `q`/`sort`/`member`/… pattern. (An earlier `i` reservation, for the
  since-rejected-and-removed deep-zoom takeover's sequence index, no
  longer exists.)

### `grid` (grid-mode plan)

A single-value KDL boolean attribute on `resource-list` only, opting the
list into a List↔Grid view toggle:

```kdl
resource-list id="character-list" class="Character" media="image" grid=#true {
    sort property="name"
}
```

- Type: KDL bool (`#true`/`#false`), scope: the `resource-list` block only
  (no `resource-detail` counterpart — a detail page has no "list" to
  present as tiles).
- **Requires `media`** on the same `resource-list` — `grid=#true` with no
  `media` attribute is a boot hard-fail (`GridWithoutMedia`); grid mode
  renders a wall of image tiles and has nothing to show without an
  embedded-image property to source them from.
- Default `false`. A wrong-typed value (e.g. `grid="true"` as a string,
  `grid=1` as an integer) is `InvalidAttributeType`, not a silent `false`.
- Effect: adds a List↔Grid toggle to the list's `.surf-head`; `?view=grid`
  renders the same, already-narrowed items as the browse gallery (`ul.gm`
  tiles) instead of rows. See `COMPONENT_LIBRARY.md`'s `resource-list`
  entry for the full markup contract.
- The reserved query key `view` (the toggle's own state) is added to
  `RESERVED_FACET_PROPERTIES` alongside the existing reserved keys — a
  facet property named `view` hard-fails at boot, matching the
  `q`/`sort`/`strip`/`member`/… pattern.

### `tile-ratio` (resource-list, layout pass)

```kdl
resource-list id="strip-list" class="BorderStrip" media="image" grid=#true tile-ratio="1:9"
```

- Optional string attribute on `resource-list`: the grid tile's box as
  `"W:H"`. Without it a tile is 3:4.
- Grammar: two integers from 1 to 9999 joined by one colon, ASCII digits
  only. No whitespace, sign, decimal point, leading zero or third part
  (`1:9`, `3:4`, `16:9`). Anything else, a non-string included, is
  `InvalidAttributeType`.
- It needs `grid=#true` on the same list: without it the ratio would be read
  and never used, which is `TileRatioWithoutGrid` (Error, config-internal).
- A ratio whose height is at least three times its width is **tall**: its
  tiles stand at full height, capped at 70% of the window's height. The same
  test (`h >= 3 * w`) on a record's stored image sizes gives a tall record
  its narrow viewer column.

### `layout` and `index` (compound-viewer, Incunabula Slice 3b, `dsp-incubator cpe/docs/adr/0012-one-member-navigator-declared-layout.md`)

Two attributes on `compound-viewer` — **both required**, each a closed
vocabulary with no default:

```kdl
compound-viewer source="$id" layout="pages" index="none" media="image"
compound-viewer source="ST_001" layout="contents" index="rail"
```

- `layout` — `"contents"` | `"pages"`. Absent ⇒
  `DiagnosticKind::MissingRequiredAttribute`; any other value ⇒
  `DiagnosticKind::InvalidAttributeType`. `"contents"` is the text-first
  reader (member head → properties → pager); `"pages"` is the scan-first
  reader and **requires `media=`**
  (`DiagnosticKind::PagesLayoutWithoutMedia` when absent). Never inferred
  from member count or from whether `media=` is set — see `CONTEXT.md`'s
  "Layout" entry.
- `index` — `"rail"` | `"collapsed"` | `"none"`. Same required/closed
  treatment as `layout`. `"rail"` is the sticky contents rail beside the
  reader; `"collapsed"` is a closed `details.cv-index` under the pager;
  `"none"` shows no member list at all. See `CONTEXT.md`'s "Page index"
  entry.
- Both are declared independently of each other and of `media` — a
  `layout="pages" index="rail"` reader is legal (the rail wraps the pages
  body), as is `layout="contents" index="none"`.
- Boot validation reports **all** missing required attributes at once, not
  just the first: a bare `compound-viewer` with no `source`, `layout` or
  `index` at all produces three separate `MissingRequiredAttribute`
  diagnostics in one pass (consistent with the "all errors at once"
  boot-validation rule — see `dsp-incubator cpe/CLAUDE.md`'s Non-obvious constraints).
- Two further diagnostics guard the "one navigator, one owner" rule
  (`dsp-incubator cpe/docs/adr/0012-one-member-navigator-declared-layout.md`), both Error: `DiagnosticKind::CompoundMediaOwnership` (a
  same-resource `resource-detail media=` beside this navigator) and
  `DiagnosticKind::MultipleImageViewers` (a `layout="pages"` reader beside a
  `resource-detail media=` on the same page — two record-viewer units would
  share page-global signals). `compound-viewer` also joins
  `DiagnosticKind::DuplicateSingletonComponent`'s component list — a page
  carries at most one navigator (its fixed `id="compound-viewer"`/
  `id="cv-picker"` assume exactly one).

### `picker` (compound-viewer, book-page redesign)

An optional attribute on a `layout="pages"` `compound-viewer`: a page picker
in the pager row's centre (a `details` panel of every page, opening upwards
over the stage), instead of the "n / N" count.

```kdl
compound-viewer source="$id" layout="pages" index="none" media="image" picker="signatures"
```

- `picker="numbered"` — runs of 20 pages by position.
- `picker="signatures"` — the book's own collation, read from each page
  label's leading signature token (`a1r, Titelblatt` → gathering `a`, leaf
  1, recto; `&`/`?` count as gathering marks; `A` and `a` are different
  gatherings). Front and back flyleaves, an implied slot for an unparsed
  label inside a gathering, and a flat "Weitere" group for any other
  unparsed run. A book where **fewer than half** the labels parse falls back
  to numbered runs by itself, per book, at render time.
- Absent ⇒ no picker. A wrong type or an unknown value is
  `InvalidAttributeType` naming the vocabulary; a `picker` on
  `layout="contents"` is `PagesLayoutOnly` with `feature: "picker"`
  (hard-fail, config-internal).
- The picker's strings are engine chrome (`ChromeMsg::Picker*`).

**Retired: `jump-to-member` and `jump-placeholder`** (book-page redesign,
2026-09-30). The page picker replaced the zero-JS jump form. Both
attributes are gone from the IR; a config that still sets them is not an
error (CPE has no unknown-attribute check) — they are simply ignored.

### `member-links` (compound-viewer child, book-page redesign)

An optional child on a `layout="pages"` `compound-viewer`: the current
member's outgoing links as one captioned row under the pager, inside
`#compound-viewer` (so it swaps with the page and never moves the pager).
The `group` shape — a required `label=` chrome id, then the member `link`
properties, in order:

```kdl
compound-viewer source="$id" layout="pages" index="none" media="image" picker="signatures" {
    member-links label="reader-strips" "strip-left" "strip-right"
}
```

Each link value renders as it does on a record (an `a.int-link` to the
target's bound detail page); no link value ⇒ no row. Valid on the `pages`
layout only (the `contents` layout already lists every member property): on
`layout="contents"` it is `PagesLayoutOnly` with `feature: "member-links"`
(hard-fail, config-internal), as for `picker`. An undeclared `label` id is `UnknownGroupLabel` (Warning) and
falls back to the default caption; a second `member-links` child is
`UnknownChildNode { component: "compound-viewer" }`; a property on no
member class is `SparseProperty` (Warning); a property not stored as `link`
is `IncompatibleGlossPart` with the role `member link` (hard-fail, both
arms).

### `annotations` (compound-viewer, Incunabula Slice 6, `dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md`)

An optional boolean on a `layout="pages"` `compound-viewer`: the shown page's
**annotations**, read from the engine's `annotations` store (see `CONTEXT.md`
and `SCHEMA.md`), drawn on the scan and listed in a register after the reader.

```kdl
compound-viewer source="$id" layout="pages" index="none" media="image" picker="signatures" annotations=#true
```

- Read with `get_bool_attr`: absent ⇒ `false`; a non-boolean value
  (`annotations="yes"`) is `InvalidAttributeType`.
- On `layout="contents"` it is `PagesLayoutOnly` with
  `feature: "annotations"` (hard-fail, config-internal), as for `picker`.
- The register and the signal declarations render only when the reader is
  paired with its record (the navigator-keyed `resource-detail` on the same
  page, `dsp-incubator cpe/docs/adr/0012-one-member-navigator-declared-layout.md`). An unpaired `pages` reader with `annotations=#true` raises
  no diagnostic, but still fetches the annotations and still renders the
  eyebrow count (an anchor with no target) and the overlay with its
  handlers (reading undeclared signals). Pair it, or leave the attribute off.
- With it on, every page of the reader carries the register's patch target
  `div#cv-annotations` (empty on a page with no entries) and `_compound`
  patches it; with it off nothing changes, byte for byte. Every word the
  surface shows is engine chrome (`ChromeMsg::Anno*`, `ChromeMsg::Colour*`,
  `chrome::anno_*`), except the remarks and link names, which are data. See
  `COMPONENT_LIBRARY.md`'s `compound-viewer` "Annotations" for the markup.

## Translations

The engine has **three** translation surfaces:

- **Built-in chrome messages** — every language-varying piece of *engine*
  UI chrome (skip-link, nav/lang-switcher aria-labels, not-found
  and empty-state copy, facet/compound/pager strings, the
  bin's site-index chrome) lives in one typed catalog — `ChromeMsg`/`L4`
  in top-level `areas/access/cpe/components/src/chrome.rs`, `pub` and re-exported from the
  engine crate. See "Built-in chrome catalog" below.
- **Chrome strings** — declared in `site { chrome { ... } }`, with
  one entry per language. Used in component attributes by id
  (`search-bar placeholder="search-placeholder"`, or its `scope="…"`
  attribute for the mono "scope note" appended after the result count while
  a search query is active). This is for *project-authored* strings (nav
  labels, placeholders, scope notes) — config-declared, not engine-owned.
- **Property values** — already language-tagged in the data
  (`value_lang` column). Rendered by picking the value matching
  the current request language, with a project-declared fallback
  chain.

**Validation:** every config-declared chrome string has every
site-declared language; otherwise hard-fail at boot. The built-in
catalog's completeness is enforced at **compile time** instead (see below).

### Built-in chrome catalog (`areas/access/cpe/components/src/chrome.rs`)

Distinct from the config-declared `chrome { string }` mechanism above: this
catalog holds every **engine-owned** UI string — text the engine's own
renderers emit regardless of what a project's `config.kdl` says. A project
cannot add to or override it; it exists so the engine's own chrome (as
opposed to project-authored labels) is never accidentally English-only in
one corner while translated everywhere else.

- **`struct L4 { en, de, fr, it }`** — all four fields `&'static str` and
  **required**. This is the completeness guarantee: it is a compile error to
  add a message without supplying every language. `L4` deliberately does
  **not** derive `Default` — a `..Default::default()` literal would let a
  language silently default to `""` and defeat the guarantee.
- **`enum ChromeMsg`** — one variant per fixed (non-parameterized) message,
  each backed by an `L4` via `fn l4(&self) -> L4`. Call sites read
  `ChromeMsg::SkipToContent.text(lang)` (`text` = `l4().pick(lang)`, `pick`
  falling back to `en` for any language outside the four builtins).
- **Parameterized / pluralized messages** (`count_resources_text`,
  `count_of_text`, `remove_filter_label`, `compound_nav_label`,
  `compound_contents_cap`, `facet_group_remainder_text`) are their own
  `pub(crate) fn`s in the same
  module — each fuses the language `match` with a literal `format!` (the
  `format!` template must be a compile-time string literal, so a
  runtime-`pick`ed template could never be threaded through it).
  ⚠️ **These helpers have NO completeness guarantee.** `L4`'s four required
  fields make a missing language a compile error for a `ChromeMsg` *variant*;
  a standalone fn matches on `lang.as_str()` with a `_` catch-all, so an
  absent `fr`/`it` arm **compiles silently and falls back to English** — no
  compile error, and no boot diagnostic either (the
  `IncompleteChromeTranslation` hard-fail applies to project-declared
  `chrome { string }` blocks, not to this catalog). Write all four arms
  explicitly; review is the only check.
- **The facet-group disclosure summary** (`facet_group_remainder_text(n,
  lang)`, HdM Slice 2 / G4) — "Show N more options" on a facet group collapsed
  past `FACET_GROUP_COLLAPSE_THRESHOLD`. Count-inflected, and deliberately
  noun-bearing so its singular arm is observable (see
  `docs/plans/hdm-slice-2-performances/translations-to-review.md`). It is a
  standalone fn, **not** a `ChromeMsg` variant: `ChromeMsg::text` returns
  `&'static str` and can neither interpolate a count nor select a plural form.
  The collapse itself is an engine-wide rendering rule with **no config
  surface** — no per-facet or per-project knob; see
  `COMPONENT_LIBRARY.md`'s `facet-panel` section.
- **Empty-states-not-found catalogue entries** — E1–E7/N1–N3 added fixed
  `ChromeMsg` variants (`ListEmptyClass`, `FacetEmptyGroupFallback`,
  `ImageAbsentLabel`/`AbsentRecord`/`AbsentPlate`/`CaptionAbsent`,
  `EmptyClearFilters`, `ValueUnresolved`/`ValueUnresolvedTitle`, and the
  `Nf*` family for the not-found pages) plus new parameterized helpers
  beside `count_resources_text`: `narrowed_to_zero_text` (generalized from
  the search-bar plan's `search_no_match_text` — query-only / filters-only
  / query+filters, all `<em>`-wrapping the varying part via `html!`, never
  `format!` + `PreEscaped`), `facet_empty_group_text`,
  `compound_no_members_text`, `index_count_text` (E6's count-aware
  `"{n} projects"`, deliberately without a "· m available" segment — no
  data source), and the `nf_*` family (`nf_home_text`, `nf_id_title_text`,
  `nf_id_msg_text`, `nf_browse_all_text`). See
  `docs/plans/empty-states-not-found/translations-to-review.md` for the
  full four-language table and open translation questions (a facet-group
  plural mechanism, German/French/Italian article agreement on an
  interpolated class noun, and the site index's pre-existing copy
  mismatch with the design handoff).
- **Pluralization** — `plural_category(n, lang) -> PluralCategory` (`One` |
  `Other`) is the one shared rule: `n == 1` → `One` for en/de/it; **French
  treats `n == 0` as `One` too** (French convention — "0 partie", not "0
  parties"); everything else → `Other`. Only four strings are genuinely
  count-inflected (`count_resources_text`, the two compound-viewer
  "N parts" strings, and `facet_group_remainder_text`). `count_of_text`
  ("N of M {noun}") inflects **nothing of its own** — the connective
  (of/von/sur/di) never inflects — but it does carry a trailing class noun,
  supplied by the caller and agreeing with `m` (the unfiltered total), never
  with the narrowed count: "1 von 8’942 Aufführungen".
- **Grouped counts** — `group_thousands(n, lang)` inserts the locale group
  separator: U+2019 (the Swiss apostrophe) for `de`/`it`, U+202F (narrow
  no-break space) for `fr`, `,` for `en` and every other language. Applied
  to the browse count line **only** (`count_resources_text`/`count_of_text`),
  deliberately not to the pager, the facet-option counts, the record-viewer
  plate counter or the site index — see the helper's doc comment.
- **The browse count line's noun is the CLASS noun, not an engine noun.**
  Since A12.1 there is no generic `Einträge`/`enregistrements`/`record`/
  `resources` arm at all: `render_surf_head` and the browse breadcrumb's
  accessible name both pass `resolve_class_label` (singular) and
  `resolve_class_plural` (plural) in. A class with no declared `plural {}`
  degrades to the singular — that is what `MissingClassPlural` warns about.
- **`pub const BUILTIN_LANGUAGES: [&str; 4] = ["en", "de", "fr", "it"]`** —
  documents the catalog's coverage. **Completeness is compile-time-guaranteed
  only across these four.** A project MAY still declare a 5th+ language (see
  above); such a page gets English *engine* chrome via `pick`'s `_ => en`
  arm for any built-in string — not a regression (this is identical to the
  pre-catalog behaviour), just outside what "completeness" covers here.
- **`pub fn resolve_builtin(prefs: &[String]) -> Lang`** (in `lang.rs`) —
  resolves a preference list against `BUILTIN_LANGUAGES` (default `en`), with
  **no project `SiteConfig` involved**. This is what lets the bin's site
  index (which has no project to resolve a language against) obtain a `Lang`
  and call the same `pub` catalog a project page's renderer does — one seam,
  reused, not mirrored. `resolve_builtin` shares its candidate-matching logic
  with the per-project `resolve` via a private `resolve_against` helper, so
  the two can never drift apart.

**Draft status.** Every string in the catalog carries a doc comment noting
whether it's a **pre-existing draft** (migrated verbatim from an
already-4-language per-module helper that predated the catalog) or **new**
(authored to fill a gap — settings.rs, the skip-link/cog-label, previously
unbranched aria-labels, and the bin's site index). Both kinds are unreviewed
fr/it drafts pending native-speaker review; see
`docs/plans/chrome-localization/translations-to-review.md` for the full
review table.

**"Complete" excludes proper nouns and brand/legal marks.** A handful of
literal strings are deliberately left as bare English constants, not
`ChromeMsg` variants: the "CPE" wordmark, the "DSP APP" link label, the
org's official name ("Swiss National Data and Service Center for the
Humanities", both standalone and inside the © line), and the © line itself.
These don't vary by language — they're proper nouns / legal text, not
descriptive or navigational chrome — so routing them through the catalog
would add four identical rows per string with no translation to review.
"Complete" for this catalog means *every language-varying descriptive and
navigational engine string*, not literally every string the engine's
renderers emit.

### Narrative language fallback (Phase 3.9)

A `narrative file="home.md"` component resolves its content via a
3-rung fallback chain, tried in order:

1. `<stem>.<requested-lang>.<ext>` (e.g. `home.fr.md`) — the visitor's
   resolved request language.
2. `<stem>.<default-language>.<ext>` (e.g. `home.en.md`) — skipped when
   the default language is itself the requested one (already tried at
   step 1).
3. the base `file` as declared in config (e.g. `home.md`).

The first rung present in the `narrative/` directory wins; title
derivation (`compose_page_title`'s narrative-heading path) uses the
exact same chain, so a page's `<title>` and body content can never
disagree on which variant was picked.

Per-language variant files are therefore **optional** — a project may
add `home.fr.md` without also authoring `home.de.md`, and visitors
requesting an un-authored language transparently see the
default-language content instead of an empty page. **Boot validation
only guarantees the terminal rung of the chain is reachable**: a
narrative `file` must resolve to the base file **or** the
default-language variant (`NarrativeFileNotFound` hard-fails
otherwise) — because those are the two rungs the chain is guaranteed to
reach. A bundle offering only a non-default variant (e.g. `home.fr.md`
with no `home.md` and no `home.en.md`, `en` being the default) fails at
boot rather than silently rendering an empty article for every visitor
who doesn't request `fr`.

## Validation surface (boot-time)

Per `cpe-poc.md` §11 and re-stated here for the config view. This is the
full v1 validation target. Data-dependent checks marked below require the
query layer; the Phase 2 boot validator performs only the config-internal
checks. Phase 3 task two implemented the resource-detail-relevant subset;
the rest land in Phase 5. See the "Validation" section of
`dsp-incubator cpe/docs/ARCHITECTURE.md`.

- KDL syntax parses cleanly.
- Every top-level node is `site`, `labels`, `page` or `remodel`. Other
  top-level nodes hard-fail (`UnexpectedTopLevelNode`), and so does a second
  `remodel`.
- The `site` block has the required attributes (`title`,
  `languages`).
- Every `page` has unique `id` and `route` (within the project).
- Every component node is one of the v1 components (or a
  project-registered custom component — the `(project, name)` registry in
  `areas/access/cpe/core/src/validate/project_components.rs`; `timeline` is the first tenant, and
  `woodcut-cases`, `edition-cards` and `strip-row` followed in Slice 7, see
  "`timeline`" and "`woodcut-cases`, `edition-cards`, `strip-row`" above and
  `dsp-incubator cpe/docs/adr/0011-per-project-components.md`).
  Unknown component → hard-fail.
- Every component's required attributes are present and have the
  right value shape.
- Component references (`target`, `source`) resolve.
- A literal-id `resource-detail` references a resource that exists
  in the data (hard-fail). *(Phase 3 task two — implemented.)*
- Property names in `show-properties` and `title-property` on a
  `resource-detail` exist on the resource's class (warn — sparse data
  is OK). *(Phase 3 task two — implemented.)* Property names named by a
  `group` are covered by the same sparse check (a shared
  `detail_property_names` helper flattens `show-properties` + every
  group's members + `title-property` into one list — Incunabula Slice 3 /
  G8).
- A property named by more than one `group`, or by a `group` and
  `show-properties`, is `DuplicateGroupedProperty` (hard-fail) — one
  diagnostic per repeated name; a repeat within one `group` alone, or
  within `show-properties` alone, is out of scope. A `group label=` that
  is not a declared `chrome { string }` id is `UnknownGroupLabel`
  (warning — falls back to the built-in caption). A `group empty=` that is
  not a declared chrome id is `UnknownGroupEmptyLabel` (warning — the group
  renders as if `empty=` were unset); a non-string `empty=` is
  `InvalidAttributeType`. *(Incunabula Slice 3 / G8 — implemented; `empty=`
  HdM Slice 3.)*
- Two `narrative-for` entries on one `resource-detail` naming the same
  resource id is `DuplicateNarrativeFor` (hard-fail). A `narrative-for`
  `file=` runs the same `InvalidNarrativePath`/`NarrativeFileNotFound`
  checks (hard-fail) a page-level `narrative` node's `file` does. A
  `narrative-for` resource id absent from the data is
  `UnknownNarrativeForResource` (warning — the intro just never attaches).
  *(Incunabula Slice 3 / O6 — implemented.)* There is no dedicated
  positional-argument `DiagnosticKind` — the first positional string (the
  resource id) reuses `MissingRequiredAttribute` (absent) and
  `InvalidAttributeType` (present but non-string), same as any other
  required attribute; the diagnostic message names "first positional
  argument (a resource id)" so it's still distinguishable from a real
  attribute failure.
- Two parameterised `resource-detail` pages binding the same `class`
  value produce `AmbiguousClassRoute` (hard-fail). *(Phase 5 — implemented.)*
- A `link` value's `target_id` that does not exist in the data produces
  `UnknownLinkTarget` (warning — the renderer degrades gracefully to
  `span.unresolved`). The check is best-effort over boot-reachable
  resources; `$param` pages cannot be pre-walked. *(Phase 5 — implemented.)*
- A `compound-viewer` with a literal `source` that has no
  `compound_parts` rows produces `EmptyCompoundSource` (warning —
  renders empty state, doesn't crash; `$param` sources are skipped).
  *(Phase 5 — implemented.)*
- Property names referenced by `where`/`sort` for `resource-list` —
  hard-fail (`UnknownResourceListProperty`) if the property exists
  nowhere in the data at all; warn (`SparseResourceListProperty`) if it
  exists somewhere in the data but on no resource *of this class*.
  *(Phase 5 — implemented.)*
  **The check is two-tier by presence, not by a coverage ratio**:
  `SparseResourceListProperty` fires only when the property is present on
  **no** resource of the class at all (`!class_props.contains(prop)`); once
  it is present on even one, the check is silent regardless of how thin the
  coverage is — HdM's `gloss property="genre-varieties"` (present on 2,363
  of 2,449 Works) and `chip property="role"` (1,221 of 1,222 People) fire no
  warning. Don't predict a sparse warning from a coverage percentage; predict
  it only from "does at least one resource of this class carry the
  property at all".
- Property names referenced by `searches` for `search-bar` — hard-fail
  (`UnknownSearchProperty`) if the property exists nowhere in the data;
  hard-fail (`IncompatibleSearchProperty`) if its `value_type` on the
  target class is neither `text` nor `richtext`; warn
  (`SparseSearchProperty`) if it exists on some resources of the class but
  not all. `q` is a reserved query key (`RESERVED_FACET_PROPERTIES`) — a
  facet property named `q` also hard-fails. *(search-bar plan —
  implemented.)*
- An optional `plural { <lang> "…" }` child under a `labels { class }`
  entry must carry every declared site language once present at all
  (hard-fail, reusing `IncompleteLabelTranslation`); an absent `plural`
  block is not a hard-fail — but it degrades the browse eyebrow, the detail
  class crumb and the 404 browse-all exit to the capitalized singular, so it
  warns (`MissingClassPlural`). *(search-bar plan — implemented; the warning
  is the backlog quick-wins batch; see the `labels` block section above for
  all four consumers.)*
- A `facet` with a valid but non-`list` kind (e.g. `date-decade`,
  `text`, `boolean`, `range`, `date-year`) → hard-fail
  (`UnsupportedFacetKind`). *(Phase 5 — implemented.)*
- Facet `kind` is compatible with the property's value type →
  hard-fail (`IncompatibleFacetKind`). A `list`-kind facet requires
  `list_value` rows on the target class. *(Phase 5 — implemented for
  the `list` kind.)*
- Facet property absent from the data entirely → hard-fail
  (`UnknownFacetProperty`). *(Phase 5 — implemented.)*
- Facet property exists in the data but not on the target class →
  warn (`SparseFacetProperty`). *(Phase 5 — implemented.)*
- Route param references in `$param` substitutions match declared
  params.
- Nav `route` values exist as declared page routes (hard-fail,
  `UnresolvedNavRoute`). *(Phase 3.8 — implemented.)*
- The one-row desk header fits: for each declared language, the resolved
  site title, every resolved nav label, the language summary (two or more
  languages) and the appearance box, with the desk form's gaps, must fit the
  1,170px the 79rem desk breakpoint leaves (the 1264px window less two
  gutters; 1,280px at 100rem before the layout pass).
  An estimate over it warns (`HeaderRowOverBudget { lang, estimated_px,
  budget_px }`, one per language) — the title ellipsises there, so it is
  advisory. Text is estimated from per-face average advances calibrated for
  Newsreader and IBM Plex Mono; a project with another `typeface` gets the
  same constants, and the estimate holds for Latin-script text only.
  Config-internal, so both validation outcomes report it (when the `site {}`
  block itself lowers, like `MissingClassPlural`), and
  like every Warning it shows in the serve log, not in `cpe --check`.
  *(general chrome — implemented; `areas/access/cpe/core/src/validate/header_budget.rs`.)*
- Narrative file references resolve to the base file or the
  default-language variant (hard-fail, `NarrativeFileNotFound`); other
  per-language variants are optional. *(Phase 3.9 — implemented; see
  "Narrative language fallback" above.)*
- A `resource-list` with `grid=#true` and no `media` attribute → hard-fail
  (`GridWithoutMedia`). *(grid-mode plan — implemented.)*
- A `resource-list tile-ratio=` outside the `W:H` grammar → hard-fail
  (`InvalidAttributeType`); with a valid ratio and no `grid=#true` →
  hard-fail (`TileRatioWithoutGrid`). *(layout pass — implemented.)*
- `site { narrative-companion | narrative-side | text-align }` with a string
  outside its two values → hard-fail (`UnknownSiteValue`); `companion=` or
  `side=` on a narrative that is not its page's first → hard-fail
  (`NarrativeLayoutNotFirst`). *(layout pass — implemented.)*
- Two narratives of one page with the same authored heading id, in the
  content one visitor is served → hard-fail (`DuplicateHeadingIdOnPage`).
  Data-dependent (it reads the narrative files), so it runs in both
  `validate_with_data` arms. *(layout pass — implemented.)*
- A single `page {}` declaring both a `compound-viewer` and a paginating
  `resource-list` → hard-fail (`CompoundViewerWithPagedList`). Both read the
  `?page=` query key — a member address (`?page=<id|name|prefix>`) vs. a page
  number (`?page=N`). "Paginating" means `per-page`, **or no `per-page`/`limit`
  at all** (the default is `per-page 20`); a `resource-list` with `limit` is
  compatible, and so is the pair on two different pages. Config-internal, so
  both validation outcomes report it. *(Incunabula Slice 2 — implemented; see
  `COMPONENT_LIBRARY.md`'s `compound-viewer` addressing contract.)*
- `incoming-links` (Incunabula Slice 5 / G7): more than one child on a
  `resource-detail` is `DuplicateIncomingLinks` (hard-fail); zero positional
  property args is `IncomingLinksWithoutProperty` (hard-fail) — both raised
  at lowering time (span-located, no `page` field). A `label=` that is not
  a declared `chrome { string }` id is `UnknownIncomingLinksLabel` (warning
  — falls back to the source class's capitalised plural). Three
  data-dependent checks, all in both `validate_with_data` arms: `class=`
  naming a class absent from the data is `UnknownIncomingLinksClass`
  (hard-fail — a query input, like `resource-list`'s `class`); a listed
  property absent from the data entirely is `UnknownIncomingLinksProperty`
  (hard-fail); a listed property present in the data but carried by no
  resource of the declared source class is `SparseIncomingLinksProperty`
  (warning — the list would always be empty for it); a listed property
  stored with a `value_type` other than `link` is
  `IncomingLinksPropertyNotLink` (hard-fail — only `link` rows carry a
  `target_id` to reverse-resolve). *(Incunabula Slice 5 / G7 —
  implemented.)*
- Grouped `incoming-links` (HdM Slice 3 / G3): `group-by=`, `layout=`,
  `note-label=`, `row-label=`, `row-aside=` and `sort-property=` are read with
  the reporting reader, so a present-but-wrong-typed value is
  `InvalidAttributeType`, and so is a `layout=` outside `cards`/`columns`.
  `layout=`/`note-label=` without `group-by=` are ignored (no diagnostic). A
  `note-label=` that is not a declared `chrome { string }` id is
  `UnknownIncomingLinksNoteLabel` (warning — the note is omitted); a declared
  one whose text lacks `{n}` in some language is
  `IncomingLinksTextMissingPlaceholder` (hard-fail, one per language). Both
  are config-internal. Three data-dependent checks, both arms, on each of
  `row-label`/`row-aside`/`sort-property` and a property `group-by` (the
  reserved `group-by="via"` names none): absent from the data or from every
  resource of the source class is `UnknownIncomingLinksRowProperty`
  (hard-fail); stored with the wrong type (`group-by`/`row-aside` need
  `list_value` or `text`, `row-label`/`sort-property` need `text`) is
  `IncomingLinksRowPropertyWrongType` (hard-fail); missing on some source
  resource that links through the declared properties is
  `SparseIncomingLinksRowProperty` (warning — those rows fall back). Each
  kind's `attribute` field names the attribute. With `group-by=` and no
  `limit=`, the limit is `GROUPED_INCOMING_LIMIT_CEILING` (1000), not 50.
  **Group-key rules** (`ir::GroupKey`, filled by `query::incoming_links`, so the
  renderer groups by plain equality): a *text* key keeps the **stored
  spelling** (never lowercased, so the group head reads as authored); case
  variants are folded in Rust (`to_lowercase`) onto the FIRST spelling met in
  row order. A *list* key is `{ key, label, seq }`; a node with no
  `list_nodes` row sorts last (`seq: i64::MAX`) and its label falls back to
  the node id. `group-by="via"` and ungrouped specs always carry `group: None`
  (the renderer groups `via` by `IncomingLink::via`). **List-valued labels
  (`row-aside`, group head) resolve requested lang → `MIN(label)`, with NO
  "universal" rung** — `list_node_labels.lang` is NOT NULL, so no
  language-less label exists (same rule as `get_list_node_label`); only text
  values use lang → universal → any.
  *(HdM Slice 3 — implemented.)*
- **Backlog hardening batch 3** (engine-hygiene-3): a `resource-detail`
  child node outside `show-properties`/`group`/`narrative-for`/
  `incoming-links` is `UnknownChildNode` (hard-fail, config-internal — see
  "`resource-detail`'s recognised child nodes" above); a `resource-detail
  class="…"` naming a class absent from the data is
  `UnknownResourceDetailClass` (hard-fail, data-dependent, both arms — see
  the `class` attribute section above); an `incoming-links` property that is
  `link`-typed, present on its declared source class, and resolves to at
  least one target class, but never resolves to the page's own host class,
  is `IncomingLinksPropertyMissesHost` (**warning**, data-dependent, both
  arms — the list it drives will always be empty on this page). Also from
  this batch: `ConflictingMediaProperty`'s fields are named
  `registered_property`/`declared_property` (not `detail_property`/
  `list_property` — a compound-viewer registration is neither a "detail"
  nor a "list"; see the `media` section above for the full
  detail→compound→list registration order); and **every optional string
  attribute in a `lower_*` fn now reports a present-but-wrong-typed value as
  `InvalidAttributeType`** (`resource-detail`'s `id`/`title-property`/
  `class`/`media`, `resource-list`'s `id`/`media`, `compound-viewer`'s
  `media`/`picker`, `search-bar`'s `placeholder`/`scope`, `sort`'s
  `direction`) — previously these degraded silently to "absent", the way
  `get_bool_attr` never did for `grid=`. *(engine-hygiene-3 — implemented.)*
- `order-by` (Incunabula Slice 8 / G9) on a `resource-list`: an absent
  `key`/`property`/`label` is `MissingRequiredAttribute`, a wrong-typed one
  (or a `direction` outside `asc`/`desc`) `InvalidAttributeType`; a key
  outside `[a-z0-9-]+` is `InvalidOrderKey`, a built-in key (`default`/
  `name`/`name-desc`/`class`/`id`) `ReservedOrderKey`, a repeated key within
  one list `DuplicateOrderKey` — all hard-fails raised at lowering time
  (span-located, no `page` field). A `label=` that is not a declared
  `chrome { string }` id is `UnknownOrderLabel` (hard-fail, config-internal,
  checked before the data fork). The option's `property` joins the
  `where`/`sort` two-tier check above in both `validate_with_data` arms.
  *(Incunabula Slice 8 step 4 — implemented.)*
- `order-default-label`/`order-name-property` on a `resource-list`: a
  wrong-typed value is `InvalidAttributeType`; `order-default-label` is
  otherwise not boot-checked (an unresolvable id falls back to "As
  curated"), and `order-name-property` joins the `where`/`sort` two-tier
  check in both arms. *(Incunabula books browse — implemented.)*
- The composed `gloss { part …; teaser … }` on a `resource-list`: a child
  other than `part`/`teaser` is `UnknownChildNode { component: "gloss" }`;
  `property=` plus children, no `part`, a second `teaser`, a repeated part
  property or a wrong-typed `unit` is `InvalidAttributeType` (all at
  lowering time). A `unit=` that is not a declared chrome string id is
  `UnknownGlossUnit`, and a unit translation without `{n}` is
  `GlossUnitMissingPlaceholder` (one per language) — both hard-fails,
  config-internal, checked before the data fork. A part stored as
  `list_value`/`link`/`boolean`, a teaser not stored as `text`/`richtext`,
  or `unit=` on a part not stored as `integer`/`decimal` is
  `IncompatibleGlossPart` (hard-fail, data-dependent, both arms). Every
  part and teaser property joins the `where`/`sort` two-tier check.
  *(Incunabula books browse — implemented.)*
- Record header slots (book-page redesign): `alt-title-property`/
  `author-property` wrong-typed is `InvalidAttributeType`; a child of
  `imprint` other than `part` is `UnknownChildNode { component: "imprint" }`
  and an empty `imprint {}` is an error (both config-internal); a slot
  property absent from the class is `SparseProperty` (Warning); a slot whose
  property holds a type it cannot render is `IncompatibleGlossPart` with the
  role `alt-title`, `author` or `imprint part` (hard-fail, data-dependent,
  both arms). `IncompatibleGlossPart`'s message names the slot it concerns.
- `compound-viewer picker=`: an unknown value or wrong type is
  `InvalidAttributeType`; `picker` on `layout="contents"` is
  `PagesLayoutOnly` (`feature: "picker"`; hard-fail, config-internal, before
  the data fork). `member-links` on `layout="contents"` is `PagesLayoutOnly`
  (`feature: "member-links"`, same tier); its other checks: see its section
  above.
- `compound-viewer annotations=` (Incunabula Slice 6): a non-boolean value is
  `InvalidAttributeType`; `annotations=#true` on `layout="contents"` is
  `PagesLayoutOnly` (`feature: "annotations"`; hard-fail, config-internal,
  before the data fork, so no `-raw` fixture).
- `group links-label=` (Incunabula Slice 6): a non-string value is
  `InvalidAttributeType`; an id that is not a declared `chrome { string }` id
  is `UnknownGroupLinksLabel` (warning — the row falls back to
  `ChromeMsg::AnnoRelated`, "Verwandt" / "Related"). Config-internal, checked
  beside `UnknownGroupLabel`.
- **Annotation rows** (Incunabula Slice 6, `dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md`): every `annotations` and
  `annotation_links` row is checked against the rules the DDL does not enforce
  (`SCHEMA.md` §"Constraints not enforced in DDL"): a box inside the image
  (0..1, with a 1e-9 tolerance) with `w` and `h` above 0; a polygon must have
  `points`, and they must parse as whitespace-separated `x,y` pairs, at least three,
  each inside the box; no `points` on a rectangle or circle; a known shape; a
  colour, when present, as lowercase `#rrggbb`; a link with at least one
  target. One `MalformedAnnotation` (hard-fail) per broken rule. Data, not
  config, so both `validate_with_data` arms run it (one shared
  `check_malformed_annotations`); a failing query is `InternalQueryError`.
  Only a project whose `data.sql` carries annotation rows can trip it, which
  today is Incunabula alone.
- `site { not-found { browse } }` (Incunabula Slice 8 / G11): a `route`
  that is not a declared page route is `UnresolvedNotFoundBrowseRoute`, and
  more than three `browse` entries is `TooManyNotFoundBrowseLinks`. Both are
  hard-fails and config-internal. *(Incunabula Slice 8 step 7 —
  implemented.)*
- `timeline` (Incunabula Slice 4, the per-project custom component — see
  "`timeline`" above): `class`/`date-property`/`lane-property` absent, or
  any attribute wrong-typed, is `MissingRequiredAttribute`/
  `InvalidAttributeType` (config-internal, span-located). An `axis-start`/
  `axis-end` integer out of `i32` range is `TimelineAxisOutOfRange`; an
  axis with `axis-end <= axis-start` or a span under three years is
  `InvalidTimelineAxis` (both config-internal). Three data-dependent
  checks, all in both `validate_with_data` arms: `class` naming a Class
  absent from the data is `UnknownTimelineClass` (hard-fail — a query
  input); `date-property`/`lane-property`/`extent-property` absent from
  the data entirely is `UnknownTimelineProperty` (hard-fail); a property
  stored with the wrong `value_type` for its role is
  `IncompatibleTimelineProperty` (hard-fail). `date-property`/
  `lane-property` present in the data but carried by no resource of
  `class` is `MissingTimelineProperty` — a **hard-fail**, not the usual
  sparse-warn tier, since a timeline with no dated or no laned markers
  would render nothing useful; `extent-property` in the same situation is
  the ordinary `SparseTimelineProperty` (warning — the extent segment is
  just omitted). *(Incunabula Slice 4 — implemented.)*
- Figure blocks and the three showcase components (Incunabula Slice 7, see
  their two sections above; the table there is the one list of their
  diagnostics): the slot's section, heading-id, more-route and label checks,
  the showcase grammar and label checks, and the data checks that keep every
  copied string verbatim. All are hard-fails. The narrative-content and data
  checks run in both `validate_with_data` arms, each pinned by a both-arms
  test.
  *(Incunabula Slice 7 — implemented.)*
- The `remodel` block (see "`remodel` block" above; the grammar table there
  has each node's errors). Every kind below is a hard-fail, located at the
  offending node (`MissingRemodelBlock` has no location), and the block is
  lowered in one pass, so one boot reports every defect of it. An error in the block fails boot like an error in any
  other block, although the web binary does not use the mapping: it
  validates the whole file. Config-internal, so both validation outcomes
  report it. A second `remodel` block is `UnexpectedTopLevelNode`, the kind
  of every stray top-level node, at boot and in `load_remodel` alike.
  *(per-project remodelling — implemented.)*
  - `MissingRemodelBlock` — the config has no `remodel` block. Raised only
    by `cpe_core::validate::load_remodel` (the entry meant for the store
    builder), never at the web binary's boot: the block is optional there.
  - `DuplicateRemodelEntry` — a node occurs more often than its count
    allows; an attribute is written twice on one node; a list name, a
    `list-node` slug within its list, a label language within a node, a
    class name or a class `iri` is declared twice; a `property` repeats an
    `iri` its class already has; a `property`, a `curated`, a
    `member-count`, an `incoming-count` or a `member-of` `seq=` takes a CPE
    name its class already has, also from a node of another kind; a
    curated key (`curated-key=` of an `id`, a `cover` or a `property`,
    `display-key=`, `key=`, or a `key` of the `hook` node) is declared twice
    on one class; a `provides` of the `hook` node takes a name its class
    already has; two `drops` name one class; two `overrides` name one
    property; an `on` or
    `link-targets` names a class twice; two `via` nodes of one
    `incoming-count` name one property; a class has a second `id`,
    `member-of` or `cover`; the block has a second `hook`.
  - `ReservedRemodelName` — a `class` is named like the `images` block's
    `class=`; in a block with an `images` node, a property is named
    `image`.
  - `UnknownRemodelAttribute` — an attribute outside those the node takes.
  - `UnknownRemodelType` — a `property` `type` that is neither supported nor
    one of the six unsupported words.
  - `UnsupportedRemodelType` — a `property` `type` of `integer`, `decimal`,
    `boolean`, `list-node`, `geometry` or `color`.
  - `InvalidRemodelValue` — a `shortcode` that is not four characters of
    `0-9A-F`, a `sentinel` that is empty or starts or ends with whitespace,
    or an `iiif-base` that is not
    `https://`, a host and `/`; an empty name (a class name of `on` or
    `link-targets` among them), slug, label, `iri`,
    `curated-key`, `display-key`, `target`, `key=`, `list=`, `authorship` or
    `license`; a positional argument beyond the one a node takes, or any on
    `remodel`, `hook`, `drops`, `images`, `annotations`, `id` or `cover`; an
    empty argument or `class=` of a `hook` child; a
    `property` `max`
    below its `min`, a `min` or `max` that is negative or above 4294967295,
    or a word outside the closed vocabulary of its `from`, `empty`,
    `sentinel` or `sentinel-within`; a
    `curated` `type` outside `text`, `list` and `per-language`, or a
    `curated` `min` other than 0 or 1; an `id` `from` outside `label`,
    `property` and `iri`; an empty `property=`, `seq=`, `of=`, `from=` of
    an `incoming-count`, `via` or `member-of` parent.
  - `IncompleteListTranslation` — a `list-node` without a label in a
    language the site declares.
  - `UnknownRemodelReference` — a `list-node` label in a language the site
    does not declare, a class name in `annotations` that no `class`
    declares, a `property` `target` that names no `class`, or a `curated`
    `list=` that names no `list`; a `member-of` parent or an
    `incoming-count` `from=` that names no `class`; a `member-count` `of=`
    that names no class that is `member-of` this one; an `id` `property=`
    that names no `text` property of its class with `max=1`; a `via` that
    names no property of the `from` class; a `class=` of a `hook` child that
    names no `class`, or an `overrides` that names no `property` node of
    its class.
  - `ConflictingRemodelAttributes` — a `property` with both an `iri` and a
    `from`, or with neither; with an `iri` and no `type`; a `link` without
    a `target`; an attribute its form does not take (including a `type`
    beside `from="label"`); `exclude-label` beside `curated-key`;
    `sentinel` or `sentinel-within` in a block without a `sentinel` node. A
    `curated` with `list=` beside a `type` other than `list`, or with
    `type="list"` and no `list=`. An `id` with neither a `curated-key` nor
    a `from`; with `property` and no `from="property"`, or the reverse;
    with `under-parent` on a class without a `member-of`, or beside
    `curated-key`. A class `image` or a `cover` in a block without an
    `images` node; a `cover` on a class no class is `member-of`, or beside
    its class's `image`. A `via` that names a property that is no `link`,
    or a link whose `target` is another class. A `drops` of the `hook`
    node that names a class another class is `member-of`.
  - `RemodelMembershipCycle` — a `member-of` names its own class, or
    following the parents' own `member-of` nodes leads back to it; reported
    at each `member-of` on the cycle.
  - `MissingRequiredAttribute` — beyond an absent required attribute,
    argument or node anywhere in the block: a `class` without an `id`, a
    `cover` without `curated-key=`, an `incoming-count` without a `via`.
- Chrome translations are complete.

**Reporting:** all errors printed in one pass at boot, with file
path and line/column where KDL provides them. Boot exits non-zero
on any hard-fail. A `--check` flag runs the validation and exits
without starting the server, suitable for CI.

## Authoring tips

(Captured here as the v1 audience is developer-authors; this is not
a user-facing guide.)

- Start by copying an existing project's `config.kdl` and editing.
- Use `cpe --check --project=<name>` during edits to surface
  validation errors without booting the server.
- `narrative` is the fastest place to iterate — Markdown files
  reload on each request (TBD — see `cpe-poc.md` §17 "Hot
  reload").
- KDL doesn't enforce indentation. Two-space indents are the
  convention; the engine's `--format` flag (if added) will
  enforce it.

## Out of scope for v1

- **Reusable templates / partials** — repetition is acceptable.
  Reintroduce if it gets painful (`cpe-poc.md` §8.4.8).
- **Expressions and conditionals** — KDL is declarative; computed
  values are out of scope. If a project needs computed binding, it
  uses the custom-component escape hatch.
- **Per-page CSS** — all CSS comes from the token-CSS system at
  `<base>/<project-id>/_static/cpe/*.css`. No `style="..."` inline declarations in config.
  Per-project theming uses the `theme {}` knob; a `style.css` override
  is possible but deferred (see BACKLOG).
- **Per-component layout primitives** — CSS handles layout; the
  config language doesn't have a grid or flex vocabulary.

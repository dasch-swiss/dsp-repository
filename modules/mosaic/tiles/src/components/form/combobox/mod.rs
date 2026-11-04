//! Combobox tile: a search input whose results, a listbox of submit buttons,
//! are rendered by the server. See `docs/src/mosaic/component-api-conventions.md`.
//!
//! The pattern is the WAI-ARIA combobox with a listbox popup, without
//! `aria-activedescendant`: the arrow keys move **focus** between the options
//! (roving focus), and each option is a real `<button type="submit">` inside the
//! caller's form, so Enter or a click submits natively and the choice is the
//! caller's POST. No signals, and no choice state lives in the tile.
//!
//! The tile has no Datastar crate dependency. The caller builds the search action
//! with `shared_datastar` and hands it over as an opaque string
//! ([`ComboboxBuilder::search_action`]); the caller's endpoint answers a
//! fragment made by [`combobox_results`], which Datastar morphs in by id. The
//! tile itself emits two Datastar attributes, `data-on:input__debounce.300ms` on
//! the input and `data-init` on the results container, so the type-ahead assumes
//! the page loads Datastar; without it the field degrades to Search plus Tab.
//!
//! Screen-reader announcements go through a persistent `role="status"` element
//! beside the results container, never inside it: a live region the morph
//! replaces is not announced reliably. [`combobox_status`] marks the visible line
//! inside the results whose text `data-init` copies there.

use maud::{html, Markup, Render};

use super::shell::FieldShell;
use crate::builder::ComponentBuilder;

/// Keyboard handling for the wrapper's `onkeydown`; `this` is the wrapper, so it
/// only ever sees its own input and results.
///
/// Enter in the input is always cancelled: left alone, implicit submission
/// presses the form's first submit button, which with no search button in the
/// form would be an option and would choose it. With `data-combobox-live` the
/// results already follow the typing, so nothing more happens; otherwise the
/// search button is clicked. An option has no text entry, so a key meant for
/// text hands focus to the input without cancelling, and the browser delivers
/// that same key there. Escape does nothing while the results are empty, so it
/// still reaches an enclosing dialog;
/// it also drops the container's `data-init`, so the next fragment re-adds it and
/// `aria-expanded` is set again even when the expression is unchanged.
const KEYDOWN: &str = "\
var box = this;\
var t = event.target;\
var input = box.querySelector('input[role=combobox]');\
var results = box.querySelector('.combobox-results');\
var opts = Array.from(box.querySelectorAll('[role=option]'));\
var at = opts.indexOf(t);\
var inResults = !!results && results.contains(t);\
if (event.isComposing || (t !== input && !inResults)) return;\
var key = event.key;\
var typing = (key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey)\
  || ['Backspace', 'Delete', 'ArrowLeft', 'ArrowRight', 'Home', 'End'].indexOf(key) >= 0;\
if (key === 'ArrowDown') {\
  if (t === input) { event.preventDefault(); if (opts.length) opts[0].focus(); }\
  else if (at >= 0) { event.preventDefault(); opts[Math.min(at + 1, opts.length - 1)].focus(); }\
} else if (key === 'ArrowUp') {\
  if (t !== input) {\
    event.preventDefault();\
    if (at > 0) { opts[at - 1].focus(); } else { input.focus(); }\
  }\
} else if (key === 'Enter') {\
  if (t === input) {\
    event.preventDefault();\
    if (!box.hasAttribute('data-combobox-live')) {\
      var go = Array.from(box.querySelectorAll('[type=submit]')).find(function (b) {\
        return !(results && results.contains(b));\
      });\
      if (go) go.click();\
    }\
  }\
} else if (key === 'Escape') {\
  if (results && results.firstChild) {\
    event.preventDefault();\
    results.replaceChildren();\
    results.removeAttribute('data-init');\
    var status = box.querySelector('[role=status]');\
    if (status) status.textContent = '';\
    input.setAttribute('aria-expanded', 'false');\
    input.focus();\
  }\
} else if (at >= 0 && typing) {\
  input.focus();\
}";

/// Escape `text` for the inside of a double-quoted CSS string.
fn css_double_quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Escape `text` for the inside of a single-quoted JavaScript string literal.
fn js_single_quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(c),
        }
    }
    out
}

/// Builder for a labelled combobox. Construct with [`combobox`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct ComboboxBuilder {
    name: String,
    label: Markup,
    label_hidden: bool,
    value: Option<String>,
    hint: Option<Markup>,
    autofocus: bool,
    search_action: Option<String>,
    search_button: Option<Markup>,
    results: Markup,
    expanded: bool,
    id: Option<String>,
    results_id: Option<String>,
    test_id: Option<String>,
}

/// Start a combobox posting its query under `name`, labelled `label`.
///
/// `name` is both the input's `name` and, unless [`ComponentBuilder::with_id`]
/// overrides it, its `id`; the results container is `{id}-results` unless
/// [`ComboboxBuilder::results_id`] names it.
pub fn combobox(name: impl Into<String>, label: impl Render) -> ComboboxBuilder {
    ComboboxBuilder {
        name: name.into(),
        label: label.render(),
        label_hidden: false,
        value: None,
        hint: None,
        autofocus: false,
        search_action: None,
        search_button: None,
        results: html! {},
        expanded: false,
        id: None,
        results_id: None,
        test_id: None,
    }
}

impl ComboboxBuilder {
    /// Keep the label for assistive technology but take it out of the layout.
    pub fn label_hidden(mut self) -> Self {
        self.label_hidden = true;
        self
    }

    /// The current query, so a re-rendered page comes back holding what was typed.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Help text below the field, announced with the input via `aria-describedby`.
    pub fn hint(mut self, hint: impl Render) -> Self {
        self.hint = Some(hint.render());
        self
    }

    pub fn autofocus(mut self) -> Self {
        self.autofocus = true;
        self
    }

    /// The Datastar action run, debounced, as the user types.
    ///
    /// Taken as an opaque string because the tile has no Datastar crate
    /// dependency: build it with `shared_datastar::get`. Rendered verbatim as the
    /// value of `data-on:input__debounce.300ms`; with none set the attribute is
    /// absent and the field is a plain search input. Setting it also marks the
    /// wrapper `data-combobox-live`, which makes Enter in the input a no-op.
    pub fn search_action(mut self, action: impl Into<String>) -> Self {
        self.search_action = Some(action.into());
        self
    }

    /// The submit control that searches without JavaScript, placed right after
    /// the input. Without a [`search_action`](Self::search_action), Enter in the
    /// input clicks the first submit control outside the results, which is this
    /// one.
    pub fn search_button(mut self, button: impl Render) -> Self {
        self.search_button = Some(button.render());
        self
    }

    /// The results to show, typically a [`combobox_listbox`] or a message.
    pub fn results(mut self, results: impl Render) -> Self {
        self.results = results.render();
        self
    }

    /// Whether the results are showing; sets the input's `aria-expanded`.
    /// Default `false`.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// The id of the results container, which the input's `aria-controls` names and
    /// the fragment endpoint's [`combobox_results`] must be given. Default
    /// `{id}-results`.
    ///
    /// Set it when the input's id is not stable: the input may be renamed on a
    /// response to force the morph to replace it, and the container must keep the
    /// id the search endpoint answers for.
    pub fn results_id(mut self, results_id: impl Into<String>) -> Self {
        self.results_id = Some(results_id.into());
        self
    }

    fn resolved_id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }

    fn markup(&self) -> Markup {
        let id = self.resolved_id();
        let shell = FieldShell {
            id,
            label: &self.label,
            hint: self.hint.as_ref(),
            error: None,
            label_hidden: self.label_hidden,
            counter: None,
        };
        let described_by = shell.described_by();
        let results_id = self.results_id.clone().unwrap_or_else(|| default_results_id(id));
        let expanded = if self.expanded { "true" } else { "false" };
        let status_id = status_id(&results_id);
        let results = combobox_results(&results_id, &self.results, self.expanded);
        let control = html! {
            div class="combobox"
                onkeydown=(KEYDOWN)
                data-combobox-live[self.search_action.is_some()]
            {
                input
                    class="field-input"
                    id=(id)
                    name=(self.name)
                    type="search"
                    role="combobox"
                    aria-autocomplete="list"
                    aria-controls=(results_id)
                    aria-expanded=(expanded)
                    autocomplete="off"
                    value=[self.value.as_deref()]
                    aria-describedby=[described_by.as_deref()]
                    "data-on:input__debounce.300ms"=[self.search_action.as_deref()]
                    data-testid=[self.test_id.as_deref()]
                    autofocus[self.autofocus];
                @if let Some(button) = &self.search_button { (button) }
                (results)
                p id=(status_id) class="sr-only" role="status" {}
            }
        };
        shell.render(control)
    }
}

impl ComponentBuilder for ComboboxBuilder {
    fn id_mut(&mut self) -> &mut Option<String> {
        &mut self.id
    }

    fn test_id_mut(&mut self) -> &mut Option<String> {
        &mut self.test_id
    }

    fn build(self) -> Markup {
        self.markup()
    }
}

impl Render for ComboboxBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

fn default_results_id(input_id: &str) -> String {
    format!("{input_id}-results")
}

fn status_id(results_id: &str) -> String {
    format!("{results_id}-status")
}

/// The results container whose id is `results_id`.
///
/// The one function behind both the page render and the fragment endpoint, so
/// the two emit identical markup. Whenever the container is added or patched its
/// `data-init` sets `aria-expanded` to match `expanded` on the input whose
/// `aria-controls` is `results_id`, takes every option out of the Tab order, and
/// copies the text of the [`combobox_status`] line inside it into the persistent
/// `{results_id}-status` element.
///
/// `results_id` is escaped for the CSS string and the JavaScript string it lands
/// in, so a field name cannot end either literal early. The `data-init` holds no
/// `;` inside a statement and no newline, because Datastar splits an expression
/// on both.
pub fn combobox_results(results_id: &str, content: impl Render, expanded: bool) -> Markup {
    let state = if expanded { "true" } else { "false" };
    let selector = format!("[aria-controls=\"{}\"]", css_double_quoted(results_id));
    let selector = js_single_quoted(&selector);
    let container = js_single_quoted(results_id);
    let status = js_single_quoted(&status_id(results_id));
    let init = format!(
        "document.querySelector('{selector}')?.setAttribute('aria-expanded','{state}');\
         document.getElementById('{container}')?.querySelectorAll('[role=option]').forEach(o=>o.tabIndex=-1);\
         (document.getElementById('{status}')||{{}}).textContent=\
         document.getElementById('{container}')?.querySelector('[data-combobox-status]')?.textContent||''"
    );
    html! {
        div id=(results_id) class="combobox-results" data-init=(init) { (content) }
    }
}

/// The visible count or empty line inside the results.
///
/// `data-init` copies its text into the persistent status element, so a screen
/// reader hears the line the page shows. It carries no `role`: the live region
/// is the persistent element outside the results.
pub fn combobox_status(text: impl Render) -> Markup {
    html! {
        p class="combobox-status" data-combobox-status { (text) }
    }
}

/// A listbox of options, each wrapped in a presentational `li`.
///
/// `label` is the listbox's accessible name, so it is plain text rather than
/// `impl Render`: markup in an `aria-label` means nothing.
pub fn combobox_listbox(label: impl Into<String>, options: impl IntoIterator<Item = Markup>) -> Markup {
    let label = label.into();
    html! {
        ul role="listbox" aria-label=(label) {
            @for option in options {
                li role="none" { (option) }
            }
        }
    }
}

/// A listbox whose options sit under group headings: each group is an `li role="group"` named by
/// its `aria-label`, its visible `p.combobox-group` heading `aria-hidden` so the name is read once.
///
/// The arrow-key script collects every `[role=option]` inside the combobox, so grouped options
/// need no change to it.
pub fn combobox_groups(label: impl Into<String>, groups: impl IntoIterator<Item = (String, Vec<Markup>)>) -> Markup {
    let label = label.into();
    html! {
        ul role="listbox" aria-label=(label) {
            @for (name, options) in groups {
                @if !options.is_empty() {
                    li role="group" aria-label=(name) {
                        p class="combobox-group" aria-hidden="true" { (name) }
                        ul role="none" {
                            @for option in options {
                                li role="none" { (option) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Builder for one option. Construct with [`combobox_option`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct ComboboxOptionBuilder {
    content: Markup,
    name: String,
    value: String,
    aria_label: Option<String>,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start an option that submits `value` under `name` when chosen.
///
/// A submit button with `role="option"`: Enter or a click posts the enclosing
/// form with this button's name and value, so the choice is the caller's POST.
/// It carries no `tabindex`: without JavaScript Tab is the only way to reach it,
/// and with JavaScript the results' `data-init` takes it out of the Tab order.
pub fn combobox_option(
    content: impl Render,
    name: impl Into<String>,
    value: impl Into<String>,
) -> ComboboxOptionBuilder {
    ComboboxOptionBuilder {
        content: content.render(),
        name: name.into(),
        value: value.into(),
        aria_label: None,
        id: None,
        test_id: None,
    }
}

impl ComboboxOptionBuilder {
    /// An accessible name for an option whose content reads badly as one, such
    /// as a name line followed by a detail line.
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    fn markup(&self) -> Markup {
        html! {
            button
                type="submit"
                role="option"
                class="combobox-option"
                name=(self.name)
                value=(self.value)
                id=[self.id.as_deref()]
                aria-label=[self.aria_label.as_deref()]
                data-testid=[self.test_id.as_deref()]
            { (self.content) }
        }
    }
}

impl ComponentBuilder for ComboboxOptionBuilder {
    fn id_mut(&mut self) -> &mut Option<String> {
        &mut self.id
    }

    fn test_id_mut(&mut self) -> &mut Option<String> {
        &mut self.test_id
    }

    fn build(self) -> Markup {
        self.markup()
    }
}

impl Render for ComboboxOptionBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_input_is_a_combobox_controlling_its_results() {
        let out = combobox("person", "Person").build().into_string();
        assert!(out.contains(r#"type="search""#), "{out}");
        assert!(out.contains(r#"role="combobox""#), "{out}");
        assert!(out.contains(r#"aria-autocomplete="list""#), "{out}");
        assert!(out.contains(r#"aria-controls="person-results""#), "{out}");
        assert!(out.contains(r#"aria-expanded="false""#), "{out}");
        assert!(out.contains(r#"autocomplete="off""#), "{out}");
        assert!(
            out.contains(r#"<label class="field-label" for="person">Person</label>"#),
            "{out}"
        );
    }

    #[test]
    fn an_explicit_id_moves_the_results_target_with_it() {
        let out = combobox("person", "Person").with_id("p1").build().into_string();
        assert!(out.contains(r#"id="p1""#), "{out}");
        assert!(out.contains(r#"aria-controls="p1-results""#), "{out}");
        assert!(out.contains(r#"id="p1-results""#), "{out}");
        assert!(out.contains(r#"name="person""#), "{out}");
    }

    #[test]
    fn expanded_sets_aria_expanded_on_the_input() {
        let out = combobox("person", "Person").expanded(true).build().into_string();
        assert!(out.contains(r#"aria-expanded="true""#), "{out}");
    }

    #[test]
    fn the_search_action_is_rendered_verbatim_and_only_when_set() {
        let with = combobox("person", "Person")
            .search_action("go('/search?q=1')")
            .build()
            .into_string();
        assert!(with.contains(r#"data-on:input__debounce.300ms="go('/search?q=1')""#), "{with}");
        let without = combobox("person", "Person").build().into_string();
        assert!(!without.contains("data-on:"), "{without}");
    }

    #[test]
    fn the_hint_and_the_query_land_on_the_input() {
        let out = combobox("person", "Person")
            .hint("Surname first.")
            .value("Mül")
            .build()
            .into_string();
        assert!(out.contains(r#"aria-describedby="person-hint""#), "{out}");
        assert!(out.contains(r#"value="Mül""#), "{out}");
    }

    #[test]
    fn a_hidden_label_still_names_the_input_but_leaves_the_layout() {
        let out = combobox("person", "Person").label_hidden().build().into_string();
        assert!(
            out.contains(r#"<label class="field-label sr-only" for="person">Person</label>"#),
            "{out}"
        );
    }

    #[test]
    fn the_wrapper_carries_the_keyboard_handler_and_the_search_button_follows_the_input() {
        let out = combobox("person", "Person")
            .search_button(html! {
                button type="submit" { "Search" }
            })
            .build()
            .into_string();
        assert!(out.contains(r#"<div class="combobox" onkeydown="var box = this;"#), "{out}");
        let input = out.find("<input").expect("input");
        let button = out.find("Search").expect("search button");
        let results = out.find(r#"id="person-results""#).expect("results");
        assert!(input < button && button < results, "{out}");
    }

    #[test]
    fn results_container_state_follows_expanded() {
        let open = combobox_results("attributions.new-results", "x", true).into_string();
        assert!(open.contains(r#"id="attributions.new-results""#), "{open}");
        assert!(open.contains(r#"class="combobox-results""#), "{open}");
        assert!(
            open.contains(
                "data-init=\"document.querySelector('[aria-controls=&quot;attributions.new-results&quot;]')?.setAttribute('aria-expanded','true');"
            ),
            "{open}"
        );
        let closed = combobox_results("attributions.new-results", "x", false).into_string();
        assert!(closed.contains("setAttribute('aria-expanded','false');"), "{closed}");
    }

    #[test]
    fn the_init_takes_options_out_of_the_tab_order_and_copies_the_status_line() {
        let out = combobox_results("person-results", "x", true).into_string();
        assert!(
            out.contains(
                "document.getElementById('person-results')?.querySelectorAll('[role=option]').forEach(o=&gt;o.tabIndex=-1);"
            ),
            "{out}"
        );
        assert!(
            out.contains(
                "(document.getElementById('person-results-status')||{}).textContent=document.getElementById('person-results')?.querySelector('[data-combobox-status]')?.textContent||''\""
            ),
            "{out}"
        );
        assert!(!out.contains('\n'), "{out}");
    }

    #[test]
    fn the_status_line_is_marked_for_the_copy_and_has_no_live_region_role() {
        let out = combobox_status("3 results").into_string();
        assert_eq!(out, r#"<p class="combobox-status" data-combobox-status>3 results</p>"#);
    }

    #[test]
    fn a_persistent_status_element_sits_beside_the_results_container() {
        let out = combobox("person", "Person")
            .results(combobox_status("1 result"))
            .build()
            .into_string();
        let status = r#"<p id="person-results-status" class="sr-only" role="status"></p>"#;
        assert!(out.contains(status), "{out}");
        let results_end = out.find("1 result</p></div>").expect("results end");
        assert!(out.find(status).expect("status") > results_end, "{out}");
        let renamed = combobox("person", "Person")
            .with_id("p-1")
            .results_id("person-results")
            .build()
            .into_string();
        assert!(renamed.contains(r#"id="person-results-status""#), "{renamed}");
    }

    #[test]
    fn the_wrapper_is_marked_live_only_with_a_search_action() {
        let with = combobox("person", "Person").search_action("go()").build().into_string();
        assert!(with.contains(r#"<div class="combobox" onkeydown="#), "{with}");
        assert!(with.contains(r#"" data-combobox-live>"#), "{with}");
        let without = combobox("person", "Person").build().into_string();
        assert!(!without.contains(r#"" data-combobox-live"#), "{without}");
    }

    #[test]
    fn an_id_with_a_quote_or_backslash_cannot_end_the_selector_or_the_javascript_string() {
        let out = combobox_results(r#"a"b'c\d"#, "x", true).into_string();
        // CSS-escaped first, then JavaScript-escaped on top: every backslash doubles and `'` gains
        // one.
        assert!(
            out.contains(r#"querySelector('[aria-controls=&quot;a\\&quot;b\'c\\\\d&quot;]')?.setAttribute"#),
            "{out}"
        );
        assert!(out.contains(r"getElementById('a&quot;b\'c\\d')"), "{out}");
    }

    #[test]
    fn a_results_id_decouples_the_container_from_the_input_id() {
        let out = combobox("person", "Person")
            .with_id("person-1a2b")
            .results_id("person-results")
            .results("hit")
            .expanded(true)
            .build()
            .into_string();
        assert!(out.contains(r#"id="person-1a2b""#), "{out}");
        assert!(out.contains(r#"aria-controls="person-results""#), "{out}");
        assert!(out.contains(r#"<div id="person-results" class="combobox-results""#), "{out}");
        assert!(!out.contains("person-1a2b-results"), "{out}");
        assert!(out.contains("[aria-controls=&quot;person-results&quot;]"), "{out}");
        // The fragment endpoint, given the same id, replaces exactly this container.
        let fragment = combobox_results("person-results", "hit", true).into_string();
        assert!(out.contains(&fragment), "{out}");
    }

    #[test]
    fn splice_equals_build_and_embeds_the_fragment_markup() {
        let built = combobox("person", "Person").results("hit").expanded(true).build().into_string();
        let spliced = html! {
            (combobox("person", "Person").results("hit").expanded(true))
        }
        .into_string();
        assert_eq!(built, spliced);
        let fragment = combobox_results("person-results", "hit", true).into_string();
        assert!(built.contains(&fragment), "{built}");
    }

    #[test]
    fn a_listbox_wraps_each_option_in_a_presentational_item() {
        let options = [
            combobox_option("Ada", "pick", "1").build(),
            combobox_option("Bo", "pick", "2").build(),
        ];
        let out = combobox_listbox("People", options).into_string();
        assert!(out.contains(r#"<ul role="listbox" aria-label="People">"#), "{out}");
        assert_eq!(out.matches(r#"<li role="none">"#).count(), 2, "{out}");
    }

    #[test]
    fn a_grouped_listbox_nests_presentational_options_in_labelled_groups() {
        let groups = [
            (
                "Humanities".to_string(),
                vec![
                    combobox_option("History", "pick", "1").build(),
                    combobox_option("Linguistics", "pick", "2").build(),
                ],
            ),
            ("Empty".to_string(), Vec::new()),
            ("Sciences".to_string(), vec![combobox_option("Physics", "pick", "3").build()]),
        ];
        let out = combobox_groups("Disciplines", groups).into_string();
        assert_eq!(out.matches(r#"role="listbox""#).count(), 1, "{out}");
        assert!(out.contains(r#"<ul role="listbox" aria-label="Disciplines">"#), "{out}");
        assert!(out.contains(r#"<li role="group" aria-label="Humanities">"#), "{out}");
        assert!(out.contains(r#"<li role="group" aria-label="Sciences">"#), "{out}");
        assert_eq!(
            out.matches(r#"<p class="combobox-group" aria-hidden="true">"#).count(),
            2,
            "{out}"
        );
        assert_eq!(out.matches(r#"role="group""#).count(), 2, "{out}");
        assert_eq!(out.matches(r#"<li role="none">"#).count(), 3, "{out}");
        assert!(!out.contains("Empty"), "{out}");
    }

    #[test]
    fn an_option_is_a_submit_button_that_stays_in_the_tab_order() {
        let out = combobox_option("Ada", "pick", "42")
            .aria_label("Ada Lovelace")
            .build()
            .into_string();
        assert!(out.contains(r#"type="submit""#), "{out}");
        assert!(out.contains(r#"role="option""#), "{out}");
        assert!(out.contains(r#"class="combobox-option""#), "{out}");
        assert!(out.contains(r#"name="pick""#), "{out}");
        assert!(out.contains(r#"value="42""#), "{out}");
        assert!(out.contains(r#"aria-label="Ada Lovelace""#), "{out}");
        assert!(!out.contains("tabindex"), "{out}");
        let plain = combobox_option("Ada", "pick", "42").build().into_string();
        assert!(!plain.contains("aria-label"), "{plain}");
    }
}

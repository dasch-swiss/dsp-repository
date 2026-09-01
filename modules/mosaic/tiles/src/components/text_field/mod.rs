//! Text field tile: a label, a single-line `<input>`, and an optional hint,
//! wired together as one group.
//!
//! `text_field(name, label)` returns a [`TextFieldBuilder`]; set options with
//! chained methods and either splice it into `html!` directly (it implements
//! [`Render`]) or call `.build()` for a standalone `Markup`. See
//! `docs/src/mosaic/component-api-conventions.md`.
//!
//! ## Why the label and hint are inside the tile, not around it
//!
//! A bare input tile would leave the caller to repeat the field name three
//! times — on `id`, on the label's `for`, and on `aria-describedby` — and a
//! mismatch in any of them is silent: the field still renders, it just stops
//! being announced correctly. Accessibility is the tile's responsibility, so
//! the name is given once and the tile derives all three. `with_id` overrides
//! the derived id (and the `for`/`aria-describedby` that follow it) for the
//! case where one page renders the same field name twice.
//!
//! The `field-*` classes are deliberately not `text-field-*`: a textarea and a
//! select want the same label, border and hint treatment, and will share this
//! shell when a screen needs them.

use maud::{html, Markup, Render};

use crate::builder::ComponentBuilder;

/// The `type` attribute values this tile renders. Arms are added when a screen
/// needs one, rather than mirroring the whole HTML input vocabulary up front.
#[derive(Clone, Copy, Debug, Default)]
pub enum InputType {
    #[default]
    Text,
    Email,
}

impl InputType {
    /// The `type` attribute value.
    pub fn as_str(self) -> &'static str {
        match self {
            InputType::Text => "text",
            InputType::Email => "email",
        }
    }
}

/// Builder for a labelled text input. Construct with [`text_field`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct TextFieldBuilder {
    name: String,
    label: Markup,
    input_type: InputType,
    value: Option<String>,
    hint: Option<Markup>,
    autocomplete: Option<String>,
    inputmode: Option<&'static str>,
    pattern: Option<String>,
    maxlength: Option<u32>,
    required: bool,
    autofocus: bool,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a text field posting under `name`, labelled `label`.
///
/// `name` is both the submitted field name and — unless [`ComponentBuilder::with_id`]
/// overrides it — the `id` the label and hint point at.
pub fn text_field(name: impl Into<String>, label: impl Render) -> TextFieldBuilder {
    TextFieldBuilder {
        name: name.into(),
        label: label.render(),
        input_type: InputType::default(),
        value: None,
        hint: None,
        autocomplete: None,
        inputmode: None,
        pattern: None,
        maxlength: None,
        required: false,
        autofocus: false,
        id: None,
        test_id: None,
    }
}

impl TextFieldBuilder {
    /// Set the input type (default `Text`).
    pub fn input_type(mut self, input_type: InputType) -> Self {
        self.input_type = input_type;
        self
    }

    /// Pre-fill the input, so a rejected form comes back holding what was typed.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Add help text below the input, announced with it via `aria-describedby`.
    pub fn hint(mut self, hint: impl Render) -> Self {
        self.hint = Some(hint.render());
        self
    }

    /// Set the `autocomplete` token — what the browser may fill in here.
    ///
    /// A standard form attribute rather than an ARIA one, and the caller is who
    /// knows what the field collects: `"email"` on a sign-in address invites
    /// autofill, while `"off"` on an administrator entering somebody else's
    /// address deliberately refuses it.
    pub fn autocomplete(mut self, token: impl Into<String>) -> Self {
        self.autocomplete = Some(token.into());
        self
    }

    /// Mark the input required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Focus this input on page load.
    pub fn autofocus(mut self) -> Self {
        self.autofocus = true;
        self
    }

    /// Configure the field as a numeric one-time code of `digits` digits.
    ///
    /// One intent method rather than four knobs, because the four attributes
    /// only work together: `autocomplete="one-time-code"` is what lets a phone
    /// offer the code from a message, `inputmode="numeric"` is what raises a
    /// number keypad instead of a letter keyboard, and the pattern and length
    /// are what stop a wrong-length entry before it is posted. Setting three of
    /// the four is a field that looks right and autofills nothing.
    pub fn one_time_code(mut self, digits: u32) -> Self {
        self.autocomplete = Some("one-time-code".to_string());
        self.inputmode = Some("numeric");
        self.pattern = Some(format!("[0-9]{{{digits}}}"));
        self.maxlength = Some(digits);
        self
    }

    /// The id the label and hint point at: the explicit one if set, else the name.
    fn resolved_id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }

    fn markup(&self) -> Markup {
        let id = self.resolved_id();
        let hint_id = self.hint.as_ref().map(|_| format!("{id}-hint"));
        html! {
            div class="field" {
                label class="field-label" for=(id) { (self.label) }
                input
                    class="field-input"
                    id=(id)
                    name=(self.name)
                    type=(self.input_type.as_str())
                    value=[self.value.as_deref()]
                    autocomplete=[self.autocomplete.as_deref()]
                    inputmode=[self.inputmode]
                    pattern=[self.pattern.as_deref()]
                    maxlength=[self.maxlength]
                    aria-describedby=[hint_id.as_deref()]
                    data-testid=[self.test_id.as_deref()]
                    required[self.required]
                    autofocus[self.autofocus];
                @if let Some(hint) = &self.hint {
                    p class="field-hint" id=[hint_id.as_deref()] { (hint) }
                }
            }
        }
    }
}

impl ComponentBuilder for TextFieldBuilder {
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

impl Render for TextFieldBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_type_mapping() {
        assert_eq!(InputType::Text.as_str(), "text");
        assert_eq!(InputType::Email.as_str(), "email");
    }

    #[test]
    fn the_label_points_at_the_input_it_labels() {
        // The whole reason the label lives inside the tile: `for` and `id` are
        // derived from one name, so they cannot drift apart.
        let out = text_field("email", "Email address").build().into_string();
        assert!(
            out.contains(r#"<label class="field-label" for="email">Email address</label>"#),
            "{out}"
        );
        assert!(out.contains(r#"id="email""#), "{out}");
        assert!(out.contains(r#"name="email""#), "{out}");
    }

    #[test]
    fn a_hint_is_announced_with_the_input_rather_than_orphaned() {
        let out = text_field("shortcodes", "Project shortcodes")
            .hint("Separated by commas.")
            .build()
            .into_string();
        assert!(out.contains(r#"aria-describedby="shortcodes-hint""#), "{out}");
        assert!(out.contains(r#"id="shortcodes-hint""#), "{out}");
        assert!(out.contains("Separated by commas."), "{out}");
    }

    #[test]
    fn no_hint_means_no_describedby_pointing_at_nothing() {
        let out = text_field("name", "Name").build().into_string();
        assert!(!out.contains("aria-describedby"), "{out}");
        assert!(!out.contains("field-hint"), "{out}");
    }

    #[test]
    fn an_explicit_id_moves_the_label_and_hint_with_it() {
        // Two forms on one page would otherwise collide on the derived id, and
        // a duplicate id silently mislabels one of them.
        let out = text_field("email", "Email address")
            .with_id("invite-email")
            .hint("Where the invitation goes.")
            .build()
            .into_string();
        assert!(out.contains(r#"for="invite-email""#), "{out}");
        assert!(out.contains(r#"id="invite-email""#), "{out}");
        assert!(out.contains(r#"aria-describedby="invite-email-hint""#), "{out}");
        assert!(out.contains(r#"id="invite-email-hint""#), "{out}");
        // The submitted name is unchanged — only the id moved.
        assert!(out.contains(r#"name="email""#), "{out}");
    }

    #[test]
    fn one_time_code_sets_all_four_attributes_together() {
        // Three of the four is a field that looks right and autofills nothing.
        let out = text_field("code", "Six-digit code").one_time_code(6).build().into_string();
        assert!(out.contains(r#"autocomplete="one-time-code""#), "{out}");
        assert!(out.contains(r#"inputmode="numeric""#), "{out}");
        assert!(out.contains(r#"pattern="[0-9]{6}""#), "{out}");
        assert!(out.contains(r#"maxlength="6""#), "{out}");
    }

    #[test]
    fn a_value_comes_back_so_a_rejected_form_keeps_what_was_typed() {
        let out = text_field("shortcodes", "Project shortcodes")
            .value("0801, nope!")
            .build()
            .into_string();
        assert!(out.contains(r#"value="0801, nope!""#), "{out}");
    }

    #[test]
    fn required_and_autofocus_render_as_boolean_attributes() {
        let out = text_field("email", "Email address")
            .required()
            .autofocus()
            .build()
            .into_string();
        assert!(out.contains("required"), "{out}");
        assert!(out.contains("autofocus"), "{out}");
    }

    #[test]
    fn omits_optional_attributes_when_unset() {
        let out = text_field("name", "Name").build().into_string();
        assert!(!out.contains("value="), "{out}");
        assert!(!out.contains("autocomplete="), "{out}");
        assert!(!out.contains("inputmode="), "{out}");
        assert!(!out.contains("pattern="), "{out}");
        assert!(!out.contains("maxlength="), "{out}");
        assert!(!out.contains("required"), "{out}");
        assert!(!out.contains("autofocus"), "{out}");
        assert!(!out.contains("data-testid="), "{out}");
    }

    #[test]
    fn autocomplete_off_is_as_expressible_as_a_token() {
        let out = text_field("email", "Email address")
            .input_type(InputType::Email)
            .autocomplete("off")
            .build()
            .into_string();
        assert!(out.contains(r#"type="email""#), "{out}");
        assert!(out.contains(r#"autocomplete="off""#), "{out}");
    }

    #[test]
    fn label_and_hint_accept_markup() {
        let hint = html! {
            "For example "
            code { "0801" }
            "."
        };
        let out = text_field(
            "shortcodes",
            html! {
                span { "Codes" }
            },
        )
        .hint(hint)
        .build()
        .into_string();
        assert!(out.contains("<span>Codes</span>"), "{out}");
        assert!(out.contains("<code>0801</code>"), "{out}");
    }

    #[test]
    fn test_id_lands_on_the_input_not_the_wrapper() {
        // The input is what a test types into.
        let out = text_field("name", "Name").with_test_id("name-input").build().into_string();
        let testid_at = out.find("data-testid").expect("test id missing");
        let input_at = out.find("<input").expect("input missing");
        assert!(input_at < testid_at, "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = text_field("name", "Name").required().build().into_string();
        let spliced = html! {
            (text_field("name", "Name").required())
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}

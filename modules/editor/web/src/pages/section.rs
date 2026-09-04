//! One form section: the rail, the fields, and the save control.
//!
//! [`page`] is the whole page; [`region`] is the part a save replaces. Both come
//! from one [`SectionView`], so the plain and enhanced paths cannot drift.
//!
//! Three invariants:
//!
//! - **The region is the rail, the status and the form together**, under one id. Patching only the
//!   `<form>` leaves the rail showing the counts from before the save, so answering the last
//!   required field goes quiet while the rail still says something is missing.
//! - **The status region is rendered empty from the first load.** An `aria-live` region announces a
//!   *change* to content it already holds; one morphed in together with its text is widely reported
//!   not to announce at all. `empty:hidden` is what keeps that free.
//! - **No field is `required`, and the form is not `novalidate`.** A draft may be missing anything
//!   (REQ-1.9) and saving one must always work (REQ-1.10), so nothing is `required`. Validation
//!   stays on because `startDate`/`endDate` are `type="date"`, which cannot hold a half-typed date
//!   — the value comes back empty, so with validation off, fiddling the year of a real date and
//!   saving would clear it. Datastar gates its form path on the same flag.

use editor_core::draft::ProjectDraft;
use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::button::{button, ButtonType};

use crate::form::obligation::{section_progress, SectionProgress};
use crate::form::registry::{sections_for, Audience, Section};
use crate::form::widgets::{field_row, Mode};

/// The id the enhanced path's patch targets. Also the anchor a save returns to.
pub const REGION_ID: &str = "project-section";

/// Why the form is read-only.
///
/// A reason rather than a `bool`, because the page has to say which it is: a
/// depositor whose work is queued and one whose work a reviewer has open should
/// not be told the same thing, and only one of them can expect it back soon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locked {
    /// Submitted and waiting for a reviewer to pick it up.
    Submitted,
    /// A reviewer has it open.
    InReview,
}

impl Locked {
    /// What the reader is told, and what they can do about it.
    const fn message(self) -> &'static str {
        match self {
            Self::Submitted => {
                "This project has been submitted for review, so the form is read-only until RDU \
                                picks it up or returns it to you. Ask RDU if you need to change something in \
                                the meantime."
            }
            Self::InReview => {
                "RDU is reviewing this project, so the form is read-only. It becomes editable \
                               again when the review finishes or the record is returned to you."
            }
        }
    }

    const fn heading(self) -> &'static str {
        match self {
            Self::Submitted => "Submitted for review",
            Self::InReview => "In review",
        }
    }
}

/// What the `POST` that led to this rendering did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice<'a> {
    /// The draft was stored.
    Saved,
    /// The save was refused, and why. Not a field-level error — those arrive
    /// with submit validation; this is the whole-form kind: a live submission,
    /// or storage that would not take the write.
    Refused(&'a str),
}

/// Everything one section rendering needs.
///
/// A struct rather than a long argument list: half of these are `Option`s of
/// similar types, and adjacent optional arguments of one type are silently
/// swappable — the same reason the form tiles' shell is a struct.
pub struct SectionView<'a> {
    pub shortcode: &'a str,
    /// The published project's name, or `None` for a shortcode the published set
    /// does not hold (REQ-2.3).
    pub project_name: Option<&'a str>,
    pub section: &'static Section,
    /// Which fields and sections this reader sees. Not a permission check in
    /// itself — the registry is consulted by the decoder too, so the form and
    /// the save cannot disagree about which fields a depositor owns.
    pub audience: Audience,
    pub draft: &'a ProjectDraft,
    /// `None` while the project is editable.
    pub locked: Option<Locked>,
    /// What RDU asked for when it returned this project to the depositor
    /// (REQ-4.5), or `None` for a draft nobody has reviewed.
    ///
    /// On the form rather than on a page of its own: the requirement retains
    /// the note and names nowhere to read it, and the place a depositor acts on
    /// it is the form they act on it *in*. It rides inside the region, so it is
    /// still there after a save patches it.
    pub reviewer_note: Option<&'a str>,
    /// When the stored draft was last written, formatted. `None` when the form
    /// is showing published metadata that nobody has saved over yet (REQ-1.1).
    pub saved_at: Option<&'a str>,
    pub notice: Option<Notice<'a>>,
}

impl SectionView<'_> {
    fn mode(&self) -> Mode {
        if self.locked.is_some() {
            Mode::ReadOnly
        } else {
            Mode::Editable
        }
    }

    fn action(&self) -> String {
        format!("/projects/{}/sections/{}", self.shortcode, self.section.id)
    }
}

/// The whole page: the project heading, then the region a save replaces.
pub fn page(view: &SectionView<'_>) -> Markup {
    html! {
        div class="max-w-5xl py-8" { (heading(view)) (region(view)) }
    }
}

/// The rail, the status region and the form — everything a save can change.
///
/// Rendered on its own for the enhanced path's patch, and spliced into [`page`]
/// for the plain one, so the two cannot drift.
pub fn region(view: &SectionView<'_>) -> Markup {
    html! {
        section id=(REGION_ID) class="grid gap-6 md:grid-cols-[16rem_1fr] items-start" {
            (rail(view))
            div { (status(view)) (form(view)) }
        }
    }
}

/// The project's name and shortcode, and the way back to the list.
fn heading(view: &SectionView<'_>) -> Markup {
    html! {
        div class="mb-6" {
            @match view.project_name {
                Some(name) => {
                    h1 class="font-display text-2xl mb-1" { (name) }
                    p class="font-mono text-sm text-gray-600" { (view.shortcode) }
                }
                None => {
                    h1 class="font-display text-2xl mb-1" { "Project " (view.shortcode) }
                    // REQ-2.3: a project may exist only locally, and REQ-1.1's
                    // "current published metadata" is then empty. Said plainly,
                    // because a blank form with no explanation reads as a
                    // failure to load.
                    p class="text-gray-600" {
                        "This project is not in the published set this deployment carries, so there was nothing \
                         to pre-fill. Anything you enter is kept as a draft."
                    }
                }
            }
            p class="mt-2" {
                a href="/projects" class="underline" { "Back to your projects" }
            }
        }
    }
}

/// The rail link's accessible name, when its two lines would otherwise run
/// together.
///
/// The title and the progress are adjacent `<span>`s with no whitespace between
/// them, because a flex column is what puts them on two lines — so the name
/// computation concatenates them into "Overview5 of 5 required". A separator in
/// the markup would either show as stray punctuation or depend on a
/// whitespace-only flex item not being rendered, so the name is stated instead.
///
/// `None` for a section with no requirements, where the visible title is already
/// the whole name and an `aria-label` repeating it would be one more string to
/// keep in step. The label always *starts* with the visible title, which is what
/// WCAG 2.5.3 (Label in Name) asks of an `aria-label` over visible text.
fn rail_link_label(title: &str, progress: &SectionProgress) -> Option<String> {
    progress.has_requirements().then(|| format!("{title}, {}", progress.summary()))
}

/// One rail link's classes.
///
/// Two complete literal strings rather than a base plus a conditional suffix:
/// Tailwind collects classes by scanning source text, so a class assembled at
/// runtime is one the build never emits — and the failure is silent, an
/// unstyled link with no error anywhere.
const fn rail_link_class(current: bool) -> &'static str {
    if current {
        "flex flex-col rounded bg-neutral-100 px-3 py-2 font-bold focus-visible:outline-2 \
         focus-visible:outline-primary-600"
    } else {
        "flex flex-col rounded px-3 py-2 hover:bg-neutral-100 focus-visible:outline-2 \
         focus-visible:outline-primary-600"
    }
}

/// The section rail: every section this reader sees, with its obligation state.
///
/// A `<nav>` with a list, because it is navigation and a screen reader user
/// needs to be able to jump to it and count the items. The current section
/// carries `aria-current="page"` rather than only a colour.
fn rail(view: &SectionView<'_>) -> Markup {
    html! {
        nav class="md:sticky md:top-4" aria-label="Form sections" {
            ol class="flex flex-col gap-1" {
                @for section in sections_for(view.audience) {
                    @let progress = section_progress(section, view.audience, view.draft);
                    @let current = section.id == view.section.id;
                    li {
                        a   href={ "/projects/" (view.shortcode) "/sections/" (section.id) }
                            class=(rail_link_class(current))
                            aria-current=[current.then_some("page")]
                            aria-label=[rail_link_label(section.title, &progress)]
                        {
                            span { (section.title) }
                            @if progress.has_requirements() {
                                // Both numbers, always, and in words: a rail
                                // that showed only what is outstanding cannot
                                // tell a finished section from an empty one, and
                                // one that marked completion with a colour or a
                                // tick alone would not say it to everyone.
                                span class="text-xs text-neutral-600" { (progress.summary()) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The live region a save writes its outcome into. Always present; see the
/// module docs. `empty:hidden` is what keeps an always-present region free —
/// without it every section carries an empty block's line box, the same reason
/// the form tiles' error region carries `.field-error:empty`.
fn status(view: &SectionView<'_>) -> Markup {
    html! {
        div class="empty:hidden" aria-live="polite" {
            @match view.notice {
                Some(Notice::Saved) => {
                    ({
                        alert("Draft saved.")
                            .variant(AlertVariant::Success)
                            .class("mb-4")
                    })
                }
                Some(Notice::Refused(message)) => {
                    (alert(message).variant(AlertVariant::Warning).class("mb-4"))
                }
                None => {}
            }
        }
    }
}

/// The section's fields, and the control that saves them.
fn form(view: &SectionView<'_>) -> Markup {
    let action = view.action();
    html! {
        @if let Some(note) = view.reviewer_note { (crate::pages::review::reviewer_note(note)) }
        @if let Some(locked) = view.locked {
            ({
                alert(locked.message())
                    .variant(AlertVariant::Warning)
                    .title(locked.heading())
                    .class("mb-4")
            })
        }
        h2 class="font-display text-xl mb-4" { (view.section.title) }
        // The `action` is the URL this form was fetched from, so a rejected save
        // re-renders somewhere that still answers `GET`. `data-on:submit` needs
        // no `__prevent`: Datastar 1.0.2 calls `preventDefault` unconditionally
        // for a `submit` event on a form element, so adding one would be noise.
        // With no script it is an ordinary POST and the server redirects.
        form
            id="section-form"
            method="post"
            action=(action)
            class="flex flex-col gap-6"
            data-on:submit={ "@post('" (action) "', {contentType: 'form'})" }
        {
            @for field in view.section.fields_for(view.audience) {
                // No wrapper and no obligation pill here: the pill is inside the
                // field's own label, which is the only way it reaches a screen
                // reader now that nothing is `required`. See `widgets::labelled`.
                (field_row(field, view.draft, view.mode()))
            }
            @if view.locked.is_none() {
                div class="flex items-center gap-4" {
                    (button("Save draft").button_type(ButtonType::Submit))
                    @if let Some(saved_at) = view.saved_at {
                        p class="text-sm text-gray-600" { "Draft last saved " (saved_at) "." }
                    } @else {
                        p class="text-sm text-gray-600" {
                            "Nothing saved yet — this form shows the project's published metadata."
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_core::draft::ProjectDraft;
    use serde_json::json;

    use super::*;
    use crate::form::registry::{field, section};

    /// A draft over a real committed project, so the fields under test hold what
    /// the corpus actually holds rather than what a fixture assumes.
    pub(super) fn published_draft() -> ProjectDraft {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        ProjectDraft::from_raw(published.get("0801d").expect("0801d is in the committed corpus"))
    }

    pub(super) fn view<'a>(draft: &'a ProjectDraft, section_id: &str, audience: Audience) -> SectionView<'a> {
        SectionView {
            shortcode: "0801d",
            project_name: Some("Bernoulli-Euler Online"),
            section: section(section_id).expect("a known section"),
            audience,
            draft,
            locked: None,
            reviewer_note: None,
            saved_at: None,
            notice: None,
        }
    }

    fn overview(draft: &ProjectDraft) -> String {
        page(&view(draft, "overview", Audience::Everyone)).into_string()
    }

    #[test]
    fn the_form_posts_to_the_url_it_was_fetched_from() {
        // A write posting to a path with no `GET` strands a rejected save on a
        // bare 405 — the dead end `POST /depositors/{id}` briefly was.
        let out = overview(&published_draft());
        assert!(out.contains(r#"action="/projects/0801d/sections/overview""#), "{out}");
        assert!(out.contains(r#"method="post""#), "{out}");
    }

    #[test]
    fn the_enhanced_path_posts_the_form_body_rather_than_signals() {
        // `contentType: 'form'` is what makes the body
        // `application/x-www-form-urlencoded`; Datastar transmits no signals on a
        // form-content-type request, which is what `editor_core::form` reads.
        // No `__prevent`: the bundle calls `preventDefault` unconditionally for a
        // `submit` event on a form element.
        let out = overview(&published_draft());
        assert!(
            out.contains(r#"data-on:submit="@post('/projects/0801d/sections/overview', {contentType: 'form'})""#),
            "{out}"
        );
        assert!(!out.contains("submit__prevent"), "{out}");
        // Keyed plugin attributes use `:`, not `-`. The hyphen form is a console
        // error and an inert control, and a snapshot asserting the attribute is
        // present passes either way.
        assert!(!out.contains("data-on-submit"), "{out}");
    }

    #[test]
    fn a_display_only_field_renders_its_value_and_no_control() {
        // REQ-1.5. A control here would post, and an empty one would clear a
        // value the reader was never able to change.
        let out = page(&view(&published_draft(), "overview", Audience::RduOnly)).into_string();
        assert!(out.contains("Shortcode"), "{out}");
        assert!(!out.contains(r#"name="shortcode""#), "{out}");
        assert!(!out.contains(r#"name="pid""#), "{out}");
        assert!(!out.contains(r#"name="id""#), "{out}");
    }

    #[test]
    fn a_field_the_form_cannot_read_yet_says_so_and_posts_nothing() {
        // Silently absent would leave a depositor who cannot find "Status" in
        // the section the published page shows it in concluding the form lost
        // it. Posting nothing is what keeps its stored value untouched
        // (REQ-1.7), because no applier names the field.
        let draft = published_draft();
        let out = overview(&draft);
        assert!(field("status").expect("status").shape.is_none(), "the premise of this test");
        assert!(out.contains("Status"), "{out}");
        assert!(out.contains("not editable here yet"), "{out}");
        assert!(!out.contains(r#"name="status""#), "{out}");
    }

    #[test]
    fn an_editable_field_renders_a_control_that_posts_even_when_empty() {
        // The other half of "absent is not empty": a section posts its own
        // fields whether or not they hold anything, so a cleared field arrives
        // present-and-empty rather than absent.
        let mut draft = published_draft();
        draft.remove("officialName");
        let out = overview(&draft);
        assert!(out.contains(r#"name="officialName""#), "{out}");
        assert!(out.contains(r#"name="name""#), "{out}");
    }

    #[test]
    fn a_placeholder_sentinel_renders_as_an_empty_control() {
        // The rule the whole untouched-save guarantee rests on: `MISSING` is
        // filtered out of DPE and of OAI-PMH, so showing it here would make this
        // the one place in the platform that presents an internal marker as a
        // value — and the depositor would then have to delete it by hand.
        let mut draft = published_draft();
        draft.set("endDate", json!("MISSING"));
        let out = overview(&draft);
        assert!(out.contains(r#"name="endDate""#), "{out}");
        assert!(!out.contains("MISSING"), "{out}");
    }

    #[test]
    fn a_language_map_renders_the_offered_languages_plus_any_the_value_carries() {
        // Offering only a closed set would drop `ar` — live in two committed
        // files — on the first save: a tag with no control posts nothing, and a
        // map rebuilt from the body would not carry it.
        let mut draft = published_draft();
        draft.set("description", json!({"en": "English text", "ar": "نص عربي"}));
        let out = overview(&draft);
        for tag in ["de", "en", "fr", "it", "ar"] {
            assert!(out.contains(&format!(r#"name="description.{tag}""#)), "{tag}: {out}");
        }
        // Named in words, not by code, for the tags the corpus actually uses.
        assert!(out.contains("Arabic"), "{out}");
        assert!(out.contains("English"), "{out}");
    }

    #[test]
    fn a_language_group_is_named_by_a_legend_because_each_control_is_a_language() {
        // A `<label for>` needs one control to point at, and this field has one
        // per language, each already labelled — so the field's own name can only
        // reach assistive technology as a `<legend>`.
        let out = overview(&published_draft());
        // The obligation pill rides inside the legend, so the group's accessible
        // name carries it too — see `a_required_field_says_so_inside_its_own_label`.
        assert!(out.contains(r#"<legend class="field-label">Description <span"#), "{out}");
        assert!(out.contains("</span></legend>"), "{out}");
    }

    #[test]
    fn the_rail_marks_the_current_section_in_more_than_a_colour() {
        let out = overview(&published_draft());
        assert!(out.contains(r#"aria-current="page""#), "{out}");
        assert_eq!(out.matches(r#"aria-current="page""#).count(), 1, "{out}");
        assert!(out.contains(r#"aria-label="Form sections""#), "{out}");
    }

    #[test]
    fn the_rail_states_both_numbers_so_a_finished_section_cannot_look_empty() {
        let draft = published_draft();
        let filled = overview(&draft);
        let empty = page(&view(&ProjectDraft::default(), "overview", Audience::Everyone)).into_string();
        // A depositor's overview is all answered for this project, so the two
        // renderings must differ — which they cannot if only what is outstanding
        // is shown.
        let progress = section_progress(section("overview").expect("overview"), Audience::Everyone, &draft);
        assert!(filled.contains(&progress.summary()), "{filled}");
        assert!(empty.contains(&format!("0 of {} required", progress.required)), "{empty}");
    }

    #[test]
    fn a_depositor_s_rail_does_not_link_to_the_rdu_only_section() {
        // Present and empty would be a rail entry that goes nowhere.
        let draft = published_draft();
        let depositor = overview(&draft);
        assert!(!depositor.contains("/sections/legal"), "{depositor}");
        let rdu = page(&view(&draft, "overview", Audience::RduOnly)).into_string();
        assert!(rdu.contains("/sections/legal"), "{rdu}");
    }

    #[test]
    fn the_status_region_is_in_the_dom_before_there_is_anything_to_announce() {
        // An `aria-live` region announces a *change* to content it already
        // holds; one inserted together with its text is widely reported not to
        // announce at all, and the enhanced path is exactly that case.
        let out = overview(&published_draft());
        assert!(out.contains(r#"aria-live="polite""#), "{out}");
    }

    #[test]
    fn a_saved_notice_lands_inside_the_live_region() {
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.notice = Some(Notice::Saved);
        let out = region(&view).into_string();
        let live = out.find(r#"aria-live="polite""#).expect("the live region");
        let saved = out.find("Draft saved.").expect("the notice");
        assert!(live < saved, "the notice must be inside the region, not before it: {out}");
    }

    #[test]
    fn a_locked_project_renders_values_and_no_way_to_save() {
        // Nothing may change under a reviewer, and a save button that refuses is
        // worse than no button: the depositor presses it, waits, and is told no.
        let draft = published_draft();
        for locked in [Locked::Submitted, Locked::InReview] {
            let mut view = view(&draft, "overview", Audience::Everyone);
            view.locked = Some(locked);
            let out = page(&view).into_string();
            assert!(!out.contains(r#"name="name""#), "{locked:?}: {out}");
            assert!(!out.contains("Save draft"), "{locked:?}: {out}");
            assert!(out.contains(locked.heading()), "{locked:?}: {out}");
            // The value is still readable — a read-only form is the same form,
            // not a blank page.
            assert!(out.contains("Bernoulli"), "{locked:?}: {out}");
        }
    }

    #[test]
    fn the_two_locked_states_do_not_say_the_same_thing() {
        // Only one of them can expect the record back soon, and a depositor
        // deciding whether to wait or to ask needs to know which.
        assert_ne!(Locked::Submitted.message(), Locked::InReview.message());
        assert_ne!(Locked::Submitted.heading(), Locked::InReview.heading());
    }

    #[test]
    fn an_unpublished_project_opens_without_reading_as_a_failure() {
        // REQ-2.3: a project may exist only locally, and REQ-1.1's "current
        // published metadata" is then empty. A blank form with no explanation
        // reads as a page that failed to load.
        let draft = ProjectDraft::default();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.project_name = None;
        let out = page(&view).into_string();
        assert!(out.contains("Project 0801d"), "{out}");
        assert!(out.contains("nothing to pre-fill"), "{out}");
        // Still editable: a local-only project is a project.
        assert!(out.contains("Save draft"), "{out}");
    }

    #[test]
    fn the_save_control_says_whether_anything_has_been_saved_yet() {
        let draft = published_draft();
        let fresh = overview(&draft);
        assert!(fresh.contains("published metadata"), "{fresh}");

        let mut view = view(&draft, "overview", Audience::Everyone);
        view.saved_at = Some("2026-09-03 08:15 UTC");
        let saved = page(&view).into_string();
        assert!(saved.contains("Draft last saved 2026-09-03 08:15 UTC."), "{saved}");
    }

    #[test]
    fn the_region_carries_the_id_the_enhanced_path_patches_and_the_rail_with_it() {
        // Datastar matches a `text/html` response by `id` in `outer` mode, so the
        // id has to be on the region's own root — and the region has to include
        // the rail, or a save that answers the last required field leaves the
        // rail still saying something is missing.
        let out = region(&view(&published_draft(), "overview", Audience::Everyone)).into_string();
        assert!(out.starts_with(&format!(r#"<section id="{REGION_ID}""#)), "{out}");
        assert!(out.contains(r#"aria-label="Form sections""#), "{out}");
        assert!(out.contains("<form"), "{out}");
        // A fragment, not a document: patching `<html>` would replace the page.
        assert!(!out.contains("<!DOCTYPE"), "{out}");
    }

    #[test]
    fn no_field_is_required_or_the_browser_would_refuse_to_save_a_draft() {
        // A draft may be missing anything (REQ-1.9) and saving one must always
        // work (REQ-1.10). `required` on the name field would make an unfinished
        // draft unsaveable on both paths — Datastar runs the same
        // `checkValidity()` the browser does.
        let out = overview(&published_draft());
        // The *attribute*, not the word — "Required" is the obligation pill and
        // "5 of 5 required" is the rail, and both must stay.
        let with_attribute: Vec<&str> = out
            .split('<')
            .filter(|tag| tag.contains(" required") && (tag.starts_with("input") || tag.starts_with("textarea")))
            .collect();
        assert!(with_attribute.is_empty(), "{with_attribute:?}");
        assert!(out.contains("Required"), "the obligation is still stated in words: {out}");
        // Validation is still left on, deliberately: see the module docs.
        assert!(!out.contains("novalidate"), "{out}");
    }

    #[test]
    fn a_required_field_says_so_inside_its_own_label() {
        // Nothing here is `required` or `aria-required` (REQ-1.9/REQ-1.10), so
        // the label is the only channel the obligation has. Rendered as a
        // sibling span it was visible and nothing else: a reader tabbing to the
        // control heard "Name, edit text".
        let out = overview(&published_draft());
        assert!(
            out.contains(r#"<label class="field-label" for="name">Name <span"#),
            "the pill must be inside the label: {out}"
        );
        let label = out.split(r#"for="name">"#).nth(1).expect("the name label");
        let label = label.split("</label>").next().expect("the label's end");
        assert!(label.contains("Required"), "{label}");
        // Same for a group, whose accessible name can only be its legend.
        assert!(out.contains(r#"<legend class="field-label">Description <span"#), "{out}");
    }

    #[test]
    fn the_refused_notice_carries_no_live_role_of_its_own() {
        // `AlertVariant::Danger` renders `role="alert"`, an implicit assertive
        // live region; nested inside this polite one, screen readers disagree
        // about which politeness wins and some interrupt. The region announces;
        // the alert only styles.
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.notice = Some(Notice::Refused("Nothing was saved."));
        let out = region(&view).into_string();
        assert!(out.contains(r#"aria-live="polite""#), "{out}");
        assert!(!out.contains(r#"role="alert""#), "{out}");
        assert!(out.contains("Nothing was saved."), "{out}");
    }

    #[test]
    fn a_rail_link_s_accessible_name_does_not_run_its_two_lines_together() {
        // The title and the progress are adjacent spans with no whitespace
        // between them, so the name computation would give "Overview5 of 5
        // required".
        let out = overview(&published_draft());
        assert!(out.contains(r#"aria-label="Overview, 5 of 5 required""#), "{out}");
        // A section with no requirements needs no label: the visible title is
        // already the whole name.
        assert!(out.contains(r#"<span>Publications</span>"#), "{out}");
        let publications = out.split(r#"/sections/publications""#).nth(1).expect("the link");
        let publications = publications.split("</a>").next().expect("the link's end");
        assert!(!publications.contains("aria-label"), "{publications}");
    }

    #[test]
    fn a_project_name_and_a_stored_value_are_both_escaped() {
        // The name comes from a project file and the values from a draft, so
        // both are data.
        let hostile = "<script>alert(1)</script>";
        let mut draft = ProjectDraft::default();
        draft.set("name", json!(hostile));
        draft.set("description", json!({"en": hostile}));
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.project_name = Some(hostile);
        let out = page(&view).into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }
}

/// Snapshots of the rendered form.
///
/// Separate from the assertions above because they answer a different question.
/// Each test up there names one rule and fails with it; these pin the *whole*
/// markup, so a change nobody was thinking about — a control quietly becoming a
/// value, a posted name changing, an `aria-*` attribute going missing — shows up
/// as a diff rather than as nothing. Neither replaces the other: a snapshot
/// cannot say which rule broke, and a rule cannot notice what it does not
/// mention.
///
/// Deterministic by construction: the data is a committed project, and every
/// timestamp reaches the view as a string the caller formats, so nothing here
/// reads a clock.
#[cfg(test)]
mod snapshots {
    use super::tests::{published_draft, view};
    use super::*;

    #[test]
    fn snapshot_a_depositor_s_editable_overview() {
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.saved_at = Some("2026-09-03 08:15 UTC");
        insta::assert_snapshot!("section_overview_depositor", page(&view).into_string());
    }

    #[test]
    fn snapshot_the_rdu_view_which_adds_the_rdu_only_fields_and_section() {
        let draft = published_draft();
        let view = view(&draft, "overview", Audience::RduOnly);
        insta::assert_snapshot!("section_overview_rdu", page(&view).into_string());
    }

    #[test]
    fn snapshot_a_project_locked_for_review() {
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.locked = Some(Locked::InReview);
        view.saved_at = Some("2026-09-03 08:15 UTC");
        insta::assert_snapshot!("section_overview_in_review", page(&view).into_string());
    }

    #[test]
    fn snapshot_a_refused_save() {
        // The region rather than the page, because this is what the enhanced
        // path actually sends back.
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.notice = Some(Notice::Refused(
            "This project is in review, so the draft cannot be changed. Nothing was saved.",
        ));
        insta::assert_snapshot!("section_overview_refused", region(&view).into_string());
    }
}

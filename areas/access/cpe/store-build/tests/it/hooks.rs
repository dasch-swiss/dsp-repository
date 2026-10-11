//! The build of a project that has a hook: `build` over the fixture project `hooked`,
//! whose `remodel` block declares one, with the hooks of this file.
//!
//! A hook here is a plain `fn`, as a project's is. [`tidy`] makes one of each change the block
//! declares; the others each do one thing a test needs.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{LazyLock, Mutex};

use cpe_core::diagnostic::{DiagnosticKind, ValidationReport};
use cpe_core::remodel::Remodel;
use cpe_core::validate::{load_remodel, ConfigLoadError};
use cpe_ports::contract::violations;
use cpe_ports::{CuratedValue, ProjectSnapshot, Resource, ValueKind};
use cpe_store_build::build;
use cpe_store_build::error::Rule;
use cpe_store_build::hook::{Draft, Hook, HookFn, Mapped, Refusals, Value};
use cpe_store_build::hooks::{resolve, HOOKS};
use cpe_store_build::store::Store;

use crate::support::{
    changed, curated, fixture_member, fixture_resource, link, link_to, lowered, minimal_snapshot, properties,
    recurated, region_on, resources, still_image, text, values, with_annotation, with_link, with_resource, with_text,
    with_value, MINIMAL,
};
use crate::{MAPPING as INCUNABULA_MAPPING, PROJECT as INCUNABULA};

/// The mapping of the fixture project `hooked`.
static HOOKED: LazyLock<Remodel> = LazyLock::new(|| {
    lowered(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/hooked/config.kdl"
    )))
});

const SHORTCODE: &str = "0123";
/// The folder name the tests give their hooks.
const PROJECT: &str = "hooked";

const VOLUME: &str = "http://rdfh.ch/0123/volume-chronik";
const SHEET_1: &str = "http://rdfh.ch/0123/sheet-1";
/// The Volume's second Sheet, whose number is the sentinel: the one [`tidy`] drops.
const SHEET_BLANK: &str = "http://rdfh.ch/0123/sheet-2";
const SHEET_3: &str = "http://rdfh.ch/0123/sheet-3";
const REGION: &str = "http://rdfh.ch/0123/region-wappen";
const LINK: &str = "http://rdfh.ch/0123/link-verweis";
const ADDED: &str = "http://rdfh.ch/0123/added";

const VOLUME_CLASS: &str = "http://www.knora.org/ontology/0123/hooked#Volume";
const SHEET_CLASS: &str = "http://www.knora.org/ontology/0123/hooked#Sheet";

const HAS_HOMEPAGE: &str = "http://www.knora.org/ontology/0123/hooked#hasHomepage";
const HAS_NUMBER: &str = "http://www.knora.org/ontology/0123/hooked#hasNumber";
const HAS_DESCRIPTION: &str = "http://www.knora.org/ontology/0123/hooked#hasDescription";
const HAS_NEXT: &str = "http://www.knora.org/ontology/0123/hooked#hasNext";

const HOMEPAGE: &str = "https://example.org/chronik";
/// An address the archive still serves and [`tidy`] replaces. It holds the sentinel, which a
/// Volume's homepage must not.
const DEAD_HOMEPAGE: &str = "https://alt.example.org/[fehlt]/chronik";
const NEW_HOMEPAGE: &str = "https://neu.example.org/chronik";

/// A Sheet of [`VOLUME`] at `seqnum` with its number, a scan and `more` values.
fn sheet(iri: &str, seqnum: i64, number: &str, more: Vec<(&str, ValueKind)>) -> Resource {
    fixture_member(iri, (SHEET_CLASS, HAS_NUMBER), VOLUME, seqnum, number, more)
}

/// The snapshot of `hooked`: a Volume with three Sheets, the second of them blank, a Region on
/// the first Sheet and a link to the Volume and the Region. The third Sheet names the first as
/// its next, and the Volume's cover is the first.
fn snapshot() -> ProjectSnapshot {
    let volume = Resource {
        values: values(vec![(HAS_HOMEPAGE, ValueKind::Uri(HOMEPAGE.to_string()))]),
        ..fixture_resource(VOLUME, VOLUME_CLASS, "Chronik")
    };
    let snapshot = ProjectSnapshot {
        shortcode: SHORTCODE.to_string(),
        resources: vec![
            sheet(SHEET_3, 3, "2r", vec![(HAS_NEXT, link(SHEET_1))]),
            volume,
            sheet(SHEET_BLANK, 2, "[fehlt]", Vec::new()),
            region_on(REGION, SHEET_1),
            sheet(SHEET_1, 1, "1r", Vec::new()),
            link_to(LINK, &[VOLUME, REGION]),
        ],
        list_nodes: Vec::new(),
        curation: vec![
            curated(VOLUME, "rank", None, "7"),
            curated(REGION, "keep", None, "yes"),
            curated(VOLUME, "cover", None, SHEET_1),
            curated(LINK, "keep", None, "yes"),
        ],
    };
    assert_eq!(violations(SHORTCODE, &snapshot), []);
    snapshot
}

/// [`snapshot`] with [`DEAD_HOMEPAGE`] as the Volume's homepage.
fn moved() -> ProjectSnapshot {
    changed(snapshot(), VOLUME, |volume| {
        volume.values = values(vec![(HAS_HOMEPAGE, ValueKind::Uri(DEAD_HOMEPAGE.to_string()))]);
    })
}

// ── The hooks ─────────────────────────────────────────────────────────────────

/// Makes one of each change the block declares: drops every Sheet whose number is the sentinel
/// alone, gives every Volume its curated rank, and replaces a Volume's homepage that is
/// [`DEAD_HOMEPAGE`] by [`NEW_HOMEPAGE`].
fn tidy(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    let sentinel = draft.sentinel();
    let mut blank = Vec::new();
    for sheet in draft.entries_of("Sheet") {
        if let Mapped::Values([Value::Text(number, _)]) = sheet.values("name") {
            if Some(number.trim()) == sentinel {
                blank.push(sheet.iri());
            }
        }
    }
    for iri in blank {
        draft.drop(iri);
    }

    let mut ranks = Vec::new();
    let mut moved = Vec::new();
    for volume in draft.entries_of("Volume") {
        if let Some([(rank, _)]) = volume.curated("rank").as_deref() {
            ranks.push((volume.iri(), *rank));
        }
        if let [Value::Text(served, _)] = volume.served("homepage").as_slice() {
            if served.trim() == DEAD_HOMEPAGE {
                moved.push(volume.iri());
            }
        }
    }
    for (iri, rank) in ranks {
        draft.provide_text(iri, "rank", rank, None);
    }
    for iri in moved {
        draft.override_values(iri, "homepage", vec![Value::Text(NEW_HOMEPAGE, None)]);
    }
}

fn quiet(_draft: &mut Draft<'_>, _refusals: &mut Refusals) {}

/// The block declares `drops` for Sheets alone.
fn drops_the_volume(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.drop(VOLUME);
}

/// The block declares `provides` for `rank` alone.
fn provides_a_note(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.provide_text(VOLUME, "note", "aus zweiter Hand", None);
}

/// The block declares `overrides` for `homepage` alone.
fn overrides_the_name(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.override_values(VOLUME, "name", vec![Value::Text("Annalen", None)]);
}

fn overrides_with_the_sentinel(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    let holds_it = Value::Text("https://example.org/[fehlt]", None);
    draft.override_values(VOLUME, "homepage", vec![holds_it]);
}

/// A homepage is an address, and a link is none.
fn overrides_with_a_link(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.override_values(VOLUME, "homepage", vec![Value::Link(SHEET_1)]);
}

/// A Volume has at most one homepage.
fn overrides_with_two_addresses(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    let two = vec![Value::Text(HOMEPAGE, None), Value::Text(NEW_HOMEPAGE, None)];
    draft.override_values(VOLUME, "homepage", two);
}

/// Makes the three changes the block does not declare, then records what became of the Volume.
fn oversteps_and_records(draft: &mut Draft<'_>, refusals: &mut Refusals) {
    drops_the_volume(draft, refusals);
    provides_a_note(draft, refusals);
    overrides_the_name(draft, refusals);
    if let Some(volume) = draft.entry(VOLUME) {
        let (kept, name) = (volume.is_kept(), volume.values("name"));
        SEEN.with(|all| all.borrow_mut().push(format!("kept={kept} name: {name:?}")));
    }
}

/// The block's class is `Sheet`.
fn reads_an_undeclared_class(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    let _ = draft.entries_of("Sheets");
}

fn grades_second(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.provide_list_value(VOLUME, "rank", "grade", "second");
}

/// The list `grade` has no node `third`.
fn grades_third(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.provide_list_value(VOLUME, "rank", "grade", "third");
}

/// Reads what the block declares nothing for: a count, which the mapping has not taken yet; a
/// key another node reads; a provided name as served. The first read is made twice.
fn reads_beyond_the_block(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    if let Some(volume) = draft.entry(VOLUME) {
        let _ = (volume.values("extent"), volume.values("extent"));
        let _ = volume.curated("cover");
        let _ = volume.served("rank");
    }
}

/// Refuses the Volume by a rule of its own, as a project's hook does.
fn refuses_the_volume(_draft: &mut Draft<'_>, refusals: &mut Refusals) {
    refusals.refuse(Rule::StaleCatalogueOverride, VOLUME, "homepage");
}

thread_local! {
    /// What [`records`] saw, for the test that called the build on this thread.
    static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Records how the draft shows the Volume and the blank Sheet, then runs [`tidy`] and records
/// the blank Sheet again.
fn records(draft: &mut Draft<'_>, refusals: &mut Refusals) {
    let mut seen = Vec::new();
    if let Some(volume) = draft.entry(VOLUME) {
        let (class, id, kept) = (volume.class(), volume.id(), volume.is_kept());
        seen.push(format!("volume: {class:?} {id:?} kept={kept}"));
        seen.push(format!("homepage: {:?}", volume.values("homepage")));
        seen.push(format!("as served: {:?}", volume.served("homepage")));
        let (rank, served) = (volume.curated("rank"), volume.curated_served("rank"));
        seen.push(format!("rank: {rank:?} as served: {served:?}"));
    }
    let blank = |draft: &Draft<'_>| {
        let sheet = draft.entry(SHEET_BLANK)?;
        let (id, kept, parents) = (sheet.id(), sheet.is_kept(), sheet.part_of());
        Some(format!("blank: {id:?} kept={kept} in {parents:?}"))
    };
    seen.extend(blank(draft));
    tidy(draft, refusals);
    seen.extend(blank(draft));
    SEEN.with(|all| all.borrow_mut().extend(seen));
}

/// What [`records`] saw when `snapshot` was built.
fn recorded(snapshot: &ProjectSnapshot) -> Vec<String> {
    SEEN.with(|all| all.borrow_mut().clear());
    let _ = build(snapshot, &HOOKED, Some(&hook(records)));
    SEEN.with(|all| all.borrow().clone())
}

// ── The cases ─────────────────────────────────────────────────────────────────

fn hook(remodel: HookFn) -> Hook {
    Hook { project: PROJECT, remodel }
}

/// (rule, subject) of every error `build` returns, in the order returned.
fn refused_under(snapshot: &ProjectSnapshot, mapping: &Remodel, remodel: Option<HookFn>) -> Vec<(Rule, String)> {
    let hook = remodel.map(hook);
    match build(snapshot, mapping, hook.as_ref()) {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .as_slice()
            .iter()
            .map(|error| (error.rule, error.subject.clone()))
            .collect(),
    }
}

/// [`refused_under`] the mapping of `hooked` and the hook `remodel`.
fn refused(snapshot: &ProjectSnapshot, remodel: HookFn) -> Vec<(Rule, String)> {
    refused_under(snapshot, &HOOKED, Some(remodel))
}

/// The store of `snapshot` under the mapping of `hooked` and the hook `remodel`.
fn built(snapshot: &ProjectSnapshot, remodel: HookFn) -> Store {
    build(snapshot, &HOOKED, Some(&hook(remodel))).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"))
}

/// One refusal that needs a hook, or the lack of one: a build that is refused by `rule` alone.
struct Case {
    /// The feature of the mapping that refuses, as the hook-less table names its own.
    feature: &'static str,
    rule: Rule,
    mapping: fn() -> &'static Remodel,
    hook: Option<HookFn>,
    snapshot: fn() -> ProjectSnapshot,
    /// What the one error names.
    subject: &'static str,
}

fn hooked() -> &'static Remodel {
    &HOOKED
}

const CASES: &[Case] = &[
    // The block declares a hook, and the build is given none. The subject is the project, as
    // far as the mapping knows it.
    Case {
        feature: "hook",
        rule: Rule::HookNotRegistered,
        mapping: hooked,
        hook: None,
        snapshot,
        subject: SHORTCODE,
    },
    // The block of `minimal` declares none. The hook is not called.
    Case {
        feature: "hook",
        rule: Rule::HookNotDeclared,
        mapping: || &MINIMAL,
        hook: Some(drops_the_volume),
        snapshot: || minimal_snapshot(Vec::new(), Vec::new()),
        subject: PROJECT,
    },
    Case {
        feature: "hook",
        rule: Rule::HookExceedsDeclaration,
        mapping: hooked,
        hook: Some(drops_the_volume),
        snapshot,
        subject: VOLUME,
    },
    Case {
        feature: "hook",
        rule: Rule::HookExceedsDeclaration,
        mapping: hooked,
        hook: Some(provides_a_note),
        snapshot,
        subject: VOLUME,
    },
    Case {
        feature: "hook",
        rule: Rule::HookExceedsDeclaration,
        mapping: hooked,
        hook: Some(overrides_the_name),
        snapshot,
        subject: VOLUME,
    },
    // The block has no class of this name: a rule that walks it would do nothing, in silence.
    Case {
        feature: "hook",
        rule: Rule::HookExceedsDeclaration,
        mapping: hooked,
        hook: Some(reads_an_undeclared_class),
        snapshot,
        subject: "Sheets",
    },
    // The third Sheet names the blank one as its next.
    Case {
        feature: "target, dropped",
        rule: Rule::LinkTargetWithoutRow,
        mapping: hooked,
        hook: Some(tidy),
        snapshot: || {
            changed(snapshot(), SHEET_3, |sheet| {
                sheet.values = values(vec![(HAS_NUMBER, text("2r")), (HAS_NEXT, link(SHEET_BLANK))]);
            })
        },
        subject: SHEET_3,
    },
    Case {
        feature: "cover, dropped",
        rule: Rule::CoverIsCalibrationScan,
        mapping: hooked,
        hook: Some(tidy),
        snapshot: || recurated(snapshot(), VOLUME, "cover", Some(SHEET_BLANK)),
        subject: VOLUME,
    },
    Case {
        feature: "annotations, dropped",
        rule: Rule::RegionOnDroppedPage,
        mapping: hooked,
        hook: Some(tidy),
        snapshot: || with_annotation(snapshot(), region_on(ADDED, SHEET_BLANK), "yes"),
        subject: ADDED,
    },
    Case {
        feature: "annotations, dropped",
        rule: Rule::LinkToDroppedPage,
        mapping: hooked,
        hook: Some(tidy),
        snapshot: || with_annotation(snapshot(), link_to(ADDED, &[SHEET_BLANK]), "yes"),
        subject: ADDED,
    },
];

/// What refuses a build only with a hook, or for the lack of one, by the feature of the
/// mapping. The hook-less table has the refusals a feature gives without a drop.
const FEATURE_RULES: &[(&str, &[Rule])] = &[
    ("target, dropped", &[Rule::LinkTargetWithoutRow]),
    ("cover, dropped", &[Rule::CoverIsCalibrationScan]),
    ("annotations, dropped", &[Rule::RegionOnDroppedPage, Rule::LinkToDroppedPage]),
    (
        "hook",
        &[
            Rule::HookNotRegistered,
            Rule::HookNotDeclared,
            Rule::HookExceedsDeclaration,
        ],
    ),
];

#[test]
fn test_cases_are_the_refusals_of_every_feature_that_needs_a_hook() {
    let listed: BTreeSet<(&str, Rule)> = FEATURE_RULES
        .iter()
        .flat_map(|(feature, rules)| rules.iter().map(move |&rule| (*feature, rule)))
        .collect();
    let cased: BTreeSet<(&str, Rule)> = CASES.iter().map(|case| (case.feature, case.rule)).collect();

    let without_a_case: Vec<_> = listed.difference(&cased).collect();
    let unlisted: Vec<_> = cased.difference(&listed).collect();
    assert_eq!((without_a_case, unlisted), (Vec::new(), Vec::new()));
}

fn refusals(case: &Case) -> Vec<(Rule, String)> {
    refused_under(&(case.snapshot)(), (case.mapping)(), case.hook)
}

/// The rules the cases' builds are refused by, for the test that every rule has a case.
pub fn produced_rules() -> BTreeSet<Rule> {
    let refusals = CASES.iter().flat_map(refusals);
    refusals.map(|(rule, _)| rule).collect()
}

#[test]
fn test_build_each_case_is_refused_by_its_rule_alone_naming_its_subject() {
    for case in CASES {
        assert_eq!(
            refusals(case),
            [(case.rule, case.subject.to_string())],
            "{}: {}",
            case.feature,
            case.rule.name()
        );
    }
}

/// The control of the cases with a drop: the same snapshots build when the blank Sheet stays.
#[test]
fn test_build_what_points_at_a_resource_is_refused_only_once_a_hook_dropped_it() {
    for case in CASES.iter().filter(|case| case.feature.ends_with("dropped")) {
        let kept = refused(&(case.snapshot)(), quiet);

        assert_eq!(kept, [], "{}: {}", case.feature, case.rule.name());
    }
}

// ── The declared changes ──────────────────────────────────────────────────────

/// The blank Sheet gets no row, no Image and no place among the parts, and the Volume's extent
/// counts the two Sheets left. The Volume has its rank, and the address the hook set.
#[test]
fn test_build_declared_drop_provide_and_override_show_in_the_store() {
    let store = built(&moved(), tidy);

    assert_eq!(
        resources(&store),
        [
            ("chronik", "Volume"),
            ("sheet-1", "Image"),
            ("sheet-3", "Image"),
            ("chronik-1r", "Sheet"),
            ("chronik-2r", "Sheet"),
        ]
    );
    let of_volume: Vec<(&str, i64, String)> = properties(&store)
        .into_iter()
        .filter(|(resource, ..)| *resource == "chronik")
        .map(|(_, name, seq, shown)| (name, seq, shown))
        .collect();
    assert_eq!(
        of_volume,
        [
            ("extent", 0, "int 2".to_string()),
            ("homepage", 0, format!("{NEW_HOMEPAGE}@-")),
            ("image", 0, "-> sheet-1".to_string()),
            ("name", 0, "Chronik@-".to_string()),
            ("rank", 0, "7@-".to_string()),
        ]
    );
    let parts: Vec<(&str, i64)> = store
        .compound_parts
        .iter()
        .map(|part| (part.child_id.as_str(), part.seq))
        .collect();
    assert_eq!(parts, [("chronik-1r", 1), ("chronik-2r", 2)]);
    assert_eq!(store.representations.len(), 2);
    assert_eq!(store.annotations.len(), 1);
}

/// The control: without a drop the blank Sheet has its row, and is counted.
#[test]
fn test_build_hook_that_changes_nothing_leaves_the_mapping_s_store() {
    let store = built(&snapshot(), quiet);

    let ids: Vec<&str> = resources(&store).into_iter().map(|(id, _)| id).collect();
    assert_eq!(
        ids,
        [
            "chronik",
            "sheet-1",
            "sheet-2",
            "sheet-3",
            "chronik-1r",
            "chronik-2r",
            "chronik-fehlt"
        ]
    );
    let shown = properties(&store);
    assert!(shown.contains(&("chronik", "extent", 0, "int 3".to_string())));
    assert!(shown.contains(&("chronik", "homepage", 0, format!("{HOMEPAGE}@-"))));
    assert!(!shown.iter().any(|(_, name, ..)| *name == "rank"));
}

/// The address the archive serves holds the sentinel. It is checked only once the hook has
/// run: replaced, it is no defect; left, it is one.
#[test]
fn test_build_overridden_property_is_checked_after_the_hook_on_what_the_hook_left() {
    assert_eq!(refused(&moved(), tidy), []);
    assert_eq!(refused(&moved(), quiet), [(Rule::MissingInValue, VOLUME.to_string())]);
}

#[test]
fn test_build_override_whose_value_holds_the_sentinel_is_refused() {
    assert_eq!(
        refused(&snapshot(), overrides_with_the_sentinel),
        [(Rule::MissingInValue, VOLUME.to_string())]
    );
}

#[test]
fn test_build_override_takes_only_a_value_of_the_property_s_kind() {
    assert_eq!(
        refused(&snapshot(), overrides_with_a_link),
        [(Rule::HookExceedsDeclaration, VOLUME.to_string())]
    );
}

/// The hook's values are held to the node's count, as the archive's are.
#[test]
fn test_build_override_with_more_values_than_the_property_takes_is_refused() {
    assert_eq!(
        refused(&snapshot(), overrides_with_two_addresses),
        [(Rule::ValueCount, VOLUME.to_string())]
    );
}

/// A block whose hook may set the one property that is told from the label, on a class that
/// takes no address.
const RENAMING: &str = r#"
site {
    languages "de" "en"
}

remodel {
    shortcode "0123"
    hook {
        overrides "name" class="Volume"
    }
    class "Volume" iri="http://www.knora.org/ontology/0123/hooked#Volume" no-addresses=#true {
        id from="iri"
        property "name" from="label"
    }
}
"#;

fn names_once(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.override_values(VOLUME, "name", vec![Value::Text("Annalen", None)]);
}

fn names_not(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.override_values(VOLUME, "name", Vec::new());
}

fn names_twice(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    let two = vec![Value::Text("Annalen", None), Value::Text("Chronik", None)];
    draft.override_values(VOLUME, "name", two);
}

/// A resource has one label, so a property told from it has one value before the hook. The
/// hook may set another, and neither none nor several.
#[test]
fn test_build_override_of_a_property_told_from_the_label_has_exactly_one_value() {
    let mapping = lowered(RENAMING);
    let snapshot = ProjectSnapshot {
        shortcode: SHORTCODE.to_string(),
        resources: vec![fixture_resource(VOLUME, VOLUME_CLASS, "Chronik")],
        list_nodes: Vec::new(),
        curation: Vec::new(),
    };
    let refused = |remodel: HookFn| refused_under(&snapshot, &mapping, Some(remodel));

    assert_eq!(refused(names_once), []);
    assert_eq!(refused(names_not), [(Rule::ValueCount, VOLUME.to_string())]);
    assert_eq!(refused(names_twice), [(Rule::ValueCount, VOLUME.to_string())]);
}

fn names_with_an_address(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    let name = vec![Value::Text("Annalen, siehe http://a.example", None)];
    draft.override_values(VOLUME, "name", name);
}

/// The name a hook sets in place of the label is held to what a label is: on a class that
/// takes no address it holds none.
#[test]
fn test_build_override_of_a_property_told_from_the_label_holds_no_address() {
    let snapshot = of(vec![fixture_resource(VOLUME, VOLUME_CLASS, "Chronik")], Vec::new());

    assert_eq!(
        refused_under(&snapshot, &lowered(RENAMING), Some(names_with_an_address)),
        [(Rule::HttpInValue, VOLUME.to_string())]
    );
}

/// A snapshot of `resources` and `curation` alone.
fn of(resources: Vec<Resource>, curation: Vec<CuratedValue>) -> ProjectSnapshot {
    ProjectSnapshot {
        shortcode: SHORTCODE.to_string(),
        resources,
        list_nodes: Vec::new(),
        curation,
    }
}

/// A block of two classes with the same two text properties, of which the hook may set one: a
/// Volume's `first`.
const PARTLY: &str = r#"
site {
    languages "de" "en"
}

remodel {
    shortcode "0123"
    hook {
        overrides "first" class="Volume"
    }
    class "Volume" iri="http://www.knora.org/ontology/0123/hooked#Volume" {
        id from="iri"
        property "first" iri="http://www.knora.org/ontology/0123/hooked#hasNumber" type="text"
        property "second" iri="http://www.knora.org/ontology/0123/hooked#hasDescription" type="text"
    }
    class "Sheet" iri="http://www.knora.org/ontology/0123/hooked#Sheet" {
        id from="iri"
        property "first" iri="http://www.knora.org/ontology/0123/hooked#hasNumber" type="text"
        property "second" iri="http://www.knora.org/ontology/0123/hooked#hasDescription" type="text"
    }
}
"#;

/// Records what the draft shows under `first` and `second` of every Volume and Sheet.
fn shows_first_and_second(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    let mut seen = Vec::new();
    for class in ["Volume", "Sheet"] {
        for entry in draft.entries_of(class) {
            let (first, second) = (entry.values("first"), entry.values("second"));
            seen.push(format!("{class} first: {first:?} second: {second:?}"));
        }
    }
    SEEN.with(|all| all.borrow_mut().extend(seen));
}

/// What waits for the hook is the one property of the one class the block lists under
/// `overrides`. Every text here is empty, and an empty text gives no row: the hook finds it
/// still under the Volume's `first`, and left out under the Volume's other property and under
/// the Sheet's property of the same name and position.
#[test]
fn test_build_only_the_overridden_property_of_the_overridden_class_waits_for_the_hook() {
    let empty = |iri: &str, class: &str| Resource {
        values: values(vec![(HAS_NUMBER, text("")), (HAS_DESCRIPTION, text(""))]),
        ..fixture_resource(iri, class, "")
    };
    let snapshot = of(vec![empty(VOLUME, VOLUME_CLASS), empty(SHEET_1, SHEET_CLASS)], Vec::new());
    SEEN.with(|all| all.borrow_mut().clear());

    let refused = refused_under(&snapshot, &lowered(PARTLY), Some(shows_first_and_second));

    assert_eq!(refused, []);
    assert_eq!(
        SEEN.with(|all| all.borrow().clone()),
        [
            r#"Volume first: Values([Text("", None)]) second: Values([])"#,
            "Sheet first: Values([]) second: Values([])",
        ]
    );
}

/// A block whose two classes make their ids from `name`. The hook may set a Volume's `title`,
/// which makes no id, and a Sheet's `name`, which makes the Sheet's.
const NUMBERED: &str = r#"
site {
    languages "de" "en"
}

remodel {
    shortcode "0123"
    sentinel "[fehlt]"
    hook {
        overrides "title" class="Volume"
        overrides "name" class="Sheet"
    }
    class "Volume" iri="http://www.knora.org/ontology/0123/hooked#Volume" {
        id from="property" property="name"
        property "name" iri="http://www.knora.org/ontology/0123/hooked#hasNumber" type="text" min=1 max=1
        property "title" iri="http://www.knora.org/ontology/0123/hooked#hasDescription" type="text" max=1
    }
    class "Sheet" iri="http://www.knora.org/ontology/0123/hooked#Sheet" {
        id from="property" property="name"
        property "name" iri="http://www.knora.org/ontology/0123/hooked#hasNumber" type="text" min=1 max=1
    }
}
"#;

/// Sets the Volume's title and the Sheet's name, and records whether the Sheet is kept.
fn retitles_and_renames(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    let kept = draft.entry(SHEET_1).is_some_and(|sheet| sheet.is_kept());
    SEEN.with(|all| all.borrow_mut().push(format!("sheet kept={kept}")));
    draft.override_values(VOLUME, "title", vec![Value::Text("Chronik", None)]);
    draft.override_values(SHEET_1, "name", vec![Value::Text("zwei", None)]);
}

/// An overridden property is checked after the hook unless it makes its class's id. The
/// Volume's title is the sentinel as served and no defect once the hook set another. The
/// Sheet's name is the sentinel too, and it makes the id: it is refused before the hook, which
/// finds the Sheet refused and cannot mend it.
#[test]
fn test_build_overridden_property_is_checked_before_the_hook_only_where_it_makes_the_id() {
    let numbered = |iri: &str, class: &str, more: Vec<(&str, ValueKind)>| {
        let mut served = vec![(HAS_NUMBER, text(if class == SHEET_CLASS { "[fehlt]" } else { "eins" }))];
        served.extend(more);
        Resource { values: values(served), ..fixture_resource(iri, class, "") }
    };
    let snapshot = of(
        vec![
            numbered(VOLUME, VOLUME_CLASS, vec![(HAS_DESCRIPTION, text("[fehlt]"))]),
            numbered(SHEET_1, SHEET_CLASS, Vec::new()),
        ],
        Vec::new(),
    );
    SEEN.with(|all| all.borrow_mut().clear());

    let refused = refused_under(&snapshot, &lowered(NUMBERED), Some(retitles_and_renames));

    assert_eq!(refused, [(Rule::MissingInValue, SHEET_1.to_string())]);
    assert_eq!(SEEN.with(|all| all.borrow().clone()), ["sheet kept=false"]);
}

/// Provides, overrides and reads on a Sheet what the block declares for a Volume.
fn treats_a_sheet_as_a_volume(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.provide_text(SHEET_1, "rank", "7", None);
    draft.override_values(SHEET_1, "homepage", vec![Value::Text(HOMEPAGE, None)]);
    if let Some(sheet) = draft.entry(SHEET_1) {
        let _ = sheet.curated("rank");
    }
}

/// What a `hook` node declares, it declares for one class: the same name on a resource of
/// another class is beyond the declaration, for `provides`, `overrides` and `key` each.
#[test]
fn test_build_declaration_for_one_class_does_not_reach_a_resource_of_another() {
    let rule = Rule::HookExceedsDeclaration;

    assert_eq!(
        refused(&snapshot(), treats_a_sheet_as_a_volume),
        [
            (rule, SHEET_1.to_string()),
            (rule, SHEET_1.to_string()),
            (rule, SHEET_1.to_string())
        ]
    );
}

/// A block whose hook may drop a Sheet and set a Sheet's one link.
const LINKED: &str = r#"
site {
    languages "de" "en"
}

remodel {
    shortcode "0123"
    hook {
        overrides "next" class="Sheet"
        drops class="Sheet"
    }
    class "Sheet" iri="http://www.knora.org/ontology/0123/hooked#Sheet" {
        id from="iri"
        property "next" iri="http://www.knora.org/ontology/0123/hooked#hasNext" type="link" max=1 target="Sheet"
    }
}
"#;

/// Drops the blank Sheet, and gives the first Sheet two links where it takes one: to the
/// dropped Sheet and to the third.
fn links_twice_and_to_a_dropped_sheet(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.drop(SHEET_BLANK);
    let two = vec![Value::Link(SHEET_BLANK), Value::Link(SHEET_3)];
    draft.override_values(SHEET_1, "next", two);
}

/// A link a hook set to a resource it dropped is reported also when the property is refused
/// for its number: the links are followed up as the hook left them, before a miscounted
/// property loses its values.
#[test]
fn test_build_miscounted_override_is_still_refused_for_its_link_to_a_dropped_resource() {
    let sheets = [SHEET_1, SHEET_BLANK, SHEET_3];
    let snapshot = of(sheets.map(|iri| fixture_resource(iri, SHEET_CLASS, "")).to_vec(), Vec::new());
    let hook = Some(links_twice_and_to_a_dropped_sheet as HookFn);

    let mut refused = refused_under(&snapshot, &lowered(LINKED), hook);
    refused.sort();

    let mut expected = vec![
        (Rule::LinkTargetWithoutRow, SHEET_1.to_string()),
        (Rule::ValueCount, SHEET_1.to_string()),
    ];
    expected.sort();
    assert_eq!(refused, expected);
}

/// A block whose hook may set a property that takes two values or more and for which a curated
/// text can stand in.
const STANDING_IN: &str = r#"
site {
    languages "de" "en"
}

remodel {
    shortcode "0123"
    hook {
        overrides "name" class="Volume"
    }
    class "Volume" iri="http://www.knora.org/ontology/0123/hooked#Volume" {
        id from="iri"
        property "name" iri="http://www.knora.org/ontology/0123/hooked#hasNumber" type="text" curated-key="name" min=2
    }
}
"#;

/// A curated text that stands in for a property's source is the property's one value, whatever
/// number the source takes: the number is asked of the archive's values, which are not read
/// then, and of a hook's. On an overridden property the hook leaves alone it is written.
#[test]
fn test_build_curated_stand_in_of_an_overridden_property_is_not_held_to_the_source_s_count() {
    let snapshot = of(
        vec![fixture_resource(VOLUME, VOLUME_CLASS, "")],
        vec![curated(VOLUME, "name", None, "Annalen")],
    );

    let store = build(&snapshot, &lowered(STANDING_IN), Some(&hook(quiet)))
        .unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"));

    assert_eq!(properties(&store), [("volume-chronik", "name", 0, "Annalen@-".to_string())]);
}

/// A block whose two classes are named by a curated title where the editors wrote one, so the
/// mapping reads no label of a resource that has one. Only a Volume takes no address.
const TITLED: &str = r#"
site {
    languages "de" "en"
}

remodel {
    shortcode "0123"
    sentinel "[fehlt]"
    hook {
        drops class="Sheet"
    }
    class "Volume" iri="http://www.knora.org/ontology/0123/hooked#Volume" no-addresses=#true {
        id from="iri"
        property "name" from="label" curated-key="title"
    }
    class "Sheet" iri="http://www.knora.org/ontology/0123/hooked#Sheet" {
        id from="iri"
        property "name" from="label" curated-key="title"
    }
}
"#;

/// Records, for every Volume and Sheet, whether its label may be written as a text.
fn tests_the_labels(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    let all = ["Volume", "Sheet"].map(|class| draft.entries_of(class));
    let seen = all.iter().flatten().map(|entry| {
        let (label, is_text) = (entry.label(), entry.label_is_text());
        format!("{label:?} is_text={is_text}")
    });
    SEEN.with(|all| all.borrow_mut().extend(seen));
}

/// The view tests a label by the mapping's rule for one, also where the mapping read none: each
/// resource here has a curated title, so no label is read and none is refused.
#[test]
fn test_view_says_whether_a_label_the_mapping_did_not_read_may_be_a_text() {
    let mapping = lowered(TITLED);
    let labelled = [
        ("http://rdfh.ch/0123/v1", VOLUME_CLASS, ""),
        ("http://rdfh.ch/0123/v2", VOLUME_CLASS, "siehe http://a.example"),
        ("http://rdfh.ch/0123/v3", VOLUME_CLASS, "Band [fehlt]"),
        ("http://rdfh.ch/0123/v4", VOLUME_CLASS, "Chronik"),
        ("http://rdfh.ch/0123/w1", SHEET_CLASS, "siehe http://a.example"),
    ];
    let snapshot = ProjectSnapshot {
        shortcode: SHORTCODE.to_string(),
        resources: labelled.map(|(iri, class, label)| fixture_resource(iri, class, label)).to_vec(),
        list_nodes: Vec::new(),
        curation: labelled.map(|(iri, _, _)| curated(iri, "title", None, "Ein Titel")).to_vec(),
    };
    SEEN.with(|all| all.borrow_mut().clear());

    let refused = refused_under(&snapshot, &mapping, Some(tests_the_labels));

    assert_eq!(refused, []);
    assert_eq!(
        SEEN.with(|all| all.borrow().clone()),
        [
            r#""" is_text=false"#,
            r#""siehe http://a.example" is_text=false"#,
            r#""Band [fehlt]" is_text=false"#,
            r#""Chronik" is_text=true"#,
            r#""siehe http://a.example" is_text=true"#,
        ]
    );
}

fn provides_a_nul(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    draft.provide_text(VOLUME, "rank", "erste\0", None);
}

fn overrides_with_a_nul(draft: &mut Draft<'_>, _refusals: &mut Refusals) {
    let address = Value::Text("https://example.org/\0", None);
    draft.override_values(VOLUME, "homepage", vec![address]);
}

/// A text a hook gives is held to the rule of every text of the store, and is not written.
#[test]
fn test_build_nul_in_a_text_a_hook_gives_is_refused() {
    let nul = [(Rule::InteriorNul, VOLUME.to_string())];

    assert_eq!(refused(&snapshot(), provides_a_nul), nul);
    assert_eq!(refused(&snapshot(), overrides_with_a_nul), nul);
}

/// Runs [`tidy`], which drops the blank Sheet, and records how many Sheets each reader gives.
fn counts_the_sheets(draft: &mut Draft<'_>, refusals: &mut Refusals) {
    tidy(draft, refusals);
    let (kept, all) = (draft.entries_of("Sheet").len(), draft.all_entries_of("Sheet").len());
    SEEN.with(|seen| seen.borrow_mut().push(format!("kept={kept} all={all}")));
}

/// A rule that walks a class sees the resources that get a row; the dropped one and one the
/// mapping refused, here a fourth Sheet without a number, are shown only to the reader that is
/// named for them.
#[test]
fn test_entries_of_gives_the_resources_that_get_a_row_and_all_entries_of_the_rest_too() {
    let unnumbered = sheet("http://rdfh.ch/0123/sheet-4", 4, "", Vec::new());
    let snapshot = with_resource(snapshot(), unnumbered);
    SEEN.with(|all| all.borrow_mut().clear());

    let refused = refused(&snapshot, counts_the_sheets);

    assert_eq!(refused, [(Rule::EmptyValue, "http://rdfh.ch/0123/sheet-4".to_string())]);
    assert_eq!(SEEN.with(|all| all.borrow().clone()), ["kept=2 all=4"]);
}

/// A change beyond the block is refused and is not made: the Volume is still kept, and still
/// has the name the mapping gave it.
#[test]
fn test_build_change_beyond_the_declaration_is_not_made() {
    SEEN.with(|all| all.borrow_mut().clear());

    let refused = refused(&snapshot(), oversteps_and_records);

    assert_eq!(refused.len(), 3, "{refused:?}");
    assert_eq!(
        SEEN.with(|all| all.borrow().clone()),
        [r#"kept=true name: Values([Text("Chronik", None)])"#]
    );
}

#[test]
fn test_build_provided_list_value_is_a_node_of_a_declared_list() {
    let store = built(&snapshot(), grades_second);

    let shown = properties(&store);
    assert!(
        shown.contains(&("chronik", "rank", 0, "node grade-second".to_string())),
        "{shown:?}"
    );
    assert_eq!(
        refused(&snapshot(), grades_third),
        [(Rule::UnknownListNode, VOLUME.to_string())]
    );
}

/// Each read beyond the block is refused, and one that is made twice is refused once.
#[test]
fn test_build_read_beyond_the_declaration_is_refused_once_per_read() {
    let rule = Rule::HookExceedsDeclaration;

    assert_eq!(
        refused(&snapshot(), reads_beyond_the_block),
        [
            (rule, VOLUME.to_string()),
            (rule, VOLUME.to_string()),
            (rule, VOLUME.to_string())
        ]
    );
}

#[test]
fn test_build_hook_s_own_refusal_is_reported_with_the_builder_s() {
    let with_a_defect = with_value(snapshot(), SHEET_1, HAS_NUMBER, ValueKind::Integer(4));

    assert_eq!(
        refused(&with_a_defect, refuses_the_volume),
        [
            (Rule::UnexpectedValueKind, SHEET_1.to_string()),
            (Rule::StaleCatalogueOverride, VOLUME.to_string())
        ]
    );
}

// ── The dropped resource ──────────────────────────────────────────────────────

/// What the mapping refuses a value for is withdrawn with the resource: a description that
/// holds the sentinel, and a link to a resource of another class. On a Sheet that stays, each
/// is a defect.
#[test]
fn test_build_drop_withdraws_the_refusals_of_the_resource_s_values() {
    let described = |sheet| with_text(snapshot(), sheet, HAS_DESCRIPTION, "[fehlt] Farbkarte");
    let linked = |sheet| with_link(snapshot(), sheet, HAS_NEXT, VOLUME);

    assert_eq!(refused(&described(SHEET_BLANK), tidy), []);
    assert_eq!(refused(&linked(SHEET_BLANK), tidy), []);
    assert_eq!(
        refused(&described(SHEET_1), tidy),
        [(Rule::MissingInValue, SHEET_1.to_string())]
    );
    assert_eq!(refused(&linked(SHEET_1), tidy), [(Rule::StripLinkTarget, SHEET_1.to_string())]);
}

/// A value of another kind is a defect of the archive's data, whatever becomes of the resource.
#[test]
fn test_build_dropped_resource_is_still_refused_for_a_value_of_another_kind() {
    let miskinded = with_value(snapshot(), SHEET_BLANK, HAS_DESCRIPTION, ValueKind::Integer(4));

    assert_eq!(
        refused(&miskinded, tidy),
        [(Rule::UnexpectedValueKind, SHEET_BLANK.to_string())]
    );
}

/// The blank Sheet has the first Sheet's file. The file is two resources', whatever the hook
/// does with one of them later: the check on files runs before the hook, over every resource
/// that has an id.
#[test]
fn test_build_dropped_resource_stays_in_the_check_on_shared_files() {
    let shared = changed(snapshot(), SHEET_BLANK, |sheet| {
        sheet.file = Some(still_image("sheet-1.jp2"));
    });

    assert_eq!(refused(&shared, tidy), [(Rule::SharedFile, "sheet-1".to_string())]);
}

// ── The table and the project folders ─────────────────────────────────────────

/// Those of `projects` that have no folder under `root` whose `remodel` block declares a hook:
/// no folder, no block, or a block without the node.
fn without_a_declared_hook(root: &str, projects: &[&str]) -> Vec<String> {
    let declares = |project: &&&str| {
        let config = std::fs::read_to_string(format!("{root}/{project}/config.kdl"));
        let mapping = config.ok().and_then(|config| load_remodel(&config).ok());
        mapping.is_some_and(|mapping| mapping.hook.is_some())
    };
    let undeclared = projects.iter().filter(|project| !declares(project));
    undeclared.map(|project| project.to_string()).collect()
}

/// A row of the table is a hook the builder would call: its project's block must say so.
#[test]
fn test_every_row_of_the_hook_table_names_a_project_whose_block_declares_a_hook() {
    let projects = concat!(env!("CARGO_MANIFEST_DIR"), "/../projects");
    let rows: Vec<&str> = HOOKS.iter().map(|hook| hook.project).collect();

    assert_eq!(without_a_declared_hook(projects, &rows), Vec::<String>::new());
    for project in rows {
        assert!(resolve(project).is_some(), "{project}");
    }
    assert!(resolve("no-such-project").is_none());
}

/// The control, over the fixture projects: one that declares a hook, one with a block and no
/// hook, and a folder that is not there.
#[test]
fn test_without_a_declared_hook_finds_a_block_without_the_node_and_a_missing_folder() {
    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

    assert_eq!(
        without_a_declared_hook(fixtures, &["hooked", "plain", "nowhere"]),
        ["plain", "nowhere"]
    );
}

/// The folders under `root` whose `remodel` block declares a hook, by name. A folder without a
/// config declares none, and neither does a config without a block. A block that does not
/// lower fails the test: what it declares cannot be read.
fn declaring_a_hook(root: &str) -> Vec<String> {
    let folders = std::fs::read_dir(root).unwrap_or_else(|error| panic!("{root}: {error}"));
    let mut found = Vec::new();
    for folder in folders {
        let path = folder.unwrap_or_else(|error| panic!("{root}: {error}")).path();
        let Ok(config) = std::fs::read_to_string(path.join("config.kdl")) else {
            continue;
        };
        let name = path.file_name().expect("a folder has a name");
        let name = name.to_string_lossy().into_owned();
        let without_a_block = |report: &ValidationReport| {
            let mut kinds = report.iter().map(|diagnostic| &diagnostic.kind);
            kinds.all(|kind| *kind == DiagnosticKind::MissingRemodelBlock)
        };
        match load_remodel(&config) {
            Ok(mapping) if mapping.hook.is_some() => found.push(name),
            Ok(_) => {}
            Err(ConfigLoadError::Validation(report)) if without_a_block(&report) => {}
            Err(error) => panic!("{name}: {error:?}"),
        }
    }
    found.sort();
    found
}

/// A block that declares a hook is refused by a build that is given none: its project must
/// have a row.
#[test]
fn test_every_project_whose_block_declares_a_hook_has_a_row_of_the_hook_table() {
    let projects = concat!(env!("CARGO_MANIFEST_DIR"), "/../projects");

    let declaring = declaring_a_hook(projects);

    let without_a_row = declaring.iter().filter(|name| resolve(name).is_none());
    assert_eq!(without_a_row.collect::<Vec<_>>(), Vec::<&String>::new());
    assert!(!declaring.is_empty(), "no project folder was read");
}

/// The control, over the fixture projects: the one whose block declares a hook is found, the
/// two whose blocks declare none are not, and the found one has no row.
#[test]
fn test_declaring_a_hook_finds_the_one_fixture_whose_block_has_the_node() {
    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

    assert_eq!(declaring_a_hook(fixtures), ["hooked"]);
    assert!(resolve("hooked").is_none());
}

// ── What a hook reads ─────────────────────────────────────────────────────────

#[test]
fn test_view_shows_the_mapping_s_result_the_values_as_served_and_the_curated_texts() {
    let padded = recurated(snapshot(), VOLUME, "rank", Some(" 7 "));

    assert_eq!(
        recorded(&padded),
        [
            r#"volume: Some("Volume") Some("chronik") kept=true"#,
            r#"homepage: Values([Text("https://example.org/chronik", None)])"#,
            r#"as served: [Text("https://example.org/chronik", None)]"#,
            r#"rank: Some([("7", None)]) as served: [(" 7 ", None)]"#,
            r#"blank: Some("chronik-fehlt") kept=true in ["http://rdfh.ch/0123/volume-chronik"]"#,
            r#"blank: Some("chronik-fehlt") kept=false in ["http://rdfh.ch/0123/volume-chronik"]"#,
        ]
    );
}

/// A refused property says why, and its values as served are still to be read: every one for
/// a count, and those of the property's kind for a kind.
#[test]
fn test_view_tells_a_refused_property_from_one_without_a_value() {
    let second = ValueKind::Uri("https://example.org/zweite".to_string());
    let two = with_value(snapshot(), VOLUME, HAS_HOMEPAGE, second);
    let as_text = with_text(snapshot(), VOLUME, HAS_HOMEPAGE, "keine Adresse");
    let none = changed(snapshot(), VOLUME, |volume| volume.values.clear());

    let homepage = |snapshot: &ProjectSnapshot| recorded(snapshot)[1..3].to_vec();
    assert_eq!(
        homepage(&two),
        [
            "homepage: Refused(Count)",
            r#"as served: [Text("https://example.org/chronik", None), Text("https://example.org/zweite", None)]"#,
        ]
    );
    assert_eq!(
        homepage(&as_text),
        [
            "homepage: Refused(Kind)",
            r#"as served: [Text("https://example.org/chronik", None)]"#,
        ]
    );
    assert_eq!(homepage(&none), ["homepage: Values([])", "as served: []"]);
}

// ── A project's hook, held to its declaration ─────────────────────────────────

// The one project that has a hook is the one the other test modules build: `INCUNABULA` is
// its folder and `INCUNABULA_MAPPING` its mapping.

/// What a draft shows of one resource: its class, whether it is kept, and what each property
/// the block lets a hook read holds, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shown {
    class: String,
    kept: bool,
    values: BTreeMap<String, String>,
}

/// [`Shown`] of every resource of a declared class, by IRI.
type View = BTreeMap<String, Shown>;

/// (class, name) pairs.
type Names = BTreeSet<(String, String)>;

/// The names a hook may read on each class of `mapping`: those of its `property` and `curated`
/// nodes, and what the `hook` node provides.
fn readable(mapping: &Remodel) -> Names {
    let mut names = provided(mapping);
    for class in &mapping.classes {
        let properties = class.properties.iter().flat_map(|node| &node.names);
        let curated = class.curated.iter().map(|node| &node.name);
        for name in properties.chain(curated) {
            names.insert((class.name.as_str().to_string(), name.as_str().to_string()));
        }
    }
    names
}

/// What the `hook` node of `mapping` declares under `provides`.
fn provided(mapping: &Remodel) -> Names {
    let provides = mapping.hook.iter().flat_map(|hook| &hook.provides);
    let named = provides.map(|provided| {
        let class = mapping.class(provided.class).name.as_str();
        (class.to_string(), provided.name.as_str().to_string())
    });
    named.collect()
}

/// What the `hook` node of `mapping` declares under `overrides`, by each name of the property.
fn overridden(mapping: &Remodel) -> Names {
    let overrides = mapping.hook.iter().flat_map(|hook| &hook.overrides);
    let named = overrides.flat_map(|overridden| {
        let class = mapping.class(overridden.class);
        let names = class.property(overridden.property).names.iter();
        names.map(|name| (class.name.as_str().to_string(), name.as_str().to_string()))
    });
    named.collect()
}

/// The classes the `hook` node of `mapping` declares under `drops`.
fn droppable(mapping: &Remodel) -> BTreeSet<String> {
    let drops = mapping.hook.iter().flat_map(|hook| &hook.drops);
    let named = drops.map(|class| mapping.class(*class).name.as_str().to_string());
    named.collect()
}

/// The draft's public view of every resource of a class [`INCUNABULA_MAPPING`] declares.
fn view_of(draft: &Draft<'_>) -> View {
    let mut view = View::new();
    for (class, name) in readable(&INCUNABULA_MAPPING) {
        for entry in draft.all_entries_of(&class) {
            let shown = view.entry(entry.iri().to_string()).or_insert(Shown {
                class: class.clone(),
                kept: entry.is_kept(),
                values: BTreeMap::new(),
            });
            let values = format!("{:?}", entry.values(&name));
            shown.values.insert(name.clone(), values);
        }
    }
    view
}

/// The two views [`recording`] took: before the project's hook ran, and after.
static RECORDED: Mutex<Option<(View, View)>> = Mutex::new(None);

/// The project's hook, as the table of hooks has it, between two copies of the draft's view.
fn recording(draft: &mut Draft<'_>, refusals: &mut Refusals) {
    let row = resolve(INCUNABULA).expect("the project has a row in the table of hooks");
    let before = view_of(draft);
    (row.remodel)(draft, refusals);
    let after = view_of(draft);
    *RECORDED.lock().expect("no hook panicked with the views") = Some((before, after));
}

/// How one view differs from an earlier one of the same draft.
#[derive(Debug, Default, PartialEq, Eq)]
struct Changes {
    /// The classes of the resources that were kept and are not.
    unkept: BTreeSet<String>,
    /// The names that held nothing on every resource whose values under them changed.
    added: Names,
    /// The other names whose values changed on a resource.
    changed: Names,
    /// What a hook cannot do: a resource that came or went, changed its class, or is kept
    /// again.
    impossible: Vec<String>,
}

fn changes(before: &View, after: &View) -> Changes {
    const NOTHING: &str = "Values([])";
    let mut found = Changes::default();
    if before.keys().ne(after.keys()) {
        found.impossible.push("the resources differ".to_string());
    }
    // The names whose values differ on a resource, and those of them that held something there.
    let (mut differing, mut held_something) = (Names::new(), Names::new());
    for (iri, was) in before {
        let Some(is) = after.get(iri) else {
            continue;
        };
        if was.class != is.class || (!was.kept && is.kept) {
            found.impossible.push(iri.clone());
        }
        if was.kept && !is.kept {
            found.unkept.insert(was.class.clone());
        }
        for (name, held) in &was.values {
            if is.values.get(name) == Some(held) {
                continue;
            }
            let pair = (was.class.clone(), name.clone());
            if held != NOTHING {
                held_something.insert(pair.clone());
            }
            differing.insert(pair);
        }
    }
    found.changed = differing.intersection(&held_something).cloned().collect();
    found.added = differing.difference(&held_something).cloned().collect();
    found
}

/// After the hook, every property holds the mapping's values unless the block lists it under
/// `overrides`, the kept resources differ only in the classes under `drops`, and the only
/// properties that came are those under `provides`. On the vendored data the hook makes every
/// change its block declares.
#[test]
fn test_incunabula_s_hook_changes_only_what_its_block_declares() {
    let hook = Hook { project: INCUNABULA, remodel: recording };

    let built = build(&crate::SNAPSHOT, &INCUNABULA_MAPPING, Some(&hook));

    assert!(built.is_ok(), "the vendored snapshot builds");
    let recorded = RECORDED.lock().expect("the hook did not panic").take();
    let (before, after) = recorded.expect("the hook ran");
    let pair = |class: &str, name: &str| (class.to_string(), name.to_string());
    let declared = Changes {
        unkept: droppable(&INCUNABULA_MAPPING),
        added: provided(&INCUNABULA_MAPPING),
        changed: overridden(&INCUNABULA_MAPPING),
        impossible: Vec::new(),
    };
    assert_eq!(changes(&before, &after), declared);
    assert_eq!(
        declared,
        Changes {
            unkept: BTreeSet::from(["Page".to_string()]),
            added: BTreeSet::from([pair("Book", "sort-key"), pair("BorderStrip", "side")]),
            changed: BTreeSet::from([pair("Book", "catalogue-uri")]),
            impossible: Vec::new(),
        }
    );
    let pages = before.values().filter(|shown| shown.class == "Page").count();
    assert!(pages > 4000, "{pages} Pages were seen");
}

/// A curated key the block's `hook` node declares is one the project's hook reads, and the
/// hook reads no key the node does not declare.
///
/// The builder takes every declared key as read: a key the node declares and the hook never
/// asks for would be accepted on any resource of its class and reach no row. So each declared
/// key is taken out of the node in turn, and the hook's read of it must then be refused as one
/// beyond the declaration. The other direction is the seam's own rule, shown here on the
/// vendored data: with the node as it is, no read is refused.
#[test]
fn test_incunabula_s_hook_reads_exactly_the_curated_keys_its_block_declares() {
    let hook = resolve(INCUNABULA);
    let declared = INCUNABULA_MAPPING.hook.as_ref().expect("the block has a hook node");
    assert!(!declared.keys.is_empty(), "the node declares no key");

    let as_declared = build(&crate::SNAPSHOT, &INCUNABULA_MAPPING, hook);
    assert!(as_declared.is_ok(), "the vendored snapshot builds");

    for (at, key) in declared.keys.iter().enumerate() {
        let mut undeclared = INCUNABULA_MAPPING.clone();
        let node = undeclared.hook.as_mut().expect("the block has a hook node");
        node.keys.remove(at);
        let read = format!("read the curated key {}", key.key.as_str());

        let built = build(&crate::SNAPSHOT, &undeclared, hook);

        let errors = built.expect_err("a read beyond the declaration is refused");
        let beyond = errors
            .as_slice()
            .iter()
            .any(|error| error.rule == Rule::HookExceedsDeclaration && error.detail.as_deref() == Some(&read));
        assert!(beyond, "the hook does not read the key {:?}", key.key);
    }
}

/// The control: each kind of change is found between two views made by hand, and a value that
/// replaces another is told from one that came where none was.
#[test]
fn test_changes_finds_a_drop_a_changed_value_an_added_one_and_what_no_hook_can_do() {
    let shown = |class: &str, kept: bool, values: &[(&str, &str)]| Shown {
        class: class.to_string(),
        kept,
        values: values.iter().map(|(name, held)| (name.to_string(), held.to_string())).collect(),
    };
    let view = |resources: &[(&str, Shown)]| -> View {
        let named = resources.iter().cloned();
        named.map(|(iri, shown)| (iri.to_string(), shown)).collect()
    };
    let before = view(&[
        ("a", shown("Leaf", true, &[("name", r#"Values([Text("1r", None)])"#)])),
        (
            "b",
            shown(
                "Work",
                true,
                &[("name", r#"Values([Text("B", None)])"#), ("rank", "Values([])")],
            ),
        ),
        ("c", shown("Work", false, &[("name", "Values([])")])),
    ]);
    let after = view(&[
        ("a", shown("Leaf", false, &[("name", r#"Values([Text("1r", None)])"#)])),
        (
            "b",
            shown(
                "Work",
                true,
                &[
                    ("name", r#"Values([Text("C", None)])"#),
                    ("rank", r#"Values([Text("7", None)])"#),
                ],
            ),
        ),
        ("c", shown("Work", true, &[("name", "Values([])")])),
    ]);
    let pair = |class: &str, name: &str| (class.to_string(), name.to_string());

    assert_eq!(changes(&before, &before), Changes::default());
    assert_eq!(
        changes(&before, &after),
        Changes {
            unkept: BTreeSet::from(["Leaf".to_string()]),
            added: BTreeSet::from([pair("Work", "rank")]),
            changed: BTreeSet::from([pair("Work", "name")]),
            impossible: vec!["c".to_string()],
        }
    );
    let fewer = view(&[]);
    assert_eq!(changes(&before, &fewer).impossible, ["the resources differ"]);
}

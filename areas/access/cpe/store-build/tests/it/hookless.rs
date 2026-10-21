//! The build of a project that has a mapping and no hook: `build` over the fixture
//! project `plain`, whose `remodel` block uses every node and attribute a project without a hook
//! can write, and over `minimal`, the least block that lowers.
//!
//! The snapshot of `plain` is hand-built and lists its resources, a resource's values and the
//! curated values in no order, so that a row order that only follows the snapshot's shows. Keep
//! it unsorted, within the port's contract, and accepted by `build`.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use cpe_core::remodel::{IdPolicy, IdSource, Remodel};
use cpe_ports::contract::violations;
use cpe_ports::{
    Annotation, ArchiveProjection, DatePrecision, DateValue, FakeArchiveProjection, File, Motivation, ProjectSnapshot,
    Resource, ResourceIri, ValueKind, DATA_ARK_PREFIX,
};
use cpe_store_build::error::Rule;
use cpe_store_build::store::{Store, Table};
use cpe_store_build::{sql, SCHEMA_SQL};
use rusqlite::Connection;

use crate::support::{
    build_with, changed, curated, fixture_member, fixture_resource, link, link_to, lowered, minimal_snapshot,
    properties, recurated, region_on, resources, shown, still_image, text, values, with_annotation, with_curation,
    with_curation_in, with_label, with_link, with_resource, with_text, with_value, year_1489, Fixture, HAS_COLOR,
    HAS_GEOMETRY, HAS_LINK_TO, IS_REGION_OF, KB_HAS_COMMENT, LINK_OBJ_CLASS, MINIMAL, RECTANGLE, REGION_CLASS, THING,
};

/// The mapping of the fixture project `plain`.
static PLAIN: LazyLock<Remodel> = LazyLock::new(|| {
    lowered(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/plain/config.kdl"
    )))
});

const SHORTCODE: &str = "0123";

const WORK: &str = "http://rdfh.ch/0123/work-predigten";
/// The Work's Leaves. `LEAF_B` and `LEAF_C` have one `seqnum`, and `LEAF_A` the last.
const LEAF_A: &str = "http://rdfh.ch/0123/leaf-a";
const LEAF_B: &str = "http://rdfh.ch/0123/leaf-b";
const LEAF_C: &str = "http://rdfh.ch/0123/leaf-c";
const ORNAMENT: &str = "http://rdfh.ch/0123/ornament-ranke";
const PERSON: &str = "http://rdfh.ch/0123/person-anna";
const KEYWORD: &str = "http://rdfh.ch/0123/keyword-predigt";
const REGION: &str = "http://rdfh.ch/0123/region-initiale";
const LINK: &str = "http://rdfh.ch/0123/link-verweis";

const WORK_CLASS: &str = "http://www.knora.org/ontology/0123/plain#Work";
const LEAF_CLASS: &str = "http://www.knora.org/ontology/0123/plain#Leaf";
const ORNAMENT_CLASS: &str = "http://www.knora.org/ontology/0123/plain#Ornament";
const PERSON_CLASS: &str = "http://www.knora.org/ontology/0123/plain#Person";
const KEYWORD_CLASS: &str = "http://www.knora.org/ontology/0123/plain#Keyword";

const HAS_TITLE: &str = "http://www.knora.org/ontology/0123/plain#hasTitle";
const HAS_PLACE: &str = "http://www.knora.org/ontology/0123/plain#hasPlace";
const HAS_NOTE: &str = "http://www.knora.org/ontology/0123/plain#hasNote";
const HAS_HOMEPAGE: &str = "http://www.knora.org/ontology/0123/plain#hasHomepage";
const HAS_DATE: &str = "http://www.knora.org/ontology/0123/plain#hasDate";
const HAS_NUMBER: &str = "http://www.knora.org/ontology/0123/plain#hasNumber";
const HAS_CAPTION: &str = "http://www.knora.org/ontology/0123/plain#hasCaption";
const HAS_ORNAMENT: &str = "http://www.knora.org/ontology/0123/plain#hasOrnament";
const HAS_ORNAMENT_VERSO: &str = "http://www.knora.org/ontology/0123/plain#hasOrnamentVerso";
const HAS_TERM: &str = "http://www.knora.org/ontology/0123/plain#hasTerm";

/// A Leaf of [`WORK`] at `seqnum` with its number, a scan and `more` values.
fn leaf(iri: &str, seqnum: i64, number: &str, more: Vec<(&str, ValueKind)>) -> Resource {
    fixture_member(iri, (LEAF_CLASS, HAS_NUMBER), WORK, seqnum, number, more)
}

/// The Work's label, which is also one of its two titles.
const WORK_TITLE: &str = "Predigten auf das Jahr";
/// The Work's homepage: an address, on a class whose texts hold none.
const HOMEPAGE: &str = "https://example.org/predigten";

/// The snapshot of `plain`: a Work with three Leaves, an Ornament one Leaf links to, a Person, a
/// Keyword, and a Region and a link the editors keep. Nothing is listed in the order the store
/// writes it in: the resources, the Work's values under its several properties, and the curated
/// values.
///
/// The Work's texts are as an archive serves them: the title that repeats the label ends in a
/// record separator, one note ends in a space and one is whitespace alone, and its place is
/// the sentinel.
fn snapshot() -> Fixture {
    let work = Resource {
        values: values(vec![
            (HAS_NOTE, text("Zweite Notiz ")),
            (HAS_TITLE, text("Jahrpredigten")),
            (HAS_DATE, ValueKind::Date(year_1489())),
            (HAS_NOTE, text(" \u{1f}")),
            (HAS_TITLE, text(&format!("{WORK_TITLE}\u{1e}\n  "))),
            (HAS_PLACE, text("[unbekannt]\n")),
            (HAS_NOTE, text("Erste Notiz")),
            (HAS_HOMEPAGE, ValueKind::Uri(HOMEPAGE.to_string())),
        ]),
        ..fixture_resource(WORK, WORK_CLASS, WORK_TITLE)
    };
    let ornament = Resource {
        values: values(vec![(HAS_TITLE, text("Ranke"))]),
        ..fixture_resource(ORNAMENT, ORNAMENT_CLASS, "Ranke")
    };
    let keyword = Resource {
        values: values(vec![(HAS_TERM, text("Predigt"))]),
        ..fixture_resource(KEYWORD, KEYWORD_CLASS, "Predigt")
    };
    let region = Resource {
        values: values(vec![
            (HAS_COLOR, ValueKind::Color("#ff3333".to_string())),
            (IS_REGION_OF, link(LEAF_A)),
            (HAS_GEOMETRY, ValueKind::Geometry(RECTANGLE.to_string())),
            (KB_HAS_COMMENT, text("Initiale")),
        ]),
        annotation: Some(Annotation {
            motivation: Motivation::Commenting,
            targets: vec![ResourceIri(LEAF_A.to_string())],
        }),
        ..fixture_resource(REGION, REGION_CLASS, "")
    };
    let link_obj = Resource {
        values: values(vec![
            (HAS_LINK_TO, link(WORK)),
            (KB_HAS_COMMENT, text("Verweis")),
            (HAS_LINK_TO, link(REGION)),
        ]),
        annotation: Some(Annotation {
            motivation: Motivation::Linking,
            targets: vec![ResourceIri(REGION.to_string()), ResourceIri(WORK.to_string())],
        }),
        ..fixture_resource(LINK, LINK_OBJ_CLASS, "")
    };
    let facts = ProjectSnapshot {
        shortcode: SHORTCODE.to_string(),
        resources: vec![
            leaf(LEAF_C, 1, "1v", Vec::new()),
            ornament,
            region,
            work,
            leaf(
                LEAF_A,
                3,
                "2r",
                vec![(HAS_ORNAMENT, link(ORNAMENT)), (HAS_CAPTION, text("Mit Ranke am Rand"))],
            ),
            link_obj,
            keyword,
            leaf(LEAF_B, 1, "1r", Vec::new()),
            fixture_resource(PERSON, PERSON_CLASS, "Anna Muster"),
        ],
        list_nodes: Vec::new(),
    };
    let curation = vec![
        curated(REGION, "lang", None, "de"),
        curated(WORK, "slug", None, "predigten"),
        curated(WORK, "blurb", Some("en"), "Sermons for every Sunday."),
        curated(LINK, "name", None, "Verweis auf das Werk"),
        curated(WORK, "date_display", Some("de"), "1489"),
        curated(ORNAMENT, "technique", None, "woodcut"),
        curated(WORK, "cover", None, LEAF_B),
        curated(REGION, "keep", None, "yes"),
        curated(WORK, "blurb", Some("de"), "Predigten für jeden Sonntag."),
        curated(WORK, "genre", None, "sermon"),
        curated(LINK, "keep", None, "yes"),
        curated(WORK, "credit", None, " Sammlung Muster\u{1f}"),
    ];
    assert_eq!(violations(SHORTCODE, &facts), []);
    Fixture { facts, curation }
}

/// [`snapshot`] as a port serves it.
fn served() -> Fixture {
    let Fixture { facts, curation } = snapshot();
    let port = FakeArchiveProjection::default().with_project(facts);
    let facts = port.snapshot(SHORTCODE).expect("the fake port serves the fixture's project");
    Fixture { facts, curation }
}

/// (rule, subject) of every error `build` returns for `snapshot` under `mapping`, in the
/// order returned.
fn refused_under(snapshot: &Fixture, mapping: &Remodel) -> Vec<(Rule, String)> {
    match build_with(snapshot, mapping, None) {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .as_slice()
            .iter()
            .map(|error| (error.rule, error.subject.clone()))
            .collect(),
    }
}

/// [`refused_under`] the mapping of `plain`.
fn refused(snapshot: &Fixture) -> Vec<(Rule, String)> {
    refused_under(snapshot, &PLAIN)
}

const ADDED: &str = "http://rdfh.ch/0123/added";
/// A second Region, where a test adds one beside another resource.
const REGION_2: &str = "http://rdfh.ch/0123/region-zwei";
/// A second Person, where a test adds one beside another resource.
const PERSON_2: &str = "http://rdfh.ch/0123/person-zwei";

const SCRAP_CLASS: &str = "http://www.knora.org/ontology/0123/plain#Scrap";

/// One refusal of the mapped build: [`snapshot`] with one defect, which is refused by `rule`
/// alone. A case of a feature goes here; a test of its own is for what one refusal cannot show,
/// such as an accepted snapshot or two refusals together.
pub struct Case {
    /// The feature of the mapping that refuses: a node or an attribute of the grammar.
    pub feature: &'static str,
    pub rule: Rule,
    pub snapshot: fn() -> Fixture,
    /// What the one error names.
    pub subject: &'static str,
}

pub const CASES: &[Case] = &[
    // The curated value on it is passed over: the resource is refused once, for its class.
    Case {
        feature: "class",
        rule: Rule::UnmappedClass,
        snapshot: || {
            let scrap = with_resource(snapshot(), fixture_resource(ADDED, SCRAP_CLASS, ""));
            with_curation(scrap, ADDED, "slug", "scrap")
        },
        subject: ADDED,
    },
    Case {
        feature: "curated keys",
        rule: Rule::UnknownCuratedKey,
        snapshot: || with_curation(snapshot(), WORK, "sulg", "predigten"),
        subject: WORK,
    },
    // `cover` is a key of the Work.
    Case {
        feature: "curated keys",
        rule: Rule::CuratedKeyOnWrongClass,
        snapshot: || with_curation(snapshot(), PERSON, "cover", LEAF_B),
        subject: PERSON,
    },
    // The one parent is no Work.
    Case {
        feature: "member-of",
        rule: Rule::PageParent,
        snapshot: || with_resource(snapshot(), fourth_leaf(&[PERSON], Some(4))),
        subject: ADDED,
    },
    Case {
        feature: "member-of",
        rule: Rule::PageWithoutSeqnum,
        snapshot: || with_resource(snapshot(), fourth_leaf(&[WORK], None)),
        subject: ADDED,
    },
    Case {
        feature: "id curated-key",
        rule: Rule::SlugCount,
        snapshot: || with_curation_in(snapshot(), WORK, "slug", "de", "predigten-de"),
        subject: WORK,
    },
    Case {
        feature: "id curated-key",
        rule: Rule::BookWithoutCuration,
        snapshot: || with_resource(snapshot(), fixture_resource(ADDED, WORK_CLASS, "Zweites Werk")),
        subject: ADDED,
    },
    // U+001F is whitespace to Python and not to `str::trim`.
    Case {
        feature: "id curated-key",
        rule: Rule::EmptySlug,
        snapshot: || recurated(snapshot(), WORK, "slug", Some(" \u{1f}")),
        subject: WORK,
    },
    // The slugs of all classes are one pool: an Ornament has the Work's.
    Case {
        feature: "id curated-key",
        rule: Rule::DuplicateSlug,
        snapshot: || with_curation(snapshot(), ORNAMENT, "slug", "predigten"),
        subject: "predigten",
    },
    // A slug holds no address on any class: the Ornament's takes addresses in its values.
    Case {
        feature: "id curated-key",
        rule: Rule::HttpInValue,
        snapshot: || with_curation(snapshot(), ORNAMENT, "slug", "http-ranke"),
        subject: ORNAMENT,
    },
    Case {
        feature: "id curated-key",
        rule: Rule::MissingInValue,
        snapshot: || recurated(snapshot(), WORK, "slug", Some("predigten-[unbekannt]")),
        subject: WORK,
    },
    // The label is a text, and none of its characters can stand in an id.
    Case {
        feature: "id from",
        rule: Rule::EmptyIdSlug,
        snapshot: || with_resource(snapshot(), fixture_resource(ADDED, PERSON_CLASS, "[ ; ]")),
        subject: ADDED,
    },
    // The classes are one id space: a Person's label slugs to the Ornament's id.
    Case {
        feature: "id from",
        rule: Rule::DuplicateId,
        snapshot: || with_resource(snapshot(), fixture_resource(ADDED, PERSON_CLASS, "Ranke")),
        subject: "ranke",
    },
    Case {
        feature: "the label",
        rule: Rule::HttpInValue,
        snapshot: || with_label(snapshot(), WORK, "Predigten, http://example.org"),
        subject: WORK,
    },
    Case {
        feature: "the label",
        rule: Rule::MissingInValue,
        snapshot: || with_label(snapshot(), WORK, "Predigten [unbekannt]"),
        subject: WORK,
    },
    // A Person's label is its name and makes its id: it is refused once, as a text.
    Case {
        feature: "the label",
        rule: Rule::EmptyValue,
        snapshot: || with_label(snapshot(), PERSON, " \u{1f}"),
        subject: PERSON,
    },
    Case {
        feature: "property type",
        rule: Rule::UnexpectedValueKind,
        snapshot: || with_value(snapshot(), WORK, HAS_NOTE, ValueKind::Integer(3)),
        subject: WORK,
    },
    Case {
        feature: "property curated-key",
        rule: Rule::CuratedCount,
        snapshot: || {
            let snapshot = with_curation(snapshot(), WORK, "title_override", "Predigten");
            with_curation_in(snapshot, WORK, "title_override", "de", "Jahrespredigten")
        },
        subject: WORK,
    },
    Case {
        feature: "property curated-key",
        rule: Rule::HttpInValue,
        snapshot: || with_curation(snapshot(), ORNAMENT, "name", "http://example.org/ranke"),
        subject: ORNAMENT,
    },
    Case {
        feature: "property curated-key",
        rule: Rule::MissingInValue,
        snapshot: || with_curation(snapshot(), ORNAMENT, "name", "[unbekannt]"),
        subject: ORNAMENT,
    },
    Case {
        feature: "min, max",
        rule: Rule::ValueCount,
        snapshot: || with_resource(snapshot(), fixture_resource(ADDED, KEYWORD_CLASS, "Leer")),
        subject: ADDED,
    },
    // The place the Work has is the sentinel alone, which gives no row and still counts.
    Case {
        feature: "min, max",
        rule: Rule::ValueCount,
        snapshot: || with_text(snapshot(), WORK, HAS_PLACE, "Musterstadt"),
        subject: WORK,
    },
    Case {
        feature: "exclude-label",
        rule: Rule::LabelNotATitle,
        snapshot: || with_label(snapshot(), WORK, "Ein anderer Titel"),
        subject: WORK,
    },
    Case {
        feature: "empty=\"refuse\"",
        rule: Rule::EmptyValue,
        snapshot: || retexted(snapshot(), ORNAMENT, HAS_TITLE, " \u{1f}"),
        subject: ORNAMENT,
    },
    Case {
        feature: "no-addresses",
        rule: Rule::HttpInValue,
        snapshot: || with_text(snapshot(), WORK, HAS_NOTE, "siehe http://example.org"),
        subject: WORK,
    },
    Case {
        feature: "sentinel, sentinel-within",
        rule: Rule::MissingInValue,
        snapshot: || with_text(snapshot(), WORK, HAS_NOTE, "[unbekannt] "),
        subject: WORK,
    },
    // Only the sentinel alone stands for a place that is not known.
    Case {
        feature: "sentinel, sentinel-within",
        rule: Rule::MissingInValue,
        snapshot: || retexted(snapshot(), WORK, HAS_PLACE, "Basel [unbekannt]"),
        subject: WORK,
    },
    Case {
        feature: "type=\"date\"",
        rule: Rule::MixedDatePrecision,
        snapshot: || with_date(|date| date.end.precision = DatePrecision::Day),
        subject: WORK,
    },
    Case {
        feature: "type=\"date\"",
        rule: Rule::DateOutOfRange,
        snapshot: || with_date(|date| date.start.jdn = 1_721_423),
        subject: WORK,
    },
    Case {
        feature: "display-key",
        rule: Rule::CuratedCount,
        snapshot: || recurated(snapshot(), WORK, "date_display", None),
        subject: WORK,
    },
    Case {
        feature: "display-key",
        rule: Rule::DateDisplayWithoutLanguage,
        snapshot: || recurated(snapshot(), WORK, "date_display", Some("1489")),
        subject: WORK,
    },
    Case {
        feature: "display-key",
        rule: Rule::UnknownDateDisplayLanguage,
        snapshot: || {
            let snapshot = recurated(snapshot(), WORK, "date_display", None);
            with_curation_in(snapshot, WORK, "date_display", "fr", "1489")
        },
        subject: WORK,
    },
    // The link's target is a Person, no Ornament.
    Case {
        feature: "target",
        rule: Rule::StripLinkTarget,
        snapshot: || with_link(snapshot(), LEAF_B, HAS_ORNAMENT, PERSON),
        subject: LEAF_B,
    },
    // A Work has a genre.
    Case {
        feature: "curated",
        rule: Rule::CuratedCount,
        snapshot: || recurated(snapshot(), WORK, "genre", None),
        subject: WORK,
    },
    Case {
        feature: "curated",
        rule: Rule::UnknownListNode,
        snapshot: || recurated(snapshot(), WORK, "genre", Some("homily")),
        subject: WORK,
    },
    // The site has two languages, and the blurb is in one.
    Case {
        feature: "curated",
        rule: Rule::CuratedLanguages,
        snapshot: || {
            let snapshot = recurated(snapshot(), WORK, "blurb", None);
            with_curation_in(snapshot, WORK, "blurb", "de", "Predigten für jeden Sonntag.")
        },
        subject: WORK,
    },
    // A Leaf has a file, and it is a still image.
    Case {
        feature: "image=#true, images",
        rule: Rule::NotStillImage,
        snapshot: || refiled(LEAF_C, None),
        subject: LEAF_C,
    },
    Case {
        feature: "image=#true, images",
        rule: Rule::EmptyStem,
        snapshot: || refiled(LEAF_C, Some(still_image(".jp2"))),
        subject: LEAF_C,
    },
    Case {
        feature: "image=#true, images",
        rule: Rule::NotJp2,
        snapshot: || refiled(LEAF_C, Some(still_image("leaf-c.tif"))),
        subject: LEAF_C,
    },
    Case {
        feature: "image=#true, images",
        rule: Rule::ZeroDimension,
        snapshot: || {
            let flat = File::StillImage { asset: "leaf-c.jp2".to_string(), width: 1200, height: 0 };
            refiled(LEAF_C, Some(flat))
        },
        subject: LEAF_C,
    },
    // Two Leaves have one file. The subject is the Image both would be.
    Case {
        feature: "image=#true, images",
        rule: Rule::SharedFile,
        snapshot: || refiled(LEAF_C, Some(still_image("leaf-a.jp2"))),
        subject: "leaf-a",
    },
    // The file's stem is the Ornament's id.
    Case {
        feature: "image=#true, images",
        rule: Rule::ImageIdTaken,
        snapshot: || refiled(LEAF_C, Some(still_image("ranke.jp2"))),
        subject: "ranke",
    },
    Case {
        feature: "cover",
        rule: Rule::CoverCount,
        snapshot: || recurated(snapshot(), WORK, "cover", None),
        subject: WORK,
    },
    Case {
        feature: "cover",
        rule: Rule::MissingInValue,
        snapshot: || recurated(snapshot(), WORK, "cover", Some("[unbekannt]")),
        subject: WORK,
    },
    // The Leaf is a Leaf of the first Work.
    Case {
        feature: "cover",
        rule: Rule::CoverNotOwnPage,
        snapshot: || {
            let second = with_second_work(snapshot(), LEAF_A);
            with_resource(second, leaf_of(ADDED, LEAF_D, 1, "1r"))
        },
        subject: ADDED,
    },
    // Two Works name one Leaf. The subject is the cover, and neither Work is reported for it
    // again, though it is no Leaf of the second.
    Case {
        feature: "cover",
        rule: Rule::SharedCover,
        snapshot: || with_second_work(snapshot(), LEAF_B),
        subject: LEAF_B,
    },
    // An annotation sits on a Leaf, and this one on the Work. The editors do not keep it, and
    // it is checked all the same.
    Case {
        feature: "annotations",
        rule: Rule::RegionTarget,
        snapshot: || with_annotation(snapshot(), region_on(ADDED, WORK), "no"),
        subject: ADDED,
    },
    // A link points at annotations, Works and Leaves, and this one at an Ornament.
    Case {
        feature: "annotations",
        rule: Rule::LinkTarget,
        snapshot: || with_annotation(snapshot(), link_to(ADDED, &[ORNAMENT]), "no"),
        subject: ADDED,
    },
];

/// A fourth Leaf, where a test adds one beside a second Work.
const LEAF_D: &str = "http://rdfh.ch/0123/leaf-d";

/// `snapshot` with a second Work, [`ADDED`], that has every value a Work must have and `cover`
/// as its cover.
fn with_second_work(snapshot: Fixture, cover: &str) -> Fixture {
    let work = Resource {
        values: values(vec![
            (HAS_TITLE, text("Zweites Werk")),
            (HAS_DATE, ValueKind::Date(year_1489())),
        ]),
        ..fixture_resource(ADDED, WORK_CLASS, "Zweites Werk")
    };
    let snapshot = with_resource(snapshot, work);
    let snapshot = with_curation(snapshot, ADDED, "slug", "zweites");
    let snapshot = with_curation(snapshot, ADDED, "genre", "other");
    let snapshot = with_curation_in(snapshot, ADDED, "date_display", "de", "1489");
    with_curation(snapshot, ADDED, "cover", cover)
}

/// A Leaf of the Work `work` at `seqnum` with its number and a scan.
fn leaf_of(work: &str, iri: &str, seqnum: i64, number: &str) -> Resource {
    Resource {
        part_of: vec![ResourceIri(work.to_string())],
        ..leaf(iri, seqnum, number, Vec::new())
    }
}

/// [`snapshot`] with `file` as the file of `resource`.
fn refiled(resource: &str, file: Option<File>) -> Fixture {
    changed(snapshot(), resource, |found| found.file = file)
}

/// `snapshot` with `new` as the text of every value `resource` has under `property`.
fn retexted(snapshot: Fixture, resource: &str, property: &str, new: &str) -> Fixture {
    changed(snapshot, resource, |found| {
        for value in &mut found.values {
            if value.property.as_str() == property {
                value.kind = text(new);
            }
        }
    })
}

/// [`snapshot`] with the Work's date changed by `change`.
fn with_date(change: fn(&mut DateValue)) -> Fixture {
    changed(snapshot(), WORK, |work| {
        for value in &mut work.values {
            if let ValueKind::Date(date) = &mut value.kind {
                change(date);
            }
        }
    })
}

fn fourth_leaf(parents: &[&str], seqnum: Option<i64>) -> Resource {
    Resource {
        part_of: parents.iter().map(|parent| ResourceIri(parent.to_string())).collect(),
        seqnum,
        ..leaf(ADDED, 4, "2v", Vec::new())
    }
}

/// `part_of` may repeat a parent: the member still has one. Two parents are refused, and so is
/// none; a member without a parent and without a position is reported for both.
#[test]
fn test_build_member_has_exactly_one_distinct_parent() {
    let repeated = with_resource(snapshot(), fourth_leaf(&[WORK, WORK], Some(4)));
    let two = with_resource(snapshot(), fourth_leaf(&[WORK, PERSON], Some(4)));
    let none = with_resource(snapshot(), fourth_leaf(&[], Some(4)));
    let neither = with_resource(snapshot(), fourth_leaf(&[], None));

    assert_eq!(refused(&repeated), []);
    assert_eq!(refused(&two), [(Rule::PageParent, ADDED.to_string())]);
    assert_eq!(refused(&none), [(Rule::PageParent, ADDED.to_string())]);
    assert_eq!(
        refused(&neither),
        [
            (Rule::PageParent, ADDED.to_string()),
            (Rule::PageWithoutSeqnum, ADDED.to_string())
        ]
    );
}

/// What each feature of the mapping refuses by in a project without a hook. A refusal that
/// needs a hook's drop is not here: its case needs a hook. Nor is a rule that no node of the
/// mapping switches on, such as the checks of an annotation's geometry.
const FEATURE_RULES: &[(&str, &[Rule])] = &[
    ("class", &[Rule::UnmappedClass]),
    ("curated keys", &[Rule::UnknownCuratedKey, Rule::CuratedKeyOnWrongClass]),
    (
        "id curated-key",
        &[
            Rule::SlugCount,
            Rule::BookWithoutCuration,
            Rule::EmptySlug,
            Rule::DuplicateSlug,
            Rule::HttpInValue,
            Rule::MissingInValue,
        ],
    ),
    ("id from", &[Rule::EmptyIdSlug, Rule::DuplicateId]),
    ("the label", &[Rule::HttpInValue, Rule::MissingInValue, Rule::EmptyValue]),
    ("member-of", &[Rule::PageParent, Rule::PageWithoutSeqnum]),
    ("property type", &[Rule::UnexpectedValueKind]),
    (
        "property curated-key",
        &[Rule::CuratedCount, Rule::HttpInValue, Rule::MissingInValue],
    ),
    ("min, max", &[Rule::ValueCount]),
    ("exclude-label", &[Rule::LabelNotATitle]),
    ("empty=\"refuse\"", &[Rule::EmptyValue]),
    ("no-addresses", &[Rule::HttpInValue]),
    ("sentinel, sentinel-within", &[Rule::MissingInValue]),
    ("type=\"date\"", &[Rule::MixedDatePrecision, Rule::DateOutOfRange]),
    (
        "display-key",
        &[
            Rule::CuratedCount,
            Rule::DateDisplayWithoutLanguage,
            Rule::UnknownDateDisplayLanguage,
        ],
    ),
    ("target", &[Rule::StripLinkTarget]),
    ("curated", &[Rule::CuratedCount, Rule::UnknownListNode, Rule::CuratedLanguages]),
    (
        "image=#true, images",
        &[
            Rule::NotStillImage,
            Rule::EmptyStem,
            Rule::NotJp2,
            Rule::ZeroDimension,
            Rule::SharedFile,
            Rule::ImageIdTaken,
        ],
    ),
    (
        "cover",
        &[
            Rule::CoverCount,
            Rule::MissingInValue,
            Rule::CoverNotOwnPage,
            Rule::SharedCover,
        ],
    ),
    ("annotations", &[Rule::RegionTarget, Rule::LinkTarget]),
];

/// Every refusal a feature gives has a case, and no case is of a feature or a refusal the
/// list does not name: a case under a misspelt feature would stand for nothing.
#[test]
fn test_cases_are_the_refusals_of_every_feature() {
    let listed: BTreeSet<(&str, Rule)> = FEATURE_RULES
        .iter()
        .flat_map(|(feature, rules)| rules.iter().map(move |&rule| (*feature, rule)))
        .collect();
    let cased: BTreeSet<(&str, Rule)> = CASES.iter().map(|case| (case.feature, case.rule)).collect();

    let without_a_case: Vec<_> = listed.difference(&cased).collect();
    let unlisted: Vec<_> = cased.difference(&listed).collect();
    assert_eq!((without_a_case, unlisted), (Vec::new(), Vec::new()));
}

/// A fixture project is its `config.kdl` and nothing else: its snapshot is built in this file,
/// and no store file of it is kept.
#[test]
fn test_fixture_directories_hold_only_their_config() {
    for project in ["plain", "minimal", "hooked"] {
        let directory = format!("{}/tests/fixtures/{project}", env!("CARGO_MANIFEST_DIR"));
        let mut files: Vec<String> = std::fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("{directory} is read: {error}"))
            .map(|entry| {
                let entry = entry.unwrap_or_else(|error| panic!("{directory} is listed: {error}"));
                entry.file_name().to_string_lossy().into_owned()
            })
            .collect();
        files.sort();

        assert_eq!(files, ["config.kdl"], "{project}");
    }
}

/// The rules the cases' snapshots are refused by, for the test that every rule has a case.
pub fn produced_rules() -> BTreeSet<Rule> {
    let refusals = CASES.iter().flat_map(|case| refused(&(case.snapshot)()));
    refusals.map(|(rule, _)| rule).collect()
}

#[test]
fn test_build_each_case_is_refused_by_its_rule_alone_naming_its_subject() {
    for case in CASES {
        assert_eq!(
            refused(&(case.snapshot)()),
            [(case.rule, case.subject.to_string())],
            "{}: {}",
            case.feature,
            case.rule.name()
        );
    }
}

/// The keys the builder reads itself are checked with the mapping's: `keep` is curated on an
/// annotation and on a link, `name` on a link and, by the fixture's block, on an Ornament.
#[test]
fn test_build_builders_own_key_on_a_class_that_does_not_read_it_is_refused() {
    let kept_work = with_curation(snapshot(), WORK, "keep", "yes");
    let named_region = with_curation(snapshot(), REGION, "name", "Initiale");

    assert_eq!(refused(&kept_work), [(Rule::CuratedKeyOnWrongClass, WORK.to_string())]);
    assert_eq!(refused(&named_region), [(Rule::CuratedKeyOnWrongClass, REGION.to_string())]);
}

/// A key is refused once per resource, whatever the number of its values.
#[test]
fn test_build_misplaced_key_in_two_languages_is_refused_once() {
    let german = with_curation_in(snapshot(), WORK, "sulg", "de", "predigten");
    let both = with_curation_in(german, WORK, "sulg", "en", "sermons");

    assert_eq!(refused(&both), [(Rule::UnknownCuratedKey, WORK.to_string())]);
}

/// Under a mapping without an `annotations` block nothing is an annotation, so the builder's
/// own keys are keys nobody reads.
#[test]
fn test_build_builders_own_key_under_a_mapping_without_annotations_is_unknown() {
    let snapshot = minimal_snapshot(Vec::new(), vec![curated(THING, "keep", None, "yes")]);

    assert_eq!(
        refused_under(&snapshot, &MINIMAL),
        [(Rule::UnknownCuratedKey, THING.to_string())]
    );
}

/// An annotation's class is held to its motivation, as in a build without a mapping: here a
/// link that is a Region by its class.
#[test]
fn test_build_annotation_of_the_class_of_another_motivation_is_refused() {
    let link = Resource {
        annotation: Some(Annotation {
            motivation: Motivation::Linking,
            targets: vec![ResourceIri(WORK.to_string())],
        }),
        ..fixture_resource(ADDED, REGION_CLASS, "")
    };

    let refusals = refused(&with_resource(snapshot(), link));

    assert_eq!(refusals, [(Rule::AnnotationClassMismatch, ADDED.to_string())]);
}

/// A mapping without an `annotations` block maps no annotation, so one is of an undeclared
/// class, whatever its class and motivation.
#[test]
fn test_build_annotation_under_a_mapping_without_annotations_is_an_unmapped_class() {
    const REGION: &str = "http://rdfh.ch/0001/region";
    let region = Resource {
        annotation: Some(Annotation {
            motivation: Motivation::Commenting,
            targets: vec![ResourceIri(THING.to_string())],
        }),
        ..fixture_resource(REGION, REGION_CLASS, "")
    };
    let snapshot = minimal_snapshot(vec![region], Vec::new());

    assert_eq!(refused_under(&snapshot, &MINIMAL), [(Rule::UnmappedClass, REGION.to_string())]);
}

/// The fixture's five classes get their ids in five ways: a curated key, a property under the
/// parent's id, a curated key with the label behind it, the label, and the IRI.
#[test]
fn test_plain_fixture_declares_an_id_from_a_curated_key_a_property_a_label_and_an_iri() {
    let policies: Vec<(&str, &IdPolicy)> = PLAIN.classes.iter().map(|class| (class.name.as_str(), &class.id)).collect();

    let [(_, work), (_, leaf), (_, ornament), (_, person), (_, keyword)] = policies.as_slice() else {
        panic!("the fixture declares five classes: {policies:?}");
    };
    assert!(matches!(work, IdPolicy::Curated { fallback: None, .. }), "{work:?}");
    assert!(
        matches!(leaf, IdPolicy::Minted { source: IdSource::Property(_), under_parent: true }),
        "{leaf:?}"
    );
    assert!(
        matches!(ornament, IdPolicy::Curated { fallback: Some(IdSource::Label), .. }),
        "{ornament:?}"
    );
    assert!(
        matches!(person, IdPolicy::Minted { source: IdSource::Label, under_parent: false }),
        "{person:?}"
    );
    assert!(
        matches!(keyword, IdPolicy::Minted { source: IdSource::Iri, under_parent: false }),
        "{keyword:?}"
    );
}

#[test]
fn test_build_plain_snapshot_gives_the_list_tables_by_id_with_the_declared_seq() {
    let store =
        build_with(&served(), &PLAIN, None).unwrap_or_else(|errors| panic!("the fixture's snapshot builds: {errors}"));

    let lists: Vec<(&str, &str)> = store.lists.iter().map(|list| (list.id.as_str(), list.name.as_str())).collect();
    assert_eq!(lists, [("list-genre", "genre"), ("list-technique", "technique")]);
    let nodes: Vec<(&str, &str, Option<&str>, i64)> = store
        .list_nodes
        .iter()
        .map(|node| (node.id.as_str(), node.list_id.as_str(), node.parent_id.as_deref(), node.seq))
        .collect();
    assert_eq!(
        nodes,
        [
            ("genre-almanac", "list-genre", None, 1),
            ("genre-other", "list-genre", None, 2),
            ("genre-sermon", "list-genre", None, 0),
            ("technique-engraving", "list-technique", None, 1),
            ("technique-woodcut", "list-technique", None, 0),
        ]
    );
    let labels: Vec<(&str, &str, &str)> = store
        .list_node_labels
        .iter()
        .map(|label| (label.node_id.as_str(), label.lang.as_str(), label.label.as_str()))
        .collect();
    assert_eq!(
        labels,
        [
            ("genre-almanac", "de", "Kalender"),
            ("genre-almanac", "en", "Almanac"),
            ("genre-other", "de", "Übriges"),
            ("genre-other", "en", "Other"),
            ("genre-sermon", "de", "Predigt"),
            ("genre-sermon", "en", "Sermon"),
            ("technique-engraving", "de", "Kupferstich"),
            ("technique-engraving", "en", "Engraving"),
            ("technique-woodcut", "de", "Holzschnitt"),
            ("technique-woodcut", "en", "Woodcut"),
        ]
    );
}

#[test]
fn test_build_mapping_without_a_list_gives_empty_list_tables() {
    let snapshot = minimal_snapshot(Vec::new(), Vec::new());

    let store = build_with(&snapshot, &MINIMAL, None).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"));

    assert!(store.lists.is_empty());
    assert!(store.list_nodes.is_empty());
    assert!(store.list_node_labels.is_empty());
}

/// The contract is checked against the mapping's shortcode, and its violation comes back
/// without the refusals the snapshot would get after it: here a text with a NUL.
#[test]
fn test_build_snapshot_of_another_shortcode_is_refused_by_the_contract_alone() {
    let mut other = with_resource(snapshot(), fixture_resource("http://rdfh.ch/0123/nul", PERSON_CLASS, "a\0b"));
    assert_eq!(refused(&other), [(Rule::InteriorNul, "http://rdfh.ch/0123/nul".to_string())]);
    other.facts.shortcode = "0124".to_string();

    assert_eq!(refused(&other), [(Rule::ContractViolation, "0124".to_string())]);
}

/// The store `build` makes of `snapshot` under the mapping of `plain`, which must load
/// under the schema with foreign keys on: the schema checks the words of a date row and the
/// key of every row.
fn built(snapshot: &Fixture) -> Store {
    let store = build_with(snapshot, &PLAIN, None).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"));
    let filled: Vec<Table> = Table::ALL.into_iter().filter(|&table| store.row_count(table) > 0).collect();
    let sql = sql::to_sql_tables(&store, &PLAIN, &sql::header("plain", &PLAIN), &filled)
        .expect("every table written has a row");
    let connection = Connection::open_in_memory().expect("an in-memory database opens");
    connection.pragma_update(None, "foreign_keys", true).expect("the pragma is set");
    connection.execute_batch(SCHEMA_SQL).expect("the schema loads");
    connection
        .execute_batch(&sql)
        .unwrap_or_else(|error| panic!("the store loads under the schema: {error}"));
    store
}

/// The ids by each class's policy, the classes in declared order and each by id; the values by
/// resource, name and number, each property's in the port's order and as served.
#[test]
fn test_build_plain_snapshot_gives_the_resources_and_their_values() {
    let store = built(&served());

    assert_eq!(
        resources(&store),
        [
            ("predigten", "Work"),
            ("leaf-a", "Image"),
            ("leaf-b", "Image"),
            ("leaf-c", "Image"),
            ("predigten-1r", "Leaf"),
            ("predigten-1v", "Leaf"),
            ("predigten-2r", "Leaf"),
            ("ranke", "Ornament"),
            ("anna-muster", "Person"),
            ("keyword-predigt", "Keyword"),
        ]
    );
    let arks: Vec<Option<&str>> = store.resources.iter().map(|row| row.ark.as_deref()).collect();
    // An Image is no resource of the archive and has no ARK.
    let ark = |local: &str| Some(format!("{DATA_ARK_PREFIX}{SHORTCODE}/{local}"));
    let expected_arks = [
        ark("work=predigten"),
        None,
        None,
        None,
        ark("leaf=b"),
        ark("leaf=c"),
        ark("leaf=a"),
        ark("ornament=ranke"),
        ark("person=anna"),
        ark("keyword=predigt"),
    ];
    let expected_arks: Vec<Option<&str>> = expected_arks.iter().map(Option::as_deref).collect();
    assert_eq!(arks, expected_arks);

    let untagged = |text: &str| format!("{text}@-");
    let made = "1489/1489 year julian 1489@de".to_string();
    let blurb_de = "Predigten für jeden Sonntag.@de".to_string();
    let blurb_en = "Sermons for every Sunday.@en".to_string();
    let authorship = || untagged("Stadtbibliothek Musterstadt");
    let license = || untagged("CC0 1.0");
    assert_eq!(
        properties(&store),
        [
            ("anna-muster", "name", 0, untagged("Anna Muster")),
            ("keyword-predigt", "name", 0, untagged("Predigt")),
            ("leaf-a", "authorship", 0, authorship()),
            ("leaf-a", "license", 0, license()),
            ("leaf-b", "authorship", 0, authorship()),
            ("leaf-b", "license", 0, license()),
            ("leaf-c", "authorship", 0, authorship()),
            ("leaf-c", "license", 0, license()),
            ("predigten", "alt-title", 0, untagged("Jahrpredigten")),
            ("predigten", "blurb", 0, blurb_de),
            ("predigten", "blurb", 1, blurb_en),
            ("predigten", "credit", 0, untagged("Sammlung Muster")),
            ("predigten", "extent", 0, "int 3".to_string()),
            ("predigten", "genre", 0, "node genre-sermon".to_string()),
            ("predigten", "homepage", 0, untagged(HOMEPAGE)),
            ("predigten", "image", 0, "-> leaf-b".to_string()),
            ("predigten", "made", 0, made),
            ("predigten", "name", 0, untagged(WORK_TITLE)),
            ("predigten", "note", 0, untagged("Zweite Notiz ")),
            ("predigten", "note", 1, untagged("Erste Notiz")),
            ("predigten-1r", "image", 0, "-> leaf-b".to_string()),
            ("predigten-1r", "leaf-id", 0, untagged("1r")),
            ("predigten-1r", "name", 0, untagged("1r")),
            ("predigten-1r", "position", 0, "int 1".to_string()),
            ("predigten-1v", "image", 0, "-> leaf-c".to_string()),
            ("predigten-1v", "leaf-id", 0, untagged("1v")),
            ("predigten-1v", "name", 0, untagged("1v")),
            ("predigten-1v", "position", 0, "int 1".to_string()),
            ("predigten-2r", "caption", 0, untagged("Mit Ranke am Rand")),
            ("predigten-2r", "image", 0, "-> leaf-a".to_string()),
            ("predigten-2r", "leaf-id", 0, untagged("2r")),
            ("predigten-2r", "name", 0, untagged("2r")),
            ("predigten-2r", "ornament", 0, "-> ranke".to_string()),
            ("predigten-2r", "position", 0, "int 3".to_string()),
            ("ranke", "name", 0, untagged("Ranke")),
            ("ranke", "technique", 0, "node technique-woodcut".to_string()),
            ("ranke", "uses", 0, "int 1".to_string()),
        ]
    );
}

/// An Image's one representation is the full-size address of its file under the block's base,
/// with the file's size and both rights as its attribution. The Ornament's class asks for no
/// image, so a file of its own gives it none.
#[test]
fn test_build_plain_snapshot_gives_one_representation_per_file_of_an_image_class() {
    let filed = changed(snapshot(), ORNAMENT, |ornament| {
        ornament.file = Some(still_image("ranke-scan.jp2"));
    });

    let store = built(&filed);

    let found: Vec<_> = store
        .representations
        .iter()
        .map(|row| {
            (
                row.resource_id.as_str(),
                row.kind.as_str(),
                row.uri.as_str(),
                (row.width, row.height, row.alt.as_deref(), row.seq),
                row.attribution.as_deref(),
            )
        })
        .collect();
    let uri = |stem: &str| format!("https://iiif.example.org/0123/{stem}.jp2/full/max/0/default.jpg");
    let (uri_a, uri_b, uri_c) = (uri("leaf-a"), uri("leaf-b"), uri("leaf-c"));
    let size = (Some(1200), Some(1800), None, 0);
    let rights = Some("Stadtbibliothek Musterstadt · CC0 1.0");
    assert_eq!(
        found,
        [
            ("leaf-a", "image", uri_a.as_str(), size, rights),
            ("leaf-b", "image", uri_b.as_str(), size, rights),
            ("leaf-c", "image", uri_c.as_str(), size, rights),
        ]
    );
}

/// An incoming count is the number of links, not of linking resources: one Leaf points at the
/// Ornament from both of its sides, and another from one. An Ornament no Leaf points at has
/// the count 0, which is written.
#[test]
fn test_build_resource_linked_under_two_via_properties_is_counted_twice() {
    let both = with_link(snapshot(), LEAF_A, HAS_ORNAMENT_VERSO, ORNAMENT);
    let unused = Resource {
        values: values(vec![(HAS_TITLE, text("Blatt"))]),
        ..fixture_resource(ADDED, ORNAMENT_CLASS, "Blatt")
    };
    let with_unused = with_resource(both.clone(), unused);
    let three = with_link(with_unused.clone(), LEAF_B, HAS_ORNAMENT_VERSO, ORNAMENT);

    let uses = |snapshot: &Fixture| -> Vec<(String, String)> {
        let store = built(snapshot);
        let counts = store.properties.iter().filter(|row| row.name == "uses");
        counts.map(|row| (row.resource_id.clone(), shown(row))).collect()
    };
    let counted = |id: &str, count: &str| (id.to_string(), count.to_string());
    assert_eq!(uses(&both), [counted("ranke", "int 2")]);
    assert_eq!(uses(&with_unused), [counted("blatt", "int 0"), counted("ranke", "int 2")]);
    assert_eq!(uses(&three), [counted("blatt", "int 0"), counted("ranke", "int 3")]);
}

/// A member count is taken by parent: the second Work has one Leaf and the first still three.
/// A member's position is its `seqnum` as served, not its number among the members.
#[test]
fn test_build_member_count_is_taken_per_parent() {
    let second = with_second_work(snapshot(), LEAF_D);
    let second = with_resource(second, leaf_of(ADDED, LEAF_D, 7, "1r"));

    let store = built(&second);

    let numbers: Vec<(&str, &str, String)> = store
        .properties
        .iter()
        .filter(|row| row.name == "extent" || row.resource_id == "zweites-1r")
        .filter(|row| row.value_type == "integer")
        .map(|row| (row.resource_id.as_str(), row.name.as_str(), shown(row)))
        .collect();
    assert_eq!(
        numbers,
        [
            ("predigten", "extent", "int 3".to_string()),
            ("zweites", "extent", "int 1".to_string()),
            ("zweites-1r", "position", "int 7".to_string()),
        ]
    );
}

/// The Region the editors keep is a row on its Leaf, with their language on its remark; the
/// link is named by its curated name and keeps its comment, and its targets are numbered from
/// 1, the annotation before the Work.
#[test]
fn test_build_plain_snapshot_gives_the_annotations_and_their_links() {
    let store = built(&served());

    let annotations: Vec<_> = store
        .annotations
        .iter()
        .map(|row| {
            (
                (row.id.as_str(), row.resource_id.as_str(), row.shape.as_str()),
                (row.color.as_deref(), row.remark.as_deref()),
                (row.remark_lang.as_deref(), row.ark.as_deref()),
            )
        })
        .collect();
    let ark = format!("{DATA_ARK_PREFIX}{SHORTCODE}/region=initiale");
    assert_eq!(
        annotations,
        [(
            ("region-initiale", "predigten-2r", "rectangle"),
            (Some("#ff3333"), Some("Initiale")),
            (Some("de"), Some(ark.as_str())),
        )]
    );
    let links: Vec<_> = store
        .annotation_links
        .iter()
        .map(|row| {
            (
                (row.id.as_str(), row.name.as_str(), row.name_lang.as_deref()),
                (row.comment.as_deref(), row.comment_lang.as_deref()),
            )
        })
        .collect();
    assert_eq!(
        links,
        [(("link-verweis", "Verweis auf das Werk", None), (Some("Verweis"), None))]
    );
    assert_eq!(
        link_targets(&store),
        [
            ("link-verweis", 1, Some("region-initiale"), None),
            ("link-verweis", 2, None, Some("predigten")),
        ]
    );
}

/// (link, seq, annotation, resource) of the store's link targets, in the store's order.
fn link_targets(store: &Store) -> Vec<(&str, i64, Option<&str>, Option<&str>)> {
    let rows = store.annotation_link_targets.iter();
    rows.map(|row| {
        (
            row.link_id.as_str(),
            row.seq,
            row.annotation_id.as_deref(),
            row.resource_id.as_deref(),
        )
    })
    .collect()
}

/// A link's targets are written annotations first, then resources by the order the mapping
/// declares their classes in, then by id, whatever order the archive serves them in: the Leaf
/// with the later IRI has the earlier id. A target named twice is one target, and an
/// annotation the editors do not keep is none.
#[test]
fn test_build_link_targets_are_annotations_then_classes_as_declared_then_ids() {
    let unkept = with_annotation(snapshot(), region_on(REGION_2, LEAF_B), "no");
    let link = link_to(ADDED, &[LEAF_A, REGION_2, WORK, LEAF_C, REGION, LEAF_A]);

    let store = built(&with_annotation(unkept, link, "yes"));

    let of_link: Vec<_> = link_targets(&store).into_iter().filter(|(link, ..)| *link == "added").collect();
    assert_eq!(
        of_link,
        [
            ("added", 1, Some("region-initiale"), None),
            ("added", 2, None, Some("predigten")),
            ("added", 3, None, Some("predigten-1v")),
            ("added", 4, None, Some("predigten-2r")),
        ]
    );
}

/// What points at a refused resource is not refused for it: the Leaf with the Region has no
/// file, and that is the one report: the Region on it and the link to that Region get no
/// refusal of their own.
#[test]
fn test_build_annotation_on_a_refused_resource_is_not_reported() {
    assert_eq!(refused(&refiled(LEAF_A, None)), [(Rule::NotStillImage, LEAF_A.to_string())]);
}

/// The store file of `snapshot` under the mapping of `plain`, with the header generated for it.
fn plain_file(snapshot: &Fixture) -> String {
    let store = build_with(snapshot, &PLAIN, None).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"));
    sql::to_sql(&store, &PLAIN, &sql::header("plain", &PLAIN)).expect("every table the mapping fills has a row")
}

/// The store file of [`snapshot`]: the generated header and all ten tables.
const PLAIN_FILE: &str = r"-- plain data (Work + Leaf + Ornament + Person + Keyword + annotations), generated by cpe-store-build.
-- Source: the archive projection's snapshot of project 0123.
-- From the bytes port: the remodel block of 0123/config.kdl and 0123/curation.csv.
-- IIIF: https://iiif.example.org/0123/
-- DO NOT hand-edit; regenerate with: just cpe store-build plain --inplace

INSERT INTO lists (id, name) VALUES
    ('list-genre', 'genre'),
    ('list-technique', 'technique');

INSERT INTO list_nodes (id, list_id, parent_id, seq) VALUES
    ('genre-almanac', 'list-genre', NULL, 1),
    ('genre-other', 'list-genre', NULL, 2),
    ('genre-sermon', 'list-genre', NULL, 0),
    ('technique-engraving', 'list-technique', NULL, 1),
    ('technique-woodcut', 'list-technique', NULL, 0);

INSERT INTO list_node_labels (node_id, lang, label) VALUES
    ('genre-almanac', 'de', 'Kalender'),
    ('genre-almanac', 'en', 'Almanac'),
    ('genre-other', 'de', 'Übriges'),
    ('genre-other', 'en', 'Other'),
    ('genre-sermon', 'de', 'Predigt'),
    ('genre-sermon', 'en', 'Sermon'),
    ('technique-engraving', 'de', 'Kupferstich'),
    ('technique-engraving', 'en', 'Engraving'),
    ('technique-woodcut', 'de', 'Holzschnitt'),
    ('technique-woodcut', 'en', 'Woodcut');

INSERT INTO resources (id, class, ark) VALUES
    ('predigten', 'Work', 'https://ark.dasch.swiss/ark:/72163/1/0123/work=predigten'),
    ('leaf-a', 'Image', NULL),
    ('leaf-b', 'Image', NULL),
    ('leaf-c', 'Image', NULL),
    ('predigten-1r', 'Leaf', 'https://ark.dasch.swiss/ark:/72163/1/0123/leaf=b'),
    ('predigten-1v', 'Leaf', 'https://ark.dasch.swiss/ark:/72163/1/0123/leaf=c'),
    ('predigten-2r', 'Leaf', 'https://ark.dasch.swiss/ark:/72163/1/0123/leaf=a'),
    ('ranke', 'Ornament', 'https://ark.dasch.swiss/ark:/72163/1/0123/ornament=ranke'),
    ('anna-muster', 'Person', 'https://ark.dasch.swiss/ark:/72163/1/0123/person=anna'),
    ('keyword-predigt', 'Keyword', 'https://ark.dasch.swiss/ark:/72163/1/0123/keyword=predigt');

INSERT INTO properties
    (resource_id, name, seq, value_type, value_text, value_lang, value_int, value_real, date_start, date_end, date_precision, date_display, date_calendar, target_id)
VALUES
    ('anna-muster', 'name', 0, 'text', 'Anna Muster', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('keyword-predigt', 'name', 0, 'text', 'Predigt', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('leaf-a', 'authorship', 0, 'text', 'Stadtbibliothek Musterstadt', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('leaf-a', 'license', 0, 'text', 'CC0 1.0', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('leaf-b', 'authorship', 0, 'text', 'Stadtbibliothek Musterstadt', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('leaf-b', 'license', 0, 'text', 'CC0 1.0', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('leaf-c', 'authorship', 0, 'text', 'Stadtbibliothek Musterstadt', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('leaf-c', 'license', 0, 'text', 'CC0 1.0', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten', 'alt-title', 0, 'text', 'Jahrpredigten', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten', 'blurb', 0, 'text', 'Predigten für jeden Sonntag.', 'de', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten', 'blurb', 1, 'text', 'Sermons for every Sunday.', 'en', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten', 'credit', 0, 'text', 'Sammlung Muster', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten', 'extent', 0, 'integer', NULL, NULL, 3, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten', 'genre', 0, 'list_value', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'genre-sermon'),
    ('predigten', 'homepage', 0, 'text', 'https://example.org/predigten', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten', 'image', 0, 'link', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'leaf-b'),
    ('predigten', 'made', 0, 'date', NULL, 'de', NULL, NULL, '1489', '1489', 'year', '1489', 'julian', NULL),
    ('predigten', 'name', 0, 'text', 'Predigten auf das Jahr', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten', 'note', 0, 'text', 'Zweite Notiz ', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten', 'note', 1, 'text', 'Erste Notiz', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten-1r', 'image', 0, 'link', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'leaf-b'),
    ('predigten-1r', 'leaf-id', 0, 'text', '1r', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten-1r', 'name', 0, 'text', '1r', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten-1r', 'position', 0, 'integer', NULL, NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten-1v', 'image', 0, 'link', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'leaf-c'),
    ('predigten-1v', 'leaf-id', 0, 'text', '1v', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten-1v', 'name', 0, 'text', '1v', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten-1v', 'position', 0, 'integer', NULL, NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten-2r', 'caption', 0, 'text', 'Mit Ranke am Rand', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten-2r', 'image', 0, 'link', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'leaf-a'),
    ('predigten-2r', 'leaf-id', 0, 'text', '2r', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten-2r', 'name', 0, 'text', '2r', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('predigten-2r', 'ornament', 0, 'link', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'ranke'),
    ('predigten-2r', 'position', 0, 'integer', NULL, NULL, 3, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('ranke', 'name', 0, 'text', 'Ranke', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('ranke', 'technique', 0, 'list_value', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'technique-woodcut'),
    ('ranke', 'uses', 0, 'integer', NULL, NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, NULL);

INSERT INTO representations (resource_id, kind, uri, width, height, alt, attribution, seq) VALUES
    ('leaf-a', 'image', 'https://iiif.example.org/0123/leaf-a.jp2/full/max/0/default.jpg', 1200, 1800, NULL, 'Stadtbibliothek Musterstadt · CC0 1.0', 0),
    ('leaf-b', 'image', 'https://iiif.example.org/0123/leaf-b.jp2/full/max/0/default.jpg', 1200, 1800, NULL, 'Stadtbibliothek Musterstadt · CC0 1.0', 0),
    ('leaf-c', 'image', 'https://iiif.example.org/0123/leaf-c.jp2/full/max/0/default.jpg', 1200, 1800, NULL, 'Stadtbibliothek Musterstadt · CC0 1.0', 0);

INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
    ('predigten', 'predigten-1r', 1),
    ('predigten', 'predigten-1v', 2),
    ('predigten', 'predigten-2r', 3);

INSERT INTO annotations (id, resource_id, shape, x, y, w, h, points, color, remark, remark_lang, ark) VALUES
    ('region-initiale', 'predigten-2r', 'rectangle', 0.25, 0.25, 0.5, 0.25, NULL, '#ff3333', 'Initiale', 'de', 'https://ark.dasch.swiss/ark:/72163/1/0123/region=initiale');

INSERT INTO annotation_links (id, name, name_lang, comment, comment_lang) VALUES
    ('link-verweis', 'Verweis auf das Werk', NULL, 'Verweis', NULL);

INSERT INTO annotation_link_targets (link_id, seq, annotation_id, resource_id) VALUES
    ('link-verweis', 1, 'region-initiale', NULL),
    ('link-verweis', 2, NULL, 'predigten');

";

/// The fixture's snapshot fills every table, so the literal shows each table's statement.
#[test]
fn test_build_plain_snapshot_gives_the_whole_store_file() {
    let store = built(&served());
    let empty: Vec<&str> = Table::ALL
        .into_iter()
        .filter(|&table| store.row_count(table) == 0)
        .map(Table::name)
        .collect();
    assert_eq!(empty, [] as [&str; 0]);

    assert_eq!(plain_file(&served()), PLAIN_FILE);
}

/// Every order the port leaves open is another one: the snapshot's resources and curated
/// values are reversed, and a resource's properties change places. The values of one property
/// keep their order, which is the archive's and is written.
#[test]
fn test_build_reversed_snapshot_writes_the_same_file_byte_for_byte() {
    let mut reversed = snapshot();
    reversed.facts.resources.reverse();
    reversed.curation.reverse();
    for resource in &mut reversed.facts.resources {
        assert!(resource.part_of.len() < 2, "parents to reverse as well");
        // Stable: the properties change places, the values of each do not.
        resource.values.sort_by(|a, b| b.property.cmp(&a.property));
    }
    assert_ne!(reversed.facts.resources[0], snapshot().facts.resources[0]);
    assert_ne!(reversed.curation[0], snapshot().curation[0]);

    assert_eq!(plain_file(&reversed), PLAIN_FILE);
}

/// The least mapping has no list, no images, no membership and no annotations: its file holds
/// the two tables no store is without, and no statement for another.
#[test]
fn test_build_minimal_project_writes_resources_and_properties_only() {
    const HAS_NAME: &str = "http://www.knora.org/ontology/0001/minimal#hasName";
    let named = changed(minimal_snapshot(Vec::new(), Vec::new()), THING, |thing| {
        thing.values = values(vec![(HAS_NAME, text("A thing"))]);
    });
    let store = build_with(&named, &MINIMAL, None).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"));

    let file = sql::to_sql(&store, &MINIMAL, &sql::header("minimal", &MINIMAL));

    let expected = "\
-- minimal data (Thing), generated by cpe-store-build.
-- Source: the archive projection's snapshot of project 0001.
-- From the bytes port: the remodel block of 0001/config.kdl.
-- DO NOT hand-edit; regenerate with: just cpe store-build minimal --inplace

INSERT INTO resources (id, class, ark) VALUES
    ('thing', 'Thing', 'https://ark.dasch.swiss/ark:/72163/1/0123/thing');

INSERT INTO properties
    (resource_id, name, seq, value_type, value_text, value_lang, value_int, value_real, date_start, date_end, date_precision, date_display, date_calendar, target_id)
VALUES
    ('thing', 'name', 0, 'text', 'A thing', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL);

";
    assert_eq!(file.as_deref(), Ok(expected));
}

/// A table is refused for being empty only where the mapping has the block that fills it. The
/// fixture declares lists, so a store without a list row is refused; under the least mapping
/// the same store is a file. A store without a property is refused under that one too.
#[test]
fn test_to_sql_empty_table_is_refused_where_the_mapping_has_its_block() {
    let mut store = built(&served());
    store.lists.clear();
    store.list_nodes.clear();
    store.list_node_labels.clear();
    let header = sql::header("plain", &PLAIN);

    let under_plain = sql::to_sql(&store, &PLAIN, &header);
    let under_minimal = sql::to_sql(&store, &MINIMAL, &header);
    store.properties.clear();
    let without_properties = sql::to_sql(&store, &MINIMAL, &header);

    assert_eq!(under_plain, Err(sql::EmptyTable(Table::Lists)));
    assert!(under_minimal.is_ok_and(|file| !file.contains("INSERT INTO lists")));
    assert_eq!(without_properties, Err(sql::EmptyTable(Table::Properties)));
}

/// A member that is refused for its file takes no number: the added Leaf would stand before
/// its twin and leave it the id `predigten-1r-2`, which a Person has. With a file it does.
#[test]
fn test_build_member_refused_for_its_file_takes_no_part_in_the_numbering() {
    let taken = with_resource(snapshot(), fixture_resource(PERSON_2, PERSON_CLASS, "Predigten 1r 2"));
    let twin = leaf(ADDED, 1, "1r", Vec::new());
    let filed = with_resource(taken.clone(), twin.clone());
    let unfiled = with_resource(taken, Resource { file: None, ..twin });

    assert_eq!(refused(&filed), [(Rule::DuplicateId, "predigten-1r-2".to_string())]);
    assert_eq!(refused(&unfiled), [(Rule::NotStillImage, ADDED.to_string())]);
}

/// A cover is named by the member's IRI with whitespace around it, as the editors' table may
/// hold it. A second Work with a Leaf of its own as its cover has that Leaf's Image.
#[test]
fn test_build_cover_is_the_image_of_the_member_the_stripped_key_names() {
    let padded = recurated(snapshot(), WORK, "cover", Some(&format!(" {LEAF_A}\n")));
    let second = with_second_work(padded, LEAF_D);
    let second = with_resource(second, leaf_of(ADDED, LEAF_D, 1, "1r"));

    let store = built(&second);

    let covers: Vec<(&str, String)> = store
        .properties
        .iter()
        .filter(|row| row.name == "image" && ["predigten", "zweites"].contains(&&*row.resource_id))
        .map(|row| (row.resource_id.as_str(), shown(row)))
        .collect();
    assert_eq!(
        covers,
        [
            ("predigten", "-> leaf-a".to_string()),
            ("zweites", "-> leaf-d".to_string())
        ]
    );
}

/// A cover that was refused for a defect of its own is that one report: its Work is not
/// reported for a cover it cannot show.
#[test]
fn test_build_cover_that_was_refused_is_not_reported_for_its_owner() {
    assert_eq!(refused(&refiled(LEAF_B, None)), [(Rule::NotStillImage, LEAF_B.to_string())]);
}

/// The Leaves have the `seqnum`s 1, 1 and 3: they are numbered from 1 in positional order, the
/// two that share a `seqnum` by the last part of their IRIs.
#[test]
fn test_build_plain_snapshot_numbers_a_parents_members_from_1_in_positional_order() {
    let store = built(&served());

    let parts: Vec<(&str, &str, i64)> = store
        .compound_parts
        .iter()
        .map(|part| (part.parent_id.as_str(), part.child_id.as_str(), part.seq))
        .collect();
    assert_eq!(
        parts,
        [
            ("predigten", "predigten-1r", 1),
            ("predigten", "predigten-1v", 2),
            ("predigten", "predigten-2r", 3),
        ]
    );
}

/// The least mapping: an id from the IRI, a property that may be without a value, and no
/// membership, so no compound part.
#[test]
fn test_build_mapping_without_membership_gives_resources_and_no_compound_part() {
    let snapshot = minimal_snapshot(Vec::new(), Vec::new());

    let store = build_with(&snapshot, &MINIMAL, None).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"));

    assert_eq!(resources(&store), [("thing", "Thing")]);
    assert!(store.properties.is_empty());
    assert!(store.compound_parts.is_empty());
}

/// A text keeps the language the archive records for it.
#[test]
fn test_build_text_is_written_with_its_language() {
    let tagged = changed(snapshot(), LEAF_A, |leaf| {
        for value in &mut leaf.values {
            if value.property.as_str() == HAS_CAPTION {
                value.kind = ValueKind::Text {
                    text: "Mit Ranke am Rand".to_string(),
                    lang: Some("de".to_string()),
                };
            }
        }
    });

    let store = built(&tagged);

    let captions: Vec<String> = store.properties.iter().filter(|row| row.name == "caption").map(shown).collect();
    assert_eq!(captions, ["Mit Ranke am Rand@de"]);
}

/// A curated text stands in for the property's source: it is written without the whitespace
/// around it, and the source, here a title that would be refused, is not read.
#[test]
fn test_build_curated_text_is_written_stripped_and_its_source_is_not_read() {
    let untitled = retexted(snapshot(), ORNAMENT, HAS_TITLE, " \u{1f}");
    let named = with_curation(untitled, ORNAMENT, "name", " Rankenleiste\u{1f}");

    let store = built(&named);

    let names: Vec<String> = store
        .properties
        .iter()
        .filter(|row| row.resource_id == "ranke" && row.name == "name")
        .map(shown)
        .collect();
    assert_eq!(names, ["Rankenleiste@-"]);
}

/// A member's id is its parent's and its own part; a part that repeats is numbered in
/// positional order. The added Leaf shares its `seqnum` and its number with another, and stands
/// before it by the last part of its IRI: it keeps the bare id, though the snapshot lists it
/// last.
#[test]
fn test_build_repeated_member_id_is_numbered_in_positional_order() {
    let twin = leaf(ADDED, 1, "1r", Vec::new());

    let store = built(&with_resource(snapshot(), twin));

    let arks: Vec<(&str, &str)> = store
        .resources
        .iter()
        .filter(|row| row.id.starts_with("predigten-1r"))
        .map(|row| {
            let ark = row.ark.as_deref().unwrap_or_default();
            (row.id.as_str(), ark.rsplit('/').next().unwrap_or(ark))
        })
        .collect();
    assert_eq!(arks, [("predigten-1r", "added"), ("predigten-1r-2", "leaf=b")]);
}

/// Each value of another kind is its own refusal, and the property is judged no further: the
/// Keyword's one term is required, and its absence is not reported beside them.
#[test]
fn test_build_two_values_of_another_kind_are_two_refusals_and_nothing_else() {
    let mistyped = changed(snapshot(), KEYWORD, |keyword| {
        keyword.values = values(vec![(HAS_TERM, ValueKind::Integer(1)), (HAS_TERM, ValueKind::Boolean(true))]);
    });

    assert_eq!(
        refused(&mistyped),
        [
            (Rule::UnexpectedValueKind, KEYWORD.to_string()),
            (Rule::UnexpectedValueKind, KEYWORD.to_string())
        ]
    );
}

/// A resource without an id is mapped no further. The Work's note would be refused for its
/// address; the Leaf's number, which makes its id, is empty, and its second caption is one too
/// many.
#[test]
fn test_build_resource_refused_for_its_id_reports_that_alone() {
    let work = recurated(snapshot(), WORK, "slug", Some(" "));
    let work = with_text(work, WORK, HAS_NOTE, "siehe http://example.org");
    let leaf = retexted(snapshot(), LEAF_A, HAS_NUMBER, " ");
    let leaf = with_text(leaf, LEAF_A, HAS_CAPTION, "Zweite Bildlegende");

    assert_eq!(refused(&work), [(Rule::EmptySlug, WORK.to_string())]);
    assert_eq!(refused(&leaf), [(Rule::EmptyValue, LEAF_A.to_string())]);
}

/// A resource without an id is not reported for its label, and its members are not reported
/// for an id they would not have got: the Work's slug is empty, its label holds the sentinel,
/// and a Leaf's number has no character an id can be made of.
#[test]
fn test_build_resource_without_an_id_silences_its_label_and_its_members_ids() {
    let work = recurated(snapshot(), WORK, "slug", Some(" "));
    let relabelled = with_label(work.clone(), WORK, "Predigten [unbekannt]");
    let unnumbered = retexted(work, LEAF_A, HAS_NUMBER, "[ ; ]");

    assert_eq!(refused(&relabelled), [(Rule::EmptySlug, WORK.to_string())]);
    assert_eq!(refused(&unnumbered), [(Rule::EmptySlug, WORK.to_string())]);
}

/// A member whose number makes no id is refused for it when its parent has an id.
#[test]
fn test_build_member_whose_number_makes_no_id_is_refused() {
    let unnumbered = retexted(snapshot(), LEAF_A, HAS_NUMBER, "[ ; ]");

    assert_eq!(refused(&unnumbered), [(Rule::EmptyIdSlug, LEAF_A.to_string())]);
}

/// An id made from the IRI is its last part as it is; an IRI that ends in `/` has none.
#[test]
fn test_build_iri_without_a_last_part_makes_no_id() {
    const BARE: &str = "http://rdfh.ch/0123/";
    let keyword = Resource {
        iri: ResourceIri(BARE.to_string()),
        values: values(vec![(HAS_TERM, text("Kalender"))]),
        ..fixture_resource(ADDED, KEYWORD_CLASS, "Kalender")
    };

    let refusals = refused(&with_resource(snapshot(), keyword));

    assert_eq!(refusals, [(Rule::EmptyIdSlug, BARE.to_string())]);
}

/// A refused label silences what reads it: the Work's label holds the sentinel and is none of
/// its titles, and only the first is reported.
#[test]
fn test_build_refused_label_is_not_also_reported_as_no_title() {
    let relabelled = with_label(snapshot(), WORK, "Ein anderer Titel [unbekannt]");

    assert_eq!(refused(&relabelled), [(Rule::MissingInValue, WORK.to_string())]);
}

/// An Ornament's id is its curated slug, or else made from its label. The label is read only
/// when no slug is served: with a slug the sentinel in the label is no defect, and a slug that
/// is empty once stripped is refused and the label does not stand in for it.
#[test]
fn test_build_label_behind_a_curated_id_is_read_only_when_no_id_is_served() {
    let sentinel_label = with_label(snapshot(), ORNAMENT, "[unbekannt]");
    let with_slug = with_curation(sentinel_label.clone(), ORNAMENT, "slug", "ranke");
    let empty_slug = with_curation(snapshot(), ORNAMENT, "slug", " \u{1f}");

    assert_eq!(refused(&sentinel_label), [(Rule::MissingInValue, ORNAMENT.to_string())]);
    assert_eq!(refused(&with_slug), []);
    assert_eq!(refused(&empty_slug), [(Rule::EmptySlug, ORNAMENT.to_string())]);
}

/// No property of an Ornament reads its label, so a label of whitespace alone is no empty
/// value: it makes no id. The Leaf that links to the Ornament is not refused for it.
#[test]
fn test_build_whitespace_label_that_only_makes_the_id_is_an_empty_id_slug() {
    let blank = with_label(snapshot(), ORNAMENT, " \u{1f}");

    assert_eq!(refused(&blank), [(Rule::EmptyIdSlug, ORNAMENT.to_string())]);
}

/// A refused label silences the property that makes the id too, where that property is told
/// from the label: the Thing's label holds the sentinel and is not its name, and only the label
/// is reported. The control is the same Thing with a label that is accepted.
#[test]
fn test_build_refused_label_silences_the_property_that_makes_the_id() {
    const CONFIG: &str = r#"
site {
    languages "en"
}

remodel {
    shortcode "0001"
    sentinel "[unknown]"
    class "Thing" iri="http://www.knora.org/ontology/0001/minimal#Thing" {
        id from="property" property="name"
        property "name" iri="http://www.knora.org/ontology/0001/minimal#hasName" type="text" max=1 exclude-label=#true
    }
}
"#;
    const HAS_NAME: &str = "http://www.knora.org/ontology/0001/minimal#hasName";
    let mapping = lowered(CONFIG);
    let labelled = |label: &str| {
        let mut snapshot = minimal_snapshot(Vec::new(), Vec::new());
        snapshot.facts.resources[0].label = label.to_string();
        snapshot.facts.resources[0].values = values(vec![(HAS_NAME, text("Another name"))]);
        snapshot
    };

    let refused_label = refused_under(&labelled("A thing [unknown]"), &mapping);
    let accepted_label = refused_under(&labelled("A thing"), &mapping);

    assert_eq!(refused_label, [(Rule::MissingInValue, THING.to_string())]);
    assert_eq!(accepted_label, [(Rule::LabelNotATitle, THING.to_string())]);
}

/// The mapping of the fixture `config` with U+0000 inside every string that is `text`. A name
/// that is referred to elsewhere in the block is changed there too, so the block still lowers.
fn with_nul_in(config: &str, text: &str) -> Remodel {
    let (head, tail) = text.split_at(1);
    let quoted = |text: &str| format!("\"{text}\"");
    assert!(config.contains(&quoted(text)), "{text} is in the fixture");
    let escaped = format!("{head}\\u{{0}}{tail}");
    lowered(&config.replace(&quoted(text), &quoted(&escaped)))
}

/// A text of the mapping reaches the store as a text of the snapshot does, and is held to the
/// same rule, wherever it stands: every place is refused by the mapping's shortcode, and the
/// refusal's detail names the place.
#[test]
fn test_build_nul_in_a_text_of_the_mapping_is_refused_naming_its_place() {
    const PLAIN_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/plain/config.kdl"));
    const HOOKED_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/hooked/config.kdl"));
    let plain = |text: &str| with_nul_in(PLAIN_CONFIG, text);
    let changed = |change: fn(&mut Remodel)| {
        let mut mapping = PLAIN.clone();
        change(&mut mapping);
        mapping
    };
    // (the text that holds the NUL, the mapping, the places refused)
    let cases: Vec<(&str, Remodel, &[&str])> = vec![
        ("a class's name", plain("Work"), &["class"]),
        ("a property's name", plain("place"), &["property"]),
        ("a property's second name", plain("leaf-id"), &["property"]),
        ("a curated node's name", plain("credit"), &["curated"]),
        ("a member count's name", plain("extent"), &["member-count"]),
        ("an incoming count's name", plain("uses"), &["incoming-count"]),
        ("a seq name", plain("position"), &["member-of"]),
        ("the images' class", plain("Image"), &["images"]),
        (
            "the IIIF base",
            changed(|mapping| {
                let images = mapping.images.as_mut().expect("the fixture has images");
                images.iiif_base.push('\0');
            }),
            &["images"],
        ),
        ("the authorship", plain("Stadtbibliothek Musterstadt"), &["authorship"]),
        ("the licence", plain("CC0 1.0"), &["license"]),
        // The list's name is also the name of the curated node that reads it.
        ("a list's name", plain("genre"), &["curated", "list"]),
        ("a list node's slug", plain("sermon"), &["list-node"]),
        ("a list node's label", plain("Predigt"), &["list-node label"]),
        (
            "the language of a list node's label",
            changed(|mapping| mapping.lists[0].nodes[0].labels[0].language.push('\0')),
            &["list-node label"],
        ),
        (
            "a site language",
            changed(|mapping| mapping.languages[0].push('\0')),
            &["languages"],
        ),
        ("a name a hook provides", with_nul_in(HOOKED_CONFIG, "rank"), &["provides"]),
    ];
    let empty = Fixture {
        facts: ProjectSnapshot {
            shortcode: SHORTCODE.to_string(),
            resources: Vec::new(),
            list_nodes: Vec::new(),
        },
        curation: Vec::new(),
    };

    for (what, mapping, places) in cases {
        let errors = build_with(&empty, &mapping, None).expect_err("a NUL is refused");
        let nuls = errors.as_slice().iter().filter(|error| error.rule == Rule::InteriorNul);
        let refused: Vec<(&str, &str)> = nuls
            .map(|error| {
                let place = error.detail.as_deref().unwrap_or_default();
                (error.subject.as_str(), place)
            })
            .collect();

        let expected: Vec<(&str, &str)> = places.iter().map(|place| (SHORTCODE, *place)).collect();
        assert_eq!(refused, expected, "{what}");
    }
}

/// A class that is not `no-addresses` takes an address in a text and in its label: both are
/// written.
#[test]
fn test_build_class_without_no_addresses_takes_an_address_in_a_text_and_in_its_label() {
    const CONFIG: &str = r#"
site {
    languages "en"
}

remodel {
    shortcode "0123"
    class "Work" iri="http://www.knora.org/ontology/0123/plain#Work" {
        id from="iri"
        property "name" from="label"
        property "note" iri="http://www.knora.org/ontology/0123/plain#hasNote" type="text"
    }
}
"#;
    let snapshot = Fixture {
        facts: ProjectSnapshot {
            shortcode: SHORTCODE.to_string(),
            resources: vec![Resource {
                values: values(vec![(HAS_NOTE, text("see http://x"))]),
                ..fixture_resource(WORK, WORK_CLASS, "Predigten, http://a.example")
            }],
            list_nodes: Vec::new(),
        },
        curation: Vec::new(),
    };

    let store =
        build_with(&snapshot, &lowered(CONFIG), None).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"));

    assert_eq!(
        properties(&store),
        [
            ("work-predigten", "name", 0, "Predigten, http://a.example@-".to_string()),
            ("work-predigten", "note", 0, "see http://x@-".to_string()),
        ]
    );
}

/// An incoming count is of the links from the class its `from` names: a link from a resource
/// of another class is not counted, also under a property of the same name.
#[test]
fn test_build_incoming_count_counts_the_links_from_its_class_only() {
    const CONFIG: &str = r#"
site {
    languages "en"
}

remodel {
    shortcode "0123"
    class "Ornament" iri="http://www.knora.org/ontology/0123/plain#Ornament" {
        id from="iri"
        incoming-count "uses" from="Leaf" {
            via "ornament"
        }
    }
    class "Leaf" iri="http://www.knora.org/ontology/0123/plain#Leaf" {
        id from="iri"
        property "ornament" iri="http://www.knora.org/ontology/0123/plain#hasOrnament" type="link" target="Ornament"
    }
    class "Scrap" iri="http://www.knora.org/ontology/0123/plain#Scrap" {
        id from="iri"
        property "ornament" iri="http://www.knora.org/ontology/0123/plain#hasOrnament" type="link" target="Ornament"
    }
}
"#;
    let linking = |iri: &str, class: &str| Resource {
        values: values(vec![(HAS_ORNAMENT, link(ORNAMENT))]),
        ..fixture_resource(iri, class, "")
    };
    let snapshot = Fixture {
        facts: ProjectSnapshot {
            shortcode: SHORTCODE.to_string(),
            resources: vec![
                fixture_resource(ORNAMENT, ORNAMENT_CLASS, ""),
                linking(LEAF_A, LEAF_CLASS),
                linking("http://rdfh.ch/0123/scrap-a", SCRAP_CLASS),
                linking("http://rdfh.ch/0123/scrap-b", SCRAP_CLASS),
            ],
            list_nodes: Vec::new(),
        },
        curation: Vec::new(),
    };
    assert_eq!(violations(SHORTCODE, &snapshot.facts), []);

    let store =
        build_with(&snapshot, &lowered(CONFIG), None).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"));

    assert_eq!(under(&store, "uses"), [("ornament-ranke", "int 1".to_string())]);
}

/// A date may be without a value where its node asks for none (`min` is 0). The text a reader
/// is shown for a date is read only for a resource that has one: without a date none is asked
/// for, and one the editors curated all the same is not read and not written. With a date the
/// text is required.
#[test]
fn test_build_date_display_text_is_read_only_for_a_resource_that_has_a_date() {
    const CONFIG: &str = r#"
site {
    languages "de" "en"
}

remodel {
    shortcode "0123"
    class "Work" iri="http://www.knora.org/ontology/0123/plain#Work" {
        id from="iri"
        property "made" iri="http://www.knora.org/ontology/0123/plain#hasDate" type="date" max=1 display-key="date_display"
    }
}
"#;
    let mapping = lowered(CONFIG);
    let work = |dated: bool, shown: bool| {
        let date = dated.then(|| (HAS_DATE, ValueKind::Date(year_1489())));
        let display = shown.then(|| curated(WORK, "date_display", Some("de"), "im Jahr 1489"));
        Fixture {
            facts: ProjectSnapshot {
                shortcode: SHORTCODE.to_string(),
                resources: vec![Resource {
                    values: values(date.into_iter().collect()),
                    ..fixture_resource(WORK, WORK_CLASS, "")
                }],
                list_nodes: Vec::new(),
            },
            curation: display.into_iter().collect(),
        }
    };
    let made = |snapshot: &Fixture| -> Vec<String> {
        let store =
            build_with(snapshot, &mapping, None).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"));
        store.properties.iter().map(shown).collect()
    };

    assert_eq!(made(&work(false, false)), Vec::<String>::new());
    assert_eq!(made(&work(false, true)), Vec::<String>::new());
    assert_eq!(made(&work(true, true)), ["1489/1489 year julian im Jahr 1489@de"]);
    assert_eq!(
        refused_under(&work(true, false), &mapping),
        [(Rule::CuratedCount, WORK.to_string())]
    );
}

/// A mapping whose parent class has members of two classes, and counts those of one.
const SHELVES: &str = r#"
site {
    languages "en"
}

remodel {
    shortcode "0123"
    class "Shelf" iri="http://www.knora.org/ontology/0123/plain#Shelf" {
        id from="iri"
        member-count "volumes" of="Volume"
    }
    class "Volume" iri="http://www.knora.org/ontology/0123/plain#Volume" {
        member-of "Shelf"
        id from="property" property="name" under-parent=#true
        property "name" iri="http://www.knora.org/ontology/0123/plain#hasNumber" type="text" min=1 max=1
    }
    class "Box" iri="http://www.knora.org/ontology/0123/plain#Box" {
        member-of "Shelf"
        id from="iri"
    }
}
"#;

/// The store of three Shelves under [`SHELVES`]: `shelf-a` holds a Volume numbered `1` and a
/// Box, `shelf-b` a Volume numbered `1`, and `shelf-c` nothing.
fn shelves() -> Store {
    const SHELF: &str = "http://www.knora.org/ontology/0123/plain#Shelf";
    const VOLUME: &str = "http://www.knora.org/ontology/0123/plain#Volume";
    const BOX: &str = "http://www.knora.org/ontology/0123/plain#Box";
    let iri = |local: &str| format!("http://rdfh.ch/0123/{local}");
    let member = |local: &str, class: &str, shelf: &str, seqnum: i64| Resource {
        values: match class {
            VOLUME => values(vec![(HAS_NUMBER, text("1"))]),
            _ => Vec::new(),
        },
        part_of: vec![ResourceIri(iri(shelf))],
        seqnum: Some(seqnum),
        ..fixture_resource(&iri(local), class, "")
    };
    let snapshot = Fixture {
        facts: ProjectSnapshot {
            shortcode: SHORTCODE.to_string(),
            resources: vec![
                fixture_resource(&iri("shelf-a"), SHELF, ""),
                fixture_resource(&iri("shelf-b"), SHELF, ""),
                fixture_resource(&iri("shelf-c"), SHELF, ""),
                member("volume-a", VOLUME, "shelf-a", 1),
                member("box-a", BOX, "shelf-a", 2),
                member("volume-b", VOLUME, "shelf-b", 1),
            ],
            list_nodes: Vec::new(),
        },
        curation: Vec::new(),
    };
    assert_eq!(violations(SHORTCODE, &snapshot.facts), []);
    build_with(&snapshot, &lowered(SHELVES), None).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"))
}

/// (resource, [`shown`]) of the store's values under `name`, in the store's order.
fn under<'a>(store: &'a Store, name: &str) -> Vec<(&'a str, String)> {
    let rows = store.properties.iter().filter(|row| row.name == name);
    rows.map(|row| (row.resource_id.as_str(), shown(row))).collect()
}

/// A parent without a member has the count 0, which is written.
#[test]
fn test_build_member_count_of_a_parent_without_a_member_is_written_as_0() {
    let store = shelves();

    let counts = under(&store, "volumes");
    assert!(counts.contains(&("shelf-c", "int 0".to_string())), "{counts:?}");
}

/// A member count is of the class its `of` names: a Shelf's Box is no Volume.
#[test]
fn test_build_member_count_counts_the_members_of_its_class_only() {
    let store = shelves();

    let counts = under(&store, "volumes");
    assert!(counts.contains(&("shelf-a", "int 1".to_string())), "{counts:?}");
}

/// A local part repeats only among one parent's members: two parents each have a member with
/// the same part, and neither member is numbered. Each id is the parent's id and the part.
#[test]
fn test_build_member_id_repeated_across_parents_is_not_numbered() {
    let store = shelves();

    let volumes: Vec<&str> = resources(&store)
        .into_iter()
        .filter(|(_, class)| *class == "Volume")
        .map(|(id, _)| id)
        .collect();
    assert_eq!(volumes, ["shelf-a-1", "shelf-b-1"]);
}

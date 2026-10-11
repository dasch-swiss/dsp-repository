//! The one constructor of hand-built snapshots, the mutators the tests change it with, and the
//! readers of the store built from one.
//!
//! Classes and properties are the archive's real IRIs: the rules match on them.

use std::sync::LazyLock;

use cpe_core::remodel::Remodel;
use cpe_core::validate::load_remodel;
use cpe_ports::contract::violations;
use cpe_ports::{
    Annotation, Calendar, ClassIri, CuratedValue, DataArk, DateBound, DatePrecision, DateValue, File, ListNode,
    ListNodeIri, Motivation, ProjectSnapshot, PropertyIri, Resource, ResourceIri, Value, ValueKind, DATA_ARK_PREFIX,
};
use cpe_store_build::store::{PropertyRow, Store, Table};
use cpe_store_build::{sql, SCHEMA_SQL};
use rusqlite::Connection;

use crate::{build, shortcode, HEADER, MAPPING};

/// The snapshot's one Book.
pub const BOOK: &str = "http://rdfh.ch/0803/CDYZPN5zVVKbIcjA1DZxKQ";
/// The Book's label, which is also its one title.
pub const BOOK_TITLE: &str = "Bereitung zu dem Heiligen Sakrament";
/// The Book's one Page, which is its cover.
pub const PAGE: &str = "http://rdfh.ch/0803/s-hBmajpVteY3fYcRjsJIQ";
/// The file of [`PAGE`].
pub const PAGE_ASSET: &str = "ScanOfThePage.jp2";

/// The IRI of the resource a case adds, and of a second one.
pub const ADDED: &str = "http://rdfh.ch/0803/added";
pub const OTHER: &str = "http://rdfh.ch/0803/other";

/// The file of the Band [`with_band`] and [`with_strip`] add.
pub const STRIP_ASSET: &str = "ScanOfTheStrip.jp2";

pub const BOOK_CLASS: &str = "http://www.knora.org/ontology/0803/incunabula#Book";
pub const PAGE_CLASS: &str = "http://www.knora.org/ontology/0803/incunabula#Page";
pub const BAND_CLASS: &str = "http://www.knora.org/ontology/0803/incunabula#Band";
pub const REGION_CLASS: &str = "http://www.knora.org/ontology/knora-base#Region";
pub const LINK_OBJ_CLASS: &str = "http://www.knora.org/ontology/knora-base#LinkObj";

pub const HAS_PAGENUM: &str = "http://www.knora.org/ontology/0803/incunabula#hasPagenum";
pub const HAS_DESCRIPTION: &str = "http://www.knora.org/ontology/0803/incunabula#hasDescription";
pub const HAS_COMMENT: &str = "http://www.knora.org/ontology/0803/incunabula#hasComment";
pub const HAS_BAND_TYPE_L: &str = "http://www.knora.org/ontology/0803/incunabula#hasBandTypeL";
pub const HAS_BAND_TYPE_R: &str = "http://www.knora.org/ontology/0803/incunabula#hasBandTypeR";
pub const HAS_TITLE: &str = "http://www.knora.org/ontology/0803/incunabula#hasTitle";
pub const HAS_CREATOR: &str = "http://www.knora.org/ontology/0803/incunabula#hasCreator";
pub const HAS_PUBLISHER: &str = "http://www.knora.org/ontology/0803/incunabula#hasPublisher";
pub const HAS_PUBLOC: &str = "http://www.knora.org/ontology/0803/incunabula#hasPubloc";
pub const HAS_PUBDATE: &str = "http://www.knora.org/ontology/0803/incunabula#hasPubdate";
pub const HAS_LOCATION: &str = "http://www.knora.org/ontology/0803/incunabula#hasLocation";
pub const HAS_PHYSICAL_DESC: &str = "http://www.knora.org/ontology/0803/incunabula#hasPhysicalDesc";
pub const HAS_URI: &str = "http://www.knora.org/ontology/0803/incunabula#hasUri";

pub const IS_REGION_OF: &str = "http://www.knora.org/ontology/knora-base#isRegionOf";
pub const HAS_LINK_TO: &str = "http://www.knora.org/ontology/knora-base#hasLinkTo";
pub const HAS_GEOMETRY: &str = "http://www.knora.org/ontology/knora-base#hasGeometry";
pub const HAS_COLOR: &str = "http://www.knora.org/ontology/knora-base#hasColor";
/// knora-base's comment: a Region's remark, a LinkObj's comment.
pub const KB_HAS_COMMENT: &str = "http://www.knora.org/ontology/knora-base#hasComment";

/// The geometry of the Region [`region`] makes: a box from a quarter to three quarters across
/// and from a quarter to a half down.
pub const RECTANGLE: &str = r#"{"type":"rectangle","points":[{"x":0.25,"y":0.25},{"x":0.75,"y":0.5}]}"#;

/// The smallest snapshot `build` accepts: a Book with what every Book needs, and its one Page,
/// which is its cover. It satisfies the port's contract, so a test's snapshot is wrong only
/// where the test changed it.
pub fn minimal() -> ProjectSnapshot {
    let empty = ProjectSnapshot {
        shortcode: shortcode().to_string(),
        resources: Vec::new(),
        list_nodes: Vec::new(),
        curation: Vec::new(),
    };
    let snapshot = with_book(empty, BOOK, "bereitung", BOOK_TITLE);
    let snapshot = with_resource(snapshot, page(PAGE, BOOK, 1, "a1r", PAGE_ASSET));
    let snapshot = with_curation(snapshot, BOOK, "cover_page", PAGE);
    assert_eq!(violations(shortcode(), &snapshot), []);
    snapshot
}

/// The minimal snapshot with a second Page of its Book, [`ADDED`], after the first.
pub fn with_page(pagenum: &str) -> ProjectSnapshot {
    let second = page(ADDED, BOOK, 2, pagenum, "ScanOfTheSecondPage.jp2");
    with_resource(minimal(), second)
}

/// The minimal snapshot with a Band labelled `label`, [`ADDED`], whose one title is its label.
pub fn with_band(label: &str) -> ProjectSnapshot {
    with_resource(minimal(), band(ADDED, label, STRIP_ASSET))
}

/// [`with_page`] with a Band labelled `label`, [`OTHER`], that the Page links to under `property`.
pub fn with_strip(label: &str, property: &str) -> ProjectSnapshot {
    let snapshot = with_resource(with_page("a1v"), band(OTHER, label, STRIP_ASSET));
    with_link(snapshot, ADDED, property, OTHER)
}

/// `snapshot` with one more Book that has what every Book needs except a cover: a label that
/// is its one title, a publication date ([`year_1489`]), and the curated slug, office and date
/// display.
pub fn with_book(snapshot: ProjectSnapshot, iri: &str, slug: &str, title: &str) -> ProjectSnapshot {
    let mut book = resource(iri, BOOK_CLASS);
    book.label = title.to_string();
    let snapshot = with_resource(snapshot, book);
    let snapshot = with_text(snapshot, iri, HAS_TITLE, title);
    let snapshot = with_value(snapshot, iri, HAS_PUBDATE, ValueKind::Date(year_1489()));
    let snapshot = with_curation(snapshot, iri, "slug", slug);
    let snapshot = with_curation(snapshot, iri, "office", "amerbach");
    with_curation_in(snapshot, iri, "date_display", "de", "1489")
}

/// The year 1489 of the Julian calendar, from its first day to its last.
pub fn year_1489() -> DateValue {
    let bound = |jdn| DateBound { jdn, precision: DatePrecision::Year };
    DateValue {
        calendar: Calendar::Julian,
        start: bound(2_264_916),
        end: bound(2_265_280),
    }
}

/// The store `build` makes of `snapshot`, which must load under the schema with foreign keys on.
/// A test that reads rows takes its store from here: the schema checks the words of a date row
/// and the key of every row, and the tests over the committed 0803 data reach only the values
/// that data has.
pub fn built(snapshot: &ProjectSnapshot) -> Store {
    let store = build(snapshot).unwrap_or_else(|errors| panic!("the snapshot builds: {errors}"));
    let filled: Vec<Table> = Table::ALL.into_iter().filter(|&table| store.row_count(table) > 0).collect();
    let sql = sql::to_sql_tables(&store, &MAPPING, &HEADER, &filled).expect("every table written has a row");
    let connection = Connection::open_in_memory().expect("an in-memory database opens");
    connection.pragma_update(None, "foreign_keys", true).expect("the pragma is set");
    connection.execute_batch(SCHEMA_SQL).expect("the schema loads");
    connection
        .execute_batch(&sql)
        .unwrap_or_else(|error| panic!("the store loads under the schema: {error}"));
    store
}

/// The resource's values under `name`, in the store's order.
pub fn rows<'a>(store: &'a Store, id: &str, name: &str) -> Vec<&'a PropertyRow> {
    store
        .properties
        .iter()
        .filter(|row| row.resource_id == id && row.name == name)
        .collect()
}

/// (seq, text, language) of the resource's values under `name`, in the store's order.
pub fn texts<'a>(store: &'a Store, id: &str, name: &str) -> Vec<(i64, &'a str, Option<&'a str>)> {
    rows(store, id, name)
        .into_iter()
        .map(|row| {
            assert_eq!(row.value_type, "text");
            let text = row.value_text.as_deref().expect("a text row has a text");
            (row.seq, text, row.value_lang.as_deref())
        })
        .collect()
}

/// A resource of `class` with nothing else: no value, file, parent or position. Its ARK has the
/// contract's shape and is no real ARK: the check digit is missing.
pub fn resource(iri: &str, class: &str) -> Resource {
    let local = iri.rsplit('/').next().unwrap_or(iri).replace('-', "=");
    Resource {
        iri: ResourceIri(iri.to_string()),
        ark: DataArk(format!("{DATA_ARK_PREFIX}{}/{local}", shortcode())),
        class: ClassIri(class.to_string()),
        label: String::new(),
        values: Vec::new(),
        file: None,
        part_of: Vec::new(),
        seqnum: None,
        annotation: None,
    }
}

/// A Page of `book` at `seqnum`, with one pagenum and a scan.
pub fn page(iri: &str, book: &str, seqnum: i64, pagenum: &str, asset: &str) -> Resource {
    Resource {
        values: vec![text_value(0, HAS_PAGENUM, pagenum)],
        file: Some(still_image(asset)),
        part_of: vec![ResourceIri(book.to_string())],
        seqnum: Some(seqnum),
        ..resource(iri, PAGE_CLASS)
    }
}

/// A Band, the archive's border strip, with a scan and its label as its one title.
pub fn band(iri: &str, label: &str, asset: &str) -> Resource {
    Resource {
        label: label.to_string(),
        values: vec![text_value(0, HAS_TITLE, label)],
        file: Some(still_image(asset)),
        ..resource(iri, BAND_CLASS)
    }
}

pub fn still_image(asset: &str) -> File {
    File::StillImage { asset: asset.to_string(), width: 1200, height: 1800 }
}

/// An annotation of `class` on `target`, which must be in the snapshot it is added to.
pub fn annotation(iri: &str, class: &str, motivation: Motivation, target: &str) -> Resource {
    Resource {
        annotation: Some(Annotation { motivation, targets: vec![ResourceIri(target.to_string())] }),
        ..resource(iri, class)
    }
}

/// A Region on `page`, which must be in the snapshot it is added to, with what every Region
/// has: one geometry ([`RECTANGLE`]) and one colour. It has no remark.
pub fn region(iri: &str, page: &str) -> Resource {
    Resource {
        values: vec![
            value(0, HAS_GEOMETRY, ValueKind::Geometry(RECTANGLE.to_string())),
            value(1, HAS_COLOR, ValueKind::Color("#ff3333".to_string())),
            link_value(IS_REGION_OF, page),
        ],
        ..annotation(iri, REGION_CLASS, Motivation::Commenting, page)
    }
}

/// A LinkObj to `targets`, at least one, each of which must be in the snapshot it is added to.
/// It has no comment.
pub fn link_obj(iri: &str, targets: &[&str]) -> Resource {
    let mut targets: Vec<ResourceIri> = targets.iter().map(|target| ResourceIri(target.to_string())).collect();
    targets.sort();
    Resource {
        values: targets.iter().map(|target| link_value(HAS_LINK_TO, target.as_str())).collect(),
        annotation: Some(Annotation { motivation: Motivation::Linking, targets }),
        ..resource(iri, LINK_OBJ_CLASS)
    }
}

/// `snapshot` with `annotation`, a Region or a LinkObj, and the editors' `keep` for it.
pub fn with_annotation(snapshot: ProjectSnapshot, annotation: Resource, keep: &str) -> ProjectSnapshot {
    let iri = annotation.iri.as_str().to_string();
    with_curation(with_resource(snapshot, annotation), &iri, "keep", keep)
}

/// `snapshot` with one more resource.
pub fn with_resource(mut snapshot: ProjectSnapshot, resource: Resource) -> ProjectSnapshot {
    snapshot.resources.push(resource);
    snapshot
}

/// `snapshot` with `resource`, which must be in it, changed by `change`.
pub fn changed(mut snapshot: ProjectSnapshot, resource: &str, change: impl FnOnce(&mut Resource)) -> ProjectSnapshot {
    let found = snapshot
        .resources
        .iter_mut()
        .find(|candidate| candidate.iri.as_str() == resource)
        .unwrap_or_else(|| panic!("{resource} is in the snapshot"));
    change(found);
    snapshot
}

/// `snapshot` with another label on `resource`.
pub fn with_label(snapshot: ProjectSnapshot, resource: &str, label: &str) -> ProjectSnapshot {
    changed(snapshot, resource, |found| found.label = label.to_string())
}

/// `snapshot` with one more text on `resource`, after the values it has.
pub fn with_text(snapshot: ProjectSnapshot, resource: &str, property: &str, text: &str) -> ProjectSnapshot {
    changed(snapshot, resource, |found| {
        let value = text_value(found.values.len(), property, text);
        found.values.push(value);
    })
}

/// `snapshot` with one more value of `kind` on `resource`, after the values it has. Not for a
/// link, which has no UUID: [`with_link`] adds one.
pub fn with_value(snapshot: ProjectSnapshot, resource: &str, property: &str, kind: ValueKind) -> ProjectSnapshot {
    changed(snapshot, resource, |found| {
        let value = value(found.values.len(), property, kind);
        found.values.push(value);
    })
}

/// `snapshot` with one more link from `resource` to `target`, which must be in it.
pub fn with_link(snapshot: ProjectSnapshot, resource: &str, property: &str, target: &str) -> ProjectSnapshot {
    changed(snapshot, resource, |found| {
        found.values.push(link_value(property, target));
    })
}

/// `snapshot` with a list node, and a value of that node on `resource`.
pub fn with_list_node(mut snapshot: ProjectSnapshot, resource: &str, property: &str) -> ProjectSnapshot {
    let node = ListNodeIri("http://rdfh.ch/lists/0803/node".to_string());
    snapshot.list_nodes.push(ListNode {
        iri: node.clone(),
        parent: None,
        position: None,
        labels: Vec::new(),
    });
    changed(snapshot, resource, |found| {
        found.values.push(Value {
            property: PropertyIri(property.to_string()),
            uuid: Some(format!("value-{}", found.values.len())),
            kind: ValueKind::ListNode(node),
        });
    })
}

/// `snapshot` with one more curated value, on any IRI.
pub fn with_curation(mut snapshot: ProjectSnapshot, resource: &str, key: &str, text: &str) -> ProjectSnapshot {
    snapshot.curation.push(curated(resource, key, None, text));
    snapshot
}

/// `snapshot` with one more curated value that has a language.
pub fn with_curation_in(
    mut snapshot: ProjectSnapshot,
    resource: &str,
    key: &str,
    lang: &str,
    text: &str,
) -> ProjectSnapshot {
    snapshot.curation.push(curated(resource, key, Some(lang), text));
    snapshot
}

/// `snapshot` with the curated values of `resource` under `key` replaced by `text`, or removed.
pub fn recurated(mut snapshot: ProjectSnapshot, resource: &str, key: &str, text: Option<&str>) -> ProjectSnapshot {
    snapshot
        .curation
        .retain(|value| value.resource.as_str() != resource || value.key != key);
    match text {
        Some(text) => with_curation(snapshot, resource, key, text),
        None => snapshot,
    }
}

/// The value's UUID is made from its position among the resource's values, so that it is unique.
fn value(position: usize, property: &str, kind: ValueKind) -> Value {
    Value {
        property: PropertyIri(property.to_string()),
        uuid: Some(format!("value-{position}")),
        kind,
    }
}

fn text_value(position: usize, property: &str, text: &str) -> Value {
    let kind = ValueKind::Text { text: text.to_string(), lang: None };
    value(position, property, kind)
}

/// A link has no UUID.
fn link_value(property: &str, target: &str) -> Value {
    Value {
        property: PropertyIri(property.to_string()),
        uuid: None,
        kind: ValueKind::Link(ResourceIri(target.to_string())),
    }
}

pub fn curated(resource: &str, key: &str, lang: Option<&str>, text: &str) -> CuratedValue {
    CuratedValue {
        resource: ResourceIri(resource.to_string()),
        key: key.to_string(),
        lang: lang.map(str::to_string),
        text: text.to_string(),
    }
}

// ── The fixture projects ──────────────────────────────────────────────────────

/// The shortcode in the ARK of a fixture project's resource.
const FIXTURE_SHORTCODE: &str = "0123";

/// The mapping of a `config.kdl`, which lowers.
pub fn lowered(config: &str) -> Remodel {
    load_remodel(config).unwrap_or_else(|error| panic!("the block lowers: {error:?}"))
}

/// A member of `parent` at `seqnum` in a fixture project: a resource of `class` whose label and
/// whose one value under `number_property` are `number`, with a scan named after its IRI and
/// `more` values.
pub fn fixture_member(
    iri: &str,
    (class, number_property): (&str, &str),
    parent: &str,
    seqnum: i64,
    number: &str,
    more: Vec<(&str, ValueKind)>,
) -> Resource {
    let local = iri.rsplit('/').next().unwrap_or(iri);
    let mut served = vec![(number_property, text(number))];
    served.extend(more);
    Resource {
        values: values(served),
        file: Some(still_image(&format!("{local}.jp2"))),
        part_of: vec![ResourceIri(parent.to_string())],
        seqnum: Some(seqnum),
        ..fixture_resource(iri, class, number)
    }
}

/// The mapping of the fixture project `minimal`.
pub static MINIMAL: LazyLock<Remodel> = LazyLock::new(|| {
    lowered(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/minimal/config.kdl"
    )))
});

/// A resource of `class` with a label and nothing else.
pub fn fixture_resource(iri: &str, class: &str, label: &str) -> Resource {
    let local = iri.rsplit('/').next().unwrap_or(iri).replace('-', "=");
    Resource {
        iri: ResourceIri(iri.to_string()),
        ark: DataArk(format!("{DATA_ARK_PREFIX}{FIXTURE_SHORTCODE}/{local}")),
        class: ClassIri(class.to_string()),
        label: label.to_string(),
        values: Vec::new(),
        file: None,
        part_of: Vec::new(),
        seqnum: None,
        annotation: None,
    }
}

/// `values` as a resource's values: each but a link gets a UUID from its position.
pub fn values(values: Vec<(&str, ValueKind)>) -> Vec<Value> {
    let numbered = values.into_iter().enumerate();
    numbered
        .map(|(position, (property, kind))| Value {
            property: PropertyIri(property.to_string()),
            uuid: match kind {
                ValueKind::Link(_) => None,
                _ => Some(format!("value-{position}")),
            },
            kind,
        })
        .collect()
}

pub fn text(text: &str) -> ValueKind {
    ValueKind::Text { text: text.to_string(), lang: None }
}

pub fn link(target: &str) -> ValueKind {
    ValueKind::Link(ResourceIri(target.to_string()))
}

/// The one resource of [`minimal_snapshot`].
pub const THING: &str = "http://rdfh.ch/0001/thing";

/// A snapshot of `minimal`: one Thing, then `more` resources, and `curation`.
pub fn minimal_snapshot(more: Vec<Resource>, curation: Vec<CuratedValue>) -> ProjectSnapshot {
    let thing = fixture_resource(THING, "http://www.knora.org/ontology/0001/minimal#Thing", "A thing");
    let mut resources = vec![thing];
    resources.extend(more);
    ProjectSnapshot {
        shortcode: MINIMAL.shortcode.as_str().to_string(),
        resources,
        list_nodes: Vec::new(),
        curation,
    }
}

/// A Region on `target` with one geometry and one colour, and no remark.
pub fn region_on(iri: &str, target: &str) -> Resource {
    Resource {
        values: values(vec![
            (HAS_GEOMETRY, ValueKind::Geometry(RECTANGLE.to_string())),
            (HAS_COLOR, ValueKind::Color("#ff3333".to_string())),
            (IS_REGION_OF, link(target)),
        ]),
        annotation: Some(Annotation {
            motivation: Motivation::Commenting,
            targets: vec![ResourceIri(target.to_string())],
        }),
        ..fixture_resource(iri, REGION_CLASS, "")
    }
}

/// A link to `targets`, in the order given, with the comment `Verweis`.
pub fn link_to(iri: &str, targets: &[&str]) -> Resource {
    let mut served = vec![(KB_HAS_COMMENT, text("Verweis"))];
    served.extend(targets.iter().map(|target| (HAS_LINK_TO, link(target))));
    let mut targets: Vec<ResourceIri> = targets.iter().map(|target| ResourceIri(target.to_string())).collect();
    targets.sort();
    Resource {
        values: values(served),
        annotation: Some(Annotation { motivation: Motivation::Linking, targets }),
        ..fixture_resource(iri, LINK_OBJ_CLASS, "")
    }
}

/// (id, class) of the store's resources, in the store's order.
pub fn resources(store: &Store) -> Vec<(&str, &str)> {
    let rows = store.resources.iter();
    rows.map(|row| (row.id.as_str(), row.class.as_str())).collect()
}

/// A row's value as one text: a text with its language after `@`, a date as its bounds, its
/// precision, its calendar and its display text, a link as `-> ` and its target, a list value
/// as `node ` and its node.
pub fn shown(row: &PropertyRow) -> String {
    let or_none = |column: &Option<String>| column.clone().unwrap_or_else(|| "-".to_string());
    match row.value_type.as_str() {
        "text" => format!("{}@{}", or_none(&row.value_text), or_none(&row.value_lang)),
        "date" => format!(
            "{}/{} {} {} {}@{}",
            or_none(&row.date_start),
            or_none(&row.date_end),
            or_none(&row.date_precision),
            or_none(&row.date_calendar),
            or_none(&row.date_display),
            or_none(&row.value_lang),
        ),
        "link" => format!("-> {}", or_none(&row.target_id)),
        "integer" => format!("int {}", row.value_int.map_or("-".to_string(), |n| n.to_string())),
        "list_value" => format!("node {}", or_none(&row.target_id)),
        other => panic!("no test reads a row of type {other}"),
    }
}

/// (resource, name, seq, [`shown`]) of the store's values, in the store's order.
pub fn properties(store: &Store) -> Vec<(&str, &str, i64, String)> {
    let rows = store.properties.iter();
    rows.map(|row| (row.resource_id.as_str(), row.name.as_str(), row.seq, shown(row)))
        .collect()
}

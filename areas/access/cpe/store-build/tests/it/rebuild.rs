//! The store is rebuilt from nothing but what the ports serve: `build` over a snapshot that
//! `FakeArchiveProjection` serves, and the curated values the bytes port would hand beside it,
//! gives every row of the store, with no adapter and no file.

use cpe_ports::{ArchiveProjection, FakeArchiveProjection};
use cpe_store_build::sql;
use cpe_store_build::store::Table;

use crate::support::{
    band, built, link_obj, minimal, region, with_annotation, with_curation, with_link, with_resource, Fixture, BOOK,
    HAS_BAND_TYPE_L, PAGE, STRIP_ASSET,
};
use crate::{shortcode, HEADER, MAPPING};

const STRIP: &str = "http://rdfh.ch/0803/strip";
const REGION: &str = "http://rdfh.ch/0803/region";
const LINK: &str = "http://rdfh.ch/0803/link";

/// The minimal Book and Page, a border strip left of the Page, a Region on the Page and a link
/// from the Book to the Region, both of which the editors keep: a row in every table.
fn snapshot() -> Fixture {
    let snapshot = with_resource(minimal(), band(STRIP, "l_1", STRIP_ASSET));
    let snapshot = with_link(snapshot, PAGE, HAS_BAND_TYPE_L, STRIP);
    let snapshot = with_annotation(snapshot, region(REGION, PAGE), "yes");
    let snapshot = with_annotation(snapshot, link_obj(LINK, &[BOOK, REGION]), "yes");
    with_curation(snapshot, LINK, "name", "identischer Holzschnitt")
}

/// The store file's statements after the three list tables, which hold the builder's own
/// vocabularies and no row from the port.
const EXPECTED: &str = r#"INSERT INTO resources (id, class, ark) VALUES
    ('bereitung', 'Book', 'https://ark.dasch.swiss/ark:/72163/1/0803/CDYZPN5zVVKbIcjA1DZxKQ'),
    ('ScanOfThePage', 'Image', NULL),
    ('ScanOfTheStrip', 'Image', NULL),
    ('bereitung-a1r', 'Page', 'https://ark.dasch.swiss/ark:/72163/1/0803/s=hBmajpVteY3fYcRjsJIQ'),
    ('l-1', 'BorderStrip', 'https://ark.dasch.swiss/ark:/72163/1/0803/strip');

INSERT INTO properties
    (resource_id, name, seq, value_type, value_text, value_lang, value_int, value_real, date_start, date_end, date_precision, date_display, date_calendar, target_id)
VALUES
    ('ScanOfThePage', 'authorship', 0, 'text', 'Universitätsbibliothek Basel', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('ScanOfThePage', 'license', 0, 'text', 'CC BY 4.0', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('ScanOfTheStrip', 'authorship', 0, 'text', 'Universitätsbibliothek Basel', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('ScanOfTheStrip', 'license', 0, 'text', 'CC BY 4.0', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('bereitung', 'extent', 0, 'integer', NULL, NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('bereitung', 'image', 0, 'link', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'ScanOfThePage'),
    ('bereitung', 'name', 0, 'text', 'Bereitung zu dem Heiligen Sakrament', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('bereitung', 'office', 0, 'list_value', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'office-amerbach'),
    ('bereitung', 'pub-date', 0, 'date', NULL, 'de', NULL, NULL, '1489', '1489', 'year', '1489', 'julian', NULL),
    ('bereitung', 'sort-key', 0, 'text', 'Bereitung zu dem Heiligen Sakrament', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('bereitung-a1r', 'image', 0, 'link', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'ScanOfThePage'),
    ('bereitung-a1r', 'name', 0, 'text', 'a1r', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('bereitung-a1r', 'page-id', 0, 'text', 'a1r', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('bereitung-a1r', 'seq-number', 0, 'integer', NULL, NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('bereitung-a1r', 'strip-left', 0, 'link', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'l-1'),
    ('l-1', 'image', 0, 'link', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'ScanOfTheStrip'),
    ('l-1', 'name', 0, 'text', 'l_1', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('l-1', 'page-uses', 0, 'integer', NULL, NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('l-1', 'side', 0, 'list_value', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'side-left');

INSERT INTO representations (resource_id, kind, uri, width, height, alt, attribution, seq) VALUES
    ('ScanOfThePage', 'image', 'https://iiif.dasch.swiss/0803/ScanOfThePage.jp2/full/max/0/default.jpg', 1200, 1800, NULL, 'Universitätsbibliothek Basel · CC BY 4.0', 0),
    ('ScanOfTheStrip', 'image', 'https://iiif.dasch.swiss/0803/ScanOfTheStrip.jp2/full/max/0/default.jpg', 1200, 1800, NULL, 'Universitätsbibliothek Basel · CC BY 4.0', 0);

INSERT INTO compound_parts (parent_id, child_id, seq) VALUES
    ('bereitung', 'bereitung-a1r', 1);

INSERT INTO annotations (id, resource_id, shape, x, y, w, h, points, color, remark, remark_lang, ark) VALUES
    ('region', 'bereitung-a1r', 'rectangle', 0.25, 0.25, 0.5, 0.25, NULL, '#ff3333', NULL, NULL, 'https://ark.dasch.swiss/ark:/72163/1/0803/region');

INSERT INTO annotation_links (id, name, name_lang, comment, comment_lang) VALUES
    ('link', 'identischer Holzschnitt', NULL, NULL, NULL);

INSERT INTO annotation_link_targets (link_id, seq, annotation_id, resource_id) VALUES
    ('link', 1, 'region', NULL),
    ('link', 2, NULL, 'bereitung');

"#;

#[test]
fn test_build_snapshot_served_by_the_fake_port_gives_every_row_of_the_store() {
    let Fixture { facts, curation } = snapshot();
    let port = FakeArchiveProjection::default().with_project(facts);
    let facts = port.snapshot(shortcode()).expect("the fake port serves 0803");

    let store = built(&Fixture { facts, curation });

    let lists = [Table::Lists, Table::ListNodes, Table::ListNodeLabels];
    assert_eq!(lists.map(|table| store.row_count(table)), [2, 7, 14]);
    let rest: Vec<Table> = Table::ALL.into_iter().filter(|table| !lists.contains(table)).collect();
    let file = sql::to_sql_tables(&store, &MAPPING, &HEADER, &rest).expect("every other table has a row");
    let (_header, inserts) = file.split_once("\n\n").expect("a blank line ends the header");
    assert_eq!(inserts, EXPECTED);
}

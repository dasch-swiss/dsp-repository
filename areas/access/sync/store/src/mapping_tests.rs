use cpe_ports::contract::violations;
use cpe_ports::{
    ArchiveProjection, Calendar, ClassIri, DateBound, DatePrecision, DateValue, File, LangString, ListNode,
    ListNodeIri, PropertyIri, Resource, ResourceIri, Value, ValueKind,
};

use crate::snapshot_tests::write_0803;
use crate::LiveArchiveProjection;

// Order

#[test]
fn test_mapping_orders_two_none_and_one_serves_none_one_two() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:value:Aq3> <urn:dsp:project:0803> .
        <urn:dsp:value:Aq3> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Aq3> <https://ontology.dasch.swiss/dao#valueHasUUID> "Aq3" <urn:dsp:project:0803> .
        <urn:dsp:value:Aq3> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:project:0803> .
        <urn:dsp:value:Aq3> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "GW 4168" <urn:dsp:project:0803> .
        <urn:dsp:value:Aq3> <https://ontology.dasch.swiss/dao#valueHasOrder> "2"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:value:zT1> <urn:dsp:project:0803> .
        <urn:dsp:value:zT1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:zT1> <https://ontology.dasch.swiss/dao#valueHasUUID> "zT1" <urn:dsp:project:0803> .
        <urn:dsp:value:zT1> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:project:0803> .
        <urn:dsp:value:zT1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "ISTC ib00512000" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:value:Mb7> <urn:dsp:project:0803> .
        <urn:dsp:value:Mb7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Mb7> <https://ontology.dasch.swiss/dao#valueHasUUID> "Mb7" <urn:dsp:project:0803> .
        <urn:dsp:value:Mb7> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:project:0803> .
        <urn:dsp:value:Mb7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Schramm Bd. XXI, S. 27" <urn:dsp:project:0803> .
        <urn:dsp:value:Mb7> <https://ontology.dasch.swiss/dao#valueHasOrder> "1"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    let uuids: Vec<Option<&str>> = snapshot.resources[0].values.iter().map(|value| value.uuid.as_deref()).collect();
    assert_eq!(uuids, vec![Some("zT1"), Some("Mb7"), Some("Aq3")]);
}

#[test]
fn test_mapping_equal_orders_serves_by_uuid_byte_order() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:value:a1> <urn:dsp:project:0803> .
        <urn:dsp:value:a1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:a1> <https://ontology.dasch.swiss/dao#valueHasUUID> "a1" <urn:dsp:project:0803> .
        <urn:dsp:value:a1> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:project:0803> .
        <urn:dsp:value:a1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "GW 4168" <urn:dsp:project:0803> .
        <urn:dsp:value:a1> <https://ontology.dasch.swiss/dao#valueHasOrder> "1"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:value:Z9> <urn:dsp:project:0803> .
        <urn:dsp:value:Z9> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Z9> <https://ontology.dasch.swiss/dao#valueHasUUID> "Z9" <urn:dsp:project:0803> .
        <urn:dsp:value:Z9> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:project:0803> .
        <urn:dsp:value:Z9> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "ISTC ib00512000" <urn:dsp:project:0803> .
        <urn:dsp:value:Z9> <https://ontology.dasch.swiss/dao#valueHasOrder> "1"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:value:_r4> <urn:dsp:project:0803> .
        <urn:dsp:value:_r4> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:_r4> <https://ontology.dasch.swiss/dao#valueHasUUID> "_r4" <urn:dsp:project:0803> .
        <urn:dsp:value:_r4> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasCitation> <urn:dsp:project:0803> .
        <urn:dsp:value:_r4> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Schramm Bd. XXI, S. 27" <urn:dsp:project:0803> .
        <urn:dsp:value:_r4> <https://ontology.dasch.swiss/dao#valueHasOrder> "1"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    let uuids: Vec<Option<&str>> = snapshot.resources[0].values.iter().map(|value| value.uuid.as_deref()).collect();
    assert_eq!(uuids, vec![Some("Z9"), Some("_r4"), Some("a1")]);
}

#[test]
fn test_mapping_two_links_under_one_property_serves_by_target_without_uuid() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.knora.org/ontology/0803/incunabula#hasBandTypeL> <http://rdfh.ch/0803/xP0n> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.knora.org/ontology/0803/incunabula#hasBandTypeL> <http://rdfh.ch/0803/Cq5h> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/xP0n> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/xP0n> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Band> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/xP0n> <http://www.w3.org/2000/01/rdf-schema#label> "Randleiste links" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Cq5h> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Cq5h> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Band> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Cq5h> <http://www.w3.org/2000/01/rdf-schema#label> "Bordüre" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    let page = snapshot
        .resources
        .iter()
        .find(|resource| resource.iri.as_str() == "http://rdfh.ch/0803/pG4w")
        .expect("the page");
    assert_eq!(
        page.values,
        vec![
            Value {
                property: PropertyIri("http://www.knora.org/ontology/0803/incunabula#hasBandTypeL".to_string()),
                uuid: None,
                kind: ValueKind::Link(ResourceIri("http://rdfh.ch/0803/Cq5h".to_string())),
            },
            Value {
                property: PropertyIri("http://www.knora.org/ontology/0803/incunabula#hasBandTypeL".to_string()),
                uuid: None,
                kind: ValueKind::Link(ResourceIri("http://rdfh.ch/0803/xP0n".to_string())),
            },
        ]
    );
}

// Annotations

#[test]
fn test_mapping_region_typed_annotation_omits_it_and_links_and_part_of_to_it() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.knora.org/ontology/0803/incunabula#hasRegion> <http://rdfh.ch/0803/Xr0e> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#isPartOf> <http://rdfh.ch/0803/Xr0e> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/oa#Annotation> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/knora-base#Region> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.w3.org/2000/01/rdf-schema#label> "Randnotiz" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.knora.org/ontology/knora-base#isRegionOf> <http://rdfh.ch/0803/pG4w> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.w3.org/ns/oa#hasTarget> <http://rdfh.ch/0803/pG4w> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.w3.org/ns/oa#motivatedBy> <http://www.w3.org/ns/oa#commenting> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Xr0e> <http://www.knora.org/ontology/knora-base#hasComment> <urn:dsp:value:c0mM> <urn:dsp:project:0803> .
        <urn:dsp:value:c0mM> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:c0mM> <https://ontology.dasch.swiss/dao#valueHasUUID> "c0mM" <urn:dsp:project:0803> .
        <urn:dsp:value:c0mM> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/knora-base#hasComment> <urn:dsp:project:0803> .
        <urn:dsp:value:c0mM> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Holzschnitt" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(snapshot.resources.len(), 1, "{:?}", snapshot.resources);
    let page = &snapshot.resources[0];
    assert_eq!(page.iri.as_str(), "http://rdfh.ch/0803/pG4w");
    assert_eq!(page.values, vec![]);
    assert_eq!(page.part_of, vec![]);
}

#[test]
fn test_mapping_annotation_of_project_region_subclass_omits_it() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/m8Yd> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/m8Yd> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/oa#Annotation> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/m8Yd> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Marginalie> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/m8Yd> <http://www.w3.org/2000/01/rdf-schema#label> "Marginalie" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/m8Yd> <http://www.knora.org/ontology/knora-base#isRegionOf> <http://rdfh.ch/0803/pG4w> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/m8Yd> <http://www.w3.org/ns/oa#hasTarget> <http://rdfh.ch/0803/pG4w> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/m8Yd> <http://www.w3.org/ns/oa#motivatedBy> <http://www.w3.org/ns/oa#highlighting> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    let iris: Vec<&str> = snapshot.resources.iter().map(|resource| resource.iri.as_str()).collect();
    assert_eq!(iris, vec!["http://rdfh.ch/0803/pG4w"]);
}

#[test]
fn test_mapping_link_object_typed_annotation_omits_it_and_its_links() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Lk2b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Lk2b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/oa#Annotation> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Lk2b> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/knora-base#LinkObj> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Lk2b> <http://www.w3.org/2000/01/rdf-schema#label> "Verweis" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Lk2b> <http://www.knora.org/ontology/knora-base#hasLinkTo> <http://rdfh.ch/0803/zR8c> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Lk2b> <http://www.knora.org/ontology/knora-base#hasLinkTo> <http://rdfh.ch/0803/pG4w> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Lk2b> <http://www.w3.org/ns/oa#hasTarget> <http://rdfh.ch/0803/zR8c> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Lk2b> <http://www.w3.org/ns/oa#hasTarget> <http://rdfh.ch/0803/pG4w> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Lk2b> <http://www.w3.org/ns/oa#motivatedBy> <http://www.w3.org/ns/oa#linking> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    let mut iris: Vec<&str> = snapshot.resources.iter().map(|resource| resource.iri.as_str()).collect();
    iris.sort_unstable();
    assert_eq!(iris, vec!["http://rdfh.ch/0803/pG4w", "http://rdfh.ch/0803/zR8c"]);
    assert!(
        snapshot.resources.iter().all(|resource| resource.values.is_empty()),
        "{:?}",
        snapshot.resources
    );
}

// Value kinds

#[test]
fn test_mapping_text_value_serves_rdf_value_verbatim() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "  Zeitglöcklein\n\tdes Lebens  " <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources[0].values,
        vec![Value {
            property: PropertyIri("http://www.knora.org/ontology/0803/incunabula#title".to_string()),
            uuid: Some("mK2x".to_string()),
            kind: ValueKind::Text {
                text: "  Zeitglöcklein\n\tdes Lebens  ".to_string(),
                lang: None
            },
        }]
    );
}

#[test]
fn test_mapping_language_tagged_text_serves_lang() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens"@de <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources[0].values[0].kind,
        ValueKind::Text {
            text: "Zeitglöcklein des Lebens".to_string(),
            lang: Some("de".to_string())
        }
    );
}

#[test]
fn test_mapping_integer_value_serves_integer() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#pages> <urn:dsp:value:n4Pg> <urn:dsp:project:0803> .
        <urn:dsp:value:n4Pg> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:n4Pg> <https://ontology.dasch.swiss/dao#valueHasUUID> "n4Pg" <urn:dsp:project:0803> .
        <urn:dsp:value:n4Pg> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#pages> <urn:dsp:project:0803> .
        <urn:dsp:value:n4Pg> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "-42"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(snapshot.resources[0].values[0].kind, ValueKind::Integer(-42));
}

#[test]
fn test_mapping_boolean_value_serves_boolean() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#isIlluminated> <urn:dsp:value:b0Ol> <urn:dsp:project:0803> .
        <urn:dsp:value:b0Ol> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:b0Ol> <https://ontology.dasch.swiss/dao#valueHasUUID> "b0Ol" <urn:dsp:project:0803> .
        <urn:dsp:value:b0Ol> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#isIlluminated> <urn:dsp:project:0803> .
        <urn:dsp:value:b0Ol> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "true"^^<http://www.w3.org/2001/XMLSchema#boolean> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(snapshot.resources[0].values[0].kind, ValueKind::Boolean(true));
}

#[test]
fn test_mapping_decimal_value_serves_lexical_unchanged() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#weight> <urn:dsp:value:d3Cm> <urn:dsp:project:0803> .
        <urn:dsp:value:d3Cm> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:d3Cm> <https://ontology.dasch.swiss/dao#valueHasUUID> "d3Cm" <urn:dsp:project:0803> .
        <urn:dsp:value:d3Cm> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#weight> <urn:dsp:project:0803> .
        <urn:dsp:value:d3Cm> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "1234.5"^^<http://www.w3.org/2001/XMLSchema#decimal> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(snapshot.resources[0].values[0].kind, ValueKind::Decimal("1234.5".to_string()));
}

#[test]
fn test_mapping_uri_value_serves_lexical_unchanged() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#url> <urn:dsp:value:u7Rl> <urn:dsp:project:0803> .
        <urn:dsp:value:u7Rl> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:u7Rl> <https://ontology.dasch.swiss/dao#valueHasUUID> "u7Rl" <urn:dsp:project:0803> .
        <urn:dsp:value:u7Rl> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#url> <urn:dsp:project:0803> .
        <urn:dsp:value:u7Rl> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "https://www.e-rara.ch/zuz/content/titleinfo/1500?lang=de#top"^^<http://www.w3.org/2001/XMLSchema#anyURI> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources[0].values[0].kind,
        ValueKind::Uri("https://www.e-rara.ch/zuz/content/titleinfo/1500?lang=de#top".to_string())
    );
}

#[test]
fn test_mapping_list_value_serves_node_and_list_nodes() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasColour> <urn:dsp:value:Lq9t> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#valueHasUUID> "Lq9t" <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasColour> <urn:dsp:project:0803> .
        <urn:dsp:value:Lq9t> <https://ontology.dasch.swiss/dao#sourceListNode> <http://rdfh.ch/lists/0803/rot7> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#prefLabel> "Rot"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <http://www.w3.org/2004/02/skos/core#broader> <http://rdfh.ch/lists/0803/Fk3a> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/rot7> <https://ontology.dasch.swiss/dao#listNodePosition> "3"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Farben"@de <urn:dsp:project:0803> .
        <http://rdfh.ch/lists/0803/Fk3a> <http://www.w3.org/2004/02/skos/core#prefLabel> "Colours"@en <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources[0].values[0].kind,
        ValueKind::ListNode(ListNodeIri("http://rdfh.ch/lists/0803/rot7".to_string()))
    );
    let red = snapshot
        .list_nodes
        .iter()
        .find(|node| node.iri.as_str() == "http://rdfh.ch/lists/0803/rot7")
        .expect("the red node");
    assert_eq!(
        red,
        &ListNode {
            iri: ListNodeIri("http://rdfh.ch/lists/0803/rot7".to_string()),
            parent: Some(ListNodeIri("http://rdfh.ch/lists/0803/Fk3a".to_string())),
            position: Some(3),
            labels: vec![LangString { text: "Rot".to_string(), lang: Some("de".to_string()) }],
        }
    );
    let root = snapshot
        .list_nodes
        .iter()
        .find(|node| node.iri.as_str() == "http://rdfh.ch/lists/0803/Fk3a")
        .expect("the root");
    assert_eq!(root.parent, None);
    assert_eq!(root.position, None);
    assert_eq!(root.labels.len(), 2, "{:?}", root.labels);
    assert!(root
        .labels
        .contains(&LangString { text: "Farben".to_string(), lang: Some("de".to_string()) }));
    assert!(root
        .labels
        .contains(&LangString { text: "Colours".to_string(), lang: Some("en".to_string()) }));
    assert_eq!(snapshot.list_nodes.len(), 2);
}

// Files

#[test]
fn test_mapping_still_image_file_serves_asset_width_and_height() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSI> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#StillImageRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#internalFilename> "8F1KTUpKKra-CaFgCYlt8im.jp2" <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#dimX> "3297"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:representation:sdSI> <https://ontology.dasch.swiss/dao#dimY> "5141"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources[0].file,
        Some(File::StillImage {
            asset: "8F1KTUpKKra-CaFgCYlt8im.jp2".to_string(),
            width: 3297,
            height: 5141
        })
    );
}

#[test]
fn test_mapping_audio_file_serves_audio_with_asset() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/au5d> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/au5d> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Recording> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/au5d> <http://www.w3.org/2000/01/rdf-schema#label> "Lesung" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/au5d> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:Wq2a> <urn:dsp:project:0803> .
        <urn:dsp:representation:Wq2a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:Wq2a> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#AudioRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:Wq2a> <https://ontology.dasch.swiss/dao#internalFilename> "3kXz9PmQ1rT-Lesung.mp3" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources[0].file,
        Some(File::Audio { asset: "3kXz9PmQ1rT-Lesung.mp3".to_string() })
    );
}

#[test]
fn test_mapping_moving_image_file_serves_moving_image_with_asset() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/mv8k> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/mv8k> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Film> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/mv8k> <http://www.w3.org/2000/01/rdf-schema#label> "Digitalisierung" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/mv8k> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:Hn6b> <urn:dsp:project:0803> .
        <urn:dsp:representation:Hn6b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:Hn6b> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#MovingImageRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:Hn6b> <https://ontology.dasch.swiss/dao#internalFilename> "Vb7Qe2LdS0w-Film.mp4" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources[0].file,
        Some(File::MovingImage { asset: "Vb7Qe2LdS0w-Film.mp4".to_string() })
    );
}

#[test]
fn test_mapping_document_file_serves_document_with_asset() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/dc1x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/dc1x> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Transcript> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/dc1x> <http://www.w3.org/2000/01/rdf-schema#label> "Transkription" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/dc1x> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:Ty4c> <urn:dsp:project:0803> .
        <urn:dsp:representation:Ty4c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:Ty4c> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#DocumentRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:Ty4c> <https://ontology.dasch.swiss/dao#internalFilename> "Ga0Jw5YpN3s-Transkription.pdf" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources[0].file,
        Some(File::Document { asset: "Ga0Jw5YpN3s-Transkription.pdf".to_string() })
    );
}

#[test]
fn test_mapping_file_of_other_kind_serves_resource_without_file() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/ar3z> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/ar3z> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Bundle> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/ar3z> <http://www.w3.org/2000/01/rdf-schema#label> "Sammlung" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/ar3z> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:Pz8d> <urn:dsp:project:0803> .
        <urn:dsp:representation:Pz8d> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
        <urn:dsp:representation:Pz8d> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#ArchiveRepresentation> <urn:dsp:project:0803> .
        <urn:dsp:representation:Pz8d> <https://ontology.dasch.swiss/dao#internalFilename> "Kc2Rv8XwM1q-Sammlung.zip" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(snapshot.resources.len(), 1);
    assert_eq!(snapshot.resources[0].file, None);
}

// Dates

#[test]
fn test_mapping_julian_date_serves_calendar_jdns_and_precisions() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:EDo-> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#valueHasUUID> "EDo-" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "JULIAN:1492 CE"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateCalendar> "JULIAN" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartJDN> "2266011"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndJDN> "2266376"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateStartPrecision> "YEAR" <urn:dsp:project:0803> .
        <urn:dsp:value:EDo-> <https://ontology.dasch.swiss/dao#dateEndPrecision> "YEAR" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources[0].values,
        vec![Value {
            property: PropertyIri("http://www.knora.org/ontology/0803/incunabula#hasPubdate".to_string()),
            uuid: Some("EDo-".to_string()),
            kind: ValueKind::Date(DateValue {
                calendar: Calendar::Julian,
                start: DateBound { jdn: 2_266_011, precision: DatePrecision::Year },
                end: DateBound { jdn: 2_266_376, precision: DatePrecision::Year },
            }),
        }]
    );
}

#[test]
fn test_mapping_gregorian_date_with_day_start_and_month_end_serves_precisions() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:g9Re> <urn:dsp:project:0803> .
        <urn:dsp:value:g9Re> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:g9Re> <https://ontology.dasch.swiss/dao#valueHasUUID> "g9Re" <urn:dsp:project:0803> .
        <urn:dsp:value:g9Re> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
        <urn:dsp:value:g9Re> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "GREGORIAN:1700-03-14 CE:1700-04 CE"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
        <urn:dsp:value:g9Re> <https://ontology.dasch.swiss/dao#dateCalendar> "GREGORIAN" <urn:dsp:project:0803> .
        <urn:dsp:value:g9Re> <https://ontology.dasch.swiss/dao#dateStartJDN> "2342045"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:g9Re> <https://ontology.dasch.swiss/dao#dateEndJDN> "2342093"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        <urn:dsp:value:g9Re> <https://ontology.dasch.swiss/dao#dateStartPrecision> "DAY" <urn:dsp:project:0803> .
        <urn:dsp:value:g9Re> <https://ontology.dasch.swiss/dao#dateEndPrecision> "MONTH" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources[0].values[0].kind,
        ValueKind::Date(DateValue {
            calendar: Calendar::Gregorian,
            start: DateBound { jdn: 2_342_045, precision: DatePrecision::Day },
            end: DateBound { jdn: 2_342_093, precision: DatePrecision::Month },
        })
    );
}

// Membership

#[test]
fn test_mapping_page_part_of_and_seqnum_serves_both() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#isPartOf> <http://rdfh.ch/0803/zR8c> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#seqnum> "7"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    let page = snapshot
        .resources
        .iter()
        .find(|resource| resource.iri.as_str() == "http://rdfh.ch/0803/pG4w")
        .expect("the page");
    assert_eq!(page.part_of, vec![ResourceIri("http://rdfh.ch/0803/zR8c".to_string())]);
    assert_eq!(page.seqnum, Some(7));
    assert!(page.values.is_empty(), "{:?}", page.values);
}

#[test]
fn test_mapping_resource_with_two_parents_serves_both() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Ba2k> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Ba2k> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Ba2k> <http://www.w3.org/2000/01/rdf-schema#label> "Narrenschiff" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#isPartOf> <http://rdfh.ch/0803/zR8c> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#isPartOf> <http://rdfh.ch/0803/Ba2k> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    let page = snapshot
        .resources
        .iter()
        .find(|resource| resource.iri.as_str() == "http://rdfh.ch/0803/pG4w")
        .expect("the page");
    let mut parents = page.part_of.clone();
    parents.sort_unstable();
    assert_eq!(
        parents,
        vec![
            ResourceIri("http://rdfh.ch/0803/Ba2k".to_string()),
            ResourceIri("http://rdfh.ch/0803/zR8c".to_string())
        ]
    );
}

#[test]
fn test_mapping_part_of_annotation_omits_it_and_keeps_seqnum() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/gOnE> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/oa#Annotation> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/gOnE> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/knora-base#Region> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/gOnE> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/gOnE> <http://www.w3.org/2000/01/rdf-schema#label> "Randnotiz" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#isPartOf> <http://rdfh.ch/0803/gOnE> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#seqnum> "12"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(snapshot.resources[0].part_of, vec![]);
    assert_eq!(snapshot.resources[0].seqnum, Some(12));
}

#[test]
fn test_mapping_resource_without_values_serves_none() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        snapshot.resources,
        vec![Resource {
            iri: ResourceIri("http://rdfh.ch/0803/zR8c".to_string()),
            class: ClassIri("http://www.knora.org/ontology/0803/incunabula#Book".to_string()),
            label: "Zeitglöcklein".to_string(),
            values: vec![],
            file: None,
            part_of: vec![],
            seqnum: None,
        }]
    );
    assert_eq!(snapshot.list_nodes, vec![]);
}

// Properties and omissions

#[test]
fn test_mapping_link_serves_predicate_full_iri() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.w3.org/2000/01/rdf-schema#label> "a1r" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/pG4w> <http://www.knora.org/ontology/knora-base#hasLinkTo> <http://rdfh.ch/0803/Cq5h> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Cq5h> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Cq5h> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Band> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/Cq5h> <http://www.w3.org/2000/01/rdf-schema#label> "Bordüre" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    let page = snapshot
        .resources
        .iter()
        .find(|resource| resource.iri.as_str() == "http://rdfh.ch/0803/pG4w")
        .expect("the page");
    assert_eq!(
        page.values,
        vec![Value {
            property: PropertyIri("http://www.knora.org/ontology/knora-base#hasLinkTo".to_string()),
            uuid: None,
            kind: ValueKind::Link(ResourceIri("http://rdfh.ch/0803/Cq5h".to_string())),
        }]
    );
}

#[test]
fn test_mapping_value_of_unlisted_datatype_omits_it_and_serves_snapshot() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r##"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/knora-base#hasColor> <urn:dsp:value:MB-C> <urn:dsp:project:0803> .
        <urn:dsp:value:MB-C> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:MB-C> <https://ontology.dasch.swiss/dao#valueHasUUID> "MB-C" <urn:dsp:project:0803> .
        <urn:dsp:value:MB-C> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/knora-base#hasColor> <urn:dsp:project:0803> .
        <urn:dsp:value:MB-C> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "#ff3333"^^<https://ontology.dasch.swiss/dao#color> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        "##,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("a valid file is served");

    let found = violations("0803", &snapshot);
    assert!(found.is_empty(), "{found:?}");
    let uuids: Vec<Option<&str>> = snapshot.resources[0].values.iter().map(|value| value.uuid.as_deref()).collect();
    assert_eq!(uuids, vec![Some("mK2x")]);
}

#[test]
fn test_mapping_edge_to_untyped_value_node_drops_it_and_serves_snapshot() {
    let dir = tempfile::tempdir().expect("create a temp dir");
    write_0803(
        &dir,
        r#"
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Book> <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.w3.org/2000/01/rdf-schema#label> "Zeitglöcklein" <urn:dsp:project:0803> .
        <http://rdfh.ch/0803/zR8c> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:value:mK2x> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#valueHasUUID> "mK2x" <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#title> <urn:dsp:project:0803> .
        <urn:dsp:value:mK2x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Zeitglöcklein des Lebens" <urn:dsp:project:0803> .
        "#,
    );

    let snapshot = LiveArchiveProjection::new(dir.path())
        .snapshot("0803")
        .expect("the one accepted violation is not refused");

    // The node lacks `rdf:type dao:Value`: the crate doc's one documented leniency.
    assert_eq!(snapshot.resources[0].values, vec![]);
}

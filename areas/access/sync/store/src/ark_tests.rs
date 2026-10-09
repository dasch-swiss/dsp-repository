use cpe_ports::contract::violations;
use cpe_ports::{ClassIri, ProjectSnapshot, Resource, ResourceIri};

use crate::ark::{check_digit, data_ark, ArkError};

// dsp-api's own vectors: `StringFormatterSpec.scala` and `Base64UrlCheckDigitZSpec.scala`.
#[test]
fn test_ark_check_digit_dsp_api_id_returns_n() {
    assert_eq!(check_digit("cmfk1DMHRBiR4-_6HXpEFA"), Ok('n'));
}

#[test]
fn test_ark_check_digit_mutated_ids_do_not_return_n() {
    for mutated in [
        "cmfk1DMHRBiR4-6HXpEFA",  // a character missing
        "cmfk1DMHRBir4-_6HXpEFA", // a character changed
        "cmfk1DMHRBiR4_-6HXpEFA", // two characters swapped
    ] {
        assert_ne!(check_digit(mutated), Ok('n'), "{mutated}");
    }
}

#[test]
fn test_ark_data_ark_dsp_api_iri_returns_dsp_api_path() {
    let ark = data_ark("http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA").expect("a resource IRI");
    assert_eq!(
        ark.as_str(),
        "https://ark.dasch.swiss/ark:/72163/1/0001/cmfk1DMHRBiR4=_6HXpEFAn"
    );
}

#[test]
fn test_ark_data_ark_eleven_production_iris_return_production_arks() {
    for (local, ark) in [
        // Book narrenschiff-dt
        ("cpQ3-JfqVZOkd7hUQ26kTg", "cpQ3=JfqVZOkd7hUQ26kTg7"),
        // Pages c1v, b3v, b1r of narrenschiff-dt
        ("twPlI6hTUzCbBJr1IYKQaw", "twPlI6hTUzCbBJr1IYKQawd"),
        ("wk5fLgvtUXupW8-ZfJRn-Q", "wk5fLgvtUXupW8=ZfJRn=Q1"),
        ("Vy27CJ6kVn6JjxlPQrwutQ", "Vy27CJ6kVn6JjxlPQrwutQ5"),
        // Regions on c1v, then b3v
        ("bsX4cDKkW3GMC8_Ms2tOpw", "bsX4cDKkW3GMC8_Ms2tOpw5"),
        ("wO-l4gfTVsGnKtusKEomhQ", "wO=l4gfTVsGnKtusKEomhQn"),
        ("EDqDFUAsXtSuBreIXyuCiQ", "EDqDFUAsXtSuBreIXyuCiQE"),
        ("XTtkp_hEUDWO828r2pb2PA", "XTtkp_hEUDWO828r2pb2PAm"),
        // Border strips l_1a, r_1
        ("NVsTou5UUrW059nmfNS0qA", "NVsTou5UUrW059nmfNS0qAr"),
        ("XtSmDsS4V0C2-FjlF4Zy_w", "XtSmDsS4V0C2=FjlF4Zy_wT"),
        // Book bereitung
        ("CDYZPN5zVVKbIcjA1DZxKQ", "CDYZPN5zVVKbIcjA1DZxKQO"),
    ] {
        let computed = data_ark(&format!("http://rdfh.ch/0803/{local}")).expect("a resource IRI");
        assert_eq!(computed.as_str(), format!("https://ark.dasch.swiss/ark:/72163/1/0803/{ark}"));
    }
}

#[test]
fn test_ark_data_ark_dash_check_digit_is_escaped() {
    assert_eq!(check_digit("bmfk1DMHRBiR4-_6HXpEFA"), Ok('-'));
    let ark = data_ark("http://rdfh.ch/0001/bmfk1DMHRBiR4-_6HXpEFA").expect("a resource IRI");
    assert_eq!(
        ark.as_str(),
        "https://ark.dasch.swiss/ark:/72163/1/0001/bmfk1DMHRBiR4=_6HXpEFA="
    );
}

#[test]
fn test_ark_data_ark_non_resource_iris_return_not_resource_iri() {
    for iri in [
        "https://rdfh.ch/0803/cpQ3-JfqVZOkd7hUQ26kTg",
        "http://example.org/0803/cpQ3-JfqVZOkd7hUQ26kTg",
        "http://rdfh.ch/users/x",
        "http://rdfh.ch/lists/0803/x",
        "http://rdfh.ch/0803/a/b",
        "http://rdfh.ch/080g/x",
        "http://rdfh.ch/0803",
    ] {
        assert_eq!(data_ark(iri), Err(ArkError::NotResourceIri), "{iri}");
    }
}

#[test]
fn test_ark_data_ark_lowercase_shortcode_returns_uppercase_shortcode() {
    let ark = data_ark("http://rdfh.ch/081c/a").expect("a resource IRI");
    assert_eq!(ark.as_str(), "https://ark.dasch.swiss/ark:/72163/1/081C/aM");
}

#[test]
fn test_ark_data_ark_bad_ids_return_their_kind() {
    assert_eq!(data_ark("http://rdfh.ch/0803/ab.c"), Err(ArkError::InvalidChar('.')));
    assert_eq!(data_ark("http://rdfh.ch/0803/abä"), Err(ArkError::InvalidChar('ä')));
    assert_eq!(data_ark("http://rdfh.ch/0803/"), Err(ArkError::Empty));
    assert_eq!(data_ark("http://rdfh.ch/0803/AAAAAA"), Err(ArkError::ZeroSum));
}

#[test]
fn test_ark_data_ark_fake_book_iri_returns_the_fakes_ark() {
    let ark = data_ark("http://rdfh.ch/0803/zz-book").expect("a resource IRI");
    assert_eq!(ark.as_str(), "https://ark.dasch.swiss/ark:/72163/1/0803/zz=booko");
}

#[test]
fn test_ark_data_ark_computed_arks_pass_the_contract() {
    let resources = [
        "http://rdfh.ch/0803/cpQ3-JfqVZOkd7hUQ26kTg",
        "http://rdfh.ch/0803/XtSmDsS4V0C2-FjlF4Zy_w",
        "http://rdfh.ch/0001/bmfk1DMHRBiR4-_6HXpEFA",
        "http://rdfh.ch/081c/a",
    ]
    .map(|iri| Resource {
        iri: ResourceIri(iri.to_string()),
        ark: data_ark(iri).expect("a resource IRI"),
        class: ClassIri("http://www.knora.org/ontology/0803/incunabula#Book".to_string()),
        label: "Buch".to_string(),
        values: vec![],
        file: None,
        part_of: vec![],
        seqnum: None,
        annotation: None,
    });
    for resource in resources {
        let iri = resource.iri.clone();
        let snapshot = ProjectSnapshot {
            shortcode: "0803".to_string(),
            resources: vec![resource],
            list_nodes: vec![],
            curation: vec![],
        };
        let found = violations("0803", &snapshot);
        assert_eq!(found, vec![], "{iri:?}");
    }
}

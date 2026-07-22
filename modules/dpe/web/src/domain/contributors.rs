// Sync, in-memory contributor resolver: hashmap lookups against
// `dpe_core::contributors`, called directly from the project page and the SSE
// fragment handler.

pub use dpe_core::contributors::ResolvedContributor;
use dpe_core::project::Attribution;

pub fn get_contributors(attributions: Vec<Attribution>) -> Vec<ResolvedContributor> {
    use dpe_core::contributors::{is_organization_id, load_organization, load_person};

    let mut result = Vec::with_capacity(attributions.len());
    for attr in attributions {
        let roles = (!attr.contributor_type.is_empty()).then(|| attr.contributor_type.join(", "));
        let id = &attr.contributor;
        if is_organization_id(id) {
            match load_organization(id) {
                Some(org) => result.push(ResolvedContributor::Organization { org, roles }),
                None => result.push(ResolvedContributor::Unknown { id: id.clone(), roles }),
            }
        } else {
            match load_person(id) {
                Some(person) => {
                    let mut affiliations = Vec::with_capacity(person.affiliations.len());
                    for aff_id in &person.affiliations {
                        if let Some(org) = load_organization(aff_id) {
                            affiliations.push(org);
                        }
                    }
                    result.push(ResolvedContributor::Person { person, affiliations, roles });
                }
                None => result.push(ResolvedContributor::Unknown { id: id.clone(), roles }),
            }
        }
    }
    result
}

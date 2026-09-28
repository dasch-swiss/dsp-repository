// Sync, in-memory contributor resolver: corpus lookups against the person and
// organization caches, called directly from the project page and the SSE
// fragment handler.

pub use dpe_core::contributors::ResolvedContributor;
use dpe_core::Corpus;
use shared_metadata::project::Attribution;

pub fn get_contributors(attributions: Vec<Attribution>, corpus: &'static Corpus) -> Vec<ResolvedContributor> {
    use shared_metadata::is_organization_id;

    let mut result = Vec::with_capacity(attributions.len());
    for attr in attributions {
        let roles = (!attr.contributor_type.is_empty()).then(|| attr.contributor_type.join(", "));
        let id = &attr.contributor;
        if is_organization_id(id) {
            match corpus.load_organization(id) {
                Some(org) => result.push(ResolvedContributor::Organization { org, roles }),
                None => result.push(ResolvedContributor::Unknown { id: id.clone(), roles }),
            }
        } else {
            match corpus.load_person(id) {
                Some(person) => {
                    let mut affiliations = Vec::with_capacity(person.affiliations.len());
                    for aff_id in &person.affiliations {
                        if let Some(org) = corpus.load_organization(aff_id) {
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

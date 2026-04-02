pub use dpe_core::contributors::ResolvedContributor;
use dpe_core::project::Attribution;

#[leptos::server]
pub async fn get_contributors(
    attributions: Vec<Attribution>,
) -> Result<Vec<ResolvedContributor>, leptos::server_fn::error::ServerFnError> {
    use dpe_core::contributors::{load_organization, load_person};

    let mut result = Vec::with_capacity(attributions.len());
    for attr in attributions {
        let roles = (!attr.contributor_type.is_empty()).then(|| attr.contributor_type.join(", "));
        let id = &attr.contributor;
        if id.starts_with("organization-") || id.contains("-organization-") {
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
    Ok(result)
}

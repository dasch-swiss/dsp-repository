// Sync, in-memory contributor resolver.
//
// Previously a `#[server]` async fn used by `Resource::new(..)` in
// `ProjectLoader`; the resulting reactive node panicked under streaming SSR
// in `<Suspense>::dry_resolve`. The body was always synchronous (just hashmap
// lookups against `dpe_core::contributors`), so we expose it as a plain
// `pub fn` gated on non-wasm and call it directly from the page component
// and the SSE fragment handler.

pub use dpe_core::contributors::ResolvedContributor;
#[cfg(not(target_arch = "wasm32"))]
use dpe_core::project::Attribution;

#[cfg(not(target_arch = "wasm32"))]
pub fn get_contributors(attributions: Vec<Attribution>) -> Vec<ResolvedContributor> {
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
    result
}

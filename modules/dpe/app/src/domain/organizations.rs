use dpe_core::Organization;
use leptos::prelude::*;

#[server]
pub async fn get_organization(id: String) -> Result<Option<Organization>, ServerFnError> {
    Ok(dpe_core::load_organization(&id))
}

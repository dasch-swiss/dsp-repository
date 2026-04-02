use dpe_core::Person;
use leptos::prelude::*;

#[server]
pub async fn get_person(id: String) -> Result<Option<Person>, ServerFnError> {
    Ok(dpe_core::load_person(&id))
}

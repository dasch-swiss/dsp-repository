use leptos::prelude::*;

use super::person::Person;

#[server]
pub async fn get_person(id: String) -> Result<Option<Person>, ServerFnError> {
    use std::fs;

    let persons_dir = "server/data/persons";

    // Read all entries in the persons directory
    let entries = fs::read_dir(persons_dir)
        .map_err(|e| ServerFnError::new(format!("Failed to read persons directory: {}", e)))?;

    // Find the file that matches the id
    for entry in entries {
        let entry = entry
            .map_err(|e| ServerFnError::new(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    // Check if the filename starts with the id and ends with .json
                    if filename_str.starts_with(&id) && filename_str.ends_with(".json") {
                        // Read and parse the JSON file
                        let json_data = fs::read_to_string(&path).map_err(|e| {
                            ServerFnError::new(format!("Failed to read file: {}", e))
                        })?;

                        let person: Person = serde_json::from_str(&json_data).map_err(|e| {
                            ServerFnError::new(format!("Failed to parse JSON: {}", e))
                        })?;

                        return Ok(Some(person));
                    }
                }
            }
        }
    }

    Ok(None)
}

use std::convert::identity;
use std::fs;
use std::path::PathBuf;

use crate::domain::project::Project;
use glob::glob;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct ProjectRepository {}

impl ProjectRepository {
    pub fn list(&self) -> Vec<PathBuf> {
        glob("./data/future/projects/*.json")
            .unwrap()
            .filter_map(Result::ok)
            .collect::<Vec<_>>()
    }

    pub fn find(&self, shortcode: String) -> Option<PathBuf> {
        // data/future/projects/0101_religious-speech.json
        let regex = Regex::new(r"^data/future/projects/(?<shortcode>[A-Z0-9]{4})_.*.json").unwrap();

        self.list().into_iter().find_map(|path| {
            let path_str = identity::<&str>(path.to_str().unwrap());

            if regex.captures(path_str).iter().any(|c| shortcode == c["shortcode"]) {
                Some(path)
            } else {
                None
            }
        })
    }

    pub fn read_contents(&self, shortcode: String) -> Option<String> {
        self.find(shortcode).map(|path| fs::read_to_string(path).unwrap())
    }

    pub fn read(&self, shortcode: String) -> Option<Project> {
        self.read_contents(shortcode).map(|json| serde_json::from_str(&json).unwrap())
    }
}

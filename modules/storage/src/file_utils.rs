use std::fs;
use std::path::{Path, PathBuf};

pub fn load_json_file_paths(data_path: &Path) -> Vec<PathBuf> {
    let mut json_dir = PathBuf::from(data_path);
    json_dir.push("json");
    find_files_by_extension_in_dir(json_dir.as_path(), "json")
}

fn find_files_by_extension_in_dir(dir: &Path, file_extension: &str) -> Vec<PathBuf> {
    if !dir.is_dir() {
        panic!("Directory does not exist.");
    };
    fs::read_dir(dir)
        .expect("Unable to read directory.")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some(file_extension) {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<PathBuf>>()
}

//! The data-free check of a project's shipped inputs, for [`crate::validate`]: what can be checked
//! of a project before any of its data is deposited. The inputs are a plain directory laid out as
//! the VRE bridge ships them, `<shortcode>/config.kdl` and `<shortcode>/narrative/<name>`; it has
//! no Ready token and no generation folder, so it is read here and not through a port.

use std::path::Path;

use crate::InputsCheck;

/// Check what `inputs_dir` ships for `shortcode`: the config parses, its remodel block loads and
/// declares `shortcode`, its site block passes the checks that need no data and no assets
/// ([`cpe_core::validate::check_site`]), and each narrative file it names has its base file or
/// its default-language variant. Page and component validation needs the project's data and is
/// not part of it. `shortcode` has passed `cpe_ports::is_shortcode`, so it is one path segment.
pub(crate) fn check(inputs_dir: &Path, shortcode: &str) -> InputsCheck {
    let dir = inputs_dir.join(shortcode);
    match std::fs::symlink_metadata(&dir) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return InputsCheck::NoneShipped,
        Err(error) => return failed(format!("{shortcode}/: {error}")),
        Ok(metadata) if !metadata.is_dir() => return failed(format!("{shortcode}/ is not a directory")),
        Ok(_) => {}
    }
    let config_file = format!("{shortcode}/config.kdl");
    let config = match regular_file(&dir.join("config.kdl")) {
        Ok(Some(bytes)) => match String::from_utf8(bytes) {
            Ok(config) => config,
            Err(_) => return failed(format!("{config_file} is not valid UTF-8")),
        },
        Ok(None) => return failed(format!("{config_file}: the inputs ship no config.kdl as a regular file")),
        Err(error) => return failed(format!("{config_file}: {error}")),
    };
    if let Err(report) = crate::activation::remodel_of(&config, &config_file, shortcode) {
        return failed(report);
    }
    // Its warnings are left to the full load, which reports them with the rest.
    match cpe_core::validate::check_site(&config) {
        Ok(_warnings) => {}
        Err(cpe_core::validate::ConfigLoadError::Parse(error)) => return failed(format!("{config_file}: {error}")),
        Err(cpe_core::validate::ConfigLoadError::Validation(report)) => {
            return failed(report.render(&config, &config_file));
        }
    }
    let named = match cpe_core::validate::named_narratives(&config) {
        Ok(named) => named,
        Err(error) => return failed(error.to_string()),
    };
    let narrative = dir.join("narrative");
    // A symbolic link is followed neither for `narrative/` nor for a file in it. A missing
    // `narrative/` holds nothing, and each named file reports itself.
    if std::fs::symlink_metadata(&narrative).is_ok_and(|metadata| !metadata.is_dir()) {
        return failed(format!("{shortcode}/narrative is not a directory"));
    }
    let mut faults = Vec::new();
    // Only a name activation asks the deposited files for is looked up: one that passes the ports'
    // `is_file_name` and has no `/`. That keeps every lookup inside `narrative/`.
    let held = |name: &str| {
        cpe_ports::is_file_name(name)
            && !name.contains('/')
            && matches!(file_kind(&narrative.join(name)), Ok(Some(true)))
    };
    for file in named.unresolved(held) {
        faults.push(if file.contains(['/', '\\']) {
            format!("{config_file} names narrative file '{file}', which is not a bare file name")
        } else if !cpe_ports::is_file_name(file) {
            format!("{config_file} names narrative file '{file}', which is not a file name the deposited files may hold")
        } else {
            format!("{config_file} names narrative file '{file}', but {shortcode}/narrative/ holds neither it nor its variant in the default language")
        });
    }
    if faults.is_empty() {
        InputsCheck::Passed
    } else {
        failed(faults.join("\n"))
    }
}

fn failed(report: String) -> InputsCheck {
    InputsCheck::Failed { report }
}

/// The bytes of `path` when it is a regular file, `None` when nothing or something else is there.
fn regular_file(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match file_kind(path)? {
        Some(true) => std::fs::read(path).map(Some),
        _ => Ok(None),
    }
}

/// Whether `path` is a regular file, `None` when nothing is there. A symbolic link is not
/// followed, as discovery follows none.
fn file_kind(path: &Path) -> std::io::Result<Option<bool>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata.is_file())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

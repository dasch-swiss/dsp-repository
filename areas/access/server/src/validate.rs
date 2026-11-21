//! The `validate` subcommand: DPE's data, then CPE's projects, strictly.

use std::io::Write;
use std::path::Path;

use cpe_server::{CpeConfig, ProjectOutcome, Validation};

/// Check DPE's data under `dpe_data_dir`, then every CPE project in `cpe_config`'s projects folder
/// from the two ports, and return whether both pass. DPE prints to stdout and CPE to `out`, every
/// project's outcome before the result. A failed CPE project fails the run; an unknown one is a
/// warning, since the check runs where a project need not be released.
pub(crate) fn validate(
    dpe_data_dir: &Path,
    cpe_config: &CpeConfig,
    projection: &dyn cpe_ports::ArchiveProjection,
    files: &dyn cpe_ports::DepositedFiles,
    out: &mut impl Write,
) -> bool {
    let dpe_passes = dpe_server::validate(dpe_data_dir);
    let cpe = cpe_server::validate(cpe_config, projection, files);
    let printed = out.write_all(cpe_report(&cpe, &cpe_config.projects_dir).as_bytes()).is_ok();
    dpe_passes && cpe.passes() && printed
}

/// What CPE's check found, one paragraph per project folder and a closing line.
fn cpe_report(validation: &Validation, projects_dir: &Path) -> String {
    let mut text = String::new();
    if let Some(error) = &validation.error {
        text.push_str(&format!("\nCPE: not checked: {error}\nCPE: failed.\n"));
        return text;
    }
    text.push_str(&format!(
        "\nCPE: {} project folder(s) under {}\n",
        validation.projects.len(),
        projects_dir.display()
    ));
    let mut failed = 0;
    for project in &validation.projects {
        let name = match &project.shortcode {
            Some(shortcode) => format!("{} ({})", printable(&project.folder), printable(shortcode)),
            None => printable(&project.folder),
        };
        match &project.outcome {
            ProjectOutcome::Served { warnings } if warnings.is_empty() => {
                text.push_str(&format!("  {name}: served\n"));
            }
            ProjectOutcome::Served { warnings } => {
                text.push_str(&format!("  {name}: served, {} warning(s)\n", warnings.len()));
                for warning in warnings {
                    text.push_str(&indented(&format!("warning: {warning}")));
                }
            }
            ProjectOutcome::Unknown { reason } => {
                text.push_str(&format!("  {name}: warning: unknown here, so not checked\n"));
                text.push_str(&indented(reason));
            }
            ProjectOutcome::Failed { report } => {
                failed += 1;
                text.push_str(&format!("  {name}: FAILED\n"));
                text.push_str(&indented(report));
            }
        }
    }
    if failed == 0 {
        text.push_str("CPE: no project failed.\n");
    } else {
        text.push_str(&format!("CPE: {failed} project(s) failed.\n"));
    }
    text
}

/// `text`, each line indented below a project's line.
fn indented(text: &str) -> String {
    text.lines().map(|line| format!("    {}\n", printable(line))).collect()
}

/// `text` with every control character escaped. A folder name or a config's text can hold a line
/// break or an escape sequence, and a CI log reads a line that starts with `::` as a command.
fn printable(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() {
                c.escape_default().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use cpe_ports::{FakeArchiveProjection, FakeDepositedFiles};
    use cpe_server::CpeConfig;

    use crate::serve::tests::{cpe_config, projects_folder, things_snapshot, THINGS};

    /// The Things config under `shortcode`.
    pub(crate) fn things_of(shortcode: &str) -> String {
        THINGS.replace("shortcode \"0001\"", &format!("shortcode \"{shortcode}\""))
    }

    /// The Things config under `shortcode` with a second page whose id is taken: CPE refuses it.
    pub(crate) fn broken_things(shortcode: &str) -> String {
        format!("{}\npage id=\"home\" route=\"/again\"\n", things_of(shortcode))
    }

    /// DPE data with no file in it, which DPE's check passes.
    fn valid_dpe_data() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("a temporary directory");
        std::fs::create_dir(root.path().join("projects")).expect("the projects folder is created");
        root
    }

    /// The projection of the Things projects `served`, and their configs with those of `broken`.
    fn ports(served: &[&str], broken: &[&str]) -> (FakeArchiveProjection, FakeDepositedFiles) {
        let mut projection = FakeArchiveProjection::default();
        let mut files = FakeDepositedFiles::default();
        for &shortcode in served {
            projection = projection.with_project(things_snapshot(shortcode));
            files = files.with_file(shortcode, "config.kdl", things_of(shortcode));
        }
        for &shortcode in broken {
            projection = projection.with_project(things_snapshot(shortcode));
            files = files.with_file(shortcode, "config.kdl", broken_things(shortcode));
        }
        (projection, files)
    }

    /// The run over `dpe` and CPE's `config`, and what it printed for CPE.
    fn run(
        dpe: &std::path::Path,
        config: &CpeConfig,
        (projection, files): &(FakeArchiveProjection, FakeDepositedFiles),
    ) -> (bool, String) {
        let mut out = Vec::new();
        let passes = super::validate(dpe, config, projection, files, &mut out);
        (passes, String::from_utf8(out).expect("the report is UTF-8"))
    }

    #[test]
    fn two_failed_projects_fail_the_run_and_both_reports_are_printed() {
        let dpe = valid_dpe_data();
        let root = projects_folder(&[("first", "0001"), ("second", "0002"), ("things", "0003")]);
        let (passes, printed) = run(dpe.path(), &cpe_config(root.path(), ""), &ports(&["0003"], &["0001", "0002"]));
        assert!(!passes, "{printed}");
        for (name, report) in [
            ("first (0001)", "0001/config.kdl:"),
            ("second (0002)", "0002/config.kdl:"),
        ] {
            assert!(printed.contains(&format!("{name}: FAILED")), "{printed}");
            assert!(printed.contains(report), "{printed}");
        }
        assert!(printed.contains("things (0003): served"), "{printed}");
        assert!(printed.contains("CPE: 2 project(s) failed."), "{printed}");
    }

    #[test]
    fn the_run_passes_when_the_only_projects_not_served_are_unknown_and_warns_of_each() {
        let dpe = valid_dpe_data();
        let root = projects_folder(&[("things", "0001"), ("gone", "0002"), ("later", "0003")]);
        let (passes, printed) = run(dpe.path(), &cpe_config(root.path(), ""), &ports(&["0001"], &[]));
        assert!(passes, "{printed}");
        for name in ["gone (0002)", "later (0003)"] {
            assert!(printed.contains(&format!("{name}: warning: unknown here")), "{printed}");
        }
        assert!(printed.contains("CPE: no project failed."), "{printed}");
    }

    #[test]
    fn a_dpe_failure_fails_the_run_when_cpe_passes_and_cpe_is_still_checked() {
        let no_dpe_data = tempfile::tempdir().expect("a temporary directory");
        let root = projects_folder(&[("things", "0001")]);
        let (passes, printed) = run(no_dpe_data.path(), &cpe_config(root.path(), ""), &ports(&["0001"], &[]));
        assert!(!passes, "DPE's data has no projects folder: {printed}");
        assert!(printed.contains("things (0001): served"), "{printed}");
    }

    #[test]
    fn a_folder_name_cannot_start_a_line_of_its_own_in_the_report() {
        let dpe = valid_dpe_data();
        let root = projects_folder(&[("things", "0001")]);
        let folder = root.path().join("x\n::error::forged");
        std::fs::create_dir(&folder).expect("the folder is created");
        std::fs::write(folder.join("SHORTCODE"), "0002\n").expect("SHORTCODE is written");
        let (passes, printed) = run(dpe.path(), &cpe_config(root.path(), ""), &ports(&["0001"], &[]));
        assert!(!passes, "the folder's name is not a project id: {printed}");
        assert!(printed.contains("x\\n::error::forged: FAILED"), "{printed}");
        assert!(!printed.lines().any(|line| line.trim_start().starts_with("::")), "{printed}");
    }

    #[test]
    fn cpes_projects_are_checked_without_a_base() {
        let dpe = valid_dpe_data();
        let root = projects_folder(&[("things", "0001"), ("broken", "0002")]);
        let config = cpe_config(root.path(), "");
        assert!(config.base.is_empty(), "CPE_BASE is unset");
        let (passes, printed) = run(dpe.path(), &config, &ports(&["0001"], &["0002"]));
        assert!(!passes, "{printed}");
        assert!(printed.contains("broken (0002): FAILED"), "{printed}");
    }
}

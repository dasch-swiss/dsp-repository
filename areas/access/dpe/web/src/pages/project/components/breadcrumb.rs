use maud::{html, Markup};
use mosaic_tiles::breadcrumb::{breadcrumb as tiles_breadcrumb, breadcrumb_current, breadcrumb_item};

/// Project-detail breadcrumb: "Projects" (link) › project name (current page).
pub fn breadcrumb(project_name: &str) -> Markup {
    let project_name_truncated = if project_name.chars().count() > 100 {
        format!("{}...", project_name.chars().take(50).collect::<String>())
    } else {
        project_name.to_string()
    };

    tiles_breadcrumb(html! {
        (breadcrumb_item("/dpe/projects", "Projects"))
        (breadcrumb_current(project_name_truncated))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_projects_link_and_current_name() {
        let out = breadcrumb("My Project").into_string();
        assert!(
            out.contains(r#"<a href="/dpe/projects" class="breadcrumb-link">Projects</a>"#),
            "{out}"
        );
        assert!(out.contains(r#"aria-current="page""#), "{out}");
        assert!(out.contains("My Project"), "{out}");
    }

    #[test]
    fn truncates_long_multibyte_name_without_panicking() {
        // A name longer than 100 chars whose 50th char boundary falls inside a
        // multi-byte UTF-8 sequence would panic under byte-index slicing.
        let name = "ä".repeat(120);
        let out = breadcrumb(&name).into_string();
        assert!(out.contains(&format!("{}...", "ä".repeat(50))), "{out}");
    }
}

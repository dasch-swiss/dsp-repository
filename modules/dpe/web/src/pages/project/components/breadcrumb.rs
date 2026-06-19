use maud::{html, Markup};
use mosaic_tiles::breadcrumb::{breadcrumb as tiles_breadcrumb, breadcrumb_item};

/// Project-detail breadcrumb: "Projects" (link) › project name (current page).
pub fn breadcrumb(project_name: &str) -> Markup {
    let project_name_truncated = if project_name.len() > 100 {
        format!("{}...", &project_name[..50])
    } else {
        project_name.to_string()
    };

    tiles_breadcrumb(html! {
        ({
            breadcrumb_item(
                Some("/dpe/projects"),
                html! {
                    "Projects"
                },
            )
        })
        ({
            breadcrumb_item(
                None,
                html! {
                    (project_name_truncated)
                },
            )
        })
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
}

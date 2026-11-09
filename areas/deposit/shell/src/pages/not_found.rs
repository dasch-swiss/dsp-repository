use maud::Markup;

pub fn not_found() -> Markup {
    super::status_page("404", "Page not found", "The page you asked for does not exist.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_page_keeps_its_wording() {
        let out = not_found().into_string();
        assert!(out.contains("Page not found"), "{out}");
        assert!(out.contains("The page you asked for does not exist."), "{out}");
    }

    #[test]
    fn test_the_status_code_precedes_the_heading_and_the_page_links_to_projects() {
        let out = not_found().into_string();
        crate::pages::assert_status_page(&out, "404");
    }
}

use maud::{html, Markup};

/// "Data Publication Year" heading + value.
pub fn publication_year(year: &str) -> Markup {
    html! {
        h3 class="dpe-subtitle" { "Data Publication Year" }
        div { (year) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_year() {
        let out = publication_year("2021").into_string();
        assert!(out.contains("Data Publication Year"), "{out}");
        assert!(out.contains("<div>2021</div>"), "{out}");
    }
}

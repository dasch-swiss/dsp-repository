use maud::{html, Markup};

pub mod forbidden;
pub mod not_found;
pub mod problem;

fn status_page(code: &str, heading: &str, message: &str) -> Markup {
    html! {
        div class="max-w-2xl py-12" {
            p class="font-mono text-sm text-primary-600 mb-1" { (code) }
            h1 class="font-display font-semibold text-2xl text-neutral-900 mb-2" { (heading) }
            p class="text-neutral-600 mb-6" { (message) }
            p {
                a href="/projects" class="text-primary-600 underline underline-offset-2" {
                    "Go to your projects"
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn assert_status_page(out: &str, code: &str) {
    let code_at = out
        .find(&format!(">{code}</p>"))
        .unwrap_or_else(|| panic!("{code} missing: {out}"));
    let h1_at = out.find("<h1").unwrap_or_else(|| panic!("h1 missing: {out}"));
    assert!(code_at < h1_at, "{out}");
    assert!(out.contains(r#"<a href="/projects""#), "{out}");
}

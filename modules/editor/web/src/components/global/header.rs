use maud::{html, Markup};

/// The global header: DaSCH logo and the service name, both linking home.
///
/// Deliberately thinner than DPE's. DPE's header carries public wayfinding
/// (Help, "Deposit Data at DaSCH"); the editor is an authenticated tool whose
/// users arrive knowing why they are here. The signed-in identity and sign-out
/// control belong here and are added with authentication.
pub fn header() -> Markup {
    html! {
        div class="bg-white shadow-xs" {
            div class="flex items-center py-2 max-w-[1536px] mx-auto px-4 w-full" {
                a href="/" aria-label="DaSCH Metadata Editor home" {
                    img src="/logo.svg" class="inline h-10 w-10 mr-2" alt="DaSCH logo";
                }

                div class="flex-1" {
                    a class="inline-flex items-center font-bold font-display text-xl" href="/" {
                        "DaSCH Metadata Editor"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_logo_and_home_link() {
        let out = header().into_string();
        assert!(out.contains(r#"<img src="/logo.svg""#), "{out}");
        assert!(out.contains(r#"aria-label="DaSCH Metadata Editor home""#), "{out}");
        assert!(out.contains("DaSCH Metadata Editor"), "{out}");
        assert!(out.contains(r#"href="/""#), "{out}");
    }
}

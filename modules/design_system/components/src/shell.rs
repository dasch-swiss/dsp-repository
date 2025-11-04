use maud::{html, Markup};

use crate::footer::footer;
use crate::header::header;

/// Builder for configuring shell layout
#[derive(Debug, Clone)]
pub struct ShellBuilder {
    header_nav_elements: Vec<crate::header::NavElement>,
    header_config: crate::header::HeaderConfig,
    content: Option<Markup>,
    footer: crate::footer::FooterConfig,
}

impl ShellBuilder {
    pub fn new(
        header_nav_elements: Vec<crate::header::NavElement>,
        header_config: crate::header::HeaderConfig,
        footer: crate::footer::FooterConfig,
    ) -> Self {
        Self { header_nav_elements, header_config, content: None, footer }
    }

    #[must_use = "builder does nothing unless you call .build()"]
    pub fn with_content(mut self, content: Markup) -> Self {
        self.content = Some(content);
        self
    }

    pub fn build(self) -> Markup {
        render_shell(self.header_nav_elements, self.header_config, self.content, self.footer)
    }
}

#[must_use = "call .build() to render the component"]
pub fn shell(
    header_nav_elements: Vec<crate::header::NavElement>,
    header_config: crate::header::HeaderConfig,
    footer: crate::footer::FooterConfig,
) -> ShellBuilder {
    ShellBuilder::new(header_nav_elements, header_config, footer)
}

fn render_shell(
    header_nav_elements: Vec<crate::header::NavElement>,
    header_config: crate::header::HeaderConfig,
    content: Option<Markup>,
    footer_config: crate::footer::FooterConfig,
) -> Markup {
    html! {
        div .bg-white .dark:bg-gray-900 {
            // Header
            (header(header_nav_elements, &header_config))

            // Main content
            main {
                @if let Some(content) = content {
                    (content)
                } @else {
                    div .mx-auto .max-w-7xl .px-6 .py-24 .sm:py-32 .lg:px-8 {
                        div .mx-auto .max-w-2xl .text-center {
                            h1 .text-4xl .font-semibold .tracking-tight .text-gray-900 .sm:text-5xl .dark:text-white {
                                "Welcome"
                            }
                            p .mt-6 .text-lg .text-gray-600 .dark:text-gray-300 {
                                "No content provided"
                            }
                        }
                    }
                }
            }

            // Footer
            (footer(&footer_config))
        }
    }
}

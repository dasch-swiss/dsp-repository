use maud::{html, Markup, PreEscaped};

/// Builder for configuring shell navigation elements
#[derive(Debug, Clone)]
pub struct ShellBuilder {
    header_nav_elements: Vec<NavElement>,
    side_nav_elements: Option<Vec<NavElement>>,
    content: Option<Markup>,
}

impl ShellBuilder {
    pub fn new(header_nav_elements: Vec<NavElement>) -> Self {
        Self { header_nav_elements, side_nav_elements: None, content: None }
    }

    pub fn with_side_nav(mut self, elements: Vec<NavElement>) -> Self {
        self.side_nav_elements = Some(elements);
        self
    }

    pub fn with_content(mut self, content: Markup) -> Self {
        self.content = Some(content);
        self
    }

    pub fn build(self) -> Markup {
        render_shell(self.header_nav_elements, self.side_nav_elements, self.content)
    }
}

/// A navigation element can be either a single item or a menu with sub-items
#[derive(Debug, Clone)]
pub enum NavElement {
    Item(NavItem),
    Menu(NavMenu),
}

/// A single navigation item
#[derive(Debug, Clone)]
pub struct NavItem {
    pub label: &'static str,
    pub href: &'static str,
}

/// A navigation menu with multiple items
#[derive(Debug, Clone)]
pub struct NavMenu {
    pub label: &'static str,
    pub items: Vec<NavMenuItem>,
}

/// An item within a navigation menu
#[derive(Debug, Clone)]
pub struct NavMenuItem {
    pub label: &'static str,
    pub href: &'static str,
}

pub fn shell(header_nav_elements: Vec<NavElement>) -> ShellBuilder {
    ShellBuilder::new(header_nav_elements)
}

fn render_shell(
    header_nav_elements: Vec<NavElement>,
    side_nav_elements: Option<Vec<NavElement>>,
    content: Option<Markup>,
) -> Markup {
    let header_nav_markup = render_nav_elements(&header_nav_elements);
    let has_side_nav = side_nav_elements.is_some();
    let header_menu_button = if has_side_nav {
        PreEscaped(
            r#"<cds-header-menu-button button-label-active="Close menu" button-label-inactive="Open menu" collapse-mode="responsive"></cds-header-menu-button>"#,
        )
    } else {
        PreEscaped("")
    };

    let side_nav_section = if let Some(side_nav_elements) = side_nav_elements {
        let side_nav_markup = render_side_nav_elements(&side_nav_elements);
        html! {
            (PreEscaped(r#"<cds-side-nav aria-label="Side navigation" collapse-mode="responsive" role="navigation" data-testid="shell-header-side-nav">
              <cds-side-nav-items role="list">"#))
              (side_nav_markup)
              (PreEscaped(r#"</cds-side-nav-items>
            </cds-side-nav>"#))
        }
    } else {
        html! {}
    };

    html! {
        div .dsp--layout {
            (PreEscaped(r#"
            <script>
            if (!window.carbonUIShell) {{
                window.carbonUIShell = true;
                import('https://1.www.s81c.com/common/carbon/web-components/version/v2.33.0/ui-shell.min.js');
            }}
            
            // Theme toggle functionality
            function toggleTheme() {{
                const html = document.documentElement;
                const isDark = html.classList.toggle('dark');
                localStorage.setItem('theme', isDark ? 'dark' : 'light');
                updateThemeIcon(isDark);
            }}
            
            function updateThemeIcon(isDark) {{
                const themeSvg = document.querySelector('[aria-label="Toggle theme"] svg');
                if (themeSvg) {{
                    if (isDark) {{
                        // Show sun icon in dark mode (to switch to light) - using Carbon awake icon
                        themeSvg.setAttribute('viewBox', '0 0 16 16');
                        themeSvg.innerHTML = '<polygon fill="var(--cds-icon-secondary, #525252)" points="7.5 0 8.5 0 8.5 3 7.5 3"></polygon><polygon fill="var(--cds-icon-secondary, #525252)" points="7.5 13 8.5 13 8.5 16 7.5 16"></polygon><polygon fill="var(--cds-icon-secondary, #525252)" points="13 7.5 16 7.5 16 8.5 13 8.5"></polygon><polygon fill="var(--cds-icon-secondary, #525252)" points="0 7.5 3 7.5 3 8.5 0 8.5"></polygon><polygon fill="var(--cds-icon-secondary, #525252)" points="2.80761719 2.10050201 4.80761719 4.10050201 4.10050964 4.80761719 2.10050964 2.80761719"></polygon><polygon fill="var(--cds-icon-secondary, #525252)" points="13.1923828 11.8994949 15.1923828 13.8994949 14.4852753 14.6066017 12.4852753 12.6066017"></polygon><polygon fill="var(--cds-icon-secondary, #525252)" points="2.80761719 13.8994949 4.80761719 11.8994949 4.10050964 11.1923828 2.10050964 13.1923828"></polygon><polygon fill="var(--cds-icon-secondary, #525252)" points="13.1923828 4.10050201 15.1923828 2.10050201 14.4852753 1.39339828 12.4852753 3.39339828"></polygon><path d="M8,10.5 C9.38071187,10.5 10.5,9.38071187 10.5,8 C10.5,6.61928813 9.38071187,5.5 8,5.5 C6.61928813,5.5 5.5,6.61928813 5.5,8 C5.5,9.38071187 6.61928813,10.5 8,10.5 Z M8,11.5 C6.06700338,11.5 4.5,9.93299662 4.5,8 C4.5,6.06700338 6.06700338,4.5 8,4.5 C9.93299662,4.5 11.5,6.06700338 11.5,8 C11.5,9.93299662 9.93299662,11.5 8,11.5 Z" fill="var(--cds-icon-secondary, #525252)"></path>';
                    }} else {{
                        // Show moon icon in light mode (to switch to dark)
                        themeSvg.setAttribute('viewBox', '0 0 15 16');
                        themeSvg.innerHTML = '<path d="M14,12 C10.2,11.8 7.2,8.7 7.2,4.9 C7.2,3.5 7.6,2.1 8.5,0.9 L8.5,0.8 C8.6,0.7 8.6,0.6 8.6,0.5 C8.6,0.2 8.4,0 8.1,0 L8,0 C3.6,0 0,3.6 0,8 C0,12.4 3.6,16 8,16 C10.5,16 12.8,14.8 14.3,12.9 C14.5,12.7 14.5,12.4 14.4,12.2 C14.3,12.1 14.1,12 14,12 Z M8,15 C4.1,15 1,11.9 1,8 C1,4.4 3.6,1.4 7.2,1 C5.1,5 6.6,9.9 10.5,12 C11.3,12.4 12.1,12.7 13,12.9 C11.7,14.3 9.9,15 8,15 Z" fill="var(--cds-icon-secondary, #525252)"></path>';
                    }}
                }}
            }}
            
            // Initialize theme from localStorage
            if (!window.themeInitialized) {{
                window.themeInitialized = true;
                const savedTheme = localStorage.getItem('theme');
                const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
                const shouldBeDark = savedTheme === 'dark' || (!savedTheme && prefersDark);
                
                if (shouldBeDark) {{
                    document.documentElement.classList.add('dark');
                }}
                
                // Update icon based on initial theme
                setTimeout(() => updateThemeIcon(shouldBeDark), 100);
            }}
            </script>
            <cds-header aria-label="DaSCH Service Platform" role="banner" data-testid="shell-header">
            "#))
            (header_menu_button)
            (PreEscaped(r#"<cds-header-name href="/" prefix="DaSCH" data-testid="shell-header-logo">Service Platform</cds-header-name>
            <cds-header-nav menu-bar-label="DaSCH Service Platform" role="navigation">
              "#)) 
              (header_nav_markup)
              (PreEscaped(r#"
            </cds-header-nav>
            <div class="cds--header__global">
              <cds-header-global-action aria-label="Search" tooltip-text="Search" kind="ghost" size="lg" tab-index="0" tooltip-alignment="" tooltip-position="bottom" type="button" data-testid="shell-header-search">
                <svg focusable="false" preserveAspectRatio="xMidYMid meet" xmlns="http://www.w3.org/2000/svg" fill="currentColor" aria-hidden="true" width="20" height="20" viewBox="0 0 32 32" slot="icon"><path d="M29,27.5859l-7.5521-7.5521a11.0177,11.0177,0,1,0-1.4141,1.4141L27.5859,29ZM4,13a9,9,0,1,1,9,9A9.01,9.01,0,0,1,4,13Z"></path></svg>
              </cds-header-global-action>
              <cds-header-global-action aria-label="Toggle theme" tooltip-text="Toggle light/dark theme" kind="ghost" size="lg" tab-index="0" tooltip-alignment="right" tooltip-position="bottom" type="button" onclick="toggleTheme()" data-testid="shell-header-theme">
                <svg focusable="false" preserveAspectRatio="xMidYMid meet" xmlns="http://www.w3.org/2000/svg" fill="currentColor" aria-hidden="true" width="20" height="20" viewBox="0 0 15 16" slot="icon"><path d="M14,12 C10.2,11.8 7.2,8.7 7.2,4.9 C7.2,3.5 7.6,2.1 8.5,0.9 L8.5,0.8 C8.6,0.7 8.6,0.6 8.6,0.5 C8.6,0.2 8.4,0 8.1,0 L8,0 C3.6,0 0,3.6 0,8 C0,12.4 3.6,16 8,16 C10.5,16 12.8,14.8 14.3,12.9 C14.5,12.7 14.5,12.4 14.4,12.2 C14.3,12.1 14.1,12 14,12 Z M8,15 C4.1,15 1,11.9 1,8 C1,4.4 3.6,1.4 7.2,1 C5.1,5 6.6,9.9 10.5,12 C11.3,12.4 12.1,12.7 13,12.9 C11.7,14.3 9.9,15 8,15 Z" fill="var(--cds-icon-secondary, #525252)"></path></svg>
              </cds-header-global-action>
            </div>
            "#))
            (side_nav_section)
            (PreEscaped(r#"</cds-header>
            "#))
            @if let Some(content) = content {
                main #main-content {
                  div .dsp--layout__main_content--with-side-nav[has_side_nav] {
                    div .dsp--layout__main_content {
                      (content)
                  }
                  }
                }
            }
        }
    }
}

fn render_nav_elements(nav_elements: &[NavElement]) -> Markup {
    html! {
        @for element in nav_elements {
            @match element {
                NavElement::Item(item) => {
                    cds-header-nav-item href=(item.href) role="listitem" { (item.label) }
                }
                NavElement::Menu(menu) => {
                    cds-header-menu menu-label=(menu.label) trigger-content=(menu.label) role="listitem" {
                        @for menu_item in &menu.items {
                            cds-header-menu-item href=(menu_item.href) tabindex="-1" role="listitem" { (menu_item.label) }
                        }
                    }
                }
            }
        }
    }
}

fn render_side_nav_elements(nav_elements: &[NavElement]) -> Markup {
    html! {
        @for element in nav_elements {
            @match element {
                NavElement::Item(item) => {
                    cds-side-nav-link href=(item.href) role="listitem" { (item.label) }
                }
                NavElement::Menu(menu) => {
                    cds-side-nav-menu title=(menu.label) role="listitem" {
                        @for menu_item in &menu.items {
                            cds-side-nav-menu-item href=(menu_item.href) role="button" tabindex="-1" { (menu_item.label) }
                        }
                    }
                }
            }
        }
    }
}

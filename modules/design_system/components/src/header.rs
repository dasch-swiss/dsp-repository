use maud::{html, Markup};

#[derive(Debug, Clone)]
pub struct HeaderConfig {
    pub company_name: &'static str,
    pub logo_light_url: &'static str,
    pub logo_dark_url: &'static str,
    pub login_href: &'static str,
}

#[derive(Debug, Clone)]
pub enum NavElement {
    Item(NavItem),
    Menu(NavMenu),
}

#[derive(Debug, Clone)]
pub struct NavItem {
    pub label: &'static str,
    pub href: &'static str,
}

#[derive(Debug, Clone)]
pub struct NavMenu {
    pub label: &'static str,
    pub items: Vec<NavMenuItem>,
}

#[derive(Debug, Clone)]
pub struct NavMenuItem {
    pub label: &'static str,
    pub href: &'static str,
}

fn generate_menu_id(label: &str) -> String {
    label.to_lowercase().replace(' ', "")
}

fn render_hamburger_icon() -> Markup {
    html! {
        svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" data-slot="icon" aria-hidden="true" class="size-6" {
            path d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" stroke-linecap="round" stroke-linejoin="round";
        }
    }
}

fn render_close_icon() -> Markup {
    html! {
        svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" data-slot="icon" aria-hidden="true" class="size-6" {
            path d="M6 18 18 6M6 6l12 12" stroke-linecap="round" stroke-linejoin="round";
        }
    }
}

fn render_chevron_down_icon() -> Markup {
    html! {
        svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden="true" class="size-5 flex-none text-gray-400 dark:text-gray-500" {
            path d="M5.22 8.22a.75.75 0 0 1 1.06 0L10 11.94l3.72-3.72a.75.75 0 1 1 1.06 1.06l-4.25 4.25a.75.75 0 0 1-1.06 0L5.22 9.28a.75.75 0 0 1 0-1.06Z" clip-rule="evenodd" fill-rule="evenodd";
        }
    }
}

fn render_chevron_down_icon_mobile() -> Markup {
    html! {
        svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden="true" class="size-5 flex-none in-aria-expanded:rotate-180" {
            path d="M5.22 8.22a.75.75 0 0 1 1.06 0L10 11.94l3.72-3.72a.75.75 0 1 1 1.06 1.06l-4.25 4.25a.75.75 0 0 1-1.06 0L5.22 9.28a.75.75 0 0 1 0-1.06Z" clip-rule="evenodd" fill-rule="evenodd";
        }
    }
}

fn render_logo(config: &HeaderConfig) -> Markup {
    html! {
        a href="#" class="-m-1.5 p-1.5" {
            span class="sr-only" { (config.company_name) }
            img src=(config.logo_light_url) alt="" class="h-8 w-auto dark:hidden";
            img src=(config.logo_dark_url) alt="" class="h-8 w-auto not-dark:hidden";
        }
    }
}

fn render_login_link(config: &HeaderConfig) -> Markup {
    html! {
        a href=(config.login_href) class="text-sm/6 font-semibold text-gray-900 dark:text-white" {
            "Log in "
            span aria-hidden="true" { "→" }
        }
    }
}

fn render_login_link_mobile(config: &HeaderConfig) -> Markup {
    html! {
        a href=(config.login_href) class="-mx-3 block rounded-lg px-3 py-2.5 text-base/7 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5" {
            "Log in"
        }
    }
}

fn render_mobile_menu_header(config: &HeaderConfig) -> Markup {
    html! {
        div class="flex items-center justify-between" {
            (render_logo(config))
            button type="button" command="close" commandfor="mobile-menu" class="-m-2.5 rounded-md p-2.5 text-gray-700 dark:text-gray-400 dark:hover:text-white" {
                span class="sr-only" { "Close menu" }
                (render_close_icon())
            }
        }
    }
}

fn render_mobile_menu_dialog(config: &HeaderConfig, nav_elements: &[NavElement]) -> Markup {
    html! {
        el-dialog {
            dialog id="mobile-menu" class="backdrop:bg-transparent lg:hidden" {
                div tabindex="0" class="fixed inset-0 focus:outline-none" {
                    el-dialog-panel class="fixed inset-y-0 right-0 z-50 w-full overflow-y-auto bg-white p-6 sm:max-w-sm sm:ring-1 sm:ring-gray-900/10 dark:bg-gray-900 dark:sm:ring-gray-100/10" {
                        (render_mobile_menu_header(config))
                        div class="mt-6 flow-root" {
                            div class="-my-6 divide-y divide-gray-500/10 dark:divide-white/10" {
                                div class="space-y-2 py-6" {
                                    (render_mobile_nav(nav_elements))
                                }
                                div class="py-6" {
                                    (render_login_link_mobile(config))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn header(nav_elements: Vec<NavElement>, config: &HeaderConfig) -> Markup {
    html! {
        header class="bg-white dark:bg-gray-900" {
            nav aria-label="Global" class="mx-auto flex max-w-7xl items-center justify-between p-6 lg:px-8" {
                div class="flex items-center gap-x-12" {
                    (render_logo(config))
                    (render_desktop_nav(&nav_elements))
                }
                div class="flex lg:hidden" {
                    button type="button" command="show-modal" commandfor="mobile-menu" class="-m-2.5 inline-flex items-center justify-center rounded-md p-2.5 text-gray-700 dark:text-gray-400 dark:hover:text-white" {
                        span class="sr-only" { "Open main menu" }
                        (render_hamburger_icon())
                    }
                }
                div class="hidden lg:flex" {
                    (render_login_link(config))
                }
            }
            (render_mobile_menu_dialog(config, &nav_elements))
        }
    }
}

fn render_desktop_nav(nav_elements: &[NavElement]) -> Markup {
    html! {
        el-popover-group class="hidden lg:flex lg:gap-x-12" {
            @for element in nav_elements {
                @match element {
                    NavElement::Item(item) => {
                        a href=(item.href) class="text-sm/6 font-semibold text-gray-900 dark:text-white" { (item.label) }
                    }
                    NavElement::Menu(menu) => {
                        div class="relative" {
                            button popovertarget=(format!("desktop-menu-{}", generate_menu_id(menu.label))) class="flex items-center gap-x-1 text-sm/6 font-semibold text-gray-900 dark:text-white" {
                                (menu.label)
                                (render_chevron_down_icon())
                            }

                            el-popover id=(format!("desktop-menu-{}", generate_menu_id(menu.label))) anchor="bottom" popover class="w-56 overflow-visible rounded-xl bg-white p-2 shadow-lg outline-1 outline-gray-900/5 transition transition-discrete [--anchor-gap:--spacing(3)] backdrop:bg-transparent open:block data-closed:translate-y-1 data-closed:opacity-0 data-enter:duration-200 data-enter:ease-out data-leave:duration-150 data-leave:ease-in dark:bg-gray-800 dark:shadow-none dark:-outline-offset-1 dark:outline-white/10" {
                                @for menu_item in &menu.items {
                                    a href=(menu_item.href) class="block rounded-lg px-3 py-2 text-sm/6 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5" { (menu_item.label) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_mobile_nav(nav_elements: &[NavElement]) -> Markup {
    html! {
        @for element in nav_elements {
            @match element {
                NavElement::Item(item) => {
                    a href=(item.href) class="-mx-3 block rounded-lg px-3 py-2 text-base/7 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5" { (item.label) }
                }
                NavElement::Menu(menu) => {
                    div class="-mx-3" {
                        button type="button" command="--toggle" commandfor=(generate_menu_id(menu.label)) class="flex w-full items-center justify-between rounded-lg py-2 pr-3.5 pl-3 text-base/7 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5" {
                            (menu.label)
                            (render_chevron_down_icon_mobile())
                        }
                        el-disclosure id=(generate_menu_id(menu.label)) hidden class="mt-2 block space-y-2" {
                            @for menu_item in &menu.items {
                                a href=(menu_item.href) class="block rounded-lg py-2 pr-3 pl-6 text-sm/7 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5" { (menu_item.label) }
                            }
                        }
                    }
                }
            }
        }
    }
}

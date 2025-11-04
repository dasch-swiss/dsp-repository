use maud::{html, Markup};

const MENU_CLASSES: &str = "w-56 origin-top-right rounded-md bg-white shadow-lg outline-1 outline-black/5 \
                            transition transition-discrete [--anchor-gap:--spacing(2)] \
                            data-closed:scale-95 data-closed:transform data-closed:opacity-0 \
                            data-enter:duration-100 data-enter:ease-out data-leave:duration-75 data-leave:ease-in \
                            dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10";

const MENU_INNER_CLASSES: &str = "py-1";

const DEFAULT_ANCHOR: &str = "bottom end";

/// Builder for constructing a menu component
///
/// The menu automatically positions itself optimally based on available screen space.
///
/// # Example
/// ```rust
/// use components::{menu, menu_item, icon, IconType};
///
/// let star_icon = icon::icon_for_menu_item(IconType::Star);
///
/// let my_menu = menu::menu()
///     .with_id("user-menu")
///     .with_item(menu_item::link_menu_item("Profile", "/profile"))
///     .with_item(menu_item::link_menu_item_with_icon("Favorites", "/favorites", star_icon))
///     .with_item(menu_item::menu_item_divider())
///     .with_item(menu_item::button_menu_item("Sign Out"))
///     .build();
/// ```
pub struct MenuBuilder {
    items: Vec<Markup>,
    id: Option<String>,
    trigger: Option<Markup>,
}

impl MenuBuilder {
    /// Creates a new menu builder with default settings
    pub fn new() -> Self {
        Self { items: Vec::new(), id: None, trigger: None }
    }

    /// Adds a single menu item to the menu
    ///
    /// # Example
    /// ```rust
    /// use components::{menu, menu_item};
    ///
    /// let my_menu = menu::menu()
    ///     .with_item(menu_item::link_menu_item("Profile", "/profile"))
    ///     .build();
    /// ```
    pub fn with_item(mut self, item: Markup) -> Self {
        self.items.push(item);
        self
    }

    /// Adds multiple menu items to the menu
    ///
    /// # Example
    /// ```rust
    /// use components::{menu, menu_item};
    ///
    /// let items = vec![
    ///     menu_item::link_menu_item("Profile", "/profile"),
    ///     menu_item::link_menu_item("Settings", "/settings"),
    /// ];
    ///
    /// let my_menu = menu::menu()
    ///     .with_items(items)
    ///     .build();
    /// ```
    pub fn with_items(mut self, items: Vec<Markup>) -> Self {
        self.items.extend(items);
        self
    }

    /// Sets the ID for the menu (required for popover targeting)
    ///
    /// The ID is used by trigger buttons with the `popovertarget` attribute.
    ///
    /// # Example
    /// ```rust
    /// use components::menu::menu;
    ///
    /// let my_menu = menu()
    ///     .with_id("user-menu")
    ///     .build();
    ///
    /// // Later, in the button:
    /// // <button popovertarget="user-menu">Open Menu</button>
    /// ```
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets a custom trigger element for the menu
    ///
    /// The trigger should be a complete button element. For convenience,
    /// use `with_text_trigger()` or `with_icon_trigger()` helper methods instead.
    ///
    /// # Example
    /// ```rust
    /// use components::{menu, menu_item};
    /// use maud::html;
    ///
    /// let custom_trigger = html! {
    ///     button popovertarget="my-menu" class="custom-classes" { "Custom Button" }
    /// };
    ///
    /// let my_menu = menu::menu()
    ///     .with_id("my-menu")
    ///     .with_trigger(custom_trigger)
    ///     .with_item(menu_item::link_menu_item("Profile", "/profile"))
    ///     .build();
    /// ```
    pub fn with_trigger(mut self, trigger_button: Markup) -> Self {
        self.trigger = Some(trigger_button);
        self
    }

    /// Creates a text button trigger for the menu
    ///
    /// This helper method creates a styled text button that triggers the menu.
    /// The button automatically includes the correct `popovertarget` attribute.
    ///
    /// # Example
    /// ```rust
    /// use components::{menu, menu_item};
    ///
    /// let my_menu = menu::menu()
    ///     .with_id("user-menu")
    ///     .with_text_trigger("Open Menu")
    ///     .with_item(menu_item::link_menu_item("Profile", "/profile"))
    ///     .build();
    /// ```
    pub fn with_text_trigger(mut self, text: impl Into<String>) -> Self {
        let menu_id = self.id.clone().unwrap_or_else(|| "menu".to_string());
        let text = text.into();

        self.trigger = Some(html! {
            button
                type="button"
                popovertarget=(menu_id)
                class="rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 dark:bg-indigo-500 dark:hover:bg-indigo-400"
                data-testid="menu-trigger"
            {
                (text)
            }
        });
        self
    }

    /// Creates an icon button trigger for the menu
    ///
    /// This helper method creates an icon button that triggers the menu.
    /// Icon buttons are compact, keyboard accessible, and semantically correct.
    /// The button automatically includes the correct `popovertarget` attribute.
    ///
    /// # Example
    /// ```rust
    /// use components::{menu, menu_item, icon, IconType};
    ///
    /// let my_menu = menu::menu()
    ///     .with_id("actions-menu")
    ///     .with_icon_trigger(icon::icon(IconType::Hamburger))
    ///     .with_item(menu_item::link_menu_item("Dashboard", "/dashboard"))
    ///     .build();
    /// ```
    pub fn with_icon_trigger(mut self, icon: Markup) -> Self {
        let menu_id = self.id.clone().unwrap_or_else(|| "menu".to_string());

        self.trigger = Some(html! {
            button
                type="button"
                popovertarget=(menu_id)
                class="rounded-md p-2 text-sm font-semibold cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-900 dark:text-gray-300"
                data-testid="menu-trigger"
            {
                (icon)
            }
        });
        self
    }

    /// Builds the menu component and returns the rendered markup
    ///
    /// When a trigger is provided via `with_trigger()`, `with_text_trigger()`, or
    /// `with_icon_trigger()`, the menu will be wrapped in a container along with
    /// its trigger button. Otherwise, only the menu itself is rendered (requiring
    /// an external trigger button).
    pub fn build(self) -> Markup {
        let menu = html! {
            el-menu
                id=[self.id]
                anchor=(DEFAULT_ANCHOR)
                popover
                class=(MENU_CLASSES)
                data-testid="menu"
            {
                div class=(MENU_INNER_CLASSES) {
                    @for item in self.items {
                        (item)
                    }
                }
            }
        };

        match self.trigger {
            Some(trigger_button) => {
                html! {
                    div class="relative inline-block" {
                        (trigger_button)
                        (menu)
                    }
                }
            }
            None => menu,
        }
    }
}

impl Default for MenuBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a new menu builder
///
/// # Example
/// ```rust
/// use components::{menu, menu_item};
///
/// let my_menu = menu::menu()
///     .with_id("actions-menu")
///     .with_item(menu_item::button_menu_item("Delete"))
///     .build();
/// ```
pub fn menu() -> MenuBuilder {
    MenuBuilder::new()
}

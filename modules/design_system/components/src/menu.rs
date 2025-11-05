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
/// let star_icon = icon::icon(IconType::Star);
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

    /// Sets a trigger button for the menu
    ///
    /// Pass a button with the `popovertarget` attribute set to the menu's ID.
    /// Use the button builder's `.popovertarget()` method to set this.
    ///
    /// # Example with text button
    /// ```rust
    /// use components::{menu, menu_item, button};
    ///
    /// let my_menu = menu::menu()
    ///     .with_id("my-menu")
    ///     .with_trigger(
    ///         button::button("Open Menu")
    ///             .popovertarget("my-menu")
    ///             .build()
    ///     )
    ///     .with_item(menu_item::link_menu_item("Profile", "/profile"))
    ///     .build();
    /// ```
    ///
    /// # Example with icon button
    /// ```rust
    /// use components::{menu, menu_item, button, icon, IconType};
    ///
    /// let my_menu = menu::menu()
    ///     .with_id("my-menu")
    ///     .with_trigger(
    ///         button::icon_button(icon::icon(IconType::Hamburger))
    ///             .popovertarget("my-menu")
    ///             .build()
    ///     )
    ///     .with_item(menu_item::link_menu_item("Profile", "/profile"))
    ///     .build();
    /// ```
    ///
    /// # External triggers
    /// If you don't provide a trigger, you can control the menu externally:
    /// - Use `.popovertarget()` on any button in your HTML
    /// - Use DataStar onclick: `document.getElementById('menu-id').showPopover()`
    /// - Use JavaScript: `document.getElementById('menu-id').togglePopover()`
    pub fn with_trigger(mut self, trigger_button: Markup) -> Self {
        self.trigger = Some(trigger_button);
        self
    }

    /// Builds the menu component and returns the rendered markup
    ///
    /// When a trigger is provided via `with_trigger()`, the menu will be wrapped
    /// in a container along with its trigger button. Otherwise, only the menu
    /// itself is rendered (requiring an external trigger button).
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
                    el-dropdown class="relative inline-block" {
                        (trigger_button)
                        (menu)
                    }
                }
            }
            None => {
                html! {
                    el-dropdown class="relative inline-block" {
                        (menu)
                    }
                }
            }
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

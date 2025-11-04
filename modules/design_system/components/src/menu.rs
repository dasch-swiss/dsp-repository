use maud::{html, Markup};

use crate::builder_common::ComponentBuilder;

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
/// use components::{menu::menu, menu_item, icon::icon, IconType, ComponentBuilder};
///
/// let star_icon = icon(IconType::Star);
///
/// let my_menu = menu()
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
    test_id: Option<String>,
    trigger: Option<Markup>,
}

impl ComponentBuilder for MenuBuilder {
    fn id_mut(&mut self) -> &mut Option<String> {
        &mut self.id
    }

    fn test_id_mut(&mut self) -> &mut Option<String> {
        &mut self.test_id
    }

    fn build(self) -> Markup {
        let test_id = self.test_id.as_deref().unwrap_or("menu");

        let menu = html! {
            el-menu
                id=[self.id]
                anchor=(DEFAULT_ANCHOR)
                popover
                class=(MENU_CLASSES)
                data-testid=(test_id)
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

impl MenuBuilder {
    /// Creates a new menu builder with default settings
    pub fn new() -> Self {
        Self { items: Vec::new(), id: None, test_id: None, trigger: None }
    }

    /// Adds a single menu item to the menu
    ///
    /// # Example
    /// ```rust
    /// use components::{menu::menu, menu_item, ComponentBuilder};
    ///
    /// let my_menu = menu()
    ///     .with_item(menu_item::link_menu_item("Profile", "/profile"))
    ///     .build();
    /// ```
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn with_item(mut self, item: Markup) -> Self {
        self.items.push(item);
        self
    }

    /// Adds multiple menu items to the menu
    ///
    /// # Example
    /// ```rust
    /// use components::{menu::menu, menu_item, ComponentBuilder};
    ///
    /// let items = vec![
    ///     menu_item::link_menu_item("Profile", "/profile"),
    ///     menu_item::link_menu_item("Settings", "/settings"),
    /// ];
    ///
    /// let my_menu = menu()
    ///     .with_items(items)
    ///     .build();
    /// ```
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn with_items(mut self, items: Vec<Markup>) -> Self {
        self.items.extend(items);
        self
    }

    /// Sets a trigger button for the menu
    ///
    /// Pass a button with the `popovertarget` attribute set to the menu's ID.
    /// Use the button builder's `.popovertarget()` method to set this.
    ///
    /// # Example with text button
    /// ```rust
    /// use components::{menu::menu, menu_item, button::button, ComponentBuilder};
    ///
    /// let my_menu = menu()
    ///     .with_id("my-menu")
    ///     .with_trigger(
    ///         button("Open Menu")
    ///             .popovertarget("my-menu")
    ///             .build()
    ///     )
    ///     .with_item(menu_item::link_menu_item("Profile", "/profile"))
    ///     .build();
    /// ```
    ///
    /// # Example with icon button
    /// ```rust
    /// use components::{menu::menu, menu_item, button::icon_button, icon::icon, IconType, ComponentBuilder};
    ///
    /// let my_menu = menu()
    ///     .with_id("my-menu")
    ///     .with_trigger(
    ///         icon_button(icon(IconType::Hamburger))
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
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn with_trigger(mut self, trigger_button: Markup) -> Self {
        self.trigger = Some(trigger_button);
        self
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
/// use components::{menu::menu, menu_item, ComponentBuilder};
///
/// let my_menu = menu()
///     .with_id("actions-menu")
///     .with_item(menu_item::button_menu_item("Delete"))
///     .build();
/// ```
#[must_use = "call .build() to render the component"]
pub fn menu() -> MenuBuilder {
    MenuBuilder::new()
}

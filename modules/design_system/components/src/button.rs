// TODO: find a solution for button-style anchor tags
use maud::{html, Markup};

const BASE_CLASSES: &str =
    "inline-flex items-center gap-2 rounded-md px-3 py-2 text-sm font-semibold shadow-xs cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed";

const ICON_BUTTON_BASE_CLASSES: &str =
    "rounded-md p-2 text-sm font-semibold cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 dark:hover:bg-gray-800";

const DEFAULT_ICON_BUTTON_COLOR: &str = "text-gray-900 dark:text-gray-300";

#[derive(Debug, Clone)]
pub enum ButtonVariant {
    Primary,
    Secondary,
}

impl ButtonVariant {
    fn variant_classes(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "bg-indigo-600 text-white hover:bg-indigo-300 focus-visible:outline-2 dark:bg-indigo-500 dark:text-white dark:shadow-none dark:hover:bg-indigo-400 dark:focus-visible:outline-indigo-500",
            ButtonVariant::Secondary => "bg-indigo-300 text-indigo-900 hover:bg-indigo-600 focus-visible:outline-2",
        }
    }

    fn test_id(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "button-primary",
            ButtonVariant::Secondary => "button-secondary",
        }
    }
}

// TODO: Create type-safe DataStar action wrapper to replace raw string onclick handlers
// This would provide compile-time validation for DataStar actions like:
// - DataStarAction::ConsoleLog(msg)
// - DataStarAction::Get { url, options }
// - DataStarAction::Post { url, options }
// See: https://data-star.dev/ for DataStar action syntax

/// Builder for constructing a button component
///
/// # Example
/// ```rust
/// use components::{button, ButtonVariant};
///
/// // Simple button
/// let simple = button("Click me").build();
///
/// // Button with options
/// let custom = button("Save")
///     .variant(ButtonVariant::Secondary)
///     .onclick("console.log('saved')")
///     .build();
///
/// // Disabled button
/// let disabled = button("Delete")
///     .disabled()
///     .onclick("console.log('this does not fire')")
///     .build();
/// ```
pub struct ButtonBuilder {
    text: String,
    variant: ButtonVariant,
    disabled: bool,
    onclick: Option<String>,
    test_id: Option<String>,
    id: Option<String>,
    leading_icon: Option<Markup>,
    trailing_icon: Option<Markup>,
    popovertarget: Option<String>,
}

impl ButtonBuilder {
    /// Creates a new button builder with default settings
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: ButtonVariant::Primary,
            disabled: false,
            onclick: None,
            test_id: None,
            id: None,
            leading_icon: None,
            trailing_icon: None,
            popovertarget: None,
        }
    }

    /// Sets the button variant (Primary or Secondary)
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Marks the button as disabled
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Sets the onclick DataStar action handler
    ///
    /// # Example
    /// ```rust
    /// button("Click").onclick("console.log('clicked')").build()
    /// button("Save").onclick("@post('/api/save')").build()
    /// ```
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn onclick(mut self, action: impl Into<String>) -> Self {
        self.onclick = Some(action.into());
        self
    }

    /// Sets a custom test ID for the button
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn with_test_id(mut self, id: impl Into<String>) -> Self {
        self.test_id = Some(id.into());
        self
    }

    /// Sets the HTML id attribute for the button
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Adds a leading icon before the button text
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn with_leading_icon(mut self, icon: Markup) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    /// Adds a trailing icon after the button text
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn with_trailing_icon(mut self, icon: Markup) -> Self {
        self.trailing_icon = Some(icon);
        self
    }

    /// Sets the popovertarget attribute for triggering popovers/menus
    ///
    /// # Example
    /// ```rust
    /// button("Open Menu")
    ///     .popovertarget("my-menu")
    ///     .build()
    /// ```
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn popovertarget(mut self, target: impl Into<String>) -> Self {
        self.popovertarget = Some(target.into());
        self
    }

    /// Builds the button component and returns the rendered markup
    pub fn build(self) -> Markup {
        let test_id = self.test_id.unwrap_or_else(|| self.variant.test_id().to_string());

        html! {
            button
                type="button"
                id=[self.id.as_deref()]
                class=(format!("{} {}", BASE_CLASSES, self.variant.variant_classes()))
                disabled[self.disabled]
                data-on-click=[self.onclick.as_deref()]
                popovertarget=[self.popovertarget.as_deref()]
                data-testid=(test_id)
            {
                @if let Some(leading) = self.leading_icon {
                    (leading)
                }
                (self.text)
                @if let Some(trailing) = self.trailing_icon {
                    (trailing)
                }
            }
        }
    }
}

/// Creates a new button builder
///
/// This is the primary entry point for creating buttons.
///
/// # Example
/// ```rust
/// use components::button;
///
/// // Simple primary button
/// let btn = button("Click me").build();
///
/// // Customized button
/// let btn = button("Save")
///     .variant(ButtonVariant::Secondary)
///     .onclick("console.log('saved')")
///     .build();
/// ```
#[must_use = "call .build() to render the component"]
pub fn button(text: impl Into<String>) -> ButtonBuilder {
    ButtonBuilder::new(text)
}

/// Builder for constructing an icon button component
///
/// # Example
/// ```rust
/// use components::{icon_button, icon, IconType};
///
/// // Simple icon button
/// let close = icon_button(icon::icon(IconType::Close)).build();
///
/// // Disabled icon button
/// let disabled = icon_button(icon::icon(IconType::Star))
///     .disabled()
///     .build();
///
/// // Icon button with custom colors
/// let colored = icon_button(icon::icon(IconType::Flag))
///     .color("text-red-500 hover:bg-red-50")
///     .onclick("console.log('flagged')")
///     .build();
/// ```
pub struct IconButtonBuilder {
    icon: Markup,
    color_class: Option<String>,
    disabled: bool,
    onclick: Option<String>,
    id: Option<String>,
    popovertarget: Option<String>,
}

impl IconButtonBuilder {
    /// Creates a new icon button builder with default settings
    pub fn new(icon: Markup) -> Self {
        Self {
            icon,
            color_class: None,
            disabled: false,
            onclick: None,
            id: None,
            popovertarget: None,
        }
    }

    /// Sets custom color classes for the icon button
    ///
    /// Overrides the default gray colors.
    ///
    /// # Example
    /// ```rust
    /// icon_button(icon).color("text-yellow-500 hover:bg-yellow-50").build()
    /// ```
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn color(mut self, color_class: impl Into<String>) -> Self {
        self.color_class = Some(color_class.into());
        self
    }

    /// Marks the icon button as disabled
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Sets the onclick DataStar action handler
    ///
    /// # Example
    /// ```rust
    /// icon_button(icon).onclick("console.log('clicked')").build()
    /// icon_button(icon).onclick("@get('/api/action')").build()
    /// ```
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn onclick(mut self, action: impl Into<String>) -> Self {
        self.onclick = Some(action.into());
        self
    }

    /// Sets the HTML id attribute for the icon button
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the popovertarget attribute for triggering popovers/menus
    ///
    /// # Example
    /// ```rust
    /// icon_button(icon::icon(IconType::Hamburger))
    ///     .popovertarget("my-menu")
    ///     .build()
    /// ```
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn popovertarget(mut self, target: impl Into<String>) -> Self {
        self.popovertarget = Some(target.into());
        self
    }

    /// Builds the icon button component and returns the rendered markup
    pub fn build(self) -> Markup {
        let color = self.color_class.as_deref().unwrap_or(DEFAULT_ICON_BUTTON_COLOR);

        html! {
            button
                type="button"
                id=[self.id.as_deref()]
                class=(format!("{} {}", ICON_BUTTON_BASE_CLASSES, color))
                disabled[self.disabled]
                data-on-click=[self.onclick.as_deref()]
                popovertarget=[self.popovertarget.as_deref()]
                data-testid="icon-button"
            {
                (self.icon)
            }
        }
    }
}

/// Creates a new icon button builder
///
/// Icon buttons are square buttons containing only an icon, commonly used for
/// compact actions like closing dialogs, opening menus, or triggering popovers.
///
/// # Example
/// ```rust
/// use components::{icon_button, icon, IconType};
///
/// // Default gray icon button
/// let close = icon_button(icon::icon(IconType::Close)).build();
///
/// // Icon button with custom colors
/// let star = icon_button(icon::icon(IconType::Star))
///     .color("text-yellow-500 hover:bg-yellow-50")
///     .onclick("console.log('starred')")
///     .build();
/// ```
#[must_use = "call .build() to render the component"]
pub fn icon_button(icon: Markup) -> IconButtonBuilder {
    IconButtonBuilder::new(icon)
}

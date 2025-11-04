# Menu

Dropdown menu component for displaying lists of actions and navigation links.

## Usage Guidelines

Menus are used to display a list of actions or navigation options triggered by a button or other interactive element. They are ideal for contextual actions, user account options, or navigation shortcuts.

## Component Architecture

The menu component uses a builder pattern for flexible construction:

### Basic Usage with Text Trigger

The recommended way to create a menu is using `with_text_trigger()`, which automatically creates a styled button trigger with the correct `popovertarget` attribute:

```rust
use components::{menu, menu_item, icon, IconType};

let star_icon = icon::icon_for_menu_item(IconType::Star);

let my_menu = menu::menu()
    .with_id("user-menu")
    .with_text_trigger("Open Menu")
    .with_item(menu_item::link_menu_item("Profile", "/profile"))
    .with_item(menu_item::link_menu_item_with_icon("Favorites", "/favorites", star_icon))
    .with_item(menu_item::menu_item_divider())
    .with_item(menu_item::button_menu_item("Sign Out"))
    .build();
```

### Menu with Icon Trigger

For a more compact UI, use `with_icon_trigger()` to create an icon button trigger. Icon buttons are keyboard accessible and semantically correct:

```rust
use components::{menu, menu_item, icon, IconType};

let icon_menu = menu::menu()
    .with_id("actions-menu")
    .with_icon_trigger(icon::icon(IconType::Hamburger))
    .with_item(menu_item::link_menu_item("Dashboard", "/dashboard"))
    .with_item(menu_item::link_menu_item("Profile", "/profile"))
    .with_item(menu_item::button_menu_item("Sign Out"))
    .build();
```

### Advanced: Custom Trigger Button

If you need full control over the trigger button, create your own button with the `popovertarget` attribute and pass it to `with_trigger()`:

```rust
use components::{menu, menu_item, button, icon, IconType};

let custom_trigger = html! {
    button
        type="button"
        popovertarget="custom-menu"
        class="custom-button-classes"
    {
        "Custom Trigger"
    }
};

let my_menu = menu::menu()
    .with_id("custom-menu")
    .with_trigger(custom_trigger)
    .with_item(menu_item::link_menu_item("Profile", "/profile"))
    .build();
```

### External Trigger Button

Alternatively, you can create the menu without any trigger and place the trigger button separately:

```rust
let my_menu = menu::menu()
    .with_id("user-menu")
    .with_item(menu_item::link_menu_item("Profile", "/profile"))
    .build();

html! {
    div {
        button popovertarget="user-menu" class="custom-button-classes" {
            "Separate Trigger"
        }
        (my_menu)
    }
}
```

## Builder Methods

### `with_id(id: impl Into<String>)`

Sets the menu ID (required for popover targeting). The ID must match the `popovertarget` attribute on the trigger button.

### `with_text_trigger(text: impl Into<String>)`

Creates a styled text button trigger for the menu. The button automatically includes the correct `popovertarget` attribute pointing to the menu ID.

**Example:**
```rust
menu::menu()
    .with_id("my-menu")
    .with_text_trigger("Open Menu")
```

### `with_icon_trigger(icon: Markup)`

Creates an icon button trigger for the menu. Icon buttons are compact, keyboard accessible, and semantically correct. The button automatically includes the correct `popovertarget` attribute.

**Example:**
```rust
menu::menu()
    .with_id("my-menu")
    .with_icon_trigger(icon::icon(IconType::Hamburger))
```

### `with_trigger(trigger_button: Markup)`

Sets a custom trigger button for the menu. Use this for full control over trigger styling. The trigger should be a complete button element with a `popovertarget` attribute matching the menu ID.

For most use cases, prefer `with_text_trigger()` or `with_icon_trigger()` instead.

**Example:**
```rust
let custom = html! {
    button popovertarget="my-menu" class="..." { "Custom" }
};

menu::menu()
    .with_id("my-menu")
    .with_trigger(custom)
```

### `with_item(item: Markup)`

Adds a single menu item. Items can be links, buttons, or dividers from the `menu_item` module.

### `with_items(items: Vec<Markup>)`

Adds multiple menu items at once.

### `build()`

Renders the menu component and returns the final markup. If a trigger was provided, returns a container with both the trigger button and menu. Otherwise, returns only the menu element.

## Conditional Menu Building

The builder pattern supports conditional item addition:

```rust
let mut builder = menu::menu()
    .with_id("actions-menu")
    .with_item(menu_item::link_menu_item("View", "/view"));

if user.is_admin() {
    builder = builder
        .with_item(menu_item::menu_item_divider())
        .with_item(menu_item::link_menu_item("Admin Panel", "/admin"));
}

let menu = builder.build();
```

## Composition with Menu Items

Menus work seamlessly with all menu item types:

- **Link Menu Items** - For navigation (`menu_item::link_menu_item()`)
- **Button Menu Items** - For actions (`menu_item::button_menu_item()`)
- **Dividers** - For grouping (`menu_item::menu_item_divider()`)
- **Icons** - Using `_with_icon` variants

## Positioning

The menu automatically positions itself optimally based on available screen space. If the menu would overflow the viewport, it adjusts its position automatically. This is handled by the CSS Anchor Positioning API and TailwindUI Elements.

## Accessibility Notes

- Uses native `<el-menu>` element from TailwindUI (built on Popover API)
- Keyboard accessible (Escape to close, Arrow keys for navigation)
- Focus management handled automatically
- ARIA attributes managed by the browser's Popover API
- Proper semantic structure with menu role

## Implementation Status

✅ Complete - Production ready with full dark mode support and animation

## Design Notes

The menu component follows TailwindUI design patterns with DSP brand customizations:
- Fixed width (`w-56`) for consistency
- Shadow and outline for elevation
- Smooth animations (`transition-discrete`)
- Entry/exit animations with scale and opacity
- Dark mode support with appropriate contrasts
- Anchor positioning via CSS anchor positioning API

### TailwindUI Elements Integration

This component uses `<el-menu>` from TailwindUI Elements, which provides:
- Automatic positioning via CSS anchor positioning
- Built-in animations and transitions
- Popover API integration
- Light dismissal (click outside to close)

### Animation States

- `data-closed` - Menu is hidden (scaled down, transparent)
- `data-enter` - Menu is appearing (100ms ease-out)
- `data-leave` - Menu is disappearing (75ms ease-in)

## Examples

### Basic Navigation Menu

```rust
menu::menu()
    .with_id("nav-menu")
    .with_item(menu_item::link_menu_item("Dashboard", "/dashboard"))
    .with_item(menu_item::link_menu_item("Projects", "/projects"))
    .with_item(menu_item::link_menu_item("Team", "/team"))
    .build()
```

### User Account Menu

```rust
menu::menu()
    .with_id("account-menu")
    .with_item(menu_item::link_menu_item("Your Profile", "/profile"))
    .with_item(menu_item::link_menu_item("Settings", "/settings"))
    .with_item(menu_item::menu_item_divider())
    .with_item(menu_item::button_menu_item("Sign Out"))
    .build()
```

### Context Menu with Actions

```rust
use components::{menu, menu_item, icon, IconType};

menu::menu()
    .with_id("context-menu")
    .with_item(menu_item::button_menu_item_with_icon(
        "Share",
        icon::icon_for_menu_item(IconType::Star)
    ))
    .with_item(menu_item::button_menu_item_with_icon(
        "Download",
        icon::icon_for_menu_item(IconType::Code)
    ))
    .with_item(menu_item::menu_item_divider())
    .with_item(menu_item::button_menu_item_with_icon(
        "Delete",
        icon::icon_for_menu_item(IconType::Flag)
    ))
    .build()
```

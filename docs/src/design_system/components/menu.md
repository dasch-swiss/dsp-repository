# Menu

Dropdown menu component for displaying lists of actions and navigation links.

## Usage Guidelines

Menus are used to display a list of actions or navigation options triggered by a button or other interactive element. They are ideal for contextual actions, user account options, or navigation shortcuts.

## Component Architecture

The menu component uses a builder pattern for flexible construction:

### Basic Usage

```rust
use components::{menu, menu_item};

let my_menu = menu::menu()
    .with_id("user-menu")
    .with_item(menu_item::link_menu_item("Profile", "/profile"))
    .with_item(menu_item::link_menu_item("Settings", "/settings"))
    .with_item(menu_item::menu_item_divider())
    .with_item(menu_item::button_menu_item("Sign Out"))
    .build();
```

### Triggering the Menu

Menus are triggered using the `popovertarget` attribute on a button:

```rust
html! {
    button popovertarget="user-menu" class="..." {
        "Open Menu"
    }
    (my_menu)
}
```

## Builder Methods

### `with_id(id: impl Into<String>)`

Sets the menu ID (required for popover targeting). The ID must match the `popovertarget` attribute on the trigger button.

### `with_item(item: Markup)`

Adds a single menu item. Items can be links, buttons, or dividers from the `menu_item` module.

### `with_items(items: Vec<Markup>)`

Adds multiple menu items at once.

### `build()`

Renders the menu component and returns the final markup.

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
menu::menu()
    .with_id("context-menu")
    .with_item(menu_item::button_menu_item_with_icon("Share", share_icon))
    .with_item(menu_item::button_menu_item_with_icon("Download", download_icon))
    .with_item(menu_item::menu_item_divider())
    .with_item(menu_item::button_menu_item_with_icon("Delete", delete_icon))
    .build()
```

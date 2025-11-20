# Menu

Dropdown menu component for displaying lists of actions and navigation links.

## Usage Guidelines

Menus are used to display a list of actions or navigation options triggered by a button or other interactive element. They are ideal for contextual actions, user account options, or navigation shortcuts.

## Component Architecture

The menu component uses a builder pattern for flexible construction. You can trigger menus using button components, external buttons, or programmatically via DataStar/JavaScript.

### Menu with Button Trigger

Use the button builder's `.popovertarget()` method to connect the button to the menu:

```rust
use components::{menu, menu_item, button, icon, IconType};

let star_icon = icon::icon(IconType::Star);

// Build the items vector separately
let items = vec![
    // Navigation links
    menu_item::link_menu_item("Edit", "/edit"),
    menu_item::link_menu_item("Duplicate", "/duplicate"),
    menu_item::menu_item_divider(),
    // Action buttons with icons
    menu_item::button_menu_item_with_icon("Share", star_icon.clone()),
    menu_item::button_menu_item_with_icon("Archive", code_icon.clone()),
    menu_item::menu_item_divider(),
    // Destructive action
    menu_item::button_menu_item_with_icon("Delete", flag_icon.clone()),
];

let my_menu = menu::menu()
    .with_id("user-menu")
    .with_trigger(
        button::button("Open Menu")
            .with_id("menu-trigger")
            .popovertarget("user-menu")
            .build()
    )
    .with_items(items)
    .build();
```

### Menu with Icon Button Trigger

For a more compact UI, use an icon button trigger:

```rust
use components::{menu, menu_item, button, icon, IconType};

let icon_menu = menu::menu()
    .with_id("actions-menu")
    .with_trigger(
        button::icon_button(icon::icon(IconType::Hamburger))
            .with_id("actions-trigger")
            .popovertarget("actions-menu")
            .build()
    )
    .with_item(menu_item::link_menu_item("Dashboard", "/dashboard"))
    .with_item(menu_item::link_menu_item("Profile", "/profile"))
    .with_item(menu_item::button_menu_item("Sign Out"))
    .build();
```

### External Trigger Buttons

Create the menu without a trigger and place trigger buttons anywhere in your HTML. Multiple buttons can trigger the same menu:

```rust
let my_menu = menu::menu()
    .with_id("user-menu")
    .with_item(menu_item::link_menu_item("Profile", "/profile"))
    .with_item(menu_item::link_menu_item("Settings", "/settings"))
    .build();

html! {
    div {
        (button::button("Trigger 1")
            .popovertarget("user-menu")
            .build())
        (button::button("Trigger 2")
            .variant(ButtonVariant::Secondary)
            .popovertarget("user-menu")
            .build())
        (my_menu)
    }
}
```

### Programmatic Triggering via DataStar

Trigger menus programmatically using DataStar onclick handlers or JavaScript:

```rust
// Using DataStar onclick
html! {
    (button::button("Open Menu")
        .onclick("document.getElementById('my-menu').showPopover()")
        .build())

    (menu::menu()
        .with_id("my-menu")
        .with_item(menu_item::link_menu_item("Dashboard", "/dashboard"))
        .build())
}

// Using DataStar with toggle
html! {
    (button::button("Toggle Menu")
        .onclick("document.getElementById('my-menu').togglePopover()")
        .build())
}
```

## Builder Methods

All menu builder methods can be chained in any order. Call `.build()` to render the final component.

- **`.with_id(impl Into<String>)`** - Sets the menu ID (required for popover targeting). Must match `popovertarget` on trigger buttons
- **`.with_test_id(impl Into<String>)`** - Sets the `data-testid` attribute for testing. Default: "menu"
- **`.with_trigger(Markup)`** - Sets a trigger button for the menu. Pass a button with `.popovertarget()` matching the menu ID
- **`.with_item(Markup)`** - Adds a single menu item (link, button, or divider from `menu_item` module)
- **`.with_items(Vec<Markup>)`** - Adds multiple menu items at once
- **`.build()`** - Consumes the builder and returns the rendered markup

### Trigger Button Examples

**With text button:**
```rust
menu::menu()
    .with_id("my-menu")
    .with_trigger(
        button::button("Open")
            .popovertarget("my-menu")
            .build()
    )
    .build()
```

**With icon button:**
```rust
menu::menu()
    .with_id("my-menu")
    .with_trigger(
        button::icon_button(icon::icon(IconType::Hamburger))
            .popovertarget("my-menu")
            .build()
    )
    .build()
```

**Note:** If no trigger is provided, control the menu externally using `.popovertarget()` on any button or via JavaScript/DataStar.

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

The menu automatically positions itself optimally based on available screen space. If the menu would overflow the viewport, it adjusts its position automatically. This is handled by the CSS Anchor Positioning API and Tailwind Elements.

## Accessibility Notes

- Uses native `<el-menu>` element from Tailwind (built on Popover API)
- Keyboard accessible (Escape to close, Arrow keys for navigation)
- Focus management handled automatically
- ARIA attributes managed by the browser's Popover API
- Proper semantic structure with menu role

## Implementation Status

✅ Complete - Production ready with full dark mode support and animation

## Design Notes

The menu component follows Tailwind design patterns with DSP brand customizations:
- Fixed width (`w-56`) for consistency
- Shadow and outline for elevation
- Smooth animations (`transition-discrete`)
- Entry/exit animations with scale and opacity
- Dark mode support with appropriate contrasts
- Anchor positioning via CSS anchor positioning API

### Tailwind Elements Integration

This component uses `<el-menu>` from Tailwind Elements, which provides:
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
        icon::icon(IconType::Star)
    ))
    .with_item(menu_item::button_menu_item_with_icon(
        "Download",
        icon::icon(IconType::Code)
    ))
    .with_item(menu_item::menu_item_divider())
    .with_item(menu_item::button_menu_item_with_icon(
        "Delete",
        icon::icon(IconType::Flag)
    ))
    .build()
```

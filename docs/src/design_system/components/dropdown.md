# Dropdown

A dropdown menu component that combines a trigger button with a menu for displaying actions or navigation options.

## Variants

### Secondary Button
A dropdown with a secondary-styled button and chevron down icon. Best for primary dropdown actions with visible labels.

### MoreVert Icon
A dropdown with a three-dot vertical icon button. Best for overflow menus with contextual actions.

### Hamburger Icon
A dropdown with a hamburger menu icon button. Best for navigation menus, especially on mobile.

## Usage

The dropdown component composes existing button and menu components. Pass a configured menu builder to any of the three dropdown functions:

```rust
use components::{dropdown, menu, menu_item};

// Secondary button dropdown with label
dropdown::dropdown_secondary(
    "actions-dropdown",
    "Options",
    menu::menu()
        .with_item(menu_item::link_menu_item("Edit", "/edit"))
        .with_item(menu_item::button_menu_item("Delete"))
)

// Icon button dropdowns
dropdown::dropdown_more_vert(
    "more-menu",
    menu::menu()
        .with_item(menu_item::link_menu_item("Settings", "/settings"))
)

dropdown::dropdown_hamburger(
    "nav-menu",
    menu::menu()
        .with_item(menu_item::link_menu_item("Home", "/"))
)
```

## Accessibility

- Uses semantic button elements for triggers
- Menu uses HTML Popover API for proper focus management
- Keyboard navigation supported (Escape to close, arrow keys to navigate)
- ARIA attributes automatically applied via menu component

## Implementation Status

✅ Complete

# Menu Item

Menu item components for use in dropdown menus and contextual menus.

## Usage Guidelines

Menu items are low-level building blocks for composing dropdown menus and action menus. Use link menu items for navigation and button menu items for triggering actions.

## Component Types

### Link Menu Item

Use `link_menu_item()` for navigating to different pages or sections. Renders as a semantic `<a>` tag with proper href attributes.

**When to use:**
- Navigating to a different page
- Linking to a different section
- Opening external resources
- Any action that changes the URL

### Button Menu Item

Use `button_menu_item()` for triggering actions. Renders as a semantic `<button>` tag with `cursor: pointer` and suitable for event handlers.

**When to use:**
- Triggering in-app actions (delete, share, copy)
- Opening modals or dialogs
- Executing JavaScript functions
- Any action that doesn't change the URL

### Menu Item Divider

Use `menu_item_divider()` to create horizontal separators between groups of menu items. Renders as an `<hr>` element with consistent styling for light and dark modes.

## Icon Support

Both link and button variants support optional icons using the `_with_icon` variants:
- `link_menu_item_with_icon(text, href, icon)`
- `button_menu_item_with_icon(text, icon)`

Icons should be provided as `Markup`. Use `icon::icon()` to create icons - the menu item component will apply the appropriate styling:

```rust
use components::{menu_item, icon, IconType};

let star_icon = icon::icon(IconType::Star);
let link_item = menu_item::link_menu_item_with_icon("Favorites", "/favorites", star_icon);

let code_icon = icon::icon(IconType::Code);
let button_item = menu_item::button_menu_item_with_icon("View Source", code_icon);
```

## Accessibility Notes

- Uses semantic HTML (`<a>` vs `<button>`) for proper screen reader support
- Includes proper focus states for keyboard navigation
- Focus outline is visible and meets WCAG contrast requirements
- Interactive elements are keyboard accessible

## Implementation Status

✅ Complete - Production ready with full dark mode support

## Design Notes

The menu item component follows TailwindUI design patterns with DSP brand customizations:
- Consistent padding and spacing (`px-4 py-2`)
- Full width (`w-full`) to span the entire menu container
- Focus states with background color changes
- Dark mode support with appropriate color contrasts
- Icon alignment and sizing (`size-5` with `mr-3` spacing)
- Button elements include `cursor: pointer` for better UX

### Semantic HTML Best Practices

This component enforces the best practice of using:
- `<a>` tags for navigation (with href)
- `<button>` tags for actions (without href)

This separation ensures proper accessibility, keyboard navigation, and semantic markup.

```

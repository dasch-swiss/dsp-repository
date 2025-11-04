# Button

Interactive button component for user actions.

## Usage Guidelines

Use buttons to trigger actions, submit forms, or navigate to different sections of the application. Choose the appropriate variant based on the action's importance and context.

## Variants

### Primary
The primary button is used for the most important action on a page. Use sparingly - typically only one primary button per page or section.

### Secondary
Secondary buttons are used for less important actions that still need emphasis. They can be used multiple times on a page.

### Outline
Outline buttons are used for tertiary actions or when you need a lighter visual treatment while maintaining accessibility.

## Icon Buttons

Icon buttons contain only an icon without text, providing a compact way to represent actions. They use subtle gray colors with hover states by default and are keyboard accessible.

### When to use icon buttons

- Compact UI spaces (toolbars, headers, mobile interfaces)
- Common actions with universally recognized icons (close, menu, search)
- Secondary actions that don't need text labels
- As triggers for menus and popovers

Icon buttons should always use semantically meaningful icons that users can recognize. Consider adding tooltips or aria-labels for accessibility.

### Basic Icon Button

```rust
use components::{button, icon, IconType};

// Default icon button with gray colors
let close_button = button::icon_button(icon::icon(IconType::Close), false);
let menu_button = button::icon_button(icon::icon(IconType::Hamburger), false);

// Disabled icon button
let disabled_button = button::icon_button(icon::icon(IconType::Star), true);
```

### Icon Buttons with Custom Colors

For custom color schemes, use `icon_button_with_color()` to override the default gray colors:

```rust
use components::{button, icon, IconType};

// Yellow star button
let star_button = button::icon_button_with_color(
    icon::icon(IconType::Star),
    Some("text-yellow-500 hover:bg-yellow-50 dark:hover:bg-yellow-950"),
    false
);

// Indigo close button
let indigo_close = button::icon_button_with_color(
    icon::icon(IconType::Close),
    Some("text-indigo-600 hover:bg-indigo-50 dark:text-indigo-400"),
    false
);

// Red danger button
let delete_button = button::icon_button_with_color(
    icon::icon(IconType::Flag),
    Some("text-red-500 hover:bg-red-50 dark:hover:bg-red-950"),
    false
);
```

## Accessibility Notes

<!-- TODO -->

Section missing...

## Implementation Status

🚧 Functional but incomplete - styling verification needed, missing accessibility features

## Design Notes

The button component follows Tailwind Plus design patterns with DSP brand customizations. Spacing and typography align with the design system tokens.

### Button vs. Link

Use buttons for actions that trigger changes or submit data. Use links for navigation to other pages or sections. Buttons should not be used for navigation purposes.

# Button

Interactive button component for user actions using the builder pattern.

## Usage Guidelines

Use buttons to trigger actions, submit forms, or navigate to different sections of the application. Choose the appropriate variant based on the action's importance and context.

## Basic Usage

The button component uses a builder pattern for flexible configuration:

```rust
use components::{button, ButtonVariant};

// Simple button with default primary variant
let btn = button("Click me").build();

// Button with custom variant
let secondary = button("Cancel")
    .variant(ButtonVariant::Secondary)
    .build();

// Button with onclick handler
let interactive = button("Save")
    .onclick("console.log('saved')")
    .build();

// Disabled button
let disabled = button("Delete")
    .disabled()
    .build();

// Combining multiple options
let custom = button("Submit")
    .variant(ButtonVariant::Primary)
    .onclick("@post('/api/submit')")
    .test_id("submit-button")
    .build();

// Button with ID for DataStar targeting
let identified = button("Target Me")
    .with_id("my-button")
    .onclick("console.log('clicked')")
    .build();

// Button with leading icon
let with_icon = button("Download")
    .with_leading_icon(icon::icon(IconType::ChevronDown))
    .onclick("console.log('downloading')")
    .build();

// Button with trailing icon
let next_button = button("Next")
    .with_trailing_icon(icon::icon(IconType::ChevronDown))
    .build();

// Button with both icons
let complex = button("Options")
    .with_leading_icon(icon::icon(IconType::Code))
    .with_trailing_icon(icon::icon(IconType::ChevronDown))
    .variant(ButtonVariant::Secondary)
    .build();
```

## Variants

### Primary
The primary button is used for the most important action on a page. Use sparingly - typically only one primary button per page or section.

### Secondary
Secondary buttons are used for less important actions that still need emphasis. They can be used multiple times on a page.

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
use components::{icon_button, icon, IconType};

// Default icon button with gray colors
let close = icon_button(icon::icon(IconType::Close)).build();

// Icon button with onclick handler
let interactive = icon_button(icon::icon(IconType::Star))
    .onclick("console.log('starred')")
    .build();

// Disabled icon button
let disabled = icon_button(icon::icon(IconType::Close))
    .disabled()
    .build();

// Icon button with ID
let identified = icon_button(icon::icon(IconType::Star))
    .with_id("star-button")
    .onclick("console.log('starred')")
    .build();
```

### Icon Buttons with Custom Colors

Use the `.color()` method to override the default gray colors:

```rust
use components::{icon_button, icon, IconType};

// Yellow star button
let star = icon_button(icon::icon(IconType::Star))
    .color("text-yellow-500 hover:bg-yellow-50 dark:hover:bg-yellow-950")
    .onclick("console.log('starred')")
    .build();

// Indigo close button
let indigo_close = icon_button(icon::icon(IconType::Close))
    .color("text-indigo-600 hover:bg-indigo-50 dark:text-indigo-400")
    .build();

// Red danger button with interaction
let delete = icon_button(icon::icon(IconType::Flag))
    .color("text-red-500 hover:bg-red-50 dark:hover:bg-red-950")
    .onclick("@post('/api/flag')")
    .build();
```

## Accessibility Notes

## Implementation Status

🚧 Functional but incomplete - styling verification needed, missing accessibility features

## Design Notes

The button component follows Tailwind Plus design patterns with DSP brand customizations. Spacing and typography align with the design system tokens.

### Button vs. Link

Use buttons for actions that trigger changes or submit data. Use links for navigation to other pages or sections. Buttons should not be used for navigation purposes.

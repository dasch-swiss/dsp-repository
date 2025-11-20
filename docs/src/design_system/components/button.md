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

// Button with custom configuration
let custom = button("Submit")
    .variant(ButtonVariant::Secondary)
    .with_id("submit-btn")
    .with_test_id("submit-button")
    .onclick("@post('/api/submit')")
    .disabled()
    .build();
```

## Builder Methods

All button builder methods can be chained in any order. Call `.build()` to render the final component.

### Text Buttons

- **`.variant(ButtonVariant)`** - Sets the button style (Primary or Secondary). Default: Primary
- **`.with_id(impl Into<String>)`** - Sets the HTML `id` attribute for the button
- **`.with_test_id(impl Into<String>)`** - Sets the `data-testid` attribute for testing
- **`.onclick(impl Into<String>)`** - Sets the DataStar action handler for click events
- **`.disabled()`** - Marks the button as disabled
- **`.with_leading_icon(Markup)`** - Adds an icon before the button text
- **`.with_trailing_icon(Markup)`** - Adds an icon after the button text
- **`.popovertarget(impl Into<String>)`** - Sets the popover target for triggering menus
- **`.build()`** - Consumes the builder and returns the rendered markup

### Icon Buttons

- **`.with_id(impl Into<String>)`** - Sets the HTML `id` attribute for the button
- **`.with_test_id(impl Into<String>)`** - Sets the `data-testid` attribute for testing
- **`.with_color(impl Into<String>)`** - Sets custom color classes (overrides default gray)
- **`.onclick(impl Into<String>)`** - Sets the DataStar action handler for click events
- **`.disabled()`** - Marks the button as disabled
- **`.popovertarget(impl Into<String>)`** - Sets the popover target for triggering menus
- **`.build()`** - Consumes the builder and returns the rendered markup

## Examples

### Button with Icon

```rust
use components::{button, icon, IconType};

let download = button("Download")
    .with_leading_icon(icon::icon(IconType::ChevronDown))
    .onclick("console.log('downloading')")
    .build();
```

### Complete Example

```rust
let complex = button("Options")
    .variant(ButtonVariant::Secondary)
    .with_id("options-button")
    .with_test_id("options-btn")
    .with_leading_icon(icon::icon(IconType::Code))
    .with_trailing_icon(icon::icon(IconType::ChevronDown))
    .onclick("@post('/api/options')")
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

Use the `.color()` method to override the default colors:

```rust
use components::{icon_button, icon, IconType};

// Yellow star button
let star = icon_button(icon::icon(IconType::Star))
    .with_color("text-yellow-500 hover:bg-yellow-50 dark:hover:bg-yellow-950")
    .onclick("console.log('starred')")
    .build();

// Indigo close button
let indigo_close = icon_button(icon::icon(IconType::Close))
    .with_color("text-indigo-600 hover:bg-indigo-50 dark:text-indigo-400")
    .build();

// Red danger button with interaction
let delete = icon_button(icon::icon(IconType::Flag))
    .with_color("text-red-500 hover:bg-red-50 dark:hover:bg-red-950")
    .onclick("@post('/api/flag')")
    .build();
```

## Accessibility Notes

## Implementation Status

🚧 Functional but incomplete - styling verification needed, missing accessibility features

## Design Notes

The button component follows Tailwind design patterns with DSP brand customizations. Spacing and typography align with the design system tokens.

### Button vs. Link

Use buttons for actions that trigger changes or submit data. Use links for navigation to other pages or sections. Buttons should not be used for navigation purposes.

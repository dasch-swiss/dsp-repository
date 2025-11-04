# Icon

Reusable icon component for displaying SVG icons from Heroicons.

## Usage Guidelines

Icons provide visual cues and improve user interface comprehension. Use icons consistently throughout your application to maintain a cohesive visual language.

## IconTypes

The icon component includes commonly used icons from Heroicons like:

- **Star** - Favorites, ratings, featured items
- **Code** - Source code, embed actions, developer tools
- **Flag** - Reporting, flagging content, markers
- **Hamburger** - Mobile menu toggle
- **Close** - Dismissing dialogs, closing modals
- **ChevronDown** - Dropdowns, expandable sections

See the components showcase for an exhausting list of available icons.

## Basic Usage

```rust
use components::{icon, IconType};

// Default styling (size-5)
let star = icon::icon(IconType::Star);
```

## Custom Styling

Icons accept custom CSS classes for size, color, and other properties:

```rust
use components::{icon, IconType};

// Custom size
let large_star = icon::icon_with_class(IconType::Star, "size-8");

// Custom color
let yellow_star = icon::icon_with_class(IconType::Star, "size-6 text-yellow-500");

// Multiple properties
let custom = icon::icon_with_class(IconType::Code, "size-10 text-blue-600 hover:text-blue-800");
```

## Menu Item Icons

For menu items, simply pass the icon and the menu item component will apply the appropriate styling:

```rust
use components::{icon, menu_item, IconType};

let star_icon = icon::icon(IconType::Star);
let menu = menu_item::link_menu_item_with_icon("Favorites", "/favorites", star_icon);
```

## Icon Sizes

Icons use Tailwind size classes:
- `size-4` - 16px (small)
- `size-5` - 20px (default)
- `size-6` - 24px (medium)
- `size-8` - 32px (large)
- `size-12` - 48px (extra large)

## Colors

Icons inherit `currentColor` by default, making them easy to style with Tailwind text color classes:

```rust
icon::icon_with_class(IconType::Star, "size-6 text-red-500")
icon::icon_with_class(IconType::Flag, "size-6 text-gray-400")
icon::icon_with_class(IconType::Code, "size-6 text-blue-500")
```

## Accessibility Notes

- All icons include `aria-hidden="true"` as they are decorative
- When icons convey meaning, ensure accompanying text is present
- Icons use semantic `<svg>` elements for proper rendering
- Dark mode compatible through currentColor inheritance

## Implementation Status

✅ Complete - Production ready with full dark mode support

## Design Notes

The icon component follows these principles:
- **Heroicons**: All icons sourced from Heroicons for consistency
- **currentColor**: Icons inherit text color for easy theming
- **Flexible sizing**: Support for all Tailwind size utilities
- **Type safety**: Enum-based icon selection prevents typos
- **SVG optimization**: Minimal path data for performance

### SVG Structure

Icons use two patterns:
- **Filled icons** (Star, Code, Flag, ChevronDown): Use `fill="currentColor"`
- **Outline icons** (Hamburger, Close): Use `stroke="currentColor"`

Both patterns ensure proper color inheritance and dark mode support.

## Adding New Icons

To add new icons from Heroicons:

1. Add icon variant to `IconType` enum in `icon.rs`
2. Add path data in `path_data()` method
3. Specify viewBox in `view_box()` method (typically `"0 0 20 20"` or `"0 0 24 24"`)
4. Indicate if icon uses stroke in `uses_stroke()` method
5. Update documentation with new icon name and use case

## Examples

### Standalone Icons

```rust
icon::icon(IconType::Star)
icon::icon(IconType::Code)
icon::icon(IconType::Flag)
```

### Custom Styled Icons

```rust
icon::icon_with_class(IconType::Star, "size-8 text-yellow-500")
icon::icon_with_class(IconType::Close, "size-6 text-red-500 hover:text-red-700")
```

### In Menus

```rust
use components::{menu, menu_item, icon, IconType};

menu::menu()
    .with_id("actions")
    .with_item(menu_item::link_menu_item_with_icon(
        "Favorites",
        "/favorites",
        icon::icon(IconType::Star)
    ))
    .with_item(menu_item::button_menu_item_with_icon(
        "Share",
        icon::icon(IconType::Code)
    ))
    .build()
```

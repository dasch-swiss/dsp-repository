# Shell

Application shell component providing navigation and layout wrapper with configurable navigation elements.

## Usage Guidelines

Use the shell component as the main application wrapper. It provides consistent navigation, theme management, and layout structure across the application.

## API

### `shell(header_nav_elements: Vec<NavElement>)`

Create a shell with header navigation using the builder pattern.

```rust
use components::shell::{shell, NavElement, NavItem, NavMenu, NavMenuItem};

// Shell with header navigation only
let header_nav = vec![
    NavElement::Item(NavItem { label: "Home", href: "/" }),
    NavElement::Item(NavItem { label: "Projects", href: "/projects" }),
    NavElement::Menu(NavMenu {
        label: "Resources",
        items: vec![
            NavMenuItem { label: "Documentation", href: "/docs" },
            NavMenuItem { label: "Tutorials", href: "/tutorials" },
        ],
    }),
];

let shell_markup = shell(header_nav).build();
```

### `with_side_nav(side_nav_elements: Vec<NavElement>)`

Optionally add side navigation to the shell.

```rust
let side_nav = vec![
    NavElement::Item(NavItem { label: "Dashboard", href: "/dashboard" }),
    NavElement::Menu(NavMenu {
        label: "My Work",
        items: vec![
            NavMenuItem { label: "Active Projects", href: "/work/active" },
            NavMenuItem { label: "Drafts", href: "/work/drafts" },
        ],
    }),
];

let shell_markup = shell(header_nav)
    .with_side_nav(side_nav)
    .build();
```

### `with_content(content: Markup)`

Add content to the shell's main content area.

```rust
use maud::html;

let content = html! {
    div style="padding: 2rem;" {
        h1 { "Welcome to the Application" }
        p { "This is the main content area of the shell." }
    }
};

let shell_markup = shell(header_nav)
    .with_content(content)
    .build();
```

### Complete Example

Full shell configuration with navigation and content:

```rust
use components::shell::{shell, NavElement, NavItem, NavMenu, NavMenuItem};
use maud::html;

let header_nav = vec![
    NavElement::Item(NavItem { label: "Home", href: "/" }),
    NavElement::Item(NavItem { label: "Projects", href: "/projects" }),
    NavElement::Menu(NavMenu {
        label: "Resources",
        items: vec![
            NavMenuItem { label: "Documentation", href: "/docs" },
            NavMenuItem { label: "API Reference", href: "/api" },
        ],
    }),
];

let side_nav = vec![
    NavElement::Item(NavItem { label: "Dashboard", href: "/dashboard" }),
    NavElement::Menu(NavMenu {
        label: "My Work",
        items: vec![
            NavMenuItem { label: "Active Projects", href: "/work/active" },
            NavMenuItem { label: "Drafts", href: "/work/drafts" },
        ],
    }),
];

let content = html! {
    div style="padding: 2rem; max-width: 1200px; margin: 0 auto;" {
        h1 { "Application Dashboard" }
        p { "Welcome to your personalized dashboard." }
        
        div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1rem; margin-top: 2rem;" {
            div style="padding: 1rem; background: var(--cds-background-hover); border-radius: 4px;" {
                h3 { "Recent Activity" }
                p { "Your latest updates and changes." }
            }
            div style="padding: 1rem; background: var(--cds-background-hover); border-radius: 4px;" {
                h3 { "Quick Actions" }
                p { "Frequently used tools and shortcuts." }
            }
        }
    }
};

let shell_markup = shell(header_nav)
    .with_side_nav(side_nav)
    .with_content(content)
    .build();
```

## Builder Pattern

The shell uses a builder pattern for clean and flexible configuration:

- **Required**: Header navigation elements must be provided
- **Optional**: Side navigation can be added with `with_side_nav()`
- **Optional**: Content can be added with `with_content()`
- **Fluent API**: Method chaining for readable configuration
- **Type Safety**: Cannot create shell without header navigation

## Navigation Structure

The shell supports flexible navigation with two types of elements:

### Navigation Items
Single navigation links that appear in both header and side navigation.

```rust
NavElement::Item(NavItem { label: "Home", href: "/" })
```

### Navigation Menus
Dropdown menus with multiple items that appear as dropdowns in header navigation and expandable menus in side navigation.

```rust
NavElement::Menu(NavMenu {
    label: "Resources",
    items: vec![
        NavMenuItem { label: "Documentation", href: "/docs" },
        NavMenuItem { label: "API Reference", href: "/api" },
    ],
})
```

## Features

- **Configurable Navigation**: Support for both navigation items and dropdown menus
- **Responsive Design**: Adapts to different screen sizes with collapsible side navigation
- **Search Functionality**: Integrated search with accessibility features
- **Theme Toggle**: Light/dark theme switching with persistence
- **Static Branding**: DaSCH Service Platform branding remains consistent
- **Accessible ARIA Labels**: Full accessibility support

## Accessibility Notes

- Comprehensive ARIA labels and roles
- Keyboard navigation support
- Screen reader optimized structure
- Focus management for modal states
- High contrast theme support
- Semantic navigation structure

## Implementation Status

✅ Fully functional with configurable navigation

## Design Notes

Currently implemented using Carbon Web Components via CDN. Provides advanced features including responsive navigation, search functionality, and theme management. The shell serves as the primary application wrapper and maintains consistent layout across different pages.

Navigation content is now configurable while maintaining static branding elements and core functionality.

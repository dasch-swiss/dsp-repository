# Shell

Application shell component providing navigation and layout wrapper with configurable navigation elements.

## Usage Guidelines

Use the shell component as the main application wrapper. It provides consistent navigation, theme management, and layout structure across the application.

## Basic Usage

The shell component uses a builder pattern to create the application layout:

```rust
use components::shell::shell;
use components::header::{HeaderConfig, NavElement, NavItem};
use components::footer::FooterConfig;
use maud::html;

let shell_markup = shell(header_nav, header_config, footer_config)
    .with_content(html! {
        div { "Page content here" }
    })
    .build();
```

## Builder Methods

The shell builder has only one optional method. Call `.build()` to render the final component.

- **`.with_content(Markup)`** - Sets the main content area of the shell. If not provided, displays a default "No content provided" message
- **`.build()`** - Consumes the builder and returns the rendered markup with header, content, and footer

**Note:** The shell constructor requires three parameters: navigation elements, header config, and footer config. These are not builder methods but constructor parameters.

## Configuration Structs

### HeaderConfig

```rust
HeaderConfig {
    company_name: "DaSCH Service Platform",
    logo_light_url: "/assets/logo-light.svg",
    logo_dark_url: "/assets/logo-dark.svg",
    login_href: "/login",
}
```

### FooterConfig

```rust
FooterConfig {
    company_name: "DaSCH",
    description: "Digital infrastructure for humanities research.",
    copyright_text: "© 2024 DaSCH, University of Basel.",
    logo_light_url: "/assets/footer-logo-light.svg",
    logo_dark_url: "/assets/footer-logo-dark.svg",
}
```

### Complete Example

Full shell configuration with navigation and content:

```rust
use components::shell::shell;
use components::header::{HeaderConfig, NavElement, NavItem, NavMenu, NavMenuItem};
use components::footer::FooterConfig;
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

let header_config = HeaderConfig {
    company_name: "DaSCH Service Platform",
    logo_light_url: "/assets/logo-light.svg",
    logo_dark_url: "/assets/logo-dark.svg",
    login_href: "/login",
};

let footer_config = FooterConfig {
    company_name: "DaSCH",
    description: "Digital infrastructure for humanities research data preservation and discovery.",
    copyright_text: "© 2024 DaSCH, University of Basel. All rights reserved.",
    logo_light_url: "/assets/footer-logo-light.svg",
    logo_dark_url: "/assets/footer-logo-dark.svg",
};

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

let shell_markup = shell(header_nav, header_config, footer_config)
    .with_content(content)
    .build();
```

## Builder Pattern

The shell uses a builder pattern for clean and flexible configuration:

- **Required**: Header navigation elements, header configuration, and footer configuration must be provided
- **Optional**: Content can be added with `with_content()`
- **Fluent API**: Method chaining for readable configuration
- **Type Safety**: Cannot create shell without required parameters

## Navigation Structure

The shell supports flexible navigation with two types of elements:

### Navigation Items
Single navigation links that appear in the header navigation.

```rust
NavElement::Item(NavItem { label: "Home", href: "/" })
```

### Navigation Menus
Dropdown menus with multiple items that appear as dropdowns in header navigation.

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
- **Responsive Design**: Adapts to different screen sizes with responsive navigation
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

Currently transitioning from web components to Tailwind Plus implementation. Provides advanced features including responsive navigation, search functionality, and theme management. The shell serves as the primary application wrapper and maintains consistent layout across different pages.

Navigation content is now configurable while maintaining static branding elements and core functionality.

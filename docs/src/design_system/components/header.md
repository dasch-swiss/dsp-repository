# Header

Application header component with responsive navigation and mobile menu support.

⚠️ **Usage Note**: This component should not be used directly in applications. Use it through a higher-order shell component that provides complete page layout and structure.

⚠️ **Brand Adjustment Required**: This Tailwind UI component has not yet been adjusted to fit the DaSCH brand identity.

## Features

- Logo and company branding with light/dark mode support
- Desktop navigation with dropdown menus
- Mobile-responsive hamburger menu
- Login link
- Full accessibility support with ARIA attributes
- DataStar integration for interactive elements

## Configuration

The header requires two main configuration elements:

### HeaderConfig
- `company_name`: Company name for accessibility
- `logo_light_url`: URL for light theme logo
- `logo_dark_url`: URL for dark theme logo
- `login_href`: URL for login link

### Navigation Elements
Navigation supports both simple items and dropdown menus:
- `NavElement::Item`: Simple navigation link
- `NavElement::Menu`: Dropdown menu with sub-items

## Usage

```rust
use components::header::{header, HeaderConfig, NavElement, NavItem, NavMenu, NavMenuItem};

let config = HeaderConfig {
    company_name: "DaSCH Service Platform",
    logo_light_url: "/assets/logos/dasch-light.svg",
    logo_dark_url: "/assets/logos/dasch-dark.svg",
    login_href: "/login",
};

let nav_elements = vec![
    NavElement::Item(NavItem {
        label: "Projects",
        href: "/projects"
    }),
    NavElement::Menu(NavMenu {
        label: "Resources",
        items: vec![
            NavMenuItem { label: "Documentation", href: "/docs" },
            NavMenuItem { label: "Support", href: "/support" },
            NavMenuItem { label: "API", href: "/api" },
        ],
    }),
    NavElement::Item(NavItem {
        label: "About",
        href: "/about"
    }),
];

let markup = header(nav_elements, &config);
```

## Interactive Features

- **Mobile Menu**: Responsive hamburger menu for mobile devices
- **Dropdown Menus**: Desktop dropdown navigation with keyboard support
- **Dark Mode**: Automatic logo switching based on theme
- **DataStar Commands**: Uses DataStar for menu interactions

## Shell Integration

This component is designed to be used within a shell component:

```rust
pub fn page_shell(content: Markup) -> Markup {
    html! {
        (header(nav_elements, &header_config))
        main { (content) }
        // ... other shell components
    }
}
```

## Brand Customization Required

Before production use, customize for DaSCH brand:

1. Replace placeholder logos with actual DaSCH logos
2. Update color scheme to match DaSCH brand colors
3. Adjust typography to use DaSCH font family
4. Update navigation items for actual site structure
5. Customize login link behavior

## Accessibility

- Semantic HTML structure with proper navigation landmarks
- Screen reader support with `sr-only` text
- Keyboard navigation support
- ARIA attributes for interactive elements
- Focus management for mobile menu

## Implementation Status

✅ Migrated from experimental implementation
✅ Responsive design with mobile menu
✅ DataStar integration for interactions
⚠️ DaSCH brand customization required
⚠️ Shell component integration pending
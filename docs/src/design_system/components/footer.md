# Footer

Comprehensive site footer component with company information, navigation links, social media, and legal information.

⚠️ **Usage Note**: This component should not be used directly in applications. Use it through a higher-order shell component that provides complete page layout and structure.

⚠️ **Brand Adjustment Required**: This Tailwind UI component has not yet been adjusted to fit the DaSCH brand identity.

## Features

- Company logo with light/dark mode support
- Company description text
- Social media links with SVG icons
- Multi-column navigation sections
- Copyright information
- Responsive grid layout
- Dark mode support
- Semantic HTML structure

## Configuration

The footer uses a single `FooterConfig` struct:

### FooterConfig
- `company_name`: Company name for logo alt text
- `description`: Company description text
- `copyright_text`: Copyright notice text
- `logo_light_url`: URL for light theme logo
- `logo_dark_url`: URL for dark theme logo

## Usage

```rust
use components::footer::{footer, FooterConfig};

let config = FooterConfig {
    company_name: "DaSCH",
    description: "Digital infrastructure for humanities research data preservation, discovery, and long-term access.",
    copyright_text: "© 2024 DaSCH, University of Basel. All rights reserved.",
    logo_light_url: "/assets/logos/dasch-light.svg",
    logo_dark_url: "/assets/logos/dasch-dark.svg",
};

let markup = footer(&config);
```

## Content Sections

### Company Information
- Logo with automatic theme switching
- Descriptive text about the organization
- Social media links

### Navigation Links
The footer includes four predefined navigation sections:

1. **Solutions**: Marketing, Analytics, Automation, Commerce, Insights
2. **Support**: Submit ticket, Documentation, Guides
3. **Company**: About, Blog, Jobs, Press
4. **Legal**: Terms of service, Privacy policy, License

### Social Media
Pre-configured social media icons and links:
- Facebook
- Instagram
- X (Twitter)
- GitHub
- YouTube

## Layout Structure

### Desktop Layout
- Three-column grid layout
- Company info in first column
- Navigation links in remaining columns
- Copyright section spans full width

### Mobile Layout
- Single column stacked layout
- Proper spacing and typography adjustments
- Maintained visual hierarchy

## Shell Integration

This component is designed to be used at the bottom of shell components:

```rust
pub fn page_shell(content: Markup) -> Markup {
    html! {
        (header(nav_elements, &header_config))
        main { (content) }
        (footer(&footer_config))
    }
}
```

## Brand Customization Required

Before production use, customize for DaSCH brand:

1. Replace placeholder logos with actual DaSCH logos
2. Update navigation sections with relevant DaSCH links
3. Update social media links to actual DaSCH accounts
4. Customize company description for DaSCH
5. Update color scheme to match DaSCH brand
6. Adjust typography to use DaSCH font family
7. Replace generic footer links with actual page URLs

## Accessibility

- Semantic footer element with proper landmarks
- Screen reader friendly structure
- Proper heading hierarchy (h2, h3)
- Alt text support for company logos
- Keyboard navigation support for all links
- Hidden heading for screen readers (`sr-only`)

## Social Media Integration

The component includes SVG icons for major social platforms:
- Proper `aria-hidden` attributes on decorative SVGs
- Screen reader text for platform identification
- Hover states for better user interaction

## Responsive Features

- **Mobile**: Single column with adjusted spacing
- **Medium Screens**: Two-column navigation grid
- **Large Screens**: Three-column layout with company info
- **Extra Large**: Maximum width constraints

## Implementation Status

✅ Migrated from experimental implementation
✅ Responsive multi-column layout
✅ Social media icons and accessibility
✅ Semantic HTML structure
⚠️ DaSCH brand customization required
⚠️ Shell component integration pending
⚠️ Actual navigation links need configuration
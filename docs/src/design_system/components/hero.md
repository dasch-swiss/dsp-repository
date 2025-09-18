# Hero

Landing page hero section component with headline, description, call-to-action buttons, and hero image.

⚠️ **Usage Note**: This component should not be used directly in applications. Use it through a higher-order shell component that provides complete page layout and structure.

⚠️ **Brand Adjustment Required**: This Tailwind UI component has not yet been adjusted to fit the DaSCH brand identity.

## Features

- Large, prominent headline with responsive typography
- Descriptive text with proper text hierarchy
- Announcement banner with link
- Primary and secondary call-to-action buttons
- Hero image with decorative SVG elements
- Responsive design that adapts to different screen sizes
- Dark mode support

## Configuration

The hero component uses a single `HeroConfig` struct:

### HeroConfig
- `headline`: Main hero headline text
- `description`: Supporting description text
- `announcement_text`: Text for the announcement banner
- `announcement_link_text`: Link text within announcement
- `announcement_href`: URL for announcement link
- `primary_button_text`: Text for primary CTA button
- `primary_button_href`: URL for primary CTA
- `secondary_button_text`: Text for secondary CTA button
- `secondary_button_href`: URL for secondary CTA
- `image_url`: URL for hero image
- `image_alt`: Alt text for hero image

## Usage

```rust
use components::hero::{hero, HeroConfig};

let config = HeroConfig {
    headline: "DaSCH Service Platform",
    description: "Long-term archive for humanities research data with discovery and presentation tools for researchers worldwide.",
    announcement_text: "New data management features now available",
    announcement_link_text: "Learn more",
    announcement_href: "/news/data-management-update",
    primary_button_text: "Get Started",
    primary_button_href: "/register",
    secondary_button_text: "View Demo",
    secondary_button_href: "/demo",
    image_url: "/assets/images/platform-hero.jpg",
    image_alt: "DaSCH platform interface showing research data",
};

let markup = hero(&config);
```

## Design Elements

### Layout
- Two-column layout on large screens
- Single column stacked layout on mobile
- Content area with max-width constraints
- Hero image fills right half on desktop

### Typography
- Large headline (text-5xl to text-7xl responsive)
- Medium-weight description text
- Proper text hierarchy and spacing

### Interactive Elements
- Primary button with strong visual emphasis
- Secondary button with subtle styling
- Announcement banner with hover effects
- Responsive button sizing

## Shell Integration

This component is designed to be used within a shell component's main content area:

```rust
pub fn landing_page_shell() -> Markup {
    html! {
        (header(nav_elements, &header_config))
        main {
            (hero(&hero_config))
            // ... other page content
        }
        (footer(&footer_config))
    }
}
```

## Brand Customization Required

Before production use, customize for DaSCH brand:

1. Update color scheme (currently uses generic indigo)
2. Replace placeholder content with actual DaSCH messaging
3. Use actual DaSCH hero images
4. Adjust button styling to match DaSCH design
5. Update typography to use DaSCH font family
6. Customize spacing and sizing for brand requirements

## Responsive Behavior

- **Mobile**: Single column layout with adjusted typography sizes
- **Tablet**: Maintains single column with optimized spacing
- **Desktop**: Two-column layout with hero image
- **Large Screens**: Maximum width constraints with centered content

## Accessibility

- Semantic HTML structure with proper headings hierarchy
- Alt text support for hero images
- Proper focus management for interactive elements
- Screen reader friendly announcement banner
- Keyboard navigation support

## Implementation Status

✅ Migrated from experimental implementation
✅ Responsive design across all breakpoints
✅ Semantic HTML and accessibility features
⚠️ DaSCH brand customization required
⚠️ Shell component integration pending
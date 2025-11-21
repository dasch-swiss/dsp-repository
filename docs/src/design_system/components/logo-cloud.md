# Logo Cloud

A component for displaying a collection of partner, client, or sponsor logos in a responsive grid layout.

⚠️ **Brand Adjustment Required**: This component has not yet been adjusted to fit the DaSCH brand identity.

## Features

- Responsive grid layout adapting to different screen sizes
- Optimized positioning for visual balance (special handling for 5 logos)
- Consistent logo sizing with aspect ratio preservation
- Semantic HTML structure with accessibility support
- Variable number of logos supported

## Basic Usage

```rust
use components::logo_cloud::{logo_cloud, Logo};

let logos = vec![
    Logo::new("https://example.com/logo1.svg", "Partner 1", 158, 48),
    Logo::new("https://example.com/logo2.svg", "Partner 2", 158, 48),
    Logo::new("https://example.com/logo3.svg", "Partner 3", 158, 48),
];

let markup = logo_cloud("Trusted by leading research institutions", logos);
```

## Configuration

### logo_cloud Function

- **`title`** (`impl Into<String>`) - Title text displayed above the logo grid
- **`logos`** (`Vec<Logo>`) - Collection of Logo instances to display

### Logo Struct

Represents a single logo with its metadata. Created using `Logo::new()`:

- **`src`** - URL or path to the logo image file
- **`alt`** - Descriptive alt text for accessibility
- **`width`** - Image width attribute in pixels (recommended: 158)
- **`height`** - Image height attribute in pixels (recommended: 48)

## Layout Behavior

### Grid Configuration

The component uses a responsive CSS grid that adapts to screen size:

- **Mobile (default)**: 4 columns with 8-column gap
- **Small screens (sm:)**: 6 columns with 10-column gap
- **Large screens (lg:)**: 5 columns, full width container

### Special Positioning for 5 Logos

When exactly 5 logos are provided, the component applies special grid positioning to create optimal visual balance:

- **Logos 1-3**: Standard 2-column span, evenly distributed
- **Logo 4**: Starts at column 2 on small screens (`sm:col-start-2`)
- **Logo 5**: Starts at column 2 on mobile, automatic positioning on larger screens (`col-start-2 sm:col-start-auto`)

This creates a visually centered arrangement that prevents awkward spacing on the last row.

### Image Sizing

- Maximum height: 48px (`max-h-12`)
- Width: Full container width (`w-full`)
- Object fit: Contain (preserves aspect ratio, no cropping)
- Each logo spans 2 columns on mobile, 1 column on large screens

## Use Cases

- Displaying partner and sponsor logos
- Showcasing client portfolios
- Technology stack visualization
- Integration partner listings
- Certification and affiliation displays

## Example Usage Patterns

### Research Institution Partners

```rust
let partners = vec![
    Logo::new("/logos/university-zurich.svg", "University of Zurich", 158, 48),
    Logo::new("/logos/eth-zurich.svg", "ETH Zurich", 158, 48),
    Logo::new("/logos/snf.svg", "Swiss National Science Foundation", 158, 48),
];

logo_cloud("Supported by", partners)
```

### Technology Stack

```rust
let technologies = vec![
    Logo::new("/tech/rust.svg", "Rust", 158, 48),
    Logo::new("/tech/axum.svg", "Axum", 158, 48),
    Logo::new("/tech/maud.svg", "Maud", 158, 48),
    Logo::new("/tech/datastar.svg", "DataStar", 158, 48),
    Logo::new("/tech/tokio.svg", "Tokio", 158, 48),
];

logo_cloud("Built with", technologies)
```

## Implementation Guidelines

### Recommended Logo Specifications

- **Format**: SVG preferred, or PNG with transparency
- **Dimensions**: 158x48px or similar aspect ratio
- **Color**: Grayscale or monochrome for visual consistency
- **Background**: Transparent to work with various page backgrounds

### Content Guidelines

- Use clear, descriptive alt text (organization or product name)
- Keep title text concise and meaningful
- Ensure all logos are high quality and properly sized
- Maintain consistent visual weight across all logos
- Consider logo legibility at the constrained height (48px max)

## Accessibility

The component implements several accessibility features:

- **Semantic HTML**: Uses proper `h2` heading for the title text
- **Image alt text**: Each logo requires descriptive alt text via the `alt` parameter
- **Image dimensions**: Width and height attributes provided to prevent layout shift
- **Screen reader support**: Title text is marked with appropriate heading level
- **Keyboard navigation**: All elements are keyboard accessible (images are not interactive)

Screen readers will announce the title followed by each logo's alt text, providing context about the organizations represented.

## Responsive Behavior

- **Mobile**: Compact 4-column grid with 8-column horizontal spacing
- **Tablet (sm)**: 6-column grid with 10-column horizontal spacing
- **Desktop (lg)**: 5-column grid with full width container
- **Spacing**: Consistent vertical gaps (gap-y-10) across all breakpoints

## Brand Customization Required

Before production use, customize for DaSCH brand:

1. Replace placeholder logos with actual DaSCH partner/client logos
2. Adjust background color if needed (currently white background)
3. Ensure logo styling matches DaSCH design guidelines
4. Update title text to match DaSCH brand messaging
5. Provide logo assets in appropriate formats and dimensions

## Implementation Status

✅ Core component implementation complete
✅ Responsive grid layout with special 5-logo positioning
✅ Accessibility features implemented
✅ Unit tests passing (12/12)
✅ Playground integration complete
⚠️ DaSCH brand customization required
⚠️ Logo assets need to be provided

# Link

Hyperlinks for navigation within the application or to external resources.

## Usage Guidelines

Use links for:
- Navigation within the application
- Opening external resources (documentation, GitHub, etc.)
- Triggering actions that navigate to new content

**General design decisions for now:** Links do not have underlines, but visited links are displayed differently than unvisited. Visited have the same color as hovered links for visual consistency.

## Basic Usage

The link component uses a builder pattern for flexible configuration:

```rust
use components::{link, LinkTarget};

// Simple link (opens in same window)
let home = link("Go to homepage", "/").build();

// External link with custom configuration
let github = link("Visit GitHub", "https://github.com")
    .target(LinkTarget::Blank)
    .with_id("github-link")
    .with_test_id("github")
    .build();
```

## Builder Methods

All link builder methods can be chained in any order. Call `.build()` to render the final component.

- **`.target(LinkTarget)`** - Sets where the link opens (SelfTarget, Blank, Parent, Top). Default: SelfTarget
- **`.with_id(impl Into<String>)`** - Sets the HTML `id` attribute for the link
- **`.with_test_id(impl Into<String>)`** - Sets the `data-testid` attribute for testing. Default: "link"
- **`.build()`** - Consumes the builder and returns the rendered markup

### Link Targets

- **`LinkTarget::SelfTarget`** - Opens in the same window/tab (default)
- **`LinkTarget::Blank`** - Opens in a new tab (includes security attributes automatically)
- **`LinkTarget::Parent`** - Opens in the parent frame
- **`LinkTarget::Top`** - Opens in the top-most frame

## Examples

### Internal Link

```rust
let nav_link = link("About", "/about")
    .with_id("about-link")
    .build();
```

### External Link

```rust
let external = link("Documentation", "https://docs.example.com")
    .target(LinkTarget::Blank)
    .with_id("docs-link")
    .build();
```

### Convenience Function

For external links, use the shorthand function:

```rust
let external = link_external("Visit GitHub", "https://github.com");
// Equivalent to: link(...).target(LinkTarget::Blank).build()
```

**Note**: Links are for navigation only. If you need to trigger actions or send data via DataStar, use a button component instead.

## Security Features

External links (target="_blank") automatically include `rel="noopener noreferrer"` to prevent:
- **Tabnabbing attacks** - Prevents opened pages from accessing `window.opener`
- **Performance issues** - Ensures new page runs in separate process
- **Phishing vulnerabilities** - Blocks malicious sites from controlling the opener

## Accessibility Notes

- Semantic `<a>` element for proper navigation
- Clear visual distinction with color and hover states
- Keyboard accessible (Tab navigation, Enter to activate)
- Screen reader friendly with proper href attributes
- `data-testid` attribute support for automated testing

## Implementation Status

✅ Complete

## Styling

- **Base color**: Indigo-900 (light mode), Indigo-400 (dark mode)
- **Hover state**: Indigo-600 (light mode), Indigo-300 (dark mode)
- **Visited state**: Same as hover state for consistency
- **Underline**: Never shown
- **Cursor**: Pointer on hover
- **Font weight**: Medium (500)

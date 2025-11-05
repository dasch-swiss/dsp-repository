# Link

Hyperlinks for navigation within the application or to external resources.

## Usage Guidelines

Use links for:
- Navigation within the application
- Opening external resources (documentation, GitHub, etc.)
- Triggering actions that navigate to new content

**General design decisions for now:** Links do not have underlines, but visited links are displayed differently than unvisited. Visited have the same color as hovered links for visual consistency.

## Usage

### Default Link
Opens in the same window/tab:

```rust
link::link("Go to homepage", "/")
    .with_id("homepage-link")
    .build()
```

### External Link
Opens in a new tab with security attributes (`rel="noopener noreferrer"`):

```rust
link::link("Visit GitHub", "https://github.com")
    .target(LinkTarget::Blank)
    .with_id("github-link")
    .build()
```

Convenience function for external links:

```rust
link::link_external("Visit GitHub", "https://github.com")
```

### Link Targets
Control where the link opens:

```rust
// Same window (default)
link::link("Home", "/")
    .with_id("home-link")
    .build()

// New tab (external)
link::link("External", "https://example.com")
    .target(LinkTarget::Blank)
    .with_id("external-link")
    .build()

// Parent frame
link::link("Parent", "/parent")
    .target(LinkTarget::Parent)
    .with_id("parent-link")
    .build()

// Top-most frame
link::link("Top", "/top")
    .target(LinkTarget::Top)
    .with_id("top-link")
    .build()
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

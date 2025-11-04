# Link

Hyperlinks for navigation within the application or to external resources.

## Usage Guidelines

Use links for:
- Navigation within the application
- Opening external resources (documentation, GitHub, etc.)
- Triggering actions that navigate to new content

**Important:** Links never have underlines, and visited links maintain the same color as hovered links for visual consistency.

## Variants

### Internal
Default link that opens in the same window/tab. Use for navigation within your application.

```rust
link::link("Go to homepage", "/")
```

### External
Opens in a new tab with security attributes (`rel="noopener noreferrer"`) to prevent security vulnerabilities.

```rust
link::link_external("Visit GitHub", "https://github.com")
```

### Custom Target
For advanced use cases requiring specific target behavior:

```rust
link::link_with_target("Open in parent", "/parent", LinkTarget::Parent)
```

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

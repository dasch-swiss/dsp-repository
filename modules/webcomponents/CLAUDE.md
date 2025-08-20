# Web Components Module

This module contains reusable web components for the DSP Repository project.

## Overview

The web components are vanilla JavaScript custom elements that can be used independently of any framework. They are designed to be lightweight, accessible, and easily embeddable in any web project.

## Structure

```
modules/webcomponents/
├── attribution/                         # Attribution components for DaSCH data
│   ├── dasch-data-attribution-badge.js  # Compact badge component
│   ├── dasch-data-attribution-card.js   # Larger card component  
│   └── demo.html                        # Interactive demo and examples
└── CLAUDE.md                            # This file
```

## Components

### Attribution Components

Located in `/attribution/`, these components help mark data as being archived at DaSCH:

- **`dasch-data-attribution-badge`**: A compact "tag-like" component suitable for inline use
- **`dasch-data-attribution-card`**: A larger "card-like" component with light/dark theme support

Both components:
- Support `permalink` attribute for custom URLs (defaults to DaSCH homepage)
- Use CSS custom properties for color customization
- Are self-contained with embedded SVG logos
- Follow web component standards with Shadow DOM encapsulation

Only the card component supports the `theme` attribute (`"light"` | `"dark"`).

## Development Guidelines

When working with these components:

1. **Standards Compliance**: Follow web component standards (Custom Elements, Shadow DOM)
2. **Accessibility**: Ensure proper ARIA attributes and keyboard navigation
3. **Performance**: Keep components lightweight with minimal dependencies
4. **Styling**: Use CSS custom properties for customization, avoid !important
5. **Testing**: Test across different browsers and in various contexts

## Usage Context

These components are intended for:
- Data providers who serve large amounts of data from DaSCH
- Project-specific websites that want to attribute their data
- NOT for small amounts of data or single resources

## Documentation

Full usage documentation is available in `/docs/src/web_components/attribution_badge.md` including:
- Installation instructions
- Usage examples with code samples
- Data attribute reference
- CSS customization options
- Demo file for testing

## File Conventions

- Component files use kebab-case naming matching their custom element names
- Each component is self-contained in a single JavaScript file
- Demo files provide interactive examples for testing and documentation
- Follow the project's overall code quality standards (formatting, linting)

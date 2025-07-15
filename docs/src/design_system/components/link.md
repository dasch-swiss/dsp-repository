# Link

Link component for navigation and external links.

## Usage Guidelines

Use links for navigation between pages, sections, or external resources. The link component provides consistent styling across the application.

## Variants

### Default
Standard link styling with brand colors and hover states. Suitable for all link use cases.

## Accessibility Notes

- Proper semantic HTML anchor tags
- Clear focus indicators for keyboard navigation
- Sufficient color contrast for readability
- External links include appropriate indicators

## Implementation Status

✅ Functional - currently using Carbon Web Components as temporary implementation

## Design Notes

Currently implemented using Carbon Web Components via CDN. This provides Carbon-styled links with brand colors and hover states. Will be migrated to native Maud implementation following the component architecture patterns.

### Link vs. Button

Use links for navigation purposes. Use buttons for actions that trigger changes or submit data. Links should not be used for actions that modify state or submit forms.

<!-- TODO: what about button-styled links? -->

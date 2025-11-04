# Playground Showcase Guidelines

This document defines what should and should not be displayed in the design system playground.

## Purpose

The playground exists to provide **visual demonstrations** of components. It is not a replacement for comprehensive documentation - that belongs in `/docs/src/design_system/components/`.

## What to Showcase

Only include examples that have **visual differences** or demonstrate **different visual functionality**:

### ✅ Include

- **Variants** - Different visual styles (Primary, Secondary, Outline, etc.)
- **States** - Disabled, active, loading, error states
- **Sizes** - Small, medium, large variations
- **Color Variations** - Custom colors, semantic colors (success, warning, danger)
- **Icon Positions** - Leading icons, trailing icons, icon-only buttons
- **Visual Modifiers** - Full width, rounded, shadows, etc.
- **Interactive Behaviors** - Hover states, animations, transitions (if visually distinct)

### ❌ Exclude

- **Implementation Details** - These belong in documentation
  - Examples showing ID usage (unless it changes visual behavior)
  - Examples showing different onclick handlers
  - Examples demonstrating data attributes
  - Examples showing accessibility attributes (unless they affect visual rendering)
- **Duplicate Visual States** - Don't show the same visual twice with only code differences
- **Every Possible Combination** - Show representative examples, not exhaustive permutations

## Example: Button Component

**Good Playground Examples:**
- Primary button (visual variant)
- Secondary button (visual variant)
- Disabled primary button (visual state)
- Icon button with default color (visual variant)
- Icon button with custom color (visual difference)
- Button with leading icon (visual layout)
- Button with trailing icon (visual layout)

**Documentation-Only Examples:**
- Button with ID attribute
- Button with custom test ID
- Button with different onclick handlers
- Button with custom ARIA labels
- Button usage in forms

## Implementation

When adding playground examples:

1. **Ask**: Does this example look visually different from existing examples?
2. **If yes**: Add it to the playground
3. **If no**: Document it in the markdown docs instead

All playground examples should include IDs, onclick handlers, and other attributes for testing purposes, but these implementation details should not be the focus of the showcase.

## Rationale

- **Reduces Cognitive Load** - Developers can quickly scan visual variations
- **Faster Comprehension** - Visual differences are immediately apparent
- **Better Documentation** - Implementation details get proper explanation in docs
- **Easier Maintenance** - Fewer examples to update when APIs change
- **Clearer Purpose** - Playground = visual showcase, Docs = implementation guide

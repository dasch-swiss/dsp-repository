# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with the DSP Design System.

## Component Architecture
- **HTML Generation**: Using Maud for HTML generation
- **Composability**: Components should be composable and reusable
- **CSS Classes**: Follow `dsp-` prefix convention for all component styles
- **File Structure**: One component per file, export via `lib.rs`
- **Design Reference**: When creating new components, reference https://carbondesignsystem.com/components for patterns and best practices

## Development Workflow
- **Test in Playground**: Always test components in the playground before using in main app
- **Component Pattern**: Each component should be a simple function that generates HTML
- **Parameterization**: Components should accept parameters for customization (see TODOs)

## Current Components
- **Button**: Basic button component (needs parameterization)
- **Banner**: Banner component (needs string handling improvement)
- **Shell**: Application shell with navigation (needs search and icon implementation)

## Visual Design Guidance
When developing components, use these methods to understand design requirements:
- **Reference Images**: Screenshots/mockups can be provided and read for visual comparison
- **Reference URLs**: WebFetch existing design systems (IBM Carbon, etc.) for component patterns
- **Visual Specifications**: Detailed written descriptions of expected appearance, states, and variants
- **Design Tokens**: Follow documented color palette, spacing, and typography rules
- **Visual Testing**: Use WebFetch on playground at `http://localhost:3400/[component]` to compare against targets

## Development Commands
```bash
# Run playground with live reload
just run-watch-playground

# Access playground at http://localhost:3400
```
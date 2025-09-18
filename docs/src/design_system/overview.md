# DSP Design System

## Introduction

The DSP Design System is built with Tailwind Plus as the foundation, providing a modern, utility-first approach to component design and styling.

The system is designed with the following principles:

- It is not a general purpose design system, but a purpose-built system. As such, it is much smaller and less complex.
- It is not generic or customizable, instead the DSP brand is baked into it, thus simplifying complexity of use.
- It is purposefully kept small:
  - It comes with two themes (dark and light) with no option for custom theming.
  - The set of available styles (colors, typography, spacing, etc.)
    is kept intentionally small to promote consistent user interfaces.
  - It only has the components that are strictly needed.
    Additional components may be added, when necessary.
  - It only has the component variants that are strictly needed.
    Additional component variants may be added, when necessary.
- It may have purpose-specific components tailored to the DSP platform's unique requirements.


# Current Implementation Status

The DSP Design System is currently in early development with the following components implemented:

## Available Components

### Button
- **Implementation**: Native Maud
- **Variants**: Primary, Secondary, Outline
- **Status**: 🚧 Functional but incomplete (styling verification needed, missing accessibility features)
- **Features**: Basic button functionality with variant support, disabled state, custom test IDs

### Banner  
- **Implementation**: Native Maud
- **Variants**: Accent only, with prefix, with suffix, full (prefix + accent + suffix)
- **Status**: ✅ Fully functional
- **Features**: Semantic HTML with proper structure, configurable text sections with accent styling

### Shell
- **Implementation**: Transitioning from web components to Tailwind Plus
- **Purpose**: Application navigation and layout wrapper
- **Status**: ✅ Fully functional with advanced features
- **Features**: Responsive navigation header, search functionality, theme toggle with persistence, side navigation, accessible ARIA labels

### Tile
- **Implementation**: Native Maud with Tailwind Plus styling
- **Variants**: Base, Clickable
- **Status**: 🚧 Functional but incomplete (styling verification needed, missing accessibility features)
- **Features**: Content containers accepting arbitrary Markup, custom test IDs

## Development Environment

### Playground
- **URL**: http://localhost:3400 (via `just run-watch-playground`)
- **Architecture**: Full-featured development environment with Rust server, TypeScript testing, and visual regression
- **Features**: 
  - **Shell Interface**: Sidebar navigation with component list and active states
  - **Component Isolation**: Iframe-based component rendering with parameter controls
  - **Variant Selection**: Dynamic component variant switching with real-time updates
  - **Theme Switching**: Light/dark theme toggle with live preview
  - **Documentation Tabs**: Component and documentation view switching
  - **Live Reload**: WebSocket-based automatic refresh on file changes
- **Testing**: Comprehensive TypeScript/Playwright test suite with functional, accessibility, responsive, and visual regression tests
- **Commands**: 
  - `just playground install` - Install frontend dependencies and browsers
  - `just playground test` - Run all automated tests
  - `just playground test-visual` - Run visual regression tests
  - `just playground test-headed` - Run tests with browser visible

<!-- TODO: Add the following pages:
  - [Aim and Purpose]()
  - [Design Principles]()
  - [Design Tokens]()
  - [Components]()
  - [Patterns]()
  - [Icons]()
  - [Typography]()
  - [Colors]()
  - [Accessibility]()
  - [Design System in Figma]()
  - [Playground]()
 -->

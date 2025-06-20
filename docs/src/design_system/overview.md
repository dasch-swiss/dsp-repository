# DSP Design System

## Introduction

The DSP Design System is a customization of the [IBM Carbon Design System](https://carbondesignsystem.com/). 
It follows Carbon in terms of design language and implementation. 
It is customized in the following ways:

- It is not a general purpose design system, but a purpose-built system. As such, it is much smaller and less complex.
- It is not generic or customizeable, instead the DSP brand is baked into it, thus simplifying complexity if use.
- It is purposefully kept small:
  - It comes with two themes (dark and light, corresponding to "gray-90" and "gray-10" in Carbon) 
    and no option for custom theming.
  - The set of available styles (colors, typography, spacing, etc.) 
    is kept intentionally small to promote consistent user interfaces.
  - It only has the components that are strictly needed.
    Additional components may be added, when necessary.
  - It only has the component variants that are strictly needed. 
    Additional component variants may be added, when necessary.
- It may have purpose-specific components. 
  (E.g. Carbon does not provide a "Card" component, but rather a "Tile" component, from which cards can be built. 
  The DSP Design System instead would provide a "Card" component.)


# Current Implementation Status

The DSP Design System is currently in early development with the following components implemented:

## Available Components

### Button
- **Variants**: Primary, Secondary, Outline
- **Status**: 🚧 Work in progress (styling verification against Carbon needed)
- **Features**: Basic button functionality with variant support

### Banner  
- **Variants**: Accent only, with prefix, with suffix, full (prefix + accent + suffix)
- **Status**: ✅ Functional
- **Features**: Configurable text sections with accent styling

### Shell
- **Purpose**: Application navigation and layout wrapper
- **Status**: 🚧 Work in progress
- **Features**: Header with logo, placeholder navigation, action buttons

### Tile
- **Variants**: Base, Clickable
- **Status**: 🚧 Work in progress (styling verification against Carbon needed)
- **Features**: Content containers with Carbon-compliant styling (no borders, shadows, or rounded corners)

## Development Environment

### Playground
- **URL**: http://localhost:3400 (via `just run-watch-playground`)
- **Features**: Live component testing with structured sections and isolated examples
- **Status**: ✅ Fully functional with improved layout

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

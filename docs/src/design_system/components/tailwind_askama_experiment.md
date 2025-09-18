# TailwindAskamaExperiment

A hybrid templating experiment that combines Maud and Askama for rendering TailwindUI components.

## Usage Guidelines

This component demonstrates the integration of Askama templates with Maud wrappers, replicating the same functionality as the TailwindExperiment component but using both templating engines.

## Architecture

- **Askama Templates**: HTML templates for header, footer, and hero components
- **Maud Wrappers**: Rust functions that use Askama templates internally but expose Maud-compatible APIs
- **Hybrid Approach**: Combines the type-safety of Askama with the composability of Maud

## Variants

### Default

Complete page layout with header, hero section, and footer using the hybrid templating approach.
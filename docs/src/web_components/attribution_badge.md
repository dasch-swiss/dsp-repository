# Attribution Badge

The Attribution Badge is a web component that marks data as being archived at DaSCH.
We kindly ask data providers or project specific websites to use this badge to attribute their data,
if large amounts of data are served from DaSCH.  
We do not expect this badge to be used for small amounts of data, such as single resources.

## Badges

The badge comes in two variants, a small "tag-like" variant and a larger "card-like" variant. 
The card-like variant has a light and a dark theme.

## Usage

Currently, the web commponents are not published to a package registry. 
When published, they can be used by importing the them as a JavaScript module.

### Basic Usage

To display the badge, you can use the following HTML:

For the small variant:

```html
<dsp-attribution-badge></dsp-attribution-badge>
```

For the large variant:

```html
<dasch-data-attribution-card></dasch-data-attribution-card>
```

### Data Attributes

#### `permalink`

Both badge and card components support a `permalink` attribute that allows customizing the URL they link to. 
When no permalink is provided, components default to linking to `https://www.dasch.swiss/`. 
This attribute is useful for linking directly to specific datasets or projects archived at DaSCH.

```html
<dasch-data-attribution-badge permalink="https://ark.dasch.swiss/ark:/72163/1/0000/xyz"></dasch-data-attribution-badge>
<dasch-data-attribution-card permalink="https://ark.dasch.swiss/ark:/72163/1/0000/xyz"></dasch-data-attribution-card>
```

#### `theme`

The card component supports a `theme` attribute with two values: `"light"` (default) and `"dark"`. 
This allows the component to adapt to different design contexts. The badge component does not support theming.

```html
<dasch-data-attribution-card theme="light"></dasch-data-attribution-card>
<dasch-data-attribution-card theme="dark"></dasch-data-attribution-card>
```

While any link may work, we strongly recommend using a permalink (ARK URL) as provided by DaSCH. 
This ensures that the link remains stable and persistent over time.

### Style Customization

The components expose CSS custom properties (CSS variables) that can be customized to inject colors into the components.

We recommend not to make use of this, unless you have a specific need to adapt the colors,
e.g. for accessibility reasons.

#### Badge Component
```css
dasch-data-attribution-badge {
  --dasch-attribution-primary: hsl(206 48% 38%);    /* Badge background color */
  --dasch-attribution-text: hsl(0, 0.00%, 90.30%); /* Text color */
}
```

#### Card Component
```css
dasch-data-attribution-card {
  --dasch-attribution-primary: hsl(206 48% 38%);      /* Primary accent color */
  --dasch-attribution-secondary: hsl(210 49% 63%);    /* Secondary accent color */
  --dasch-attribution-bg-light: hsl(0, 0.00%, 96.30%); /* Light theme background */
  --dasch-attribution-bg-dark: hsl(0, 0.00%, 9.20%);   /* Dark theme background */
  --dasch-attribution-text-light: hsl(0, 0.00%, 90.30%); /* Light theme text */
  --dasch-attribution-text-dark: hsl(0, 0.00%, 15.20%);  /* Dark theme text */
}
```

## Demo

An interactive demo is available at `modules/webcomponents/attribution/demo.html`. 
The demo showcases both components with various configuration options 
including different themes, custom permalinks, and styling examples.

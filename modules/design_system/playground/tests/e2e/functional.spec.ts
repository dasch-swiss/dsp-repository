import { test, expect } from '@playwright/test';

test.describe('Design System Components - Functional Tests', () => {
  test('homepage loads correctly', async ({ page }) => {
    await page.goto('/');

    // Check title
    await expect(page).toHaveTitle('DSP Design System Playground');

    // Check navigation links are present
    await expect(page.getByRole('link', { name: 'Button' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Banner' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Shell' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Tile' })).toBeVisible();
  });

  test.describe('Button Component', () => {
    test('default fallback displays correctly', async ({ page }) => {
      await page.goto('/?component=button');

      // Get button from iframe
      const frame = page.locator('#component-iframe').contentFrame();
      const button = frame.getByTestId('button-primary');

      // Check button
      await expect(button).toBeVisible();
      await expect(button).toHaveText('Sample Button');
      await button.click();
    });

    test('primary button displays correctly', async ({ page }) => {
      await page.goto('/?component=button&theme=light&view=component&variant=primary');

      // Get button from iframe
      const frame = page.locator('#component-iframe').contentFrame();
      const button = frame.getByTestId('button-primary');

      // Check button
      await expect(button).toBeVisible();
      await expect(button).toHaveText('Sample Button');
      await button.click();
    });
  });

  test.describe('Banner Component', () => {
    test('default fallback displays correctly', async ({ page }) => {
      await page.goto('/?component=banner');

      // Get elements from iframe
      const frame = page.locator('#component-iframe').contentFrame();

      // Check that default variant loads
      await expect(frame.getByTestId('banner-accent-only')).toBeVisible();
      await expect(frame.getByTestId('banner-accent-only-accent')).toContainText('Sample Banner');
    });

    test('accent only variant displays correctly', async ({ page }) => {
      await page.goto('/?component=banner&theme=light&view=component&variant=accent_only');

      // Get elements from iframe
      const frame = page.locator('#component-iframe').contentFrame();

      // Check banner variant using test ID
      await expect(frame.getByTestId('banner-accent-only')).toBeVisible();
      await expect(frame.getByTestId('banner-accent-only-accent')).toContainText('Sample Banner');
    });

    test('with prefix variant displays correctly', async ({ page }) => {
      await page.goto('/?component=banner&theme=light&view=component&variant=with_prefix');

      // Get elements from iframe
      const frame = page.locator('#component-iframe').contentFrame();

      // Check banner variant using test ID
      await expect(frame.getByTestId('banner-with-prefix')).toBeVisible();
      await expect(frame.getByTestId('banner-with-prefix-prefix')).toContainText('Sample Prefix');
      await expect(frame.getByTestId('banner-with-prefix-accent')).toContainText('Sample Banner');
    });

    test('with suffix variant displays correctly', async ({ page }) => {
      await page.goto('/?component=banner&theme=light&view=component&variant=with_suffix');

      // Get elements from iframe
      const frame = page.locator('#component-iframe').contentFrame();

      // Check banner variant using test ID
      await expect(frame.getByTestId('banner-with-suffix')).toBeVisible();
      await expect(frame.getByTestId('banner-with-suffix-accent')).toContainText('Sample Banner');
      await expect(frame.getByTestId('banner-with-suffix-suffix')).toContainText('Sample Suffix');
    });

    test('full variant displays correctly', async ({ page }) => {
      await page.goto('/?component=banner&theme=light&view=component&variant=full');

      // Get elements from iframe
      const frame = page.locator('#component-iframe').contentFrame();

      // Check banner variant using test ID
      await expect(frame.getByTestId('banner-full')).toBeVisible();
      await expect(frame.getByTestId('banner-full-prefix')).toContainText('Sample Prefix');
      await expect(frame.getByTestId('banner-full-accent')).toContainText('Sample Banner');
      await expect(frame.getByTestId('banner-full-suffix')).toContainText('Sample Suffix');
    });
  });

  test.describe.skip('Shell Component', () => {
    test('default fallback displays correctly', async ({ page }) => {
      await page.goto('/?component=shell');

      // Get elements from iframe
      const frame = page.locator('#component-iframe').contentFrame();

      // Check shell components using test IDs
      await expect(frame.getByTestId('shell-header')).toBeVisible();
      await expect(frame.getByTestId('shell-header-logo')).toBeVisible();
      await expect(frame.getByTestId('shell-header-search')).toBeVisible();
      await expect(frame.getByTestId('shell-header-theme')).toBeVisible();
      await expect(frame.getByTestId('shell-header-side-nav')).toBeVisible();
    });

    test('header displays correctly', async ({ page }) => {
      await page.goto('/?component=shell&theme=light&view=component');

      // Get elements from iframe
      const frame = page.locator('#component-iframe').contentFrame();

      // Check shell components using test IDs
      await expect(frame.getByTestId('shell-header')).toBeVisible();
      await expect(frame.getByTestId('shell-header-logo')).toBeVisible();
      await expect(frame.getByTestId('shell-header-search')).toBeVisible();
      await expect(frame.getByTestId('shell-header-theme')).toBeVisible();
      await expect(frame.getByTestId('shell-header-side-nav')).toBeVisible();
    });
  });

  test.describe('Tile Component', () => {
    test('default fallback displays correctly', async ({ page }) => {
      await page.goto('/?component=tile');

      // Get elements from iframe
      const frame = page.locator('#component-iframe').contentFrame();

      // Check that default variant loads
      await expect(frame.getByTestId('tile-base')).toBeVisible();
    });

    test('base tile displays correctly', async ({ page }) => {
      await page.goto('/?component=tile&theme=light&view=component&variant=base');

      // Get elements from iframe
      const frame = page.locator('#component-iframe').contentFrame();

      // Check tile variant using test ID
      await expect(frame.getByTestId('tile-base')).toBeVisible();
    });

    test('clickable tile displays correctly', async ({ page }) => {
      await page.goto('/?component=tile&theme=light&view=component&variant=clickable');

      // Get elements from iframe
      const frame = page.locator('#component-iframe').contentFrame();

      // Check tile variant using test ID
      const clickableTile = frame.getByTestId('tile-clickable');
      await expect(clickableTile).toBeVisible();
      // Note: Could test navigation but keeping simple for now
    });
  });

  test.describe.skip('Responsive Design', () => {
    test('mobile viewport displays correctly', async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 667 }); // iPhone size

      await page.goto('/');

      // Test navigation links are accessible on mobile
      await expect(page.getByRole('link', { name: 'Button' })).toBeVisible();

      // Check components render properly on mobile
      await page.goto('/?component=button');
      const frame = page.locator('#component-iframe').contentFrame();
      await expect(frame.getByTestId('button-primary')).toBeVisible();
    });
  });

  test.describe.skip('Accessibility', () => {
    test('basic checks pass', async ({ page }) => {
      await page.goto('/');

      // Check for proper heading hierarchy
      await expect(page.locator('h1')).toHaveCount(1);

      // Check navigation list structure exists
      await expect(page.locator('ul')).toBeVisible();

      // Check buttons have accessible text
      await page.goto('/?component=button');
      const frame = page.locator('#component-iframe').contentFrame();
      const buttons = frame.locator('button');
      const buttonCount = await buttons.count();

      for (let i = 0; i < buttonCount; i++) {
        const button = buttons.nth(i);
        const text = await button.textContent();
        expect(text?.trim()).toBeTruthy();
      }
    });
  });
});

import { test, expect } from '@playwright/test';

test.describe('Design System Components - Functional Tests', () => {
  test('homepage loads correctly', async ({ page }) => {
    await page.goto('/');

    // Check title
    await expect(page).toHaveTitle('DSP Design System Playground');

    // Check navigation links are present
    await expect(page.getByRole('link', { name: 'Button' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Shell' })).toBeVisible();
  });

  test.describe('Button Component', () => {
    test('default fallback displays correctly', async ({ page }) => {
      await page.goto('/?component=button');

      // Get button from iframe
      const frame = page.locator('#component-store-iframe').contentFrame();
      const button = frame.getByTestId('button-primary');

      // Check button
      await expect(button).toBeVisible();
      await expect(button).toHaveText('Sample Button');
      await button.click();
    });

    test('primary button displays correctly', async ({ page }) => {
      await page.goto('/?component=button&theme=light&view=component-store&variant=primary');

      // Get button from iframe
      const frame = page.locator('#component-store-iframe').contentFrame();
      const button = frame.getByTestId('button-primary');

      // Check button
      await expect(button).toBeVisible();
      await expect(button).toHaveText('Sample Button');
      await button.click();
    });
  });

  test.describe('Icon Component', () => {
    test('close icon displays correctly', async ({ page }) => {
      await page.goto('/?component=icon&theme=light&view=component-store&variant=close');

      // Get icon from iframe
      const frame = page.locator('#component-store-iframe').contentFrame();
      const icon = frame.locator('svg');

      // Check icon exists and is visible
      await expect(icon).toBeVisible();
    });
  });

  test.describe('Logo Cloud Component', () => {
    test('default variant displays correctly', async ({ page }) => {
      await page.goto('/?component=logo-cloud&theme=light&view=component-store&variant=default');

      // Get logo cloud from iframe
      const frame = page.locator('#component-store-iframe').contentFrame();

      // Check title is visible
      await expect(frame.locator('h2')).toBeVisible();
      await expect(frame.locator('h2')).toHaveText("Trusted by the world's most innovative teams");

      // Check that logos are present
      const logos = frame.locator('img');
      await expect(logos).toHaveCount(5);

      // Verify first logo has proper attributes
      const firstLogo = logos.first();
      await expect(firstLogo).toBeVisible();
      await expect(firstLogo).toHaveAttribute('alt', 'Transistor');
      await expect(firstLogo).toHaveAttribute('width', '158');
      await expect(firstLogo).toHaveAttribute('height', '48');
    });

    test('responsive grid classes are present', async ({ page }) => {
      await page.goto('/?component=logo-cloud&theme=light&view=component-store&variant=default');

      const frame = page.locator('#component-store-iframe').contentFrame();

      // Check grid container exists with responsive classes
      const gridContainer = frame.locator('div.grid');
      await expect(gridContainer).toBeVisible();

      // Verify grid has responsive classes
      const gridClass = await gridContainer.getAttribute('class');
      expect(gridClass).toContain('grid-cols-4');
      expect(gridClass).toContain('sm:grid-cols-6');
      expect(gridClass).toContain('lg:grid-cols-5');
    });

    test('all logos load successfully', async ({ page }) => {
      await page.goto('/?component=logo-cloud&theme=light&view=component-store&variant=default');

      const frame = page.locator('#component-store-iframe').contentFrame();
      const logos = frame.locator('img');

      // Wait for all logos to be visible
      for (let i = 0; i < (await logos.count()); i++) {
        await expect(logos.nth(i)).toBeVisible();
      }

      // Check that images have loaded (naturalWidth > 0)
      const imageCount = await logos.count();
      for (let i = 0; i < imageCount; i++) {
        const logo = logos.nth(i);
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const naturalWidth = await logo.evaluate((img: any) => img.naturalWidth);
        expect(naturalWidth).toBeGreaterThan(0);
      }
    });
  });

  test.describe('Shell Component', () => {
    test.skip('default fallback displays correctly', async ({ page }) => {
      await page.goto('/?component=shell');

      // Get elements from iframe
      const frame = page.locator('#component-store-iframe').contentFrame();
      await page.waitForTimeout(500);

      // Check shell components using test IDs
      await expect(frame.getByTestId('shell-header')).toBeVisible();
      await expect(frame.getByTestId('shell-header-logo')).toBeVisible();
      await expect(frame.getByTestId('shell-header-search')).toBeVisible();
      await expect(frame.getByTestId('shell-header-theme')).toBeVisible();

      // Check content is present
      await expect(frame.getByText('Welcome to the Application Shell')).toBeVisible();

      // Check header navigation items
      await expect(frame.locator('cds-header-nav').getByText('Home')).toBeVisible();
      await expect(frame.locator('cds-header-nav').getByText('Projects')).toBeVisible();
      await expect(frame.locator('cds-header-nav').getByText('Resources')).toBeVisible();
      await expect(frame.locator('cds-header-nav').getByText('Contact')).toBeVisible();
    });

    test.skip('header-only variant displays correctly', async ({ page }) => {
      await page.goto('/?component=shell&theme=light&view=component-store&variant=header-only');

      // Get elements from iframe
      const frame = page.locator('#component-store-iframe').contentFrame();
      await page.waitForTimeout(500);

      // Check shell header components
      await expect(frame.getByTestId('shell-header')).toBeVisible();
      await expect(frame.getByTestId('shell-header-logo')).toBeVisible();
      await expect(frame.getByTestId('shell-header-search')).toBeVisible();
      await expect(frame.getByTestId('shell-header-theme')).toBeVisible();

      // Check content is present
      await expect(frame.getByText('Welcome to the Application Shell')).toBeVisible();

      // Check header navigation items
      await expect(frame.locator('cds-header-nav').getByText('Home')).toBeVisible();
      await expect(frame.locator('cds-header-nav').getByText('Projects')).toBeVisible();
      await expect(frame.locator('cds-header-nav').getByText('Resources')).toBeVisible();
      await expect(frame.locator('cds-header-nav').getByText('Contact')).toBeVisible();
    });

    test.skip('theme toggle functionality', async ({ page }) => {
      await page.goto('/?component=shell&theme=light&view=component-store');

      // Get elements from iframe
      const frame = page.locator('#component-store-iframe').contentFrame();
      await page.waitForTimeout(500);

      // Check theme toggle button is present and clickable
      const themeToggle = frame.getByTestId('shell-header-theme');
      await expect(themeToggle).toBeVisible();
      await expect(themeToggle).toHaveAttribute('aria-label', 'Toggle theme');

      // Click theme toggle (functional test - doesn't verify theme change)
      await themeToggle.click();
    });

    test.skip('search functionality', async ({ page }) => {
      await page.goto('/?component=shell&theme=light&view=component-store');

      // Get elements from iframe
      const frame = page.locator('#component-store-iframe').contentFrame();
      await page.waitForTimeout(500);

      // Check search button is present and clickable
      const searchButton = frame.getByTestId('shell-header-search');
      await expect(searchButton).toBeVisible();
      await expect(searchButton).toHaveAttribute('aria-label', 'Search');

      // Click search button (functional test)
      await searchButton.click();
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
      const frame = page.locator('#component-store-iframe').contentFrame();
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
      const frame = page.locator('#component-store-iframe').contentFrame();
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

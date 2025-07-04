import { test, expect } from '@playwright/test';

test.describe('Design System Components - Functional Tests', () => {
  test('homepage loads correctly', async ({ page }) => {
    await page.goto('/');

    // Check title
    await expect(page).toHaveTitle('Playground - Home');

    // Check main heading
    await expect(page.locator('h1')).toContainText('DSP Design System Playground');

    // Check navigation links are present
    await expect(page.locator('a[href="/button"]')).toBeVisible();
    await expect(page.locator('a[href="/banner"]')).toBeVisible();
    await expect(page.locator('a[href="/shell"]')).toBeVisible();
    await expect(page.locator('a[href="/tile"]')).toBeVisible();
  });

  test('button component page displays correctly', async ({ page }) => {
    await page.goto('/button');

    // Check page title
    await expect(page.locator('h1.playground-page-title')).toContainText('Button');

    // Check button is present using test ID
    await expect(page.getByTestId('button-primary')).toBeVisible();

    // Test button interactions
    const button = page.getByTestId('button-primary');
    await button.click();
    // Note: Add specific interaction expectations based on actual button behavior
  });

  test('banner component page displays correctly', async ({ page }) => {
    await page.goto('/banner');

    // Check page title (not banner h1s)
    await expect(page.locator('h1.playground-page-title')).toContainText('Banner');

    // Check banner variants using test IDs
    await expect(page.getByTestId('banner-accent-only')).toBeVisible();
    await expect(page.getByTestId('banner-with-prefix')).toBeVisible();
    await expect(page.getByTestId('banner-with-suffix')).toBeVisible();
    await expect(page.getByTestId('banner-full')).toBeVisible();

    // Check specific banner content
    await expect(page.getByTestId('banner-accent-only-accent')).toContainText('DaSCH');
  });

  test('shell component page displays correctly', async ({ page }) => {
    await page.goto('/shell');

    // Check page title
    await expect(page.locator('h1.playground-page-title')).toContainText('Shell');

    // Check shell components using test IDs
    await expect(page.getByTestId('shell-header')).toBeVisible();
    await expect(page.getByTestId('shell-header-logo')).toBeVisible();
    await expect(page.getByTestId('shell-header-search')).toBeVisible();
    await expect(page.getByTestId('shell-header-theme')).toBeVisible();
    await expect(page.getByTestId('shell-header-side-nav')).toBeVisible();
  });

  test('tile component page displays correctly', async ({ page }) => {
    await page.goto('/tile');

    // Check page title
    await expect(page.locator('h1.playground-page-title')).toContainText('Tile');

    // Check tile variants using test IDs (get first instance of each)
    await expect(page.getByTestId('tile-base').first()).toBeVisible();
    await expect(page.getByTestId('tile-clickable').first()).toBeVisible();

    // Test clickable tile interaction
    const clickableTile = page.getByTestId('tile-clickable').first();
    await expect(clickableTile).toBeVisible();
    // Note: Could test navigation but keeping simple for now
  });

  test('responsive design - mobile viewport', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 }); // iPhone size

    await page.goto('/');
    await expect(page.locator('h1')).toBeVisible();

    // Test navigation links are accessible on mobile
    await expect(page.locator('ul')).toBeVisible();
    await expect(page.locator('a[href="/button"]')).toBeVisible();

    // Check components render properly on mobile
    await page.goto('/button');
    await expect(page.locator('button').first()).toBeVisible();
  });

  test('accessibility - basic checks', async ({ page }) => {
    await page.goto('/');

    // Check for proper heading hierarchy
    await expect(page.locator('h1')).toHaveCount(1);

    // Check navigation list structure exists
    await expect(page.locator('ul')).toBeVisible();

    // Check buttons have accessible text
    await page.goto('/button');
    const buttons = page.locator('button');
    const buttonCount = await buttons.count();

    for (let i = 0; i < buttonCount; i++) {
      const button = buttons.nth(i);
      const text = await button.textContent();
      expect(text?.trim()).toBeTruthy();
    }
  });
});
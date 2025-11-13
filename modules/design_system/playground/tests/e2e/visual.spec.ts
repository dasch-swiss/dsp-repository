import { test, expect } from '@playwright/test';

test.describe('Design System Components - Visual Regression Tests', () => {
  test('component screenshots', async ({ page }) => {
    // Helper function to ensure fonts are loaded before taking screenshots
    const waitForFontsLoaded = async () => {
      await page.waitForLoadState('networkidle');
      // Wait for IBM Plex Sans font to be loaded
      await page.waitForFunction(() => {
        return document.fonts.ready.then(() => {
          // Check if IBM Plex Sans is available
          const testElement = document.createElement('div');
          testElement.style.fontFamily = '"IBM Plex Sans", Arial, sans-serif';
          testElement.style.position = 'absolute';
          testElement.style.visibility = 'hidden';
          testElement.textContent = 'Test';
          document.body.appendChild(testElement);

          const computedStyle = window.getComputedStyle(testElement);
          const fontFamily = computedStyle.fontFamily;
          document.body.removeChild(testElement);

          return fontFamily.includes('IBM Plex Sans') || fontFamily.includes('Arial');
        });
      });
    };

    // Button component - component store view
    await page.goto('/?component=button&view=component-store');
    await waitForFontsLoaded();

    // Wait for iframe to load
    const iframe = page.frameLocator('#component-store-iframe');
    await iframe.locator('button').first().waitFor({ state: 'visible' });

    // Take screenshot of the iframe content
    await expect(iframe.locator('body')).toHaveScreenshot(
      'button-component.png'
    );
  });
});

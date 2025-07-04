import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  
  // Only run visual tests
  testMatch: '**/visual.spec.ts',

  // Run tests in files in parallel
  fullyParallel: true,

  // Fail the build on CI if you accidentally left test.only in the source code
  forbidOnly: !!process.env.CI,

  // More retries for visual tests to handle potential rendering variations
  retries: 3,

  // Single worker for consistent visual results
  workers: 1,

  // Reporter to use
  reporter: [
    ['html', { outputFolder: 'playwright-report-visual' }],
    ['json', { outputFile: 'test-results-visual.json' }],
  ],

  use: {
    // Base URL to use in actions like `await page.goto('/')`
    baseURL: 'http://localhost:3400',

    // Collect trace when retrying the failed test
    trace: 'on-first-retry',

    // Take screenshot on failure
    screenshot: 'only-on-failure',

    // Record video on failure
    video: 'retain-on-failure',

    // Longer timeouts for visual tests
    actionTimeout: 45000,
    navigationTimeout: 45000,
  },

  // Visual regression testing configuration
  expect: {
    toHaveScreenshot: {
      // OS-independent snapshot naming (excludes project name and platform)
      pathTemplate: '{testDir}/{testFileDir}/{testFileName}-snapshots/{arg}{ext}',

      // Threshold for pixel differences (2% tolerance)
      threshold: 0.02,

      // Maximum allowed pixel differences
      maxDiffPixels: 200,

      // Animation handling for consistent screenshots
      animations: 'disabled',

      // Clip to avoid edge rendering differences
      clip: { x: 0, y: 0, width: 1200, height: 800 },
    },
  },

  // Configure only chromium for visual consistency
  projects: [
    {
      name: 'chromium-visual',
      use: { 
        ...devices['Desktop Chrome'],
        viewport: { width: 1200, height: 800 },
        launchOptions: {
          args: [
            '--disable-font-subpixel-positioning',
            '--disable-features=VizDisplayCompositor',
            '--force-device-scale-factor=1',
            '--font-render-hinting=none',
            '--disable-system-font-check',
            '--disable-font-smoothing',
            '--disable-lcd-text',
            '--disable-background-timer-throttling',
            '--disable-backgrounding-occluded-windows',
            '--disable-renderer-backgrounding',
            '--no-sandbox',
            '--disable-gpu-sandbox',
          ],
        },
      },
    },
  ],

  // Run your local dev server before starting the tests
  webServer: {
    command: process.env.CI
      ? 'cd ../../../ && ./target/release/playground-server'
      : 'cd ../../../ && cargo run --bin playground-server',
    url: 'http://127.0.0.1:3400',
    reuseExistingServer: !process.env.CI,
    timeout: 45000,
  },

  // Output directories
  outputDir: './output/visual-test-results',
});
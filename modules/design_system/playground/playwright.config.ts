import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',

  // Only run functional tests (exclude visual tests)
  testMatch: '**/functional.spec.ts',

  // Run tests in files in parallel
  fullyParallel: true,

  // Fail the build on CI if you accidentally left test.only in the source code
  forbidOnly: !!process.env.CI,

  // Retry on CI only
  retries: process.env.CI ? 2 : 0,

  // Opt out of parallel tests on CI
  workers: process.env.CI ? 1 : 4,

  // Reporter to use
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['json', { outputFile: 'test-results.json' }],
  ],

  use: {
    // Base URL to use in actions like `await page.goto('/')`
    baseURL: 'http://localhost:3400',

    // Viewport size to ensure nav elements are visible
    viewport: { width: 1440, height: 720 },

    // Collect trace when retrying the failed test
    trace: 'on-first-retry',

    // Take screenshot on failure
    screenshot: 'only-on-failure',

    // Record video on failure
    video: 'retain-on-failure',

    // Global timeout for all tests
    actionTimeout: 30000,
    navigationTimeout: 30000,
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
    },
  },

  // Configure projects for major browsers
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1440, height: 720 },
        launchOptions: {
          args: [
            '--disable-font-subpixel-positioning',
            '--disable-features=VizDisplayCompositor',
            '--force-device-scale-factor=1',
            '--font-render-hinting=none',
            '--disable-system-font-check',
            '--disable-font-smoothing',
          ],
        },
      },
    },

    // TODO: Add other browsers when needed for broader compatibility testing
    // {
    //   name: 'firefox',
    //   use: { ...devices['Desktop Firefox'] },
    // },

    // {
    //   name: 'webkit',
    //   use: { ...devices['Desktop Safari'] },
    // },
  ],

  // Run your local dev server before starting the tests
  webServer: {
    command: process.env.CI
      ? 'cd ../../../ && ./target/release/playground-server'
      : 'cd ../../../ && cargo run --bin playground-server',
    url: 'http://127.0.0.1:3400',
    reuseExistingServer: !process.env.CI,
    timeout: 30000,
  },

  // Output directories
  outputDir: './output/test-results',
});

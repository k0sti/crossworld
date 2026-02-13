import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  outputDir: './test-output',
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  retries: 0,
  use: {
    baseURL: 'http://localhost:5174/crossworld/',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    launchOptions: {
      executablePath: '/run/current-system/sw/bin/chromium',
      args: [
        '--no-sandbox',
        '--disable-gpu',
        '--use-gl=swiftshader',
        '--enable-webgl',
      ],
    },
  },
  projects: [
    {
      name: 'chromium',
      use: { browserName: 'chromium' },
    },
  ],
  webServer: {
    command: 'bun run dev',
    url: 'http://localhost:5174/crossworld/',
    reuseExistingServer: true,
    timeout: 30_000,
  },
});

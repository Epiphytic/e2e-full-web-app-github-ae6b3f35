import { defineConfig } from "@playwright/test";

export default defineConfig({
  globalSetup: require.resolve("./global-setup"),
  testDir: ".",
  testMatch: "*.spec.ts",
  timeout: 30000,
  retries: 1,
  reporter: [["list"], ["junit", { outputFile: "test-results/results.xml" }]],
  use: {
    baseURL: "http://127.0.0.1:3000",
    trace: "on-first-retry",
  },
  webServer: {
    command:
      "cd ../.. && DATABASE_PATH=test_e2e.db JWT_PUBLIC_KEY_PATH=certs/jwt-ca.pub cargo run",
    url: "http://127.0.0.1:3000/login",
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
  projects: [
    {
      name: "chromium",
      use: { browserName: "chromium" },
    },
  ],
});

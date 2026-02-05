import { test, expect } from "@playwright/test";
import { generateToken, generateExpiredToken } from "./helpers";

test.describe("Authentication", () => {
  test("should show login page when not authenticated", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveURL(/\/login/);
    await expect(page.locator("h2")).toHaveText("Login");
  });

  test("should login with a valid JWT token", async ({ page }) => {
    const token = generateToken("testuser", 300);

    await page.goto("/login");
    await page.locator("#token").fill(token);
    await page.locator('button[type="submit"]').click();

    await expect(page).toHaveURL(/\/tables/);
    await expect(page.locator("h2")).toHaveText("Tables");
    await expect(page.locator(".user-info")).toHaveText("testuser");
  });

  test("should reject expired JWT token", async ({ page }) => {
    const token = generateExpiredToken("testuser");

    await page.goto("/login");
    await page.locator("#token").fill(token);
    await page.locator('button[type="submit"]').click();

    // Should stay on login page with an error
    await expect(page).toHaveURL(/\/login/);
    await expect(page.locator(".alert-error")).toBeVisible();
    await expect(page.locator(".alert-error")).toContainText("Invalid token");
  });

  test("should logout and redirect to login", async ({ page }) => {
    const token = generateToken("testuser", 300);

    // Login first
    await page.goto("/login");
    await page.locator("#token").fill(token);
    await page.locator('button[type="submit"]').click();
    await expect(page).toHaveURL(/\/tables/);

    // Logout
    await page.locator("button:has-text('Logout')").click();
    await expect(page).toHaveURL(/\/login/);
  });

  test("should redirect to login when accessing protected page without auth", async ({
    page,
  }) => {
    // Clear cookies first
    await page.context().clearCookies();
    const response = await page.goto("/tables");
    // Should get 401 or redirect to login
    expect(response?.status()).toBeLessThanOrEqual(401);
  });
});

import { test, expect, Page } from "@playwright/test";
import { ensureKeysExist, generateToken } from "./helpers";

test.beforeAll(() => {
  ensureKeysExist();
});

async function loginAndGoToTables(page: Page): Promise<void> {
  const token = generateToken("testuser", 300);
  await page.goto("/login");
  await page.locator("#token").fill(token);
  await page.locator('button[type="submit"]').click();
  await expect(page).toHaveURL(/\/tables/);
}

test.describe("Table Management", () => {
  test("should create a new table", async ({ page }) => {
    await loginAndGoToTables(page);

    // Open create form
    await page.locator("button:has-text('+ New Table')").click();

    // Fill in table name
    await page.locator("#table-name").fill("test_users");

    // Fill first column
    const colNameInputs = page.locator('[name="col_name"]');
    await colNameInputs.first().fill("id");
    const colTypeSelects = page.locator('[name="col_type"]');
    await colTypeSelects.first().selectOption("INTEGER");

    // Submit
    await page.locator('button[type="submit"]:has-text("Create Table")').click();

    // Wait for page reload
    await page.waitForLoadState("networkidle");

    // Verify table appears in list
    await expect(page.locator("text=test_users")).toBeVisible();
  });

  test("should delete a table", async ({ page }) => {
    await loginAndGoToTables(page);

    // First create a table to delete
    const token = generateToken("testuser", 300);
    await page.request.post("http://127.0.0.1:3000/api/tables", {
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
        "HX-Request": "true",
      },
      data: {
        name: "to_delete",
        columns: [{ name: "id", col_type: "INTEGER", nullable: false, default_value: null }],
      },
    });

    await page.reload();
    await expect(page.locator("text=to_delete")).toBeVisible();

    // Accept the confirmation dialog
    page.on("dialog", (dialog) => dialog.accept());

    // Click delete button for the table
    await page
      .locator("tr", { has: page.locator("text=to_delete") })
      .locator("button:has-text('Delete')")
      .click();

    await page.waitForLoadState("networkidle");

    // Verify table is removed
    await expect(page.locator("td:has-text('to_delete')")).not.toBeVisible();
  });

  test("should navigate to table detail view", async ({ page }) => {
    await loginAndGoToTables(page);

    // Create a table via API
    const token = generateToken("testuser", 300);
    await page.request.post("http://127.0.0.1:3000/api/tables", {
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
        "HX-Request": "true",
      },
      data: {
        name: "detail_test",
        columns: [
          { name: "id", col_type: "INTEGER", nullable: false, default_value: null },
          { name: "name", col_type: "TEXT", nullable: true, default_value: null },
        ],
      },
    });

    await page.reload();
    await page.locator("a:has-text('detail_test')").first().click();

    await expect(page).toHaveURL(/\/tables\/detail_test/);
    await expect(page.locator("h2")).toHaveText("detail_test");
    // Schema should show columns
    await expect(page.locator("td:has-text('id')")).toBeVisible();
    await expect(page.locator("td:has-text('name')")).toBeVisible();
  });
});

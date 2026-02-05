import { test, expect, Page } from "@playwright/test";
import { ensureKeysExist, generateToken } from "./helpers";

test.beforeAll(() => {
  ensureKeysExist();
});

let tableCounter = 0;

function uniqueTableName(): string {
  tableCounter++;
  return `schema_test_${Date.now()}_${tableCounter}`;
}

async function loginAndCreateTable(
  page: Page,
  tableName: string,
  columns: Array<{ name: string; col_type: string; nullable: boolean; default_value: null }>
): Promise<string> {
  const token = generateToken("testuser", 300);

  // Login
  await page.goto("/login");
  await page.locator("#token").fill(token);
  await page.locator('button[type="submit"]').click();
  await expect(page).toHaveURL(/\/tables/);

  // Create table via API
  await page.request.post("http://127.0.0.1:3000/api/tables", {
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      "HX-Request": "true",
    },
    data: { name: tableName, columns },
  });

  return token;
}

test.describe("Schema Management", () => {
  test("should add a column to a table", async ({ page }) => {
    const tableName = uniqueTableName();
    const token = await loginAndCreateTable(page, tableName, [
      { name: "id", col_type: "INTEGER", nullable: false, default_value: null },
    ]);

    // Navigate to table detail
    await page.goto(`/tables/${tableName}`);
    await expect(page.locator("h2")).toHaveText(tableName);

    // Click add column button
    await page.locator("button:has-text('+ Add Column')").click();

    // Fill in new column details
    await page.locator("#new-col-name").fill("email");
    await page.locator("#new-col-type").selectOption("TEXT");
    await page.locator("#new-col-nullable").selectOption("true");

    // Submit
    await page.locator("#add-column-form button[type='submit']").click();

    await page.waitForLoadState("networkidle");

    // Verify the column appears in schema
    await expect(page.locator("td:has-text('email')")).toBeVisible();
  });

  test("should remove a column from a table", async ({ page }) => {
    const tableName = uniqueTableName();
    const token = await loginAndCreateTable(page, tableName, [
      { name: "id", col_type: "INTEGER", nullable: false, default_value: null },
      { name: "to_remove", col_type: "TEXT", nullable: true, default_value: null },
    ]);

    // Navigate to table detail
    await page.goto(`/tables/${tableName}`);
    await expect(page.locator("td:has-text('to_remove')")).toBeVisible();

    // Accept confirmation dialog
    page.on("dialog", (dialog) => dialog.accept());

    // Click remove button for the column
    await page
      .locator("tr", { has: page.locator("td:has-text('to_remove')") })
      .locator("button:has-text('Remove')")
      .click();

    await page.waitForLoadState("networkidle");

    // Verify the column is removed
    await expect(page.locator("td:has-text('to_remove')")).not.toBeVisible();
    // id column should still be there
    await expect(page.locator("td:has-text('id')")).toBeVisible();
  });

  test("should reflect column changes in table detail", async ({ page }) => {
    const tableName = uniqueTableName();
    const token = await loginAndCreateTable(page, tableName, [
      { name: "name", col_type: "TEXT", nullable: true, default_value: null },
    ]);

    // Add a column via API
    await page.request.post(
      `http://127.0.0.1:3000/api/tables/${tableName}/columns`,
      {
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
          "HX-Request": "true",
        },
        data: {
          name: "age",
          col_type: "INTEGER",
          nullable: true,
          default_value: null,
        },
      }
    );

    // Navigate to table detail
    await page.goto(`/tables/${tableName}`);

    // Verify both columns are visible
    await expect(page.locator("td:has-text('name')").first()).toBeVisible();
    await expect(page.locator("td:has-text('age')")).toBeVisible();
    await expect(page.locator("td:has-text('INTEGER')")).toBeVisible();
  });
});

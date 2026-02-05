import { test, expect, Page } from "@playwright/test";
import { generateToken } from "./helpers";

test.describe.configure({ mode: "parallel" });

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

    // Insert a row before adding the column to verify data is preserved
    const insertRes = await page.request.post(
      `http://127.0.0.1:3000/api/tables/${tableName}/rows`,
      {
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
        data: { id: "42" },
      }
    );
    expect(insertRes.ok()).toBeTruthy();

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

    // Verify existing row data is preserved after column addition
    const rowsRes = await page.request.get(
      `http://127.0.0.1:3000/api/tables/${tableName}/rows`,
      {
        headers: { Authorization: `Bearer ${token}` },
      }
    );
    expect(rowsRes.ok()).toBeTruthy();
    const rowsData = await rowsRes.json();
    expect(rowsData.total).toBe(1);
    expect(rowsData.rows.length).toBe(1);
    // The original id value should still be intact
    const row = rowsData.rows[0];
    const idIndex = rowsData.columns.indexOf("id");
    expect(row[idIndex]).toBe(42);
  });

  test("should remove a column from a table", async ({ page }) => {
    const tableName = uniqueTableName();
    const token = await loginAndCreateTable(page, tableName, [
      { name: "id", col_type: "INTEGER", nullable: false, default_value: null },
      { name: "to_remove", col_type: "TEXT", nullable: true, default_value: null },
    ]);

    // Insert rows before removing the column to verify data integrity
    // SQLite column removal requires table recreation and data copy
    for (const [id, text] of [["1", "alpha"], ["2", "beta"]]) {
      const res = await page.request.post(
        `http://127.0.0.1:3000/api/tables/${tableName}/rows`,
        {
          headers: {
            Authorization: `Bearer ${token}`,
            "Content-Type": "application/json",
          },
          data: { id, to_remove: text },
        }
      );
      expect(res.ok()).toBeTruthy();
    }

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

    // Verify existing row data in the remaining column is preserved
    const rowsRes = await page.request.get(
      `http://127.0.0.1:3000/api/tables/${tableName}/rows`,
      {
        headers: { Authorization: `Bearer ${token}` },
      }
    );
    expect(rowsRes.ok()).toBeTruthy();
    const rowsData = await rowsRes.json();
    expect(rowsData.total).toBe(2);
    expect(rowsData.rows.length).toBe(2);
    // The removed column should no longer appear in the columns list
    expect(rowsData.columns).not.toContain("to_remove");
    expect(rowsData.columns).toContain("id");
    // The id values should be preserved after table recreation
    const idIndex = rowsData.columns.indexOf("id");
    const idValues = rowsData.rows.map((row: any[]) => row[idIndex]).sort();
    expect(idValues).toEqual([1, 2]);
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

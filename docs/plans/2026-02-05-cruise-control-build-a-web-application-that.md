# Plan: SQLite Database Editor Web Application

## Overview

Build a Rust web application that provides a browser-based UI for editing SQLite databases. The application uses Axum as the web framework, htmx for the interactive frontend, JWT (RS256) authentication with a local Certificate Authority, and Playwright for end-to-end testing.

**Architecture Summary:**
- **Backend**: Rust + Axum 0.7 (current stable), serving both API endpoints and HTML templates
- **Database**: SQLite via `rusqlite` (simpler for direct DDL operations like ALTER TABLE)
- **Auth**: RS256 JWT with a local CA — the app validates tokens but does not issue them. A CLI tool or script generates tokens for testing. A `.well-known/jwks.json` endpoint exposes the public key.
- **Frontend**: Server-rendered HTML with htmx for dynamic interactions. MiniJinja templates for rendering.
- **Testing**: Playwright (Node.js) E2E tests run against the compiled binary
- **CI/CD**: GitHub Actions with Super-Linter and dependency-review-action

**Key Design Decisions:**
1. **rusqlite over sqlx** — DDL operations (CREATE TABLE, ALTER TABLE, DROP TABLE) are not well-suited to sqlx's compile-time query checking. rusqlite gives direct control over raw SQL needed for schema manipulation.
2. **External token generation** — The app only validates JWTs, keeping auth simple. A script generates short-lived tokens for testing.
3. **MiniJinja** — Interpreted templates with hot-reload support, Jinja2 syntax, and `render_block` for htmx partial responses.
4. **Playwright in Node.js** — The Node.js Playwright ecosystem is far more mature than Rust ports. Tests live in a `tests/e2e/` directory with their own `package.json`.

## Risk Areas

1. **SQLite DDL limitations** — SQLite has limited ALTER TABLE support (no DROP COLUMN before 3.35.0, no RENAME COLUMN before 3.25.0). The app must handle column removal by recreating the table.
2. **JWT key management** — The local CA private key must never be committed. Keys should be generated at build/test time and excluded via `.gitignore`.
3. **Concurrent SQLite access** — SQLite has limited write concurrency. WAL mode helps but the app should use a single connection pool with serialized writes.
4. **htmx + complex table editing** — Adding/removing columns with type definitions and constraints requires careful form design to avoid a confusing UX.
5. **Playwright test stability** — E2E tests against a local server can be flaky. Tests need proper server startup/shutdown lifecycle management.
6. **Super-Linter configuration** — Super-Linter runs many linters by default. Need to configure it to only run relevant linters (Rust/clippy, HTML, JS, YAML) to avoid false positives.

## Implementation Plan

```json
{
  "title": "SQLite Database Editor Web Application",
  "overview": "Rust/Axum web app with htmx frontend for editing SQLite databases, JWT RS256 auth with local CA, Playwright E2E tests, and GitHub Actions CI/CD",
  "spawn_instances": [
    {
      "id": "SPAWN-001",
      "name": "Project Foundation & Configuration",
      "use_spawn_team": false,
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep --timeout 300",
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "task_ids": ["CRUISE-001", "CRUISE-002", "CRUISE-003"]
    },
    {
      "id": "SPAWN-002",
      "name": "JWT Authentication & Crypto",
      "use_spawn_team": true,
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep --timeout 300",
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "task_ids": ["CRUISE-004", "CRUISE-005"]
    },
    {
      "id": "SPAWN-003",
      "name": "Database Layer & API",
      "use_spawn_team": true,
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep --timeout 300",
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "task_ids": ["CRUISE-006", "CRUISE-006A", "CRUISE-006B", "CRUISE-006C", "CRUISE-007A", "CRUISE-007B"]
    },
    {
      "id": "SPAWN-004",
      "name": "Frontend Templates & htmx",
      "use_spawn_team": false,
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep --timeout 300",
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "task_ids": ["CRUISE-008A", "CRUISE-008B", "CRUISE-009"]
    },
    {
      "id": "SPAWN-005",
      "name": "E2E Testing with Playwright",
      "use_spawn_team": true,
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep --timeout 300",
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "task_ids": ["CRUISE-010", "CRUISE-011"]
    },
    {
      "id": "SPAWN-006",
      "name": "CI/CD Pipeline",
      "use_spawn_team": false,
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Glob,Grep --timeout 180",
      "permissions": ["Read", "Write", "Edit", "Glob", "Grep"],
      "task_ids": ["CRUISE-012"]
    }
  ],
  "tasks": [
    {
      "id": "CRUISE-001",
      "subject": "Set up .gitignore for the entire project",
      "description": "Create a comprehensive .gitignore that excludes: Rust build artifacts (target/), SQLite database files (*.db, *.sqlite, *.sqlite3), private keys and certificates (*.pem, *.key, *.crt, *.p12, certs/), environment files (.env, .env.*), Node.js dependencies (node_modules/), Playwright artifacts (test-results/, playwright-report/), editor/IDE files (.vscode/, .idea/, *.swp, *.swo, *~, .DS_Store), OS files (Thumbs.db, Desktop.ini, .DS_Store), log files (*.log, logs/), .fork-join directories, and temporary files (tmp/, *.tmp).",
      "blocked_by": [],
      "complexity": "low",
      "acceptance_criteria": [
        ".gitignore file exists at project root",
        "Covers Rust build artifacts (target/)",
        "Covers private keys (*.pem, *.key, *.crt, certs/private/)",
        "Covers SQLite files (*.db, *.sqlite, *.sqlite3)",
        "Covers Node.js (node_modules/)",
        "Covers Playwright output (test-results/, playwright-report/)",
        "Covers environment files (.env, .env.*)",
        "Covers editor/IDE files (.vscode/, .idea/, *.swp, etc.)",
        "Covers OS files (.DS_Store, Thumbs.db, Desktop.ini)",
        "Covers log files (*.log, logs/)",
        "Covers .fork-join directories",
        "Covers temporary files (tmp/, *.tmp)"
      ],
      "permissions": ["Read", "Write", "Edit"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit",
      "spawn_instance": "SPAWN-001"
    },
    {
      "id": "CRUISE-002",
      "subject": "Initialize Rust project with Cargo.toml and dependencies",
      "description": "Create the Cargo.toml with all required dependencies: axum 0.7 (with macros feature), tokio (full features), rusqlite (with bundled feature for portable SQLite), jsonwebtoken (with aws_lc_rs or ring backend), minijinja (with loader feature), serde/serde_json, tower-http (cors, static file serving), tracing/tracing-subscriber for logging. Set up the basic src/ directory structure: main.rs, lib.rs, config.rs. The main.rs should have a skeleton that initializes logging, loads config, and starts the Axum server.",
      "blocked_by": ["CRUISE-001"],
      "complexity": "medium",
      "acceptance_criteria": [
        "Cargo.toml exists with all required dependencies",
        "src/main.rs compiles and starts a basic Axum server",
        "src/lib.rs exists as the library root",
        "src/config.rs handles server configuration (port, database path, JWT public key path)",
        "cargo build succeeds without errors",
        "cargo clippy passes"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash",
      "spawn_instance": "SPAWN-001"
    },
    {
      "id": "CRUISE-003",
      "subject": "Create JWT key generation script and .well-known endpoint",
      "description": "Create a script (scripts/generate-keys.sh) that generates an RSA 2048-bit key pair using openssl: a private key (certs/private/jwt-ca.key) and a self-signed certificate (certs/jwt-ca.crt). Also extract the public key in PEM format (certs/jwt-ca.pub). Create a script (scripts/generate-token.sh) that takes a username and expiry as arguments and generates a signed JWT token using the private key (for testing purposes). The certs/private/ directory should be in .gitignore. Create a certs/.gitkeep to ensure the directory structure exists.",
      "blocked_by": ["CRUISE-001"],
      "complexity": "medium",
      "acceptance_criteria": [
        "scripts/generate-keys.sh creates RSA key pair",
        "scripts/generate-token.sh creates a valid JWT with configurable username and expiry",
        "certs/private/ is excluded by .gitignore",
        "Generated tokens can be verified using the public key",
        "Scripts are executable (chmod +x)"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash",
      "spawn_instance": "SPAWN-001"
    },
    {
      "id": "CRUISE-004",
      "subject": "Implement JWT validation middleware",
      "description": "Create src/auth.rs with: (1) A function to load the RSA public key from PEM file. (2) An Axum middleware/extractor that extracts the JWT from the Authorization header (Bearer token) or from a cookie, validates it using RS256, and extracts claims (sub, exp, iat). (3) A Claims struct with serde Deserialize. (4) An AuthUser extractor that can be used in handler signatures. (5) Proper error responses (401 Unauthorized) for missing/invalid/expired tokens. Include unit tests for token validation with valid and expired tokens.",
      "blocked_by": ["CRUISE-002", "CRUISE-003"],
      "complexity": "high",
      "acceptance_criteria": [
        "src/auth.rs exists with JWT validation logic",
        "AuthUser extractor works in Axum handler signatures",
        "Validates RS256 tokens using the public key",
        "Returns 401 for missing, invalid, or expired tokens",
        "Supports both Authorization header and cookie-based auth (cookie must use HttpOnly, Secure, SameSite=Strict attributes)",
        "Unit tests pass for valid token, expired token, and invalid signature cases",
        "cargo test --lib passes"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-002"
    },
    {
      "id": "CRUISE-005",
      "subject": "Implement .well-known/jwks.json endpoint",
      "description": "Create a handler that serves the public key in JWKS (JSON Web Key Set) format at /.well-known/jwks.json. The endpoint should read the RSA public key and convert it to JWK format with the correct kid, kty, alg, use, n, and e fields. This endpoint should be publicly accessible (no auth required). Add it to the Axum router.",
      "blocked_by": ["CRUISE-004"],
      "complexity": "medium",
      "acceptance_criteria": [
        "GET /.well-known/jwks.json returns valid JWKS JSON",
        "Response includes kty=RSA, alg=RS256, use=sig, and correct n/e values",
        "Endpoint is publicly accessible without authentication",
        "Response has correct Content-Type: application/json header"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-002"
    },
    {
      "id": "CRUISE-006",
      "subject": "Database initialization and connection management",
      "description": "Create src/db.rs with: (1) Database connection pool initialization using rusqlite with WAL mode. (2) Functions to list all user tables (excluding sqlite_ internal tables). (3) A strict identifier validation function for table names and column names. SQL parameter binding does NOT support identifiers (table names, column names) — only data values — so all identifiers used in DDL (CREATE TABLE, ALTER TABLE, DROP TABLE) and DML (SELECT, INSERT, UPDATE, DELETE) statements must be validated against a strict allowlist regex (^[a-zA-Z_][a-zA-Z0-9_]*$) before being interpolated into SQL strings. This function must reject any input that does not match, returning an error. It will be reused by all subsequent database tasks (CRUISE-006A, CRUISE-006B, CRUISE-006C) as the sole mechanism for identifier safety. Include unit tests for connection initialization (WAL mode enabled), table listing, and identifier validation (including tests that verify malicious input like SQL injection attempts, e.g. 'users; DROP TABLE', 'col\"name', and unicode tricks, are all rejected).",
      "blocked_by": ["CRUISE-002"],
      "complexity": "medium",
      "acceptance_criteria": [
        "src/db.rs exists with database connection and initialization logic",
        "Database connection pool initializes with WAL mode enabled",
        "Can list all user tables (excluding sqlite_ internal tables)",
        "Identifier validation function exists and validates against allowlist regex (^[a-zA-Z_][a-zA-Z0-9_]*$)",
        "Malicious identifier inputs (e.g., containing SQL injection attempts) are rejected with an error",
        "Unit tests pass for connection initialization, table listing, and identifier validation"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-003"
    },
    {
      "id": "CRUISE-006A",
      "subject": "Table-level schema operations (create, drop, get schema)",
      "description": "Extend src/db.rs with table-level schema operations: (1) Create a new table with a given name and column definitions (name, type, nullable, default). (2) Drop a table by name. (3) Get table schema/structure (column names, types, constraints). Use SQL parameter binding for all data values to prevent injection. Since DDL statements (CREATE TABLE, DROP TABLE) do not support parameter binding for identifiers (table names, column names), reuse the identifier validation function from CRUISE-006 to validate all identifiers before interpolation into SQL strings. Include unit tests for all table-level schema operations.",
      "blocked_by": ["CRUISE-006"],
      "complexity": "medium",
      "acceptance_criteria": [
        "Can create tables with column definitions (name, type, nullable, default)",
        "Can drop tables by name",
        "Can get table schema with column details",
        "All SQL data values use parameter binding (no string interpolation for values)",
        "All SQL identifiers are validated using the identifier validation function from CRUISE-006 before interpolation into DDL statements",
        "Unit tests pass for all table-level schema operations"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-003"
    },
    {
      "id": "CRUISE-006C",
      "subject": "Column-level schema operations (add, remove, rename)",
      "description": "Extend src/db.rs with column-level schema operations: (1) Add a column to a table. (2) Remove a column from a table (using table recreation for SQLite < 3.35). (3) Rename a column. Use SQL parameter binding for all data values to prevent injection. Since DDL statements (ALTER TABLE) do not support parameter binding for identifiers (table names, column names), reuse the identifier validation function from CRUISE-006 to validate all identifiers before interpolation into SQL strings. Include unit tests for all column-level schema operations, including the table recreation fallback for column removal.",
      "blocked_by": ["CRUISE-006A"],
      "complexity": "high",
      "acceptance_criteria": [
        "Can add a column to an existing table",
        "Can remove a column from a table",
        "Can rename a column",
        "Column removal works via table recreation for SQLite < 3.35",
        "All SQL data values use parameter binding (no string interpolation for values)",
        "All SQL identifiers are validated using the identifier validation function from CRUISE-006 before interpolation into DDL statements",
        "Unit tests pass for all column-level schema operations, including table recreation fallback"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-003"
    },
    {
      "id": "CRUISE-006B",
      "subject": "Row-level CRUD operations",
      "description": "Extend src/db.rs with row-level CRUD operations: (1) List rows from a table with pagination support (offset/limit). (2) Insert a new row with given column values. (3) Update an existing row by rowid. (4) Delete a row by rowid. Use proper SQL parameter binding for all data values to prevent injection. Since DML statements (SELECT, INSERT, UPDATE, DELETE) do not support parameter binding for identifiers (table names, column names), all identifiers MUST be strictly validated against an allowlist regex (^[a-zA-Z_][a-zA-Z0-9_]*$) before being interpolated into SQL strings — reuse the identifier validation function from CRUISE-006. Include unit tests for all row CRUD operations, including tests that verify identifier validation rejects malicious input.",
      "blocked_by": ["CRUISE-006"],
      "complexity": "medium",
      "acceptance_criteria": [
        "Row listing with pagination (offset/limit) works correctly",
        "Can insert a row with arbitrary column values",
        "Can update an existing row by rowid",
        "Can delete a row by rowid",
        "All SQL data values use parameter binding (no string interpolation for values)",
        "All SQL identifiers (table names, column names) in DML statements are validated against the strict allowlist regex (^[a-zA-Z_][a-zA-Z0-9_]*$) before interpolation, since SQL parameter binding does not support identifiers",
        "Malicious identifier inputs (e.g., containing SQL injection attempts) are rejected with an error",
        "Unit tests pass for all row CRUD operations, including identifier validation rejection tests"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-003"
    },
    {
      "id": "CRUISE-007A",
      "subject": "Implement Table/Schema API route handlers",
      "description": "Create src/routes.rs (or src/routes/ module) with Axum handlers for table and schema management: (1) GET /api/tables — list all tables. (2) POST /api/tables — create a new table. (3) DELETE /api/tables/:name — drop a table. (4) GET /api/tables/:name/schema — get table structure. (5) POST /api/tables/:name/columns — add a column. (6) DELETE /api/tables/:name/columns/:col — remove a column. All routes require authentication (use AuthUser extractor). Wire all routes into the main Axum router.",
      "blocked_by": ["CRUISE-004", "CRUISE-006A", "CRUISE-006C"],
      "complexity": "medium",
      "acceptance_criteria": [
        "All table/schema API endpoints are implemented and wired to the router",
        "All endpoints require authentication",
        "Endpoints return appropriate HTTP status codes",
        "Error responses include meaningful messages",
        "Table name and column name inputs are validated",
        "Integration tests pass for table and schema CRUD operations"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-003"
    },
    {
      "id": "CRUISE-007B",
      "subject": "Implement Row Data API route handlers",
      "description": "Extend src/routes.rs (or src/routes/ module) with Axum handlers for row-level data operations: (1) GET /api/tables/:name/rows — list rows (with pagination). (2) POST /api/tables/:name/rows — insert a row. (3) PUT /api/tables/:name/rows/:id — update a row. (4) DELETE /api/tables/:name/rows/:id — delete a row. All routes require authentication (use AuthUser extractor). Wire all routes into the main Axum router.",
      "blocked_by": ["CRUISE-007A", "CRUISE-006B"],
      "complexity": "medium",
      "acceptance_criteria": [
        "All row data API endpoints are implemented and wired to the router",
        "All endpoints require authentication",
        "Endpoints return appropriate HTTP status codes",
        "Pagination works correctly with offset/limit parameters",
        "Error responses include meaningful messages",
        "Integration tests pass for row CRUD operations"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-003"
    },
    {
      "id": "CRUISE-008A",
      "subject": "Create base layout and auth HTML templates",
      "description": "Create templates/ directory with MiniJinja templates for the base layout and authentication: (1) base.html — base layout with htmx script tag (CDN), navigation bar, CSS, and block definitions for child templates. (2) login.html — login page extending base.html with a form that accepts a JWT token (paste-based for simplicity), error message display, and a submit button. (3) tables.html — main page extending base.html that lists all tables with create/delete buttons (this establishes the authenticated shell that the database editor partials plug into).",
      "blocked_by": ["CRUISE-007A"],
      "complexity": "medium",
      "acceptance_criteria": [
        "templates/ directory exists",
        "base.html includes htmx CDN script and defines content blocks",
        "login.html extends base.html and accepts JWT token input",
        "tables.html extends base.html and lists all tables with create/delete actions",
        "Templates use MiniJinja syntax ({% block %}, {% extends %}, etc.)"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-004"
    },
    {
      "id": "CRUISE-008B",
      "subject": "Create database editor templates and htmx partials",
      "description": "Create the remaining MiniJinja templates for the database editor UI: (1) table_detail.html — shows table schema and rows with edit/delete controls, extending base.html. (2) partials/table_list.html — htmx partial for table list updates. (3) partials/table_schema.html — htmx partial for schema display. (4) partials/row_list.html — htmx partial for row listing with pagination. (5) partials/add_column_form.html — htmx partial for adding a column (name, type, nullable, default). (6) partials/add_row_form.html — htmx partial for adding a row. Use htmx attributes (hx-get, hx-post, hx-delete, hx-target, hx-swap) for dynamic updates without full page reloads.",
      "blocked_by": ["CRUISE-008A", "CRUISE-007B"],
      "complexity": "medium",
      "acceptance_criteria": [
        "table_detail.html shows schema and rows with edit/delete controls",
        "All partials exist in templates/partials/",
        "htmx partials enable dynamic updates without full page reloads",
        "Forms use hx-post/hx-delete for AJAX submissions",
        "Tables and rows update without full page reloads",
        "Partials are compatible with the base layout from CRUISE-008A"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-004"
    },
    {
      "id": "CRUISE-009",
      "subject": "Implement HTML-serving route handlers",
      "description": "Create src/views.rs with handlers that render MiniJinja templates and serve HTML pages: (1) GET / — redirect to /tables if authenticated, /login otherwise. (2) GET /login — render login page. (3) POST /login — accept token, set it as an HttpOnly, Secure, SameSite=Strict cookie, redirect to /tables. (4) GET /tables — render tables list page. (5) GET /tables/:name — render table detail page. (6) POST /logout — clear auth cookie, redirect to /login. When setting the authentication cookie, always use security attributes: HttpOnly (prevent XSS-based access), Secure (HTTPS-only), and SameSite=Strict (CSRF mitigation). Implement htmx-aware responses: if the request has HX-Request header, return only the partial; otherwise return the full page. Wire these routes into the Axum router alongside the API routes.",
      "blocked_by": ["CRUISE-007A", "CRUISE-007B", "CRUISE-008B"],
      "complexity": "medium",
      "acceptance_criteria": [
        "All HTML-serving routes are implemented",
        "Login flow works: paste token -> set cookie -> redirect",
        "Auth cookie is set with HttpOnly, Secure, and SameSite=Strict attributes",
        "Logout clears the auth cookie",
        "htmx requests receive partial responses",
        "Non-htmx requests receive full page responses",
        "Unauthenticated users are redirected to /login",
        "The application compiles and runs end-to-end"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-004"
    },
    {
      "id": "CRUISE-010",
      "subject": "Set up Playwright E2E test infrastructure",
      "description": "Create tests/e2e/ directory with: (1) package.json with @playwright/test dependency. (2) playwright.config.ts configured to start the Rust server (cargo run) as a webServer before tests, with proper startup detection. (3) A test helper/fixture that generates a short-lived JWT token using the scripts/generate-token.sh script. (4) Global setup that ensures keys exist (runs generate-keys.sh if needed) and builds the Rust binary. (5) A .npmrc or configuration to store Playwright browsers locally.",
      "blocked_by": ["CRUISE-009"],
      "complexity": "medium",
      "acceptance_criteria": [
        "tests/e2e/package.json exists with Playwright dependency",
        "playwright.config.ts starts the Rust server automatically",
        "Test fixture can generate valid JWT tokens",
        "Global setup ensures keys and binary exist",
        "npx playwright test runs without configuration errors"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-005"
    },
    {
      "id": "CRUISE-011",
      "subject": "Write Playwright E2E tests",
      "description": "Create E2E test files in tests/e2e/: (1) auth.spec.ts — test login with a valid short-lived JWT token, verify redirect to tables page, test that expired tokens are rejected, test logout. (2) tables.spec.ts — test creating a new table with specified columns, verify table appears in list, test deleting a table, verify table is removed from list. (3) schema.spec.ts — test adding a column to an existing table, test removing a column, test that column changes are reflected in the table detail view. All tests should generate test results in JUnit XML format for CI reporting.",
      "blocked_by": ["CRUISE-010"],
      "complexity": "high",
      "acceptance_criteria": [
        "auth.spec.ts tests login, expired token rejection, and logout",
        "tables.spec.ts tests creating and deleting tables",
        "schema.spec.ts tests adding and removing columns",
        "All tests pass when run with npx playwright test",
        "Test results are output in JUnit XML format",
        "Tests use short-lived tokens (< 5 minute expiry)",
        "Tests clean up after themselves (delete test tables)"
      ],
      "permissions": ["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Bash,Glob,Grep",
      "spawn_instance": "SPAWN-005"
    },
    {
      "id": "CRUISE-012",
      "subject": "Create GitHub Actions CI/CD workflows",
      "description": "Create .github/workflows/ with: (1) lint.yml — runs on all PRs, uses github/super-linter with configuration to run clippy for Rust, eslint for JS/TS, htmlhint for HTML, yamllint for YAML. Disable linters that are not relevant. (2) dependency-review.yml — runs on all PRs, uses actions/dependency-review-action to check for vulnerable dependencies in both Cargo.lock and package-lock.json. (3) test.yml — runs on all PRs, builds the Rust project, runs cargo test, sets up Node.js, installs Playwright, runs E2E tests, uploads test results as artifacts. Ensure all workflows trigger on pull_request events targeting main.",
      "blocked_by": ["CRUISE-011"],
      "complexity": "medium",
      "acceptance_criteria": [
        ".github/workflows/lint.yml exists and uses github/super-linter",
        ".github/workflows/dependency-review.yml exists and uses actions/dependency-review-action",
        ".github/workflows/test.yml exists and runs both cargo test and Playwright tests",
        "All workflows trigger on pull_request to main",
        "Super-Linter is configured to only run relevant linters",
        "Test results are uploaded as artifacts",
        "Workflows use appropriate caching (cargo, npm)"
      ],
      "permissions": ["Read", "Write", "Edit", "Glob", "Grep"],
      "cli_params": "claude --model sonnet --allowedTools Read,Write,Edit,Glob,Grep",
      "spawn_instance": "SPAWN-006"
    }
  ],
  "risks": [
    "SQLite ALTER TABLE limitations — DROP COLUMN only available in SQLite >= 3.35.0. The app must implement table recreation as a fallback for older versions, which is complex and error-prone.",
    "JWT key management — Private keys must never be committed. Test scripts must generate keys on-demand and .gitignore must be set up before any keys are created.",
    "Concurrent write access to SQLite — Multiple simultaneous writes can cause SQLITE_BUSY errors. WAL mode and a connection pool with serialized writes mitigate this.",
    "htmx partial rendering complexity — Serving both full pages and htmx partials from the same handlers requires careful template design and HX-Request header detection.",
    "Playwright test flakiness — Tests depend on the Rust server starting successfully. The playwright.config.ts webServer configuration must have proper health checks and timeouts.",
    "Super-Linter false positives — Super-Linter runs many linters by default. Without proper configuration, it may flag legitimate code patterns. Must disable irrelevant linters.",
    "Cross-platform build differences — rusqlite with bundled SQLite may have different behavior on CI (Linux) vs local development (macOS). The bundled feature helps but version differences can still occur.",
    "Token-paste login UX — Having users paste JWT tokens is not a standard login flow. The UI must clearly explain the workflow and provide good error messages for invalid tokens."
  ]
}
```

## Dependency Graph

```
CRUISE-001 (.gitignore)
  ├── CRUISE-002 (Cargo.toml & project init)
  │     ├── CRUISE-004 (JWT middleware) ─────────────────┐
  │     │     └── CRUISE-005 (.well-known)               │
  │     └── CRUISE-006 (DB init & connection mgmt)       │
  │           ├── CRUISE-006A (Table-level schema ops)   │
  │           │     └── CRUISE-006C (Column-level ops)   │
  │           │           └── CRUISE-007A (Table/Schema API) ←─┘
  │           └── CRUISE-006B (Row CRUD)
  │                 └── CRUISE-007B (Row Data API) ←── CRUISE-007A
  │                       │
  │     CRUISE-008A (Base/Auth templates) ←── CRUISE-007A
  │           └── CRUISE-008B (DB Editor templates) ←── CRUISE-007B
  │                 └── CRUISE-009 (View handlers)
  │                                   └── CRUISE-010 (Playwright setup)
  │                                         └── CRUISE-011 (E2E tests)
  │                                               └── CRUISE-012 (CI/CD)
  └── CRUISE-003 (Key generation scripts)
        └── CRUISE-004 (JWT middleware)
```

## Spawn Instance Grouping Rationale

| Instance | Tasks | Why Grouped | Why spawn_team |
|----------|-------|-------------|----------------|
| SPAWN-001 | 001, 002, 003 | Foundation tasks, sequential, low-risk | No — straightforward file creation |
| SPAWN-002 | 004, 005 | Security-critical auth code | Yes — crypto code needs review |
| SPAWN-003 | 006, 006A, 006B, 006C, 007A, 007B | Core data layer, tightly coupled | Yes — SQL injection prevention needs review |
| SPAWN-004 | 008A, 008B, 009 | Frontend rendering, tightly coupled | No — templates are low-risk |
| SPAWN-005 | 010, 011 | E2E test infrastructure and tests | Yes — test reliability needs review |
| SPAWN-006 | 012 | CI/CD configuration | No — YAML config, no Bash needed |

## File Structure

```
.
├── .github/
│   └── workflows/
│       ├── lint.yml
│       ├── dependency-review.yml
│       └── test.yml
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── scripts/
│   ├── generate-keys.sh
│   └── generate-token.sh
├── certs/
│   ├── .gitkeep
│   ├── jwt-ca.crt          (generated, gitignored)
│   ├── jwt-ca.pub          (generated, gitignored)
│   └── private/
│       └── jwt-ca.key      (generated, gitignored)
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs
│   ├── auth.rs
│   ├── db.rs
│   ├── routes.rs
│   └── views.rs
├── templates/
│   ├── base.html
│   ├── login.html
│   ├── tables.html
│   ├── table_detail.html
│   └── partials/
│       ├── table_list.html
│       ├── table_schema.html
│       ├── row_list.html
│       ├── add_column_form.html
│       └── add_row_form.html
├── tests/
│   └── e2e/
│       ├── package.json
│       ├── playwright.config.ts
│       ├── auth.spec.ts
│       ├── tables.spec.ts
│       └── schema.spec.ts
└── docs/
    └── plans/
        └── 2026-02-05-cruise-control-build-a-web-application-that.md
```

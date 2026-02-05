import { execFileSync } from "child_process";
import path from "path";

const PROJECT_ROOT = path.resolve(__dirname, "../..");

// Key generation is handled once in global-setup.ts (via Playwright's globalSetup).
// Do NOT add ensureKeysExist() or similar per-spec beforeAll hooks here — doing so
// is redundant and risks race conditions when parallel workers try to write the
// same key files simultaneously.

export function generateToken(
  username: string,
  expirySecs: number = 300
): string {
  const tokenScript = path.join(PROJECT_ROOT, "scripts/generate-token.sh");
  const token = execFileSync("bash", [tokenScript, username, String(expirySecs)], {
    cwd: PROJECT_ROOT,
    encoding: "utf-8",
  }).trim();
  return token;
}

export function generateExpiredToken(username: string): string {
  // Generate a token that expired 100 seconds ago
  return generateToken(username, -100);
}

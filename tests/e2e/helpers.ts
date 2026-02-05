import { execFileSync } from "child_process";
import path from "path";

const PROJECT_ROOT = path.resolve(__dirname, "../..");

export function ensureKeysExist(): void {
  const keyScript = path.join(PROJECT_ROOT, "scripts/generate-keys.sh");
  execFileSync("bash", [keyScript], { cwd: PROJECT_ROOT, stdio: "pipe" });
}

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

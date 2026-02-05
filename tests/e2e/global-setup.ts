import { execFileSync } from "child_process";
import path from "path";

const PROJECT_ROOT = path.resolve(__dirname, "../..");

export default function globalSetup(): void {
  const keyScript = path.join(PROJECT_ROOT, "scripts/generate-keys.sh");
  execFileSync("bash", [keyScript], { cwd: PROJECT_ROOT, stdio: "pipe" });
}

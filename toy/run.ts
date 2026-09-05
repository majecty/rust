import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const target = join(root, "test.rs");
const file = process.argv[2] ?? target;

const build = spawnSync("cargo", ["build", "--quiet"], { cwd: root, encoding: "utf8", shell: false });
if (build.status !== 0) {
  console.error(build.stderr || "cargo build failed");
  process.exit(1);
}
const run = spawnSync("cargo", ["run", "--quiet", "--", file], { cwd: root, encoding: "utf8", shell: false });
process.stdout.write(run.stdout ?? "");
process.stderr.write(run.stderr ?? "");
process.exit(run.status ?? 1);

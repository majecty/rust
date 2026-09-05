import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const user = (process.env.USER ?? process.env.USERNAME ?? process.env.LOGNAME ?? "default").replace(/[^a-zA-Z0-9_-]+/g, "_") || "default";
const targetDir = join(root, `.target-${user}`);
const env = { ...process.env, CARGO_TARGET_DIR: targetDir };
const target = join(root, "test.rs");
const file = process.argv[2] ?? target;

const build = spawnSync("cargo", ["build", "--quiet", "-p", "rtoy-driver"], { cwd: root, encoding: "utf8", shell: false, env });
if (build.status !== 0) {
  console.error(build.stderr || "cargo build failed");
  process.exit(1);
}
const run = spawnSync("cargo", ["run", "--quiet", "-p", "rtoy-driver", "--", file], { cwd: root, encoding: "utf8", shell: false, env });
process.stdout.write(run.stdout ?? "");
process.stderr.write(run.stderr ?? "");
process.exit(run.status ?? 1);

import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { sh } from "../../../jhconfig/copilot-skills/ts-run/lib/sh.ts";

function main(): void {
  const toyDir = dirname(fileURLToPath(import.meta.url));
  const root = join(toyDir, "..");
  const skipBuild = process.argv.includes("--skip-build");
  const dir = mkdtempSync(join(tmpdir(), "rustc-macro-"));
  const sample = join(dir, "macro_dup.rs");
  writeFileSync(sample, "macro_rules! def_fn { ($n:ident) => { fn $n() {} } }\ndef_fn!(foo);\nfn foo() {}\n");
  console.log(`[run-rustc] pid=${process.pid} sample=${sample}`);

  if (skipBuild) {
    console.log("[run-rustc] --skip-build: x.py build 생략");
  } else {
    const started = Date.now();
    console.log(`[run-rustc] build 시작: python3 x.py build --stage 1 (cwd=${root})`);
    console.log("[run-rustc] sh는 끝날 때까지 버퍼링하므로, 다른 터미널에서 ps aux | grep 'x.py build'로 진행 확인");
    const build = sh("python3", ["x.py", "build", "--stage", "1"], { cwd: root, timeoutMs: 3600000 });
    console.log(`[run-rustc] build 종료: ok=${build.ok} elapsed=${((Date.now() - started) / 1000).toFixed(0)}s`);
    console.log(build.ok ? build.output : build.error);
    if (!build.ok) throw new Error("x.py build failed");
  }

  const rustc = join(root, "build", "aarch64-unknown-linux-gnu", "stage1", "bin", "rustc");
  const run = sh(rustc, ["--edition", "2021", sample], { cwd: dir, timeoutMs: 120000 });
  console.log(run.ok ? run.output : run.error);
}

try {
  main();
} catch (e) {
  console.error(e);
  process.exitCode = 1;
}

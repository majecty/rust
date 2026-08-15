#!/usr/bin/env node
// Stage-by-stage compile test for someany/test.rs with the stage1 rustc.
//
// rustc lexes on demand while parsing, so there is no "lex only" CLI flag.
// `-Z parse-crate-root-only` is the earliest stop point and still surfaces
// lexer errors (illegal chars, unterminated strings, ...).
//
// Stages:
//   1. lex   : -Z parse-crate-root-only   (lex + parse crate root, stop)
//   2. parse : -Z no-analysis             (parse + expand + resolve, no analysis)
//   3. hir   : -Z unpretty=hir            (lower to HIR, dump, stop)
//   4. mir   : --emit mir                 (analysis + build MIR, dump, no codegen)
//   5. full  : (default)                  (codegen + link)
//
// usage: satest.mts [--build] [--build-std] [--sample=some|any] [--log[=LEVEL]] [lex|parse|hir|mir|full] ...
// Run directly: node satest.mts ...   (Node >= 23.6)
// Quick ref:  node satest.mts                 # all stages
//             node satest.mts --log hir       # stage w/ output + logs
//             node satest.mts --build lex parse # rebuild stage1 then 2 stages
//             node satest.mts --build-std full # rebuild stage1 + stdlib then compile
//             node satest.mts full --run      # build + run the binary
//             node satest.mts --sample=any full --run  # any_test.rs

import { spawnSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

type StageName = "lex" | "parse" | "hir" | "mir" | "full";

interface Stage {
  desc: string;
  flags: string[];
  out: string | null;
}

// Stage name -> (description, rustc flags, emit path). Add a stage by adding
// one entry here.
const STAGES: Record<StageName, Stage> = {
  lex:   { desc: "parse crate root only",    flags: ["-Z", "parse-crate-root-only"], out: null },
  parse: { desc: "parse + expand + resolve", flags: ["-Z", "no-analysis"],           out: null },
  hir:   { desc: "lower to HIR",             flags: ["-Z", "unpretty=hir"],          out: null },
  mir:   { desc: "build MIR",                flags: ["--emit", "mir"],               out: "test.mir" },
  full:  { desc: "codegen and link",         flags: [],                              out: "test" },
};

const SAMPLES: Record<string, string> = {
  some: "./someany/test.rs",
  any: "./someany/any_test.rs",
};

const RUSTC = ["rustup", "run", "stage1", "rustc"];
// `--keep-stage-std 1` reuses the previous stdlib build. If the stage1 compiler
// changed since std was last built, the stale rmeta's pre-interned symbol
// indices no longer match (see the `--build-std` note below) and names like
// `Iterator`/`Vec` fail to resolve. `--build-std` drops this flag so the
// stdlib is rebuilt with the current compiler.
const BUILD_CMD = ["build", "--stage", "1", "--keep-stage-std", "1"];
const BUILD_STD_CMD = ["build", "--stage", "1"];
const ALL_STAGES = Object.keys(STAGES) as StageName[];

let build = false;
let buildStd = false;
let run = false;
let logLevel = "";
let sample = "some";
const requested: StageName[] = [];

function usage(): void {
  console.error(`usage: $0 [--build] [--build-std] [--sample=some|any] [--log[=LEVEL]] [lex|parse|hir|mir|full] ...`);
  console.error(`  --build          run ./x.py build --stage 1 --keep-stage-std 1 first`);
  console.error(`  --build-std      run ./x.py build --stage 1 (rebuilds stdlib too) first`);
  console.error(`  --sample=NAME    test sample source (default: some -> someany/test.rs)`);
  console.error(`  --run            run the compiled binary after the full stage`);
  console.error(`  --log[=LEVEL]    show compiler output and set RUSTC_LOG (default: info)`);
}

for (let i = 2; i < process.argv.length; i++) {
  const arg = process.argv[i];
  if (arg === "-b" || arg === "--build") {
    build = true;
  } else if (arg === "--build-std") {
    buildStd = true;
  } else if (arg === "-r" || arg === "--run") {
    run = true;
  } else if (arg === "-l" || arg === "--log") {
    logLevel ||= "info";
  } else if (arg.startsWith("--log=")) {
    logLevel = arg.slice("--log=".length);
  } else if (arg.startsWith("--sample=")) {
    sample = arg.slice("--sample=".length);
  } else if (arg === "-h" || arg === "--help") {
    usage();
    process.exit(0);
  } else if (arg.startsWith("-")) {
    console.error(`unknown option: ${arg}`); usage(); process.exit(2);
  } else if (!(arg in STAGES)) {
    console.error(`unknown stage: ${arg}`); usage(); process.exit(2);
  } else {
    requested.push(arg as StageName);
  }
}

if (!(sample in SAMPLES)) {
  console.error(`unknown sample: ${sample} (expected one of: ${Object.keys(SAMPLES).join(", ")})`);
  process.exit(2);
}

const outDir = join(process.cwd(), "build", "satest");
mkdirSync(outDir, { recursive: true });
const stages: StageName[] = requested.length ? requested : ALL_STAGES;

function runRustc(args: string[], label: string): { ok: boolean; output: string } {
  const env = logLevel ? { ...process.env, RUSTC_LOG: logLevel } : process.env;
  const { status, stdout, stderr } = spawnSync(
    RUSTC[0], [...RUSTC.slice(1), ...args], { encoding: "utf8", env },
  );
  const output = stdout + stderr;
  if (logLevel) writeFileSync(join(outDir, `${label}.log`), output);
  return { ok: status === 0, output };
}

if (build || buildStd) {
  const cmd = buildStd ? BUILD_STD_CMD : BUILD_CMD;
  console.log(`== build  ${cmd.join(" ")}`);
  const { status, stdout, stderr } = spawnSync("./x.py", cmd, { encoding: "utf8" });
  const output = stdout + stderr;
  console.log(status === 0 ? "   OK" : "   FAILED");
  if (status !== 0) {
    console.error(output);
    process.exit(1);
  }
}

let failed: StageName | null = null;
const src = SAMPLES[sample];
for (const label of stages) {
  const { desc, flags, out } = STAGES[label];
  const args = [src, ...flags, ...(out ? ["-o", join(outDir, out)] : [])];
  console.log(`== ${label.padEnd(6)}  ${desc}`);

  const { ok, output } = runRustc(args, label);
  if (!ok) {
    console.log("   FAILED");
    if (logLevel) console.log(`   log: ${join(outDir, `${label}.log`)}`);
    console.error(output);
    failed = label;
    break;
  }

  console.log("   OK");
  if (logLevel && output.trim().length) {
    console.log(output.trimEnd());
  }
}

if (failed) {
  console.error(`\nstage '${failed}' failed`);
  process.exit(1);
}

if (requested.length === 0) {
  console.log(`\nAll stages passed. binary: ${join(outDir, "test")}`);
}
if (logLevel) {
  console.log(`logs: ${outDir} (RUSTC_LOG=${logLevel})`);
}

if (run) {
  const binary = join(outDir, "test");
  if (failed || !(stages.includes("full"))) {
    console.error(`\n--run requires the 'full' stage; run: node satest.mts full --run`);
    process.exit(2);
  }
  console.log(`\n== run     ${binary}`);
  const { status, stdout, stderr, error } = spawnSync(binary, [], { encoding: "utf8" });
  if (error) {
    console.error(error.message);
    process.exit(1);
  }
  const output = (stdout ?? "") + (stderr ?? "");
  console.log(output.trimEnd());
  console.log(status === 0 ? "   OK" : "   FAILED (exit " + status + ")");
  if (status !== 0) process.exit(1);
}

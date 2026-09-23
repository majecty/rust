// fib-bench.ts — 재귀 fib(n) 언어별 실행 시간 비교 (rtoy 포함).
// USE FOR: python/ruby/node/php/perl/lua/luajit/go/c/rust/rtoy 중 설치된 것만 측정
// DO NOT USE FOR: 마이크로 최적화 검증 (AST 워킹 vs VM 차이만 보는 용도)
// ARGS: [n=25] [runs=3] [--md] [--json=<path>]
// 예: ts_run toy/bench/fib-bench.ts 25 3 --md   |   node toy/bench/fib-bench.ts 25 3 --json=/tmp/pi/fib.json
//
// - 컴파일 언어(c/rust/go)는 빌드 시간을 실행 시간에서 제외한다.
// - rtoy도 release로 빌드한다 (다른 언어가 -O2/-O로 측정되므로 조건을 맞춤).
// - lua/luajit는 바이너리가 없으면 /tmp/pi/luasrc에 소스 빌드한다 (root 불필요, 네트워크 필요).
// - warmup 1회는 버리고 runs회 측정한다 (JIT/프로세스 기동비용 제외).
// - 출력: min/median/max ms, `--md`면 markdown 표, `--json=<path>`면 결과 저장.
// - rtoy는 argv가 없어 n을 소스에 박는다({N} 치환).
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const DIR = join(tmpdir(), "fibbench");
const LUA_DIR = join(tmpdir(), "luasrc");
const TOY_DIR = dirname(dirname(fileURLToPath(import.meta.url)));
const RTOY_BIN = join(tmpdir(), "rtoy-bench", "release", "rtoy-driver");
const RUSTC = join(homedir(), ".cargo", "bin", "rustc");

type Run = { ok: boolean; out: string };

function run(cmd: string, args: string[], cwd = DIR, timeout = 600_000): Run {
  const r = spawnSync(cmd, args, { cwd, encoding: "utf8", timeout, maxBuffer: 4 * 1024 * 1024 });
  const out = `${r.stdout ?? ""}${r.stderr ?? ""}`.slice(0, 4000);
  return { ok: r.status === 0, out };
}

const SOURCES: Record<string, [string, string]> = {
  python: ["fib.py", `import sys
def fib(n):
    return n if n < 2 else fib(n - 1) + fib(n - 2)
print(fib(int(sys.argv[1])))`],
  ruby: ["fib.rb", `def fib(n)\n  n < 2 ? n : fib(n - 1) + fib(n - 2)\nend\nputs fib(ARGV[0].to_i)`],
  node: ["fib.js", `const fib = (n) => (n < 2 ? n : fib(n - 1) + fib(n - 2));
console.log(fib(Number(process.argv[2])));`],
  php: ["fib.php", `<?php
function fib(int $n): int { return $n < 2 ? $n : fib($n - 1) + fib($n - 2); }
echo fib((int) $argv[1]), "\\n";`],
  perl: ["fib.pl", `sub fib { my $n = shift; return $n < 2 ? $n : fib($n-1) + fib($n-2); }
print fib($ARGV[0]), "\\n";`],
  lua: ["fib.lua", `local fib\nfib = function(n) return n < 2 and n or fib(n - 1) + fib(n - 2) end\nprint(fib(tonumber(arg[1])))`],
  c: ["fib.c", `#include <stdio.h>
#include <stdlib.h>
long fib(long n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
int main(int argc, char **argv) { printf("%ld\\n", fib(atol(argv[1]))); return 0; }`],
  rust: ["fib.rs", `fn fib(n: u64) -> u64 { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
fn main() { let n: u64 = std::env::args().nth(1).unwrap().parse().unwrap(); println!("{}", fib(n)); }`],
  go: ["fib.go", `package main

import ("fmt"; "os"; "strconv")

func fib(n int) int {
	if n < 2 {
		return n
	}
	return fib(n-1) + fib(n-2)
}

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	fmt.Println(fib(n))
}`],
  rtoy: ["fib.rtoy.rs", `fn fib() {
    let n = 0;
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main() {
    let n = {N};
    fib(n)
}
`],
};

type Target = { lang: string; tool: string; script: string; build?: string[]; absPath?: boolean; prefix?: string; runs?: number; skip?: string };

function luaBin(kind: "lua" | "luajit"): string {
  if (kind === "luajit") return join(LUA_DIR, "LuaJIT", "src", "luajit");
  // 버전을 고정하지 않는다 (lua.org 최신 5.4.x를 받아 풀기 때문).
  const dir = existsSync(LUA_DIR) ? readdirSync(LUA_DIR).find((d) => d.startsWith("lua-5.4")) : undefined;
  return dir ? join(LUA_DIR, dir, "src", "lua") : join(LUA_DIR, "lua-5.4", "src", "lua");
}

/** lua/luajit 바이너리가 없으면 소스 빌드 (실패는 skip 사유로). */
function ensureLua(): string {
  if (existsSync(luaBin("lua")) && existsSync(luaBin("luajit"))) return "";
  mkdirSync(LUA_DIR, { recursive: true });
  // 5.4 계열로 고정한다 (README 수치와 재현성 맞춤).
  const tarball = run("bash", ["-c", "curl -s https://www.lua.org/ftp/ | grep -o 'lua-5\\.4\\.[0-9]*\\.tar\\.gz' | sort -uV | tail -1"], LUA_DIR, 60_000).out.trim();
  if (!tarball) return "lua tarball 조회 실패(네트워크?)";
  const dl = run("bash", ["-c", `curl -sSLO https://www.lua.org/ftp/${tarball} && tar xzf ${tarball}`], LUA_DIR, 300_000);
  if (!dl.ok) return `lua 다운로드 실패: ${dl.out.slice(0, 80)}`;
  const make = run("bash", ["-c", `cd ${LUA_DIR}/${tarball.replace(/\.tar\.gz$/, "")} && make -j4 all >/dev/null`], LUA_DIR, 600_000);
  if (!make.ok) return `lua 빌드 실패: ${make.out.slice(0, 80)}`;
  const jit = run("bash", ["-c", `cd ${LUA_DIR} && (test -d LuaJIT || git clone --depth 1 -b v2.1 https://github.com/LuaJIT/LuaJIT.git) && cd LuaJIT && make -j4 >/dev/null`], LUA_DIR, 600_000);
  if (!jit.ok) return `LuaJIT 빌드 실패: ${jit.out.slice(0, 80)}`;
  return "";
}

/** PATH 또는 절대경로 존재 확인. */
function missing(tool: string): boolean {
  return tool.includes("/") ? !existsSync(tool) : !run("bash", ["-c", `command -v ${tool}`], DIR, 30_000).ok;
}

function refFib(n: number): number {
  let a = 0;
  let b = 1;
  for (let i = 0; i < n; i++) [a, b] = [b, a + b];
  return a;
}

type Stats = { min: number; median: number; max: number; out: string; raw: string };
type Row = { lang: string; min: number; median: number; max: number; note: string };

const fmt = (x: number): string => (Number.isFinite(x) ? x.toFixed(1) : "-");

/** warmup 1회 버리고 runs회 측정 → min/median/max. 검증값은 첫 측정 run에서 고정. */
function measure(cmd: string, args: string[], runs: number, prefix?: string): Stats {
  const samples: number[] = [];
  let out = "";
  let raw = "";
  for (let i = 0; i <= runs; i++) {
    const t0 = process.hrtime.bigint();
    const r = run(cmd, args);
    const ms = Number(process.hrtime.bigint() - t0) / 1e6;
    if (i === 0) continue; // warmup — JIT/기동비용 제외
    samples.push(ms);
    if (i === 1) {
      const lines = r.out.split("\n").map((l) => l.trim()).filter(Boolean);
      raw = lines.slice(0, 2).join(" | ");
      const hit = lines.find((l) => (prefix ? l.startsWith(prefix) : /^\d+$/.test(l))) ?? "";
      out = prefix ? hit.slice(prefix.length) : hit;
    }
  }
  const sorted = [...samples].sort((a, b) => a - b);
  const mid = sorted[Math.floor(sorted.length / 2)];
  const median = sorted.length % 2 ? mid : (sorted[sorted.length / 2 - 1] + mid) / 2;
  return { min: sorted[0], median, max: sorted[sorted.length - 1], out, raw };
}

function mdTable(rows: Row[], base: number, baseLabel: string): string {
  const head = "| lang | min | median | max | ratio | note |\n|---|---|---|---|---|---|";
  const body = rows.map((r) => {
    const ratio = Number.isFinite(r.min) ? `${(r.min / base).toFixed(2)}x ${baseLabel}` : "-";
    return `| ${r.lang} | ${fmt(r.min)} | ${fmt(r.median)} | ${fmt(r.max)} | ${ratio} | ${r.note} |`;
  });
  return [head, ...body].join("\n");
}

/** 컴파일 언어는 미리 빌드하고 실행 시간에서 제외한다. */
function prepare(target: Target, n: number): { cmd: string; args: string[]; skip: string } {
  if (!target.build) {
    const arg = target.absPath ? join(DIR, target.script) : target.script;
    const args = target.lang === "rtoy" ? [arg] : [arg, String(n)];
    return { cmd: target.tool, args, skip: target.skip ?? "" };
  }
  const bin = `fib-${target.lang}`;
  const r = run(target.tool, [...target.build, "-o", bin, target.script]);
  return { cmd: `./${bin}`, args: [String(n)], skip: r.ok ? "" : `BUILD FAILED: ${r.out.split("\n")[0].slice(0, 80)}` };
}

function main(): void {
  const argv = process.argv.slice(2);
  const jsonPath = argv.find((a) => a.startsWith("--json="))?.slice("--json=".length);
  const wantMd = argv.includes("--md");
  const positional = argv.filter((a) => !a.startsWith("--"));
  const n = Number(positional[0] ?? 25);
  const runs = Number(positional[1] ?? 3);
  const expect = refFib(n);
  mkdirSync(DIR, { recursive: true });
  for (const [file, src] of Object.values(SOURCES)) writeFileSync(join(DIR, file), src.replace("{N}", String(n)));

  const luaNote = ensureLua();
  const rtoyEnv = { ...process.env, CARGO_TARGET_DIR: join(tmpdir(), "rtoy-bench") };
  // 다른 언어가 최적화 빌드(c -O2, rust -O)로 측정되므로 rtoy도 release로 맞춘다.
  const rtoyBuild = spawnSync("cargo", ["build", "--release", "-p", "rtoy-driver"], {
    cwd: TOY_DIR,
    env: rtoyEnv,
    encoding: "utf8",
    timeout: 300_000,
  });
  const rtoyOk = rtoyBuild.status === 0;

  const targets: Target[] = [
    { lang: "python", tool: "python3", script: "fib.py" },
    { lang: "ruby", tool: "ruby", script: "fib.rb" },
    { lang: "node", tool: "node", script: "fib.js" },
    { lang: "php", tool: "php", script: "fib.php" },
    { lang: "perl", tool: "perl", script: "fib.pl" },
    { lang: "lua", tool: luaBin("lua"), script: "fib.lua", skip: luaNote },
    { lang: "luajit", tool: luaBin("luajit"), script: "fib.lua", skip: luaNote },
    { lang: "c", tool: "gcc", script: "fib.c", build: ["-O2"] },
    { lang: "rust", tool: RUSTC, script: "fib.rs", build: ["-O"] },
    { lang: "go", tool: "go", script: "fib.go", build: ["build"] },
    { lang: "rtoy", tool: RTOY_BIN, script: "fib.rtoy.rs", absPath: true, prefix: "value: ", skip: rtoyOk ? "" : `cargo build 실패: ${(rtoyBuild.stderr ?? "").slice(0, 60)}` },
  ];

  console.log(`=== fib(${n}) min/median/max ms, warmup 1 + x${runs} runs, expected=${expect} ===`);
  const skipped = (lang: string, note: string): Row => ({ lang, min: Infinity, median: Infinity, max: Infinity, note });
  const rows: Row[] = [];
  for (const target of targets) {
    const skip = target.skip || (missing(target.tool) ? `${target.tool} 없음` : "");
    if (skip) {
      rows.push(skipped(target.lang, `SKIP (${skip})`));
      continue;
    }
    const { cmd, args, skip: buildNote } = prepare(target, n);
    if (buildNote) {
      rows.push(skipped(target.lang, buildNote));
      continue;
    }
    const { min, median, max, out, raw } = measure(cmd, args, target.runs ?? runs, target.prefix);
    rows.push({ lang: target.lang, min, median, max, note: out === String(expect) ? "ok" : `MISMATCH out=${out} raw=${raw.slice(0, 120)}` });
  }
  rows.sort((a, b) => a.min - b.min);
  // python이 없으면 가장 빠른 유한값을 기준으로 삼는다 (base=Infinity면 전부 "-"가 되는 문제 방지).
  const baseRow = rows.find((r) => r.lang === "python" && Number.isFinite(r.min)) ?? rows.find((r) => Number.isFinite(r.min));
  const base = baseRow?.min ?? Infinity;
  const baseLabel = baseRow?.lang ?? "-";
  for (const r of rows) {
    const ratio = Number.isFinite(r.min) ? `${(r.min / base).toFixed(2)}x ${baseLabel}` : "-";
    console.log(`${r.lang.padEnd(7)} ${fmt(r.min).padStart(8)} ${fmt(r.median).padStart(8)} ${fmt(r.max).padStart(8)}  ${ratio.padStart(12)}  ${r.note}`);
  }
  if (wantMd) console.log(`\n${mdTable(rows, base, baseLabel)}`);
  if (jsonPath) writeFileSync(jsonPath, JSON.stringify({ n, runs, expected: expect, base: baseLabel, rows }, null, 2));
}

main();

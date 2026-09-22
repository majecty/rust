// fib-bench.ts — 재귀 fib(n) 언어별 실행 시간 비교 (rtoy 포함).
// USE FOR: python/ruby/node/php/perl/lua/luajit/go/c/rust/rtoy 중 설치된 것만 측정
// DO NOT USE FOR: 마이크로 최적화 검증 (AST 워킹 vs VM 차이만 보는 용도)
// ARGS: [n=25] [runs=3]
// 예: ts_run toy/bench/fib-bench.ts 25 3   |   node toy/bench/fib-bench.ts 25 3
//
// - 컴파일 언어(c/rust/go)는 빌드 시간을 실행 시간에서 제외한다.
// - lua/luajit는 바이너리가 없으면 /tmp/pi/luasrc에 소스 빌드한다 (root 불필요, 네트워크 필요).
// - rtoy는 argv가 없어 n을 소스에 박는다({N} 치환).
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const DIR = join(tmpdir(), "fibbench");
const LUA_DIR = join(tmpdir(), "luasrc");
const TOY_DIR = dirname(dirname(fileURLToPath(import.meta.url)));
const RTOY_BIN = join(tmpdir(), "rtoy-bench", "debug", "rtoy-driver");
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

function measure(cmd: string, args: string[], runs: number, prefix?: string): { ms: number; out: string; raw: string } {
  let best = Infinity;
  let out = "";
  let raw = "";
  for (let i = 0; i < runs; i++) {
    const t0 = process.hrtime.bigint();
    const r = run(cmd, args);
    best = Math.min(best, Number(process.hrtime.bigint() - t0) / 1e6);
    const lines = r.out.split("\n").map((l) => l.trim()).filter(Boolean);
    raw = lines.slice(0, 2).join(" | ");
    const hit = lines.find((l) => (prefix ? l.startsWith(prefix) : /^\d+$/.test(l))) ?? "";
    out = prefix ? hit.slice(prefix.length) : hit;
  }
  return { ms: best, out, raw };
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
  const n = Number(process.argv[2] ?? 25);
  const runs = Number(process.argv[3] ?? 3);
  const expect = refFib(n);
  mkdirSync(DIR, { recursive: true });
  for (const [file, src] of Object.values(SOURCES)) writeFileSync(join(DIR, file), src.replace("{N}", String(n)));

  const luaNote = ensureLua();
  const rtoyEnv = { ...process.env, CARGO_TARGET_DIR: join(tmpdir(), "rtoy-bench") };
  const rtoyBuild = spawnSync("cargo", ["build", "-p", "rtoy-driver"], {
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
    { lang: "rtoy", tool: RTOY_BIN, script: "fib.rtoy.rs", absPath: true, prefix: "value: ", runs: 2, skip: rtoyOk ? "" : `cargo build 실패: ${(rtoyBuild.stderr ?? "").slice(0, 60)}` },
  ];

  console.log(`=== fib(${n}) 최소값, x${runs} runs, expected=${expect} ===`);
  const rows: { lang: string; ms: number; note: string }[] = [];
  for (const target of targets) {
    const skip = target.skip || (missing(target.tool) ? `${target.tool} 없음` : "");
    if (skip) {
      rows.push({ lang: target.lang, ms: Infinity, note: `SKIP (${skip})` });
      continue;
    }
    const { cmd, args, skip: buildNote } = prepare(target, n);
    if (buildNote) {
      rows.push({ lang: target.lang, ms: Infinity, note: buildNote });
      continue;
    }
    const { ms, out, raw } = measure(cmd, args, target.runs ?? runs, target.prefix);
    rows.push({ lang: target.lang, ms, note: out === String(expect) ? "ok" : `MISMATCH out=${out} raw=${raw.slice(0, 120)}` });
  }
  rows.sort((a, b) => a.ms - b.ms);
  const base = rows.find((r) => r.lang === "python")?.ms ?? Infinity;
  for (const r of rows) {
    const ratio = Number.isFinite(r.ms) ? `${(r.ms / base).toFixed(2)}x py` : "-";
    console.log(`${r.lang.padEnd(7)} ${r.ms.toFixed(1).padStart(9)} ms  ${ratio.padStart(9)}  ${r.note}`);
  }
}

main();

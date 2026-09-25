// serve.ts
// USE FOR: rtoy 문법 → MIR 실시간 변환 + 한 줄씩 실행(리플레이 스테퍼) 웹페이지 로컬 서버 (학습용)
// DO NOT USE FOR: 배포용 서버 (인증·동시성 고려 없음)
// ARGS: [port=8787]
// 예: node toy/web/serve.ts 8787   |   ts_run toy/web/serve.ts 8787
//
// - GET  /                     → index.html
// - POST /api/compile          → 본문(소스)을 끝까지 실행해 { ok, hir, thir, thirTree, mir, value, error } 반환
// - POST /api/compile?steps=N  → N스텝까지만 실행해 { ok, hir, thir, thirTree, mir, trace, cursor, value, error } 반환
//   (스텝 실행은 드라이버 `--mir-steps=N` 출력을 파싱한 것 — 매 요청 처음부터 재실행하는 리플레이 방식)
// - hir/thir/thirTree/mir는 드라이버를 각각 `--hir`/`--thir`/`--thir-tree`/`--mir`로 실행해 받은 덤프다(요청당 프로세스 4~5개).
// - 드라이버는 워크스페이스 공용 target/debug/rtoy-driver 를 쓴다 (없으면 빌드 안내를 반환).
import { createServer } from "node:http";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { networkInterfaces, tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = dirname(dirname(HERE)); // toy/web → toy → rust 루트
const DRIVER = join(REPO, "target", "debug", "rtoy-driver");
const INDEX = join(HERE, "index.html");
const WORK = mkdtempSync(join(tmpdir(), "rtoy-web-"));
const MAX_SRC = 20_000;
const HUB = process.env.PI_WEB_HUB ?? "http://172.17.0.1:8899";

/** 스텝 1개 (드라이버 trace 줄). */
type TraceRow = { i: number; depth: number; fn: string; bb: number; stmt: number | null; kind: "stmt" | "term"; text: string };

/** 현재 위치 + 그 프레임 지역변수. */
type Cursor = { fn: string; bb: number; depth: number; steps: number; budget: number; halted: boolean; locals: { name: string; value: string }[] };

/** 컨테이너 자신의 IPv4 (허브 등록용). */
function selfIp(): string {
  for (const list of Object.values(networkInterfaces())) {
    for (const ni of list ?? []) if (ni.family === "IPv4" && !ni.internal) return ni.address;
  }
  return "127.0.0.1";
}

/** 허브에 30초 주기 등록(하트비트) — TTL 45초 안에 갱신되어야 목록에 남는다. */
function registerWithHub(port: number): void {
  const url = `http://${selfIp()}:${port}`;
  const label = process.cwd();
  const beat = () => fetch(`${HUB}/register`, { method: "POST", body: JSON.stringify({ url, label }) })
    .catch((e) => console.error("hub register 실패:", (e as Error).message));
  beat();
  setInterval(beat, 30_000);
  process.on("SIGTERM", () => {
    void fetch(`${HUB}/unregister`, { method: "POST", body: JSON.stringify({ url }) })
      .catch(() => {})
      .finally(() => process.exit(0));
  });
}

let seq = 0;

/** 드라이버 stdout을 { trace, cursor, value } 로 나눈다 (`--mir-steps=N` 출력 계약). */
function parseSteps(out: string): { trace: TraceRow[]; cursor: Cursor | null; value: string } {
  const sections = new Map<string, string[]>();
  let cur = "";
  for (const line of out.split("\n")) {
    const m = /^== (.+) ==$/.exec(line);
    if (m) {
      cur = m[1];
      sections.set(cur, []);
      continue;
    }
    sections.get(cur)?.push(line);
  }
  const trace: TraceRow[] = [];
  for (const line of sections.get("trace") ?? []) {
    const m = /^#(\d+) d(\d+) (\S+) bb(\d+)(?::(\d+))? (stmt|term) \| (.*)$/.exec(line);
    if (!m) continue;
    trace.push({ i: Number(m[1]), depth: Number(m[2]), fn: m[3], bb: Number(m[4]), stmt: m[5] === undefined ? null : Number(m[5]), kind: m[6] as "stmt" | "term", text: m[7] });
  }
  const head = sections.get("cursor") ?? [];
  const h = /^fn=(\S+) bb=(\d+) depth=(\d+) steps=(\d+) budget=(\d+) halted=([01])$/.exec(head[0] ?? "");
  const locals = head.slice(1).flatMap((line) => {
    const m = /^(\S+) = (.*)$/.exec(line);
    return m ? [{ name: m[1], value: m[2] }] : [];
  });
  const cursor: Cursor | null = h
    ? { fn: h[1], bb: Number(h[2]), depth: Number(h[3]), steps: Number(h[4]), budget: Number(h[5]), halted: h[6] === "1", locals }
    : null;
  return { trace, cursor, value: (sections.get("value") ?? []).join("\n").trim() };
}

/** 덤프 1회 실행 (`--hir`/`--thir`/`--mir`) — 실패하면 stderr 원문을 그대로 준다. */
function dump(flag: string, file: string): string {
  const r = spawnSync(DRIVER, [flag, file], { encoding: "utf8", timeout: 10_000 });
  return `${r.stdout ?? ""}${r.stderr ?? ""}`.trim();
}

/** 소스를 임시 파일로 써서 드라이버를 실행한다 (`--mir` 덤프 + 실행). */
function compile(source: string, steps?: number) {
  if (!existsSync(DRIVER)) {
    return { ok: false, hir: "", thir: "", thirTree: "", mir: "", value: "", error: `드라이버가 없습니다: ${DRIVER}\n빌드: cargo build -p rtoy-driver` };
  }
  const file = join(WORK, `in-${seq++}.rs`);
  writeFileSync(file, source);

  const mir = spawnSync(DRIVER, ["--mir", file], { encoding: "utf8", timeout: 10_000 });
  if (mir.status !== 0) {
    return { ok: false, hir: "", thir: "", thirTree: "", mir: "", value: "", error: `${mir.stdout ?? ""}${mir.stderr ?? ""}`.trim() };
  }
  const hir = dump("--hir", file);
  const thir = dump("--thir", file);
  const thirTree = dump("--thir-tree", file);
  const mirText = (mir.stdout ?? "").trim();
  if (steps !== undefined) {
    const run = spawnSync(DRIVER, [`--mir-steps=${steps}`, file], { encoding: "utf8", timeout: 10_000 });
    const out = `${run.stdout ?? ""}${run.stderr ?? ""}`;
    if (run.status !== 0) {
      return { ok: false, hir, thir, thirTree, mir: mirText, value: "", error: out.trim() };
    }
    const parsed = parseSteps(out);
    return { ok: true, hir, thir, thirTree, mir: mirText, ...parsed, error: "" };
  }
  const ev = spawnSync(DRIVER, [file], { encoding: "utf8", timeout: 10_000 });
  const value = `${ev.stdout ?? ""}${ev.stderr ?? ""}`.trim();
  return { ok: true, hir, thir, thirTree, mir: mirText, value, error: "" };
}

function sendJson(res: import("node:http").ServerResponse, body: unknown): void {
  res.writeHead(200, { "content-type": "application/json; charset=utf-8" });
  res.end(JSON.stringify(body));
}

function main(): void {
  const port = Number(process.argv[2] ?? 8787);
  if (!Number.isInteger(port) || port <= 0 || port > 65535) {
    console.error(`포트가 올바르지 않습니다: ${process.argv[2]}`);
    process.exitCode = 2;
    return;
  }

  createServer((req, res) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    if (req.method === "GET" && (url.pathname === "/" || url.pathname === "/index.html")) {
      res.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      res.end(readFileSync(INDEX));
      return;
    }
    if (req.method === "POST" && url.pathname === "/api/compile") {
      const raw = url.searchParams.get("steps");
      const steps = raw === null ? undefined : Math.max(0, Math.floor(Number(raw)));
      if (steps !== undefined && !Number.isFinite(steps)) {
        sendJson(res, { ok: false, hir: "", thir: "", thirTree: "", mir: "", value: "", error: `steps 값이 올바르지 않습니다: ${raw}` });
        return;
      }
      const chunks: Buffer[] = [];
      req.on("data", (c: Buffer) => chunks.push(c));
      req.on("end", () => sendJson(res, compile(Buffer.concat(chunks).toString("utf8").slice(0, MAX_SRC), steps)));
      return;
    }
    res.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    res.end("not found");
  }).listen(port, () => {
    console.log(`rtoy hir/thir/mir web → http://localhost:${port}/`);
    console.log(`driver = ${DRIVER}`);
    if (!existsSync(DRIVER)) console.log("경고: 드라이버가 없습니다. `cargo build -p rtoy-driver` 먼저 실행하세요.");
    registerWithHub(port);
    console.log(`근거: hub = ${HUB}`);
  });
}

main();

// serve.ts
// USE FOR: rtoy 문법 → MIR 실시간 변환 웹페이지 로컬 서버 (학습용)
// DO NOT USE FOR: 배포용 서버 (인증·동시성 고려 없음)
// ARGS: [port=8787]
// 예: node toy/web/serve.ts 8787   |   ts_run toy/web/serve.ts 8787
//
// - GET  /            → index.html
// - POST /api/compile → 본문(소스)을 rtoy-driver로 컴파일해 { ok, mir, value, error } 반환
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
  process.on("SIGTERM", () => { void fetch(`${HUB}/unregister`, { method: "POST", body: JSON.stringify({ url }) }).catch(() => {}); });
}

let seq = 0;

/** 소스를 임시 파일로 써서 드라이버를 두 번 실행한다 (--mir 덤프, 기본 eval). */
function compile(source: string): { ok: boolean; mir: string; value: string; error: string } {
  if (!existsSync(DRIVER)) {
    return { ok: false, mir: "", value: "", error: `드라이버가 없습니다: ${DRIVER}\n빌드: cargo build -p rtoy-driver` };
  }
  const file = join(WORK, `in-${seq++}.rs`);
  writeFileSync(file, source);

  const mir = spawnSync(DRIVER, ["--mir", file], { encoding: "utf8", timeout: 10_000 });
  if (mir.status !== 0) {
    return { ok: false, mir: "", value: "", error: `${mir.stdout ?? ""}${mir.stderr ?? ""}`.trim() };
  }
  const ev = spawnSync(DRIVER, [file], { encoding: "utf8", timeout: 10_000 });
  const value = `${ev.stdout ?? ""}${ev.stderr ?? ""}`.trim();
  return { ok: true, mir: (mir.stdout ?? "").trim(), value, error: "" };
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
    if (req.method === "GET" && (req.url === "/" || req.url === "/index.html")) {
      res.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      res.end(readFileSync(INDEX));
      return;
    }
    if (req.method === "POST" && req.url === "/api/compile") {
      const chunks: Buffer[] = [];
      req.on("data", (c: Buffer) => chunks.push(c));
      req.on("end", () => sendJson(res, compile(Buffer.concat(chunks).toString("utf8").slice(0, MAX_SRC))));
      return;
    }
    res.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    res.end("not found");
  }).listen(port, () => {
    console.log(`rtoy mir web → http://localhost:${port}/`);
    console.log(`driver = ${DRIVER}`);
    if (!existsSync(DRIVER)) console.log("경고: 드라이버가 없습니다. `cargo build -p rtoy-driver` 먼저 실행하세요.");
    registerWithHub(port);
    console.log(`근거: hub = ${HUB}`);
  });
}

main();

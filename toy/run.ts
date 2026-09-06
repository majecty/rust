import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// 스크립트 위치 = toy/ 디렉터리. 마운트 공유 경로라 산출물은 두지 않는다.
function resolveRoot(): string {
  return dirname(fileURLToPath(import.meta.url));
}

// 도커 안팎 USER가 달라 suffix로 구분한다.
function resolveUser(): string {
  const raw = process.env.USER ?? process.env.USERNAME ?? process.env.LOGNAME ?? "default";
  const clean = raw.replace(/[^a-zA-Z0-9_-]+/g, "_");
  return clean === "" ? "default" : clean;
}

// target은 공유 마운트 밖(/tmp)에 둔다. toy/ 안에 두면 반대편 소유권 때문에
// 디렉터리 생성 자체가 Permission denied로 실패한다.
function resolveTargetDir(): string {
  const explicit = process.env.CARGO_TARGET_DIR;
  if (explicit !== undefined && explicit !== "") return explicit;
  return join(tmpdir(), `rtoy-${resolveUser()}`);
}

function cargoOrExit(args: string[], cwd: string, env: NodeJS.ProcessEnv, label: string): string {
  const r = spawnSync("cargo", args, { cwd, encoding: "utf8", shell: false, env });
  if (r.error !== undefined) {
    const e = r.error as NodeJS.ErrnoException;
    console.error(`[rtoy] ${label} 실패: cargo 실행 불가 (${e.message})`);
    console.error(`[rtoy] 가장 안쪽: code=${e.code ?? "-"} errno=${e.errno ?? "-"} syscall=${e.syscall ?? "-"} path=${e.path ?? "cargo"}`);
    console.error(`[rtoy] cwd=${cwd} CARGO_TARGET_DIR=${env.CARGO_TARGET_DIR} PATH=${env.PATH ?? "-"}`);
    process.exit(1);
  }
  // driver 출력(stdout)은 성공/실패 관계없이 그대로 전달한다.
  // 실패를 cargo 탓으로 뭉뚱그리면 (이번 --lex 건처럼) 원인이 가려진다.
  if (r.stdout !== "") process.stdout.write(r.stdout);
  if (r.stderr !== "") process.stderr.write(r.stderr);
  if ((r.status ?? 1) !== 0) {
    console.error(`[rtoy] ${label} 실패: cargo ${args.join(" ")} (exit=${r.status} signal=${r.signal ?? "-"})`);
    console.error(`[rtoy] cwd=${cwd} CARGO_TARGET_DIR=${env.CARGO_TARGET_DIR}`);
    if (label === "build") {
      console.error(`[rtoy] 원인 확인: CARGO_TARGET_DIR 쓰기권한/잔여 lock, 툴체인 확인 (cargo --version), 수동 재현: cargo build -p rtoy-driver`);
    }
    // run 실패는 driver 종료코드 그대로 전달 (cargo 탓 아님)
    process.exit(r.status ?? 1);
  }
  return r.stdout;
}

const root = resolveRoot();
const passthrough = process.argv.slice(2);
// 플래그만 있고 파일이 없으면 (--lex/--ast 등) 기본 입력 test.rs를 뒤에 붙인다.
const hasFile = passthrough.some((a) => !a.startsWith("-"));
const driverArgs = hasFile ? passthrough : [...passthrough, join(root, "test.rs")];
const env: NodeJS.ProcessEnv = { ...process.env, CARGO_TARGET_DIR: resolveTargetDir() };

cargoOrExit(["build", "--quiet", "-p", "rtoy-driver"], root, env, "build");
cargoOrExit(["run", "--quiet", "-p", "rtoy-driver", "--", ...driverArgs], root, env, "run");

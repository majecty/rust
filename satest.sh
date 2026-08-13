#!/usr/bin/env bash
# Stage-by-stage compile test for someany/test.rs with the stage1 rustc.
#
# rustc lexes on demand while parsing, so there is no "lex only" CLI flag.
# `-Z parse-crate-root-only` is the earliest stop point and still surfaces
# lexer errors (illegal chars, unterminated strings, ...).
#
# Stages:
#   1. lex   : -Z parse-crate-root-only   (lex + parse crate root, stop)
#   2. parse : -Z no-analysis             (parse + expand + resolve, no analysis)
#   3. hir   : -Z unpretty=hir            (lower to HIR, dump, stop)
#   4. mir   : --emit mir                 (analysis + build MIR, dump, no codegen)
#   5. full  : (default)                  (codegen + link)
#
# usage: $0 [--build] [--log[=LEVEL]] [lex|parse|hir|mir|full] ...

set -euo pipefail

RUSTC=(rustup run stage1 rustc)
SRC="./someany/test.rs"
OUT_DIR="$(mktemp -d)"
LOG_LEVEL=""
DO_BUILD=0
STAGES=()

usage() {
    echo "usage: $0 [--build] [--log[=LEVEL]] [lex|parse|hir|mir|full] ..." >&2
    echo "  --build          run ./x.py build --stage 1 --keep-stage-std 1 first" >&2
    echo "  --log[=LEVEL]    show compiler output and set RUSTC_LOG (default: info)" >&2
}

for arg in "$@"; do
    case "$arg" in
        -b|--build) DO_BUILD=1 ;;
        -l|--log) LOG_LEVEL="${LOG_LEVEL:-info}" ;;
        --log=*) LOG_LEVEL="${arg#--log=}" ;;
        -h|--help) usage; exit 0 ;;
        -*) echo "unknown option: $arg" >&2; usage; exit 2 ;;
        *) STAGES+=("$arg") ;;
    esac
done

stage() {
    local label="$1"; shift
    local desc="$1"; shift
    local log="$OUT_DIR/$label.log"

    printf '== %-6s  %s\n' "$label" "$desc"
    if [[ -n "$LOG_LEVEL" ]]; then
        if RUSTC_LOG="$LOG_LEVEL" "${RUSTC[@]}" "$SRC" "$@" >"$log" 2>&1; then
            printf '   OK\n'
        else
            printf '   FAILED\n'
            cat "$log"
            exit 1
        fi
        if [[ -s "$log" ]]; then
            cat "$log"
        fi
    else
        if "${RUSTC[@]}" "$SRC" "$@" >/dev/null 2>"$log"; then
            printf '   OK\n'
            rm -f "$log"
        else
            printf '   FAILED\n'
            cat "$log"
            rm -f "$log"
            exit 1
        fi
    fi
}

run_stage() {
    case "$1" in
        lex)   stage "lex"   "parse crate root only"     -Z parse-crate-root-only ;;
        parse) stage "parse" "parse + expand + resolve"  -Z no-analysis ;;
        hir)   stage "hir"   "lower to HIR"              -Z unpretty=hir ;;
        mir)   stage "mir"   "build MIR"                 --emit mir -o "$OUT_DIR/test.mir" ;;
        full)  stage "full"  "codegen and link"          -o "$OUT_DIR/test" ;;
        *)     echo "unknown stage: $1" >&2; usage; exit 2 ;;
    esac
}

if (( DO_BUILD == 1 )); then
    printf '== build  %s\n' "./x.py build --stage 1 --keep-stage-std 1"
    if ./x.py build --stage 1 --keep-stage-std 1; then
        printf '   OK\n'
    else
        printf '   FAILED\n'
        exit 1
    fi
fi

if (( ${#STAGES[@]} > 0 )); then
    for s in "${STAGES[@]}"; do
        run_stage "$s"
    done
else
    for s in lex parse hir mir full; do
        run_stage "$s"
    done
    printf '\nAll stages passed. binary: %s/test\n' "$OUT_DIR"
fi

if [[ -n "$LOG_LEVEL" ]]; then
    printf 'logs: %s/*.log (RUSTC_LOG=%s)\n' "$OUT_DIR" "$LOG_LEVEL"
fi

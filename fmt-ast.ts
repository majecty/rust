#!/usr/bin/env node
// rustc -Z unpretty=ast-tree 출력을 사람이 읽기 쉽게 포매팅
// span/syntax_context/Id/NodeId 등 내부 정보를 제거하고 구조만 남김
// usage: node fmt-ast.ts <file>  또는  echo '...' | node fmt-ast.ts

import { readFileSync } from "node:fs";

const input = process.argv[2]
  ? readFileSync(process.argv[2], "utf8")
  : readFileSync(0, "utf8");

function clean(s: string): string {
  let r = s;

  // Span 정보 제거: ./someany/test.rs:3:1: 3:3 (#0) → 제거
  r = r.replace(/\.\//g, "");
  r = r.replace(/:\d+:\d+:\s*\d+:\d+\s*\(#\d+\)/g, "");
  r = r.replace(/:\d+:\d+/g, "");

  // Id(...) 내부 정리 - 텍스트만 남기기
  r = r.replace(/Id\((\d+)\)/g, "$1");

  // NodeId(...) 정리
  r = r.replace(/NodeId\((\d+)\)/g, "$1");

  // 불필요한 속성 제거 (선택적)
  r = r.replace(/Sugar: false,?\s*/g, "");
  r = r.replace(/Attrs: \[\],?\s*/g, "");
  r = r.replace(/Tokens: None,?\s*/g, "");

  // 중복 공백/쉼표 정리
  r = r.replace(/,\s*\}/g, " }");
  r = r.replace(/,\s*\]/g, " ]");
  r = r.replace(/\n\s*\n/g, "\n");

  // 불필요한 중괄호 라인 정리
  const lines = r.split("\n");
  const out: string[] = [];
  let prevBlank = false;

  for (const line of lines) {
    const trimmed = line.trim();
    // 순수 닫는 괄호만 있는 라인은 들여쓰기만 유지
    if (/^[}\])]/.test(trimmed) && trimmed.length <= 3) {
      out.push(line);
      prevBlank = false;
      continue;
    }
    // 빈 줄 연속 제거
    if (trimmed === "") {
      if (!prevBlank) out.push(line);
      prevBlank = true;
      continue;
    }
    out.push(line);
    prevBlank = false;
  }

  return out.join("\n");
}

console.log(clean(input));

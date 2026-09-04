#!/usr/bin/env node
// rustc TokenStream Debug 포맷 → 사람이 읽기 쉬운 형태로 변환
// usage: node fmt-tokenstream.ts <file>  또는  echo '...' | node fmt-tokenstream.ts

import { readFileSync } from "node:fs";

const input = process.argv[2]
  ? readFileSync(process.argv[2], "utf8")
  : readFileSync(0, "utf8");

function findMatching(s: string, start: number): number {
  let depth = 0;
  for (let i = start; i < s.length; i++) {
    if (s[i] === "(") depth++;
    else if (s[i] === ")") {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

function splitLastComma(s: string): string[] {
  let depth = 0;
  for (let i = s.length - 1; i >= 0; i--) {
    if (s[i] === ")") depth++;
    else if (s[i] === "(") depth--;
    else if (s[i] === "," && depth === 0) {
      return [s.slice(0, i), s.slice(i + 1)];
    }
  }
  return [s, ""];
}

function decodeTokenKind(kind: string, body: string): string {
  const punct: Record<string, string> = {
    Semi: ";", Comma: ",", Colon: ":", Dot: ".", Eq: "=", Bang: "!",
    Lt: "<", Gt: ">", Plus: "+", Minus: "-", Star: "*", Slash: "/",
    And: "&", Or: "|", Caret: "^", RArrow: "->", OpenParen: "(",
    CloseParen: ")", OpenBrace: "{", CloseBrace: "}",
    OpenBracket: "[", CloseBracket: "]", DotDot: "..",
    Pound: "#", At: "@", Tilde: "~", Question: "?", Percent: "%",
  };
  if (kind === "Ident") {
    const m = body.match(/Ident\("([^"]*)"/);
    return m ? m[1] : "ident";
  }
  if (kind === "Literal") {
    const m = body.match(/symbol:\s*"([^"]*)"/);
    return m ? m[1] : "lit";
  }
  return punct[kind] || kind;
}

function render(s: string, depth: number): string {
  const pad = "  ".repeat(depth);
  const out: string[] = [];
  let i = 0;

  while (i < s.length) {
    while (i < s.length && /[\s,]/.test(s[i])) i++;
    if (i >= s.length) break;

    // TokenStream([]) 빈 것
    if (s.slice(i, i + 16) === "TokenStream([])") {
      i += 16;
      continue;
    }
    // TokenStream([ 开始
    if (s.slice(i, i + 14) === "TokenStream([") {
      i += 14;
      continue;
    }
    // ] 끝 (TokenStream 닫기)
    if (s[i] === "]") { i++; continue; }

    // DelimSpan { ... }
    if (s.slice(i, i + 10) === "DelimSpan ") {
      let j = i;
      let d = 0;
      while (j < s.length) {
        if (s[j] === "{") d++;
        else if (s[j] === "}") { d--; if (d === 0) { i = j + 1; break; } }
        j++;
      }
      continue;
    }
    // DelimSpacing { ... }
    if (s.slice(i, i + 13) === "DelimSpacing ") {
      let j = i;
      let d = 0;
      while (j < s.length) {
        if (s[j] === "{") d++;
        else if (s[j] === "}") { d--; if (d === 0) { i = j + 1; break; } }
        j++;
      }
      continue;
    }
    // Parenthesis / Brace / Bracket (delim type)
    if (/^(Parenthesis|Brace|Bracket)/.test(s.slice(i))) {
      const m = s.slice(i).match(/^(Parenthesis|Brace|Bracket)/)!;
      i += m[1].length;
      continue;
    }

    // Delimited(...)
    if (s.slice(i, i + 10) === "Delimited(") {
      const end = findMatching(s, i + 9);
      if (end === -1) break;
      const inner = s.slice(i + 10, end);

      // DelimSpan, DelimSpacing, delim type 건너뛰고 마지막 TokenStream內容만 추출
      const parts = splitLastComma(inner);
      const tsContent = parts[1]?.trim() || "";

      // delim type 추출
      const delimM = inner.match(/,\s*(Parenthesis|Brace|Bracket),/);
      const delim = delimM ? delimM[1] : "Parenthesis";
      const opener = delim === "Brace" ? "{" : delim === "Bracket" ? "[" : "(";
      const closer = delim === "Brace" ? "}" : delim === "Bracket" ? "]" : ")";

      out.push(`${pad}${opener}`);
      if (tsContent && tsContent !== "TokenStream([])") {
        // TokenStream([...]) 벗기기
        const innerTs = tsContent
          .replace(/^TokenStream\(\[/, "")
          .replace(/\]\)$/, "");
        out.push(render(innerTs, depth + 1));
      }
      out.push(`${pad}${closer}`);
      i = end + 1;
      continue;
    }

    // Token(Token { ... }, Spacing)
    if (s.slice(i, i + 14) === "Token(Token {") {
      const bodyEnd = s.indexOf("}, ", i + 14);
      if (bodyEnd === -1) break;
      const body = s.slice(i + 14, bodyEnd);

      const spStart = bodyEnd + 3;
      let spEnd = spStart;
      while (spEnd < s.length && s[spEnd] !== ")") spEnd++;
      const spacing = s.slice(spStart, spEnd).trim();
      i = spEnd + 1;

      const kindM = body.match(/kind:\s*(\w+)/);
      const kind = kindM ? kindM[1] : "?";
      const display = decodeTokenKind(kind, body);

      const spanM = body.match(/span:\s*([^,]+)/);
      const span = spanM ? spanM[1].trim() : "";

      // Joint이면 공백 없이, 아니면 공백 추가
      const suffix = spacing === "Joint" ? "" : " ";
      out.push(`${pad}${display}${span ? ` /*${span}*/` : ""}${suffix}`);
      continue;
    }

    i++;
  }

  return out.join("\n");
}

// ── 실행 ──
const cleaned = input.trim();
const inner = cleaned
  .replace(/^TokenStream\(\[/, "")
  .replace(/\]\)\s*$/, "");

console.log(render(inner, 0));

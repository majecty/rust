const fs = require("fs");
const path = process.argv[2];
if (!path) { console.error("usage: node smoke_test_ast.js <file.html>"); process.exit(1); }
const src = fs.readFileSync(path, "utf8");

function extractArray(name) {
  const re = new RegExp("const " + name + "\\s*=\\s*\\[", "g");
  const m = re.exec(src);
  if (!m) throw new Error("cannot find " + name);
  let i = m.index + m[0].length;
  let depth = 1, inStr = false, strCh = "", esc = false;
  let out = "";
  while (i < src.length && depth > 0) {
    const c = src[i];
    if (inStr) {
      if (esc) { esc = false; }
      else if (c === "\\") { esc = true; }
      else if (c === strCh) { inStr = false; }
      i++; continue;
    }
    if (c === '"' || c === "'" || c === "`") { inStr = true; strCh = c; i++; continue; }
    if (c === "[") depth++;
    if (c === "]") { depth--; if (depth === 0) break; }
    out += c;
    i++;
  }
  return out;
}

const codeBody = extractArray("CODE");
let len = 0, inStr = false, strCh = "", esc = false, depth = 0;
for (const c of codeBody) {
  if (inStr) {
    if (esc) esc = false;
    else if (c === "\\") esc = true;
    else if (c === strCh) inStr = false;
    continue;
  }
  if (c === '"' || c === "'" || c === "`") { inStr = true; strCh = c; continue; }
  if (c === "[") depth++;
  if (c === "]") depth--;
  if (c === "," && depth === 0) { len++; }
}
len++;
console.log("CODE length =", len);

const cardsRe = /start:\s*(\d+),\s*end:\s*(\d+)/g;
let m, idx = 0, ok = true;
while ((m = cardsRe.exec(src))) {
  const s = +m[1], e = +m[2];
  const valid = s >= 0 && e >= s && e < len;
  console.log("card[" + idx + "] start=" + s + " end=" + e + " -> " + (valid ? "OK" : "BAD"));
  if (!valid) ok = false;
  idx++;
}
console.log("cards found =", idx);

const opens = (src.match(/\{/g)||[]).length;
const closes = (src.match(/\}/g)||[]).length;
console.log("braces { =", opens, "} =", closes, opens === closes ? "OK" : "MISMATCH");

if (ok && opens === closes) {
  console.log("SMOKE_TEST_OK: page looks structurally valid (" + len + " code lines, " + idx + " cards)");
} else {
  console.log("SMOKE_TEST_FAIL");
  process.exit(1);
}

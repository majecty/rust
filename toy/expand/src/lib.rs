//! rtoy expand — 미니 함수형 매크로 (span 계층 전).
//! twice!(x) -> x + x, my_let! -> let tmp. 생성토큰은 call span, 인자는 원본 span.

use rtoy_lexer::{Token, TokenKind};
use rtoy_span::{Span, SpanError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MToken {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandError {
    pub mac: String,
    pub expected: String,
    pub got: usize,
    pub source: Option<SpanError>,
}

impl std::fmt::Display for ExpandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expand {}: expected {} but got {} arg(s)", self.mac, self.expected, self.got)
    }
}

impl std::error::Error for ExpandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e as &dyn std::error::Error)
    }
}

/// lexer 토큰에 원문 텍스트를 붙여 매크로 인자로 승격. 실패 원인은 호출자가 문구로 노출.
pub fn lift(tokens: &[Token], src: &str) -> Result<Vec<MToken>, SpanError> {
    tokens
        .iter()
        .map(|t| t.span.try_snippet(src).map(|s| MToken { kind: t.kind, text: s.to_string(), span: t.span }).map_err(|e| e))
        .collect()
}

/// 미니 전개. 성공 시 토큰열, 실패 시 가장 안쪽 원인까지 담은 에러.
pub fn expand(mac: &str, args: Vec<MToken>, call: Span) -> Result<Vec<MToken>, ExpandError> {
    match mac {
        "twice" => {
            if args.len() != 1 {
                return Err(ExpandError { mac: mac.into(), expected: "1 arg".into(), got: args.len(), source: None });
            }
            let a = args.into_iter().next().expect("checked len");
            Ok(vec![
                a.clone(),
                MToken { kind: TokenKind::Punct, text: "+".into(), span: call },
                a,
            ])
        }
        "my_let" => {
            if !args.is_empty() {
                return Err(ExpandError { mac: mac.into(), expected: "0 args".into(), got: args.len(), source: None });
            }
            Ok(vec![
                MToken { kind: TokenKind::Ident, text: "let".into(), span: call },
                MToken { kind: TokenKind::Ident, text: "tmp".into(), span: call },
            ])
        }
        other => Err(ExpandError { mac: other.into(), expected: "twice|my_let".into(), got: args.len(), source: None }),
    }
}

/// AST 매크로 전개. `twice!(x)` -> `x + x` (인자 span 복제, `+`는 call span).
/// 재귀적으로 자식을 먼저 전개하고, 가장 안쪽 실패까지 에러로 전달한다.
pub fn expand_expr(e: rtoy_ast::Expr) -> Result<rtoy_ast::Expr, ExpandError> {
    use rtoy_ast::{BinOp, Expr, ExprKind};
    let span = e.span;
    let kind = match e.kind {
        ExprKind::Macro { name, args } if name.name == "twice" => {
            if args.len() != 1 {
                return Err(ExpandError { mac: name.name, expected: "1 arg".into(), got: args.len(), source: None });
            }
            let mut it = args.into_iter().map(expand_expr);
            let a0 = it.next().expect("checked len")?;
            let lhs = Box::new(a0.clone());
            let rhs = Box::new(a0);
            ExprKind::Binary { op: BinOp::Add, lhs, rhs }
        }
        ExprKind::Macro { name, args } => {
            let n = args.len();
            let mut out = Vec::with_capacity(n);
            for a in args {
                out.push(expand_expr(a)?);
            }
            ExprKind::Macro { name, args: out }
        }
        ExprKind::Binary { op, lhs, rhs } => ExprKind::Binary {
            op,
            lhs: Box::new(expand_expr(*lhs)?),
            rhs: Box::new(expand_expr(*rhs)?),
        },
        ExprKind::Call { callee, args } => {
            let mut out = Vec::with_capacity(args.len());
            for a in args {
                out.push(expand_expr(a)?);
            }
            ExprKind::Call { callee, args: out }
        }
        other => other,
    };
    Ok(Expr { kind, span })
}

/// 아이템 매크로 전개. `def_fn!(foo)` -> `fn foo(){}` (빈 몸, 이름 span=인자 span).
/// fn 몸 안의 `twice!`는 그대로 재귀 전개한다.
pub fn expand_item(item: rtoy_ast::Item) -> Result<rtoy_ast::Item, ExpandError> {
    use rtoy_ast::{Block, FnItem, Item, ItemKind};
    match item.kind {
        ItemKind::Macro { name, args } if name.name == "def_fn" => {
            if args.len() != 1 {
                return Err(ExpandError { mac: name.name, expected: "1 arg (fn name)".into(), got: args.len(), source: None });
            }
            let mut it = args.into_iter().map(expand_expr);
            let arg = it.next().expect("checked len")?;
            let rtoy_ast::ExprKind::Var { name: fn_name, .. } = arg.kind else {
                return Err(ExpandError { mac: "def_fn".into(), expected: "ident (e.g. def_fn!(foo))".into(), got: 1, source: None });
            };
            let span = item.span;
            let name = rtoy_ast::Ident { name: fn_name.name, span: rtoy_span::Span::copied_arg(fn_name.span, item.span) };
            let empty = Block { stmts: vec![], tail: None, span };
            Ok(Item { name, kind: ItemKind::Fn(FnItem { body: empty, locals: 0, span }), span })
        }
        ItemKind::Macro { name, args } => {
            let n = args.len();
            let mut out = Vec::with_capacity(n);
            for a in args {
                out.push(expand_expr(a)?);
            }
            Ok(Item { name: item.name, kind: ItemKind::Macro { name, args: out }, span: item.span })
        }
        ItemKind::Fn(mut f) => {
            let mut stmts = std::mem::take(&mut f.body.stmts);
            for s in &mut stmts {
                match &mut s.kind {
                    rtoy_ast::StmtKind::Expr(e) => *e = expand_expr(std::mem::replace(e, dummy_expr()))?,
                    rtoy_ast::StmtKind::Let(l) => {
                        if let Some(init) = l.init.take() {
                            l.init = Some(expand_expr(init)?);
                        }
                    }
                }
            }
            f.body.stmts = stmts;
            if let Some(t) = f.body.tail.take() {
                f.body.tail = Some(expand_expr(t)?);
            }
            Ok(rtoy_ast::Item { name: item.name, kind: ItemKind::Fn(f), span: item.span })
        }
        ItemKind::MacroDef(def) => {
            // macro_rules! 정의는 그대로 유지 (나중에 resolve에서 사용)
            Ok(rtoy_ast::Item { name: item.name, kind: ItemKind::MacroDef(def), span: item.span })
        }
        ItemKind::Struct(s) => {
            // 구조체 정의는 그대로 유지 (eval에서 사용)
            Ok(rtoy_ast::Item { name: item.name, kind: ItemKind::Struct(s), span: item.span })
        }
    }
}

/// Crate 전체의 `twice!`/`def_fn!`를 전개한다.
pub fn expand_crate(krate: rtoy_ast::Crate) -> Result<rtoy_ast::Crate, ExpandError> {
    let mut items = Vec::with_capacity(krate.items.len());
    for item in krate.items {
        items.push(expand_item(item)?);
    }
    Ok(rtoy_ast::Crate { items, span: krate.span })
}

fn dummy_expr() -> rtoy_ast::Expr {
    rtoy_ast::Expr { kind: rtoy_ast::ExprKind::Int(0), span: Span::root(0, 0) }
}

// ─── macro_rules! 패턴 매칭 + 전개 ───────────────────────────────

use rtoy_ast::{MacroDef, TokenTree, TokenNode};

/// 토큰 리스트를 TokenTree로 변환 (최소 파서).
pub fn parse_token_trees(toks: &[Token], src: &str) -> Result<Vec<TokenTree>, ExpandError> {
    let mut trees = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        let text = t.span.try_snippet(src).unwrap_or("?").to_string();
        match t.kind {
            TokenKind::Punct if text == "$" && i + 1 < toks.len() => {
                // $x:expr 같은 플레이스홀더
                i += 1;
                let var_name = toks[i].span.try_snippet(src).unwrap_or("?").to_string();
                if i + 2 < toks.len() && toks[i + 1].span.try_snippet(src).unwrap_or("") == ":" {
                    i += 2;
                    let frag = toks[i].span.try_snippet(src).unwrap_or("?").to_string();
                    trees.push(TokenTree::Placeholder { name: var_name, frag });
                    i += 1;
                } else {
                    trees.push(TokenTree::Placeholder { name: var_name, frag: "expr".into() });
                }
            }
            TokenKind::Ident => {
                trees.push(TokenTree::Token(TokenNode::Ident(text)));
                i += 1;
            }
            TokenKind::Int => {
                trees.push(TokenTree::Token(TokenNode::Literal(text)));
                i += 1;
            }
            TokenKind::Punct if text == "(" || text == "{" || text == "[" => {
                let close = match text.as_str() {
                    "(" => ')',
                    "{" => '}',
                    "[" => ']',
                    _ => unreachable!(),
                };
                let (inner, end) = parse_delimited(toks, src, i, close)?;
                let kind = match text.as_str() {
                    "(" => rtoy_ast::DelimKind::Paren,
                    "{" => rtoy_ast::DelimKind::Brace,
                    _ => rtoy_ast::DelimKind::Bracket,
                };
                trees.push(TokenTree::Delimited { kind, trees: inner });
                i = end;
            }
            TokenKind::Punct if text == ")" || text == "}" || text == "]" => {
                return Err(ExpandError { mac: "parse".into(), expected: "no close delim".into(), got: i, source: None });
            }
            TokenKind::Punct => {
                trees.push(TokenTree::Token(TokenNode::Punct(text)));
                i += 1;
            }
            _ => {
                // Whitespace, Comment 등 무시
                i += 1;
            }
        }
    }
    Ok(trees)
}

fn parse_delimited(toks: &[Token], src: &str, start: usize, close: char) -> Result<(Vec<TokenTree>, usize), ExpandError> {
    let mut inner = Vec::new();
    let mut depth = 1;
    let mut i = start + 1;
    while i < toks.len() && depth > 0 {
        let t = &toks[i];
        let text = t.span.try_snippet(src).unwrap_or("?").to_string();
        match t.kind {
            TokenKind::Punct if text == "(" || text == "{" || text == "[" => {
                depth += 1;
                inner.push(TokenTree::Token(TokenNode::Punct(text)));
                i += 1;
            }
            TokenKind::Punct if text == ")" || text == "}" || text == "]" => {
                if text.chars().next() == Some(close) {
                    depth -= 1;
                    if depth > 0 {
                        inner.push(TokenTree::Token(TokenNode::Punct(text)));
                    }
                    i += 1;
                } else {
                    return Err(ExpandError { mac: "parse".into(), expected: format!("closing {:?}", close), got: i, source: None });
                }
            }
            TokenKind::Ident if text.starts_with('$') && text.len() > 1 => {
                let rest = &text[1..];
                if let Some((name, frag)) = rest.split_once(':') {
                    inner.push(TokenTree::Placeholder { name: name.into(), frag: frag.into() });
                } else {
                    inner.push(TokenTree::Token(TokenNode::Ident(text)));
                }
                i += 1;
            }
            TokenKind::Ident => {
                inner.push(TokenTree::Token(TokenNode::Ident(text)));
                i += 1;
            }
            TokenKind::Int => {
                inner.push(TokenTree::Token(TokenNode::Literal(text)));
                i += 1;
            }
            TokenKind::Punct => {
                inner.push(TokenTree::Token(TokenNode::Punct(text)));
                i += 1;
            }
            _ => {
                // Whitespace, Comment 등 무시
                i += 1;
            }
        }
    }
    Ok((inner, i))
}

/// 패턴 매칭: args(인자 토큰)와 pattern이 일치하면 매핑 반환.
pub fn match_pattern(args: &[TokenTree], pattern: &[TokenTree]) -> Result<Vec<(String, Vec<TokenTree>)>, ExpandError> {
    let mut bindings = Vec::new();
    let mut ai = 0;
    let mut pi = 0;
    while pi < pattern.len() {
        match &pattern[pi] {
            TokenTree::Placeholder { name, .. } => {
                // 다음 패턴 토큰이 나올 때까지 토큰을 capture
                let mut captured = Vec::new();
                while ai < args.len() {
                    // 다음 패턴이 Token이면, 현재 arg가 그 Token과 일치하는지 확인
                    if pi + 1 < pattern.len() {
                        if let TokenTree::Token(expected) = &pattern[pi + 1] {
                            if let TokenTree::Token(actual) = &args[ai] {
                                let matches = match (actual, expected) {
                                    (TokenNode::Ident(a), TokenNode::Ident(b)) => a == b,
                                    (TokenNode::Punct(a), TokenNode::Punct(b)) => a == b,
                                    (TokenNode::Literal(a), TokenNode::Literal(b)) => a == b,
                                    _ => false,
                                };
                                if matches {
                                    break; // 다음 토큰과 일치하므로 여기서 중단
                                }
                            }
                        }
                    }
                    captured.push(args[ai].clone());
                    ai += 1;
                }
                bindings.push((name.clone(), captured));
                pi += 1;
            }
            TokenTree::Token(expected) => {
                if ai >= args.len() {
                    return Err(ExpandError { mac: "match".into(), expected: format!("token {:?}", expected), got: 0, source: None });
                }
                match (&args[ai], expected) {
                    (TokenTree::Token(TokenNode::Ident(a)), TokenNode::Ident(b)) if a == b => {}
                    (TokenTree::Token(TokenNode::Punct(a)), TokenNode::Punct(b)) if a == b => {}
                    (TokenTree::Token(TokenNode::Literal(a)), TokenNode::Literal(b)) if a == b => {}
                    _ => return Err(ExpandError { mac: "match".into(), expected: format!("{:?}", expected), got: 0, source: None }),
                }
                ai += 1;
                pi += 1;
            }
            _ => {
                // Delimited 등은 단순 비교 (미지원)
                pi += 1;
            }
        }
    }
    if ai != args.len() {
        return Err(ExpandError { mac: "match".into(), expected: "exact match".into(), got: args.len() - ai, source: None });
    }
    Ok(bindings)
}

/// 전개: 템플릿의 Placeholder를 바인딩으로 치환.
pub fn transcribe(template: &[TokenTree], bindings: &[(String, Vec<TokenTree>)], call_span: Span) -> Vec<TokenTree> {
    let mut out = Vec::new();
    for t in template {
        match t {
            TokenTree::Placeholder { name, .. } => {
                if let Some((_, captured)) = bindings.iter().find(|(n, _)| n == name) {
                    out.extend(captured.iter().cloned());
                }
            }
            TokenTree::Delimited { kind, trees } => {
                let expanded = transcribe(trees, bindings, call_span);
                out.push(TokenTree::Delimited { kind: kind.clone(), trees: expanded });
            }
            other => out.push(other.clone()),
        }
    }
    out
}

/// TokenTree를 MToken 리스트로 변환.
pub fn trees_to_mtokens(trees: &[TokenTree], call_span: Span) -> Vec<MToken> {
    let mut out = Vec::new();
    for t in trees {
        match t {
            TokenTree::Token(TokenNode::Ident(s)) => {
                out.push(MToken { kind: TokenKind::Ident, text: s.clone(), span: call_span });
            }
            TokenTree::Token(TokenNode::Literal(s)) => {
                out.push(MToken { kind: TokenKind::Int, text: s.clone(), span: call_span });
            }
            TokenTree::Token(TokenNode::Punct(s)) => {
                out.push(MToken { kind: TokenKind::Punct, text: s.clone(), span: call_span });
            }
            TokenTree::Placeholder { .. } => {} // 치환 후에는 없어야 함
            TokenTree::Delimited { .. } => {} // 간소화: 중첩 무시
        }
    }
    out
}

/// macro_rules! 정의를 AST로 변환.
pub fn expand_macro_def(def: MacroDef, defs: &mut Vec<MacroDef>) -> Result<MacroDef, ExpandError> {
    // 현재는 정의를 저장만 하고 패턴 검증
    for arm in &def.arms {
        let pat = &arm.pattern;
        let _ = pat; // 패턴 유효성은 실제 호출 시 검증
    }
    defs.push(def.clone());
    Ok(def)
}

/// 매크로 호출을 확장 (macro_rules! 기반).
pub fn expand_macro_call(
    name: &str,
    args: Vec<TokenTree>,
    defs: &[MacroDef],
    call_span: Span,
) -> Result<Vec<TokenTree>, ExpandError> {
    for def in defs {
        if def.name.name == name {
            for arm in &def.arms {
                let pat_trees = match &arm.pattern {
                    TokenTree::Delimited { trees, .. } => trees,
                    _ => continue,
                };
                if let Ok(bindings) = match_pattern(&args, pat_trees) {
                    let tmpl_trees = match &arm.template {
                        TokenTree::Delimited { trees, .. } => trees,
                        _ => continue,
                    };
                    return Ok(transcribe(tmpl_trees, &bindings, call_span));
                }
            }
            return Err(ExpandError {
                mac: name.into(),
                expected: "matching arm".into(),
                got: args.len(),
                source: None,
            });
        }
    }
    Err(ExpandError { mac: name.into(), expected: "defined macro".into(), got: 0, source: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtoy_lexer::tokenize;

    fn lifted_arg(src: &str) -> (MToken, Span) {
        let toks = tokenize(src);
        let arg = toks.iter().find(|t| t.kind == TokenKind::Ident && t.span.try_snippet(src).unwrap_or("") == "y").expect("y");
        let all = lift(&toks, src).unwrap();
        let y = all.into_iter().find(|m| m.text == "y").unwrap();
        let _ = arg;
        (y, Span::root(0, 9))
    }

    #[test]
    fn twice_clones_arg_span() {
        let (y, call) = lifted_arg("twice!(y)");
        let out = expand("twice", vec![y.clone()], call).unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!([out[0].text.clone(), out[1].text.clone(), out[2].text.clone()], ["y", "+", "y"]);
        assert_eq!(out[0].span, y.span);
        assert_eq!(out[2].span, y.span);
        assert_eq!(out[1].span, call);
    }

    #[test]
    fn my_let_collides_without_hygiene() {
        // 문제 재현: 매크로 tmp와 사용자 tmp가 span만으로 구분 불가.
        let call = Span::root(10, 20);
        let out = expand("my_let", vec![], call).unwrap();
        let user_tmp = MToken { kind: TokenKind::Ident, text: "tmp".into(), span: Span::root(0, 3) };
        assert_eq!(out[1].text, user_tmp.text);
        assert_ne!(out[1].span, user_tmp.span);
        // 텍스트·span 범위만으로는 같은 변수인지 판단 불가 -> ctxt 필요.
    }

    #[test]
    fn unknown_macro_reports_cause() {
        let e = expand("nope", vec![], Span::root(0, 1)).unwrap_err();
        assert!(e.to_string().contains("nope"));
    }

    #[test]
    fn def_fn_creates_empty_fn() {
        use rtoy_ast::{Expr, ExprKind, Ident, Item, ItemKind};
        let call = Span::root(0, 12);
        let arg_span = Span::root(7, 10);
        let item = Item {
            name: Ident { name: "def_fn".into(), span: Span::root(0, 6) },
            kind: ItemKind::Macro {
                name: Ident { name: "def_fn".into(), span: Span::root(0, 6) },
                args: vec![Expr { kind: ExprKind::Var { name: Ident { name: "foo".into(), span: arg_span }, slot: None }, span: arg_span }],
            },
            span: call,
        };
        let out = expand_item(item).unwrap();
        assert_eq!(out.name.name, "foo");
        // copied_arg는 lo/hi/ctxt를 인자 그대로 두고 parent로 확장 정보만 부여한다.
        assert_eq!(out.name.span.lo, arg_span.lo);
        assert_eq!(out.name.span.hi, arg_span.hi);
        assert_eq!(out.name.span.ctxt, arg_span.ctxt);
        assert!(out.name.span.parent.is_some());
        assert!(matches!(out.kind, ItemKind::Fn(f) if f.body.stmts.is_empty() && f.body.tail.is_none()));
    }

    #[test]
    fn parse_single_token() {
        let toks = rtoy_lexer::tokenize("x");
        let trees = parse_token_trees(&toks, "x").unwrap();
        assert_eq!(trees.len(), 1);
        assert!(matches!(&trees[0], TokenTree::Token(TokenNode::Ident(s)) if s == "x"));
    }

    #[test]
    fn parse_placeholder() {
        let toks = rtoy_lexer::tokenize("$x:expr");
        let trees = parse_token_trees(&toks, "$x:expr").unwrap();
        assert_eq!(trees.len(), 1);
        assert!(matches!(&trees[0], TokenTree::Placeholder { name, frag } if name == "x" && frag == "expr"));
    }

    #[test]
    fn match_simple_pattern() {
        let pattern = vec![TokenTree::Placeholder { name: "x".into(), frag: "expr".into() }];
        let args = vec![TokenTree::Token(TokenNode::Ident("foo".into()))];
        let bindings = match_pattern(&args, &pattern).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].0, "x");
        assert_eq!(bindings[0].1, args);
    }

    #[test]
    fn match_with_literal_separator() {
        let pattern = vec![
            TokenTree::Placeholder { name: "a".into(), frag: "expr".into() },
            TokenTree::Token(TokenNode::Punct("+".into())),
            TokenTree::Placeholder { name: "b".into(), frag: "expr".into() },
        ];
        let args = vec![
            TokenTree::Token(TokenNode::Ident("x".into())),
            TokenTree::Token(TokenNode::Punct("+".into())),
            TokenTree::Token(TokenNode::Ident("y".into())),
        ];
        let bindings = match_pattern(&args, &pattern).unwrap();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].0, "a");
        assert_eq!(bindings[1].0, "b");
    }

    #[test]
    fn transcribe_simple() {
        let template = vec![
            TokenTree::Placeholder { name: "x".into(), frag: "expr".into() },
            TokenTree::Token(TokenNode::Punct("+".into())),
            TokenTree::Placeholder { name: "x".into(), frag: "expr".into() },
        ];
        let bindings = vec![
            ("x".into(), vec![TokenTree::Token(TokenNode::Ident("foo".into()))]),
        ];
        let result = transcribe(&template, &bindings, Span::root(0, 0));
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[0], TokenTree::Token(TokenNode::Ident(s)) if s == "foo"));
        assert!(matches!(&result[1], TokenTree::Token(TokenNode::Punct(s)) if s == "+"));
        assert!(matches!(&result[2], TokenTree::Token(TokenNode::Ident(s)) if s == "foo"));
    }

    #[test]
    fn expand_macro_rules_call() {
        let defs = vec![MacroDef {
            name: rtoy_ast::Ident { name: "twice".into(), span: Span::root(0, 5) },
            arms: vec![rtoy_ast::MacroArm {
                pattern: TokenTree::Delimited {
                    kind: rtoy_ast::DelimKind::Paren,
                    trees: vec![TokenTree::Placeholder { name: "x".into(), frag: "expr".into() }],
                },
                template: TokenTree::Delimited {
                    kind: rtoy_ast::DelimKind::Brace,
                    trees: vec![
                        TokenTree::Placeholder { name: "x".into(), frag: "expr".into() },
                        TokenTree::Token(TokenNode::Punct("+".into())),
                        TokenTree::Placeholder { name: "x".into(), frag: "expr".into() },
                    ],
                },
                span: Span::root(0, 30),
            }],
            vis: rtoy_ast::Vis::Private,
            span: Span::root(0, 35),
        }];

        let args = vec![TokenTree::Token(TokenNode::Ident("y".into()))];
        let result = expand_macro_call("twice", args, &defs, Span::root(10, 20)).unwrap();
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[0], TokenTree::Token(TokenNode::Ident(s)) if s == "y"));
        assert!(matches!(&result[1], TokenTree::Token(TokenNode::Punct(s)) if s == "+"));
        assert!(matches!(&result[2], TokenTree::Token(TokenNode::Ident(s)) if s == "y"));
    }

    #[test]
    fn vis_pub_crate() {
        use rtoy_ast::{MacroDef, Vis};
        let def = MacroDef {
            name: rtoy_ast::Ident { name: "my_mac".into(), span: Span::root(0, 6) },
            arms: vec![],
            vis: Vis::PubCrate,
            span: Span::root(0, 10),
        };
        assert!(matches!(def.vis, Vis::PubCrate));
    }
}

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
            let rtoy_ast::ExprKind::Var(fn_name) = arg.kind else {
                return Err(ExpandError { mac: "def_fn".into(), expected: "ident (e.g. def_fn!(foo))".into(), got: 1, source: None });
            };
            let span = item.span;
            let empty = Block { stmts: vec![], tail: None, span };
            Ok(Item { name: fn_name, kind: ItemKind::Fn(FnItem { body: empty, span }), span })
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
                args: vec![Expr { kind: ExprKind::Var(Ident { name: "foo".into(), span: arg_span }), span: arg_span }],
            },
            span: call,
        };
        let out = expand_item(item).unwrap();
        assert_eq!(out.name.name, "foo");
        assert_eq!(out.name.span, arg_span);
        assert!(matches!(out.kind, ItemKind::Fn(f) if f.body.stmts.is_empty() && f.body.tail.is_none()));
    }
}

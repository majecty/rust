//! `--trace`용 단계별 AST 요약 출력.

pub fn trace_crate(label: &str, krate: &rtoy_ast::Crate, src: &str) {
    println!("== {label} ==");
    for item in &krate.items {
        println!("fn {} {}", item.name.name, short_span(&item.span));
        let rtoy_ast::ItemKind::Fn(f) = &item.kind else {
            println!("  <unexpanded macro>");
            continue;
        };
        for s in &f.body.stmts {
            match &s.kind {
                rtoy_ast::StmtKind::Let(l) => println!(
                    "  let {} = {} {}",
                    l.name.name,
                    l.init
                        .as_ref()
                        .map(|e| expr_sum(e, src))
                        .unwrap_or("-".into()),
                    short_span(&s.span)
                ),
                rtoy_ast::StmtKind::Expr(e) => {
                    println!("  expr {} {}", expr_sum(e, src), short_span(&s.span))
                }
            }
        }
        match &f.body.tail {
            Some(t) => println!("  tail {}", expr_sum(t, src)),
            None => println!("  tail -"),
        }
    }
}

fn expr_sum(e: &rtoy_ast::Expr, src: &str) -> String {
    let snip = e.span.try_snippet(src).unwrap_or("<invalid>");
    let kind = match &e.kind {
        rtoy_ast::ExprKind::Int(n) => format!("Int({n})"),
        rtoy_ast::ExprKind::Var(v) => format!("Var({})", v.name),
        rtoy_ast::ExprKind::Call { callee, args } => {
            format!("Call({}/{})", callee.name, args.len())
        }
        rtoy_ast::ExprKind::Binary { op, .. } => format!("Binary({op:?})"),
        rtoy_ast::ExprKind::Macro { name, args } => {
            format!("Macro({}!/{})", name.name, args.len())
        }
    };
    format!("{kind} {snip:?} {}", short_span(&e.span))
}

/// trace용 한 줄 요약. 장황한 {:#?} 대신 snippet+span만 노출한다.
fn short_span(s: &rtoy_span::Span) -> String {
    format!("[{}..{}]", s.lo, s.hi)
}

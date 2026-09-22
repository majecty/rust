//! rtoy resolve — AST 이름 검사 (rustc `compiler/rustc_resolve`의 최소 부분).
//! 첫 단계: 함수 이름 중복 정의를 찾아 span과 원문 스니펫을 에러로 노출한다.

use rtoy_ast::{Crate, Expr, ExprKind, ItemKind, StmtKind};
use rtoy_span::{Span, SpanError};
use std::collections::HashMap;

/// 이름 중복 정의. 두 정의의 span·스니펫을 모두 담아 진단에 쓰인다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveError {
    pub context: &'static str,
    pub name: String,
    /// 중복(두 번째) 정의 span — 에러를 가리키는 지점.
    pub span: Span,
    /// 첫 번째 정의 span — "여기서 이미 정의됨" 힌트.
    pub first_span: Span,
    /// 두 span의 원문. src가 없으면 `<invalid span>`.
    pub snippet: String,
    pub first_snippet: String,
    pub source: Option<SpanError>,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: duplicate definition of `{}` (first: {:?} at [{}..{}], duplicate: {:?} at [{}..{}])",
            self.context, self.name, self.first_snippet, self.first_span.lo, self.first_span.hi,
            self.snippet, self.span.lo, self.span.hi
        )
    }
}

impl std::error::Error for ResolveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e as &dyn std::error::Error)
    }
}

fn snippet_of(span: &Span, src: &str) -> String {
    span.try_snippet(src).unwrap_or("<invalid span>").to_string()
}

/// 함수 이름 중복 검사 후, 구조체 필드 참조를 slot 번호로 치환한다 (AST 제자리 변형).
pub fn resolve(krate: &mut Crate, src: &str) -> Result<(), Vec<ResolveError>> {
    check_duplicate_names(krate, src)?;
    resolve_fields(krate);
    Ok(())
}

/// struct 이름 → 필드 선언 순서 (index = slot 번호).
fn struct_field_order(krate: &Crate) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for item in &krate.items {
        if let ItemKind::Struct(s) = &item.kind {
            let names = s.fields.iter().map(|f| f.name.name.clone()).collect();
            map.insert(item.name.name.clone(), names);
        }
    }
    map
}

/// 함수 본문을 훑어 리터럴/필드 접근에 slot 번호를 새긴다.
/// 제어흐름이 아직 없어 지역변수 타입은 선형 추적만으로 충분하다.
fn resolve_fields(krate: &mut Crate) {
    let structs = struct_field_order(krate);
    for item in &mut krate.items {
        let ItemKind::Fn(f) = &mut item.kind else { continue };
        let mut types: HashMap<String, String> = HashMap::new();
        for stmt in &mut f.body.stmts {
            match &mut stmt.kind {
                StmtKind::Let(l) => {
                    if let Some(init) = &mut l.init {
                        patch_expr(init, &types, &structs);
                        if let Some(ty) = expr_struct_name(init, &types) {
                            types.insert(l.name.name.clone(), ty);
                        }
                    }
                }
                StmtKind::Expr(e) => patch_expr(e, &types, &structs),
            }
        }
        if let Some(tail) = &mut f.body.tail {
            patch_expr(tail, &types, &structs);
        }
    }
}

/// 식 트리 재부: 아는 struct 이름에서 필드 slot을 찾아 채운다 (모르면 None 유지).
fn patch_expr(
    expr: &mut Expr,
    types: &HashMap<String, String>,
    structs: &HashMap<String, Vec<String>>,
) {
    match &mut expr.kind {
        ExprKind::StructLiteral { name, fields } => {
            let order = structs.get(&name.name);
            for field in fields.iter_mut() {
                field.slot = slot_of(order, &field.name.name);
                patch_expr(&mut field.value, types, structs);
            }
        }
        ExprKind::FieldAccess { base, field, slot } => {
            patch_expr(base, types, structs);
            *slot = expr_struct_name(base, types)
                .and_then(|ty| slot_of(structs.get(&ty), &field.name));
        }
        ExprKind::If { cond, then_block, else_block } => {
            patch_expr(cond, types, structs);
            patch_block(then_block, types, structs);
            if let Some(block) = else_block {
                patch_block(block, types, structs);
            }
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            patch_expr(lhs, types, structs);
            patch_expr(rhs, types, structs);
        }
        ExprKind::Call { args, .. } | ExprKind::Macro { args, .. } => {
            for arg in args.iter_mut() {
                patch_expr(arg, types, structs);
            }
        }
        ExprKind::Int(_) | ExprKind::Var(_) => {}
    }
}

/// 블록 안 stmt/tail을 순회하며 slot을 새긴다 (분기 안 let은 타입 추적하지 않음).
fn patch_block(
    block: &mut rtoy_ast::Block,
    types: &HashMap<String, String>,
    structs: &HashMap<String, Vec<String>>,
) {
    for stmt in &mut block.stmts {
        match &mut stmt.kind {
            StmtKind::Let(l) => {
                if let Some(init) = &mut l.init {
                    patch_expr(init, types, structs);
                }
            }
            StmtKind::Expr(e) => patch_expr(e, types, structs),
        }
    }
    if let Some(tail) = &mut block.tail {
        patch_expr(tail, types, structs);
    }
}

/// 필드 이름 → 선언 순서(slot 번호).
fn slot_of(order: Option<&Vec<String>>, name: &str) -> Option<u16> {
    Some(order?.iter().position(|n| n == name)? as u16)
}

/// 식이 어떤 struct 값인지 — 리터럴이거나 이미 아는 지역변수만 (그 외 None).
fn expr_struct_name(expr: &Expr, types: &HashMap<String, String>) -> Option<String> {
    match &expr.kind {
        ExprKind::StructLiteral { name, .. } => Some(name.name.clone()),
        ExprKind::Var(ident) => types.get(&ident.name).cloned(),
        _ => None,
    }
}

/// 함수 이름 중복 검사. 첫 정의는 두고, 이후 중복마다 에러를 모은다.
fn check_duplicate_names(krate: &Crate, src: &str) -> Result<(), Vec<ResolveError>> {
    let mut seen: Vec<(&str, Span)> = Vec::new();
    let mut errs = Vec::new();
    for item in &krate.items {
        // 미전개 매크로는 이름 비교에서 제외 (expand 후에는 남지 않음).
        if matches!(item.kind, rtoy_ast::ItemKind::Macro { .. }) {
            continue;
        }
        if let Some((_, first_span)) = seen.iter().find(|(n, _)| *n == item.name.name) {
            errs.push(ResolveError {
                context: "resolve",
                name: item.name.name.clone(),
                span: item.name.span,
                first_span: *first_span,
                snippet: snippet_of(&item.name.span, src),
                first_snippet: snippet_of(first_span, src),
                source: None,
            });
        } else {
            seen.push((&item.name.name, item.name.span));
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtoy_lexer::tokenize;
    use rtoy_tokenstream_lowering::try_lower;

    #[test]
    fn unique_fn_names_pass() {
        let src = "fn main() { 1 } fn foo() { 2 }";
        let mut krate = try_lower(&tokenize(src), src).unwrap();
        assert!(resolve(&mut krate, src).is_ok());
    }

    #[test]
    fn duplicate_fn_names_error() {
        let src = "fn main() { 1 } fn main() { 2 }";
        let mut krate = try_lower(&tokenize(src), src).unwrap();
        let errs = resolve(&mut krate, src).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].name, "main");
        assert_eq!(errs[0].span.snippet(src), "main");
        assert_eq!(errs[0].first_span.snippet(src), "main");
        assert!(errs[0].span.lo > errs[0].first_span.lo);
        let msg = errs[0].to_string();
        assert!(msg.contains("duplicate definition of `main`"), "{msg}");
        assert!(msg.contains("[19..23]"), "{msg}");
    }

    #[test]
    fn def_fn_and_explicit_fn_duplicate() {
        // 고민용: 매크로 생성 foo(span=호출 속 foo) vs 직접 정의 foo.
        let src = "def_fn!(foo) fn foo() { 1 }";
        let krate = try_lower(&tokenize(src), src).unwrap();
        let krate = rtoy_expand::expand_crate(krate).unwrap();
        assert_eq!(krate.items.len(), 2);
        let mut krate = krate;
        let errs = resolve(&mut krate, src).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].name, "foo");
        // 둘 다 원문 "foo"를 가리키지만 위치가 다름: 호출 속 이름 vs fn 이름.
        assert_eq!(errs[0].first_span.snippet(src), "foo");
        assert_eq!(errs[0].span.snippet(src), "foo");
        assert!(errs[0].span.lo > errs[0].first_span.lo);
        let msg = errs[0].to_string();
        assert!(msg.contains("duplicate definition of `foo`"), "{msg}");
    }

    /// main의 tail 필드 접근과 리터럴 필드에서 slot 번호를 꺼낸다.
    fn slots_of(krate: &Crate) -> (Vec<Option<u16>>, (String, Option<u16>)) {
        let ItemKind::Fn(f) = &krate.items[1].kind else { panic!("fn이 아님") };
        let Some(init) = f.body.stmts.first().and_then(|s| match &s.kind {
            StmtKind::Let(l) => l.init.as_ref(),
            StmtKind::Expr(_) => None,
        }) else {
            panic!("let 초기값이 없음")
        };
        let ExprKind::StructLiteral { fields, .. } = &init.kind else {
            panic!("구조체 리터럴이 아님")
        };
        let literal = fields.iter().map(|f| f.slot).collect();
        let tail = f.body.tail.as_ref().expect("tail 없음");
        let ExprKind::FieldAccess { field, slot, .. } = &tail.kind else {
            panic!("필드 접근이 아님")
        };
        (literal, (field.name.clone(), *slot))
    }

    #[test]
    fn struct_literal_and_access_get_slots() {
        let src = "struct Point { x: i64, y: i64 }\nfn main() { let p = Point { x: 1, y: 2 }; p.y }";
        let mut krate = try_lower(&tokenize(src), src).unwrap();
        assert!(resolve(&mut krate, src).is_ok());
        let (literal, (field, slot)) = slots_of(&krate);
        assert_eq!(literal, vec![Some(0), Some(1)]);
        assert_eq!(field, "y");
        assert_eq!(slot, Some(1));
    }

    #[test]
    fn unresolved_base_leaves_slot_none() {
        // foo()의 반환 타입을 모르므로 slot을 새기지 않는다 (eval이 fallback).
        let src = "fn foo() { 1 }\nfn main() { let p = foo(); p.x }";
        let mut krate = try_lower(&tokenize(src), src).unwrap();
        assert!(resolve(&mut krate, src).is_ok());
        let ItemKind::Fn(f) = &krate.items[1].kind else { panic!("fn이 아님") };
        let Some(tail) = &f.body.tail else { panic!("tail 없음") };
        let ExprKind::FieldAccess { slot, .. } = &tail.kind else { panic!("필드 접근이 아님") };
        assert_eq!(*slot, None);
    }
}

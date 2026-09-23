//! rtoy resolve — AST 이름 검사 (rustc `compiler/rustc_resolve`의 최소 부분).
//! 첫 단계: 함수 이름 중복 정의를 찾아 span과 원문 스니펫을 에러로 노출한다.

use rtoy_ast::{collect_local_names, Block, Crate, Expr, ExprKind, Ident, ItemKind, StmtKind};
use rtoy_span::{Span, SpanError};
use std::collections::HashMap;

/// resolve 실패 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveKind {
    /// 같은 이름 재정의 (`first_span` = 먼저 정의된 곳).
    Duplicate,
    /// 이 함수 프레임에 선언이 없는 변수 참조.
    UndefinedVar,
}

/// resolve 진단 1건. span·스니펫을 담아 driver가 rustc 스타일로 렌더링한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveError {
    pub kind: ResolveKind,
    pub context: &'static str,
    pub name: String,
    /// 에러를 가리키는 지점 (중복이면 두 번째 정의, 미정의면 참조 지점).
    pub span: Span,
    /// `Duplicate`일 때 먼저 정의된 곳. `UndefinedVar`면 None.
    pub first_span: Option<Span>,
    /// 두 span의 원문. src가 없으면 `<invalid span>`.
    pub snippet: String,
    pub first_snippet: String,
    pub source: Option<SpanError>,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ResolveKind::Duplicate => {
                let first = self
                    .first_span
                    .map(|s| format!("{:?} at [{}..{}]", self.first_snippet, s.lo, s.hi))
                    .unwrap_or_else(|| "<none>".into());
                write!(
                    f,
                    "{}: duplicate definition of `{}` (first: {}, duplicate: {:?} at [{}..{}])",
                    self.context, self.name, first, self.snippet, self.span.lo, self.span.hi
                )
            }
            ResolveKind::UndefinedVar => write!(
                f,
                "{}: cannot find value `{}` in this function frame ({:?} at [{}..{}])",
                self.context, self.name, self.snippet, self.span.lo, self.span.hi
            ),
        }
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

/// 이름 중복 검사 + 필드 참조 slot 치환 + 지역변수 slot 배정 (모두 제자리 변형).
pub fn resolve(krate: &mut Crate, src: &str) -> Result<(), Vec<ResolveError>> {
    let mut errs = Vec::new();
    if let Err(mut e) = check_duplicate_names(krate, src) {
        errs.append(&mut e);
    }
    resolve_fields(krate);
    if let Err(mut e) = resolve_locals(krate, src) {
        errs.append(&mut e);
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
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

/// 함수별 지역변수 slot 배정 — eval의 이름 조회를 인덱스 접근으로 바꾼다.
/// 선언을 못 찾은 Var는 미정의 변수 에러다 (조용히 None으로 두지 않는다).
fn resolve_locals(krate: &mut Crate, src: &str) -> Result<(), Vec<ResolveError>> {
    let mut errs = Vec::new();
    for item in &mut krate.items {
        let ItemKind::Fn(f) = &mut item.kind else { continue };
        let mut names = Vec::new();
        collect_local_names(&f.body, &mut names);
        f.locals = names.len() as u16;
        patch_block_slots(&mut f.body, &names, src, &mut errs);
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

/// 블록 안 stmt/tail의 지역변수 slot을 채운다.
fn patch_block_slots(block: &mut Block, names: &[String], src: &str, errs: &mut Vec<ResolveError>) {
    for stmt in &mut block.stmts {
        match &mut stmt.kind {
            StmtKind::Let(l) => {
                l.slot = slot_of_name(names, &l.name.name);
                if let Some(init) = &mut l.init {
                    patch_expr_slots(init, names, src, errs);
                }
            }
            StmtKind::Expr(e) => patch_expr_slots(e, names, src, errs),
        }
    }
    if let Some(tail) = &mut block.tail {
        patch_expr_slots(tail, names, src, errs);
    }
}

/// 식 트리를 훑어 Var 참조에 지역변수 slot을 새긴다 (선언이 없으면 에러).
fn patch_expr_slots(expr: &mut Expr, names: &[String], src: &str, errs: &mut Vec<ResolveError>) {
    match &mut expr.kind {
        ExprKind::Var { name, slot } => match slot_of_name(names, &name.name) {
            Some(i) => *slot = Some(i),
            None => errs.push(undefined_var(name, src)),
        },
        ExprKind::If { cond, then_block, else_block } => {
            patch_expr_slots(cond, names, src, errs);
            patch_block_slots(then_block, names, src, errs);
            if let Some(block) = else_block {
                patch_block_slots(block, names, src, errs);
            }
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            patch_expr_slots(lhs, names, src, errs);
            patch_expr_slots(rhs, names, src, errs);
        }
        ExprKind::Call { args, .. } | ExprKind::Macro { args, .. } => {
            for arg in args {
                patch_expr_slots(arg, names, src, errs);
            }
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                patch_expr_slots(&mut field.value, names, src, errs);
            }
        }
        ExprKind::FieldAccess { base, .. } => patch_expr_slots(base, names, src, errs),
        ExprKind::Int(_) => {}
    }
}

/// 이 함수 프레임에 선언이 없는 변수 참조.
fn undefined_var(name: &Ident, src: &str) -> ResolveError {
    ResolveError {
        kind: ResolveKind::UndefinedVar,
        context: "resolve",
        name: name.name.clone(),
        span: name.span,
        first_span: None,
        snippet: snippet_of(&name.span, src),
        first_snippet: String::new(),
        source: None,
    }
}

/// 지역변수 이름 → slot 번호 (선언 순서 = index).
fn slot_of_name(names: &[String], name: &str) -> Option<u16> {
    Some(names.iter().position(|n| n == name)? as u16)
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
        ExprKind::Int(_) | ExprKind::Var { .. } => {}
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
        ExprKind::Var { name, .. } => types.get(&name.name).cloned(),
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
                kind: ResolveKind::Duplicate,
                context: "resolve",
                name: item.name.name.clone(),
                span: item.name.span,
                first_span: Some(*first_span),
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
        assert_eq!(errs[0].kind, ResolveKind::Duplicate);
        assert_eq!(errs[0].name, "main");
        assert_eq!(errs[0].span.snippet(src), "main");
        assert_eq!(errs[0].first_span.unwrap().snippet(src), "main");
        assert!(errs[0].span.lo > errs[0].first_span.unwrap().lo);
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
        assert_eq!(errs[0].first_span.unwrap().snippet(src), "foo");
        assert_eq!(errs[0].span.snippet(src), "foo");
        assert!(errs[0].span.lo > errs[0].first_span.unwrap().lo);
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

    /// main의 let stmt slot 나열 (Expr stmt는 None).
    fn let_slots(f: &rtoy_ast::FnItem) -> Vec<Option<u16>> {
        f.body
            .stmts
            .iter()
            .map(|s| match &s.kind {
                StmtKind::Let(l) => l.slot,
                StmtKind::Expr(_) => None,
            })
            .collect()
    }

    fn var_slot(expr: &Expr) -> Option<u16> {
        match &expr.kind {
            ExprKind::Var { slot, .. } => *slot,
            other => panic!("Var가 아님: {other:?}"),
        }
    }

    #[test]
    fn locals_get_slots_in_declaration_order() {
        let src = "fn main() { let x = 1; let y = 2; x + y }";
        let mut krate = try_lower(&tokenize(src), src).unwrap();
        assert!(resolve(&mut krate, src).is_ok());
        let ItemKind::Fn(f) = &krate.items[0].kind else { panic!("fn이 아님") };
        assert_eq!(let_slots(f), vec![Some(0), Some(1)]);
        assert_eq!(f.locals, 2, "프레임 크기 = 지역변수 slot 수");
        let Some(tail) = &f.body.tail else { panic!("tail 없음") };
        let ExprKind::Binary { lhs, rhs, .. } = &tail.kind else { panic!("이항식이 아님") };
        assert_eq!((var_slot(lhs), var_slot(rhs)), (Some(0), Some(1)));
    }

    #[test]
    fn shadowed_local_reuses_slot() {
        // 같은 이름 재선언은 같은 slot을 덮어쓴다 (선형 스캔의 overwrite와 같은 의미).
        let src = "fn main() { let x = 1; let x = 2; x }";
        let mut krate = try_lower(&tokenize(src), src).unwrap();
        assert!(resolve(&mut krate, src).is_ok());
        let ItemKind::Fn(f) = &krate.items[0].kind else { panic!("fn이 아님") };
        assert_eq!(let_slots(f), vec![Some(0), Some(0)]);
        assert_eq!(f.body.tail.as_ref().map(var_slot), Some(Some(0)));
    }

    #[test]
    fn branch_local_joins_same_frame() {
        // 블록 스코프가 없어 분기 안 let도 같은 프레임 slot을 쓴다.
        // (조건 끝이 식별자면 `x {`가 struct literal로 오파싱되므로 리터럴 조건을 쓴다)
        let src = "fn main() { let x = 1; if 0 < 1 { let y = 2; y } else { x } }";
        let mut krate = try_lower(&tokenize(src), src).unwrap();
        assert!(resolve(&mut krate, src).is_ok());
        let ItemKind::Fn(f) = &krate.items[0].kind else { panic!("fn이 아님") };
        assert_eq!(let_slots(f), vec![Some(0)]);
        assert_eq!(f.locals, 2, "분기 안 let도 같은 프레임에 들어간다");
        let Some(tail) = &f.body.tail else { panic!("tail 없음") };
        let ExprKind::If { then_block, else_block, .. } = &tail.kind else { panic!("if가 아님") };
        let StmtKind::Let(y) = &then_block.stmts[0].kind else { panic!("let이 아님") };
        assert_eq!(y.slot, Some(1));
        assert_eq!(then_block.tail.as_ref().map(var_slot), Some(Some(1)));
        let else_block = else_block.as_ref().expect("else 없음");
        assert_eq!(else_block.tail.as_ref().map(var_slot), Some(Some(0)));
    }

    #[test]
    fn undefined_var_is_an_error() {
        // 선언을 못 찾은 Var는 resolve 단계에서 에러다 (eval까지 가지 않는다).
        let src = "fn main() { let x = 1; y }";
        let mut krate = try_lower(&tokenize(src), src).unwrap();
        let errs = resolve(&mut krate, src).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].kind, ResolveKind::UndefinedVar);
        assert_eq!(errs[0].name, "y");
        assert_eq!(errs[0].span.snippet(src), "y");
        assert_eq!(errs[0].first_span, None);
        assert!(errs[0].to_string().contains("cannot find value `y`"), "{}", errs[0]);
    }
}

# report_conflict를 쌓아 올리는 튜토리얼

목표: 첫 단계부터 마지막과 같은 얼굴로 시작해 뒤에만 덧붙인다.
방식: 서명은 1단계부터 원본과 같고 몸통만 자란다.
1단계 코드가 11단계 코드 안에 그대로 있다.

## 1단계: 순서잡기

```rust
pub(crate) fn report_conflict(
    &mut self,
    ident: IdentKey,
    ns: Namespace,
    old_binding: Decl<'ra>,
    new_binding: Decl<'ra>,
) {
    if old_binding.span.lo() > new_binding.span.lo() {
        return self.report_conflict(ident, ns, new_binding, old_binding);
    }
    // 뒤 단계에서 채움
    let _ = (ident, ns);
}
```

늦게 온 쪽을 항상 new로 둔다.
이 첫 if는 마지막까지 첫 줄로 남는다.

## 2단계: 그릇 설명 얹기

1단계 몸통 뒤에 그대로 덧붙인다.

```rust
pub(crate) fn report_conflict(
    &mut self,
    ident: IdentKey,
    ns: Namespace,
    old_binding: Decl<'ra>,
    new_binding: Decl<'ra>,
) {
    if old_binding.span.lo() > new_binding.span.lo() {
        return self.report_conflict(ident, ns, new_binding, old_binding);
    }

    let container = match old_binding.parent_module.unwrap().expect_local().kind {
        ModuleKind::Def(kind, def_id, _, _) => kind.descr(def_id),
        ModuleKind::Block => \"block\",
    };
    let _ = (ident, ns, container);
}
```

직접 조회 대신 매칭으로 구해 질의 순환을 피한다.

## 3단계: 이름과 밑줄과 본 기록 얹기

```rust
pub(crate) fn report_conflict(
    &mut self,
    ident: IdentKey,
    ns: Namespace,
    old_binding: Decl<'ra>,
    new_binding: Decl<'ra>,
) {
    if old_binding.span.lo() > new_binding.span.lo() {
        return self.report_conflict(ident, ns, new_binding, old_binding);
    }

    let container = match old_binding.parent_module.unwrap().expect_local().kind {
        ModuleKind::Def(kind, def_id, _, _) => kind.descr(def_id),
        ModuleKind::Block => \"block\",
    };

    let (name, span) =
        (ident.name, self.tcx.sess.source_map().guess_head_span(new_binding.span));

    if self.name_already_seen.get(&name) == Some(&span) {
        return;
    }
    let _ = (ns, container);
}
```

밑줄은 이름 토큰만 자르고 본 것은 바로 돌려보낸다.

## 4단계: 주인 문구 얹기

```rust
pub(crate) fn report_conflict(
    &mut self,
    ident: IdentKey,
    ns: Namespace,
    old_binding: Decl<'ra>,
    new_binding: Decl<'ra>,
) {
    if old_binding.span.lo() > new_binding.span.lo() {
        return self.report_conflict(ident, ns, new_binding, old_binding);
    }

    let container = match old_binding.parent_module.unwrap().expect_local().kind {
        ModuleKind::Def(kind, def_id, _, _) => kind.descr(def_id),
        ModuleKind::Block => \"block\",
    };

    let (name, span) =
        (ident.name, self.tcx.sess.source_map().guess_head_span(new_binding.span));

    if self.name_already_seen.get(&name) == Some(&span) {
        return;
    }

    let old_kind = match (ns, old_binding.res()) {
        (ValueNS, _) => \"value\",
        (MacroNS, _) => \"macro\",
        (TypeNS, _) if old_binding.is_extern_crate() => \"extern crate\",
        (TypeNS, Res::Def(DefKind::Mod, _)) => \"module\",
        (TypeNS, Res::Def(DefKind::Trait, _)) => \"trait\",
        (TypeNS, _) => \"type\",
    };
    let _ = (ns, container, name, span, old_kind);
}
```

## 5단계: 번호 고르기 얹기

```rust
pub(crate) fn report_conflict(
    &mut self,
    ident: IdentKey,
    ns: Namespace,
    old_binding: Decl<'ra>,
    new_binding: Decl<'ra>,
) {
    if old_binding.span.lo() > new_binding.span.lo() {
        return self.report_conflict(ident, ns, new_binding, old_binding);
    }

    let container = match old_binding.parent_module.unwrap().expect_local().kind {
        ModuleKind::Def(kind, def_id, _, _) => kind.descr(def_id),
        ModuleKind::Block => \"block\",
    };

    let (name, span) =
        (ident.name, self.tcx.sess.source_map().guess_head_span(new_binding.span));

    if self.name_already_seen.get(&name) == Some(&span) {
        return;
    }

    let old_kind = match (ns, old_binding.res()) {
        (ValueNS, _) => \"value\",
        (MacroNS, _) => \"macro\",
        (TypeNS, _) if old_binding.is_extern_crate() => \"extern crate\",
        (TypeNS, Res::Def(DefKind::Mod, _)) => \"module\",
        (TypeNS, Res::Def(DefKind::Trait, _)) => \"trait\",
        (TypeNS, _) => \"type\",
    };

    let code = match (old_binding.is_extern_crate(), new_binding.is_extern_crate()) {
        (true, true) => E0259,
        (true, _) | (_, true) => match new_binding.is_import() && old_binding.is_import() {
            true => E0254,
            false => E0260,
        },
        _ => match (old_binding.is_import_user_facing(), new_binding.is_import_user_facing()) {
            (false, false) => E0428,
            (true, true) => E0252,
            _ => E0255,
        },
    };
    let _ = (container, name, span, old_kind, code);
}
```

## 6단계: 새 문장과 옛 문장 얹기

```rust
pub(crate) fn report_conflict(
    &mut self,
    ident: IdentKey,
    ns: Namespace,
    old_binding: Decl<'ra>,
    new_binding: Decl<'ra>,
) {
    if old_binding.span.lo() > new_binding.span.lo() {
        return self.report_conflict(ident, ns, new_binding, old_binding);
    }

    let container = match old_binding.parent_module.unwrap().expect_local().kind {
        ModuleKind::Def(kind, def_id, _, _) => kind.descr(def_id),
        ModuleKind::Block => \"block\",
    };

    let (name, span) =
        (ident.name, self.tcx.sess.source_map().guess_head_span(new_binding.span));

    if self.name_already_seen.get(&name) == Some(&span) {
        return;
    }

    let old_kind = match (ns, old_binding.res()) {
        (ValueNS, _) => \"value\",
        (MacroNS, _) => \"macro\",
        (TypeNS, _) if old_binding.is_extern_crate() => \"extern crate\",
        (TypeNS, Res::Def(DefKind::Mod, _)) => \"module\",
        (TypeNS, Res::Def(DefKind::Trait, _)) => \"trait\",
        (TypeNS, _) => \"type\",
    };

    let code = match (old_binding.is_extern_crate(), new_binding.is_extern_crate()) {
        (true, true) => E0259,
        (true, _) | (_, true) => match new_binding.is_import() && old_binding.is_import() {
            true => E0254,
            false => E0260,
        },
        _ => match (old_binding.is_import_user_facing(), new_binding.is_import_user_facing()) {
            (false, false) => E0428,
            (true, true) => E0252,
            _ => E0255,
        },
    };

    let label = match new_binding.is_import_user_facing() {
        true => diagnostics::NameDefinedMultipleTimeLabel::Reimported { span, name },
        false => diagnostics::NameDefinedMultipleTimeLabel::Redefined { span, name },
    };

    let old_binding_label =
        (!old_binding.span.is_dummy() && old_binding.span != span).then(|| {
            let span = self.tcx.sess.source_map().guess_head_span(old_binding.span);
            match old_binding.is_import_user_facing() {
                true => diagnostics::NameDefinedMultipleTimeOldBindingLabel::Import {
                    span,
                    old_kind,
                    name,
                },
                false => diagnostics::NameDefinedMultipleTimeOldBindingLabel::Definition {
                    span,
                    old_kind,
                    name,
                },
            }
        });
    let _ = (container, code, label, old_binding_label);
}
```

## 7단계: 묶어 쏠 준비 얹기

6단계 뒤에 err 만들기를 덧붙인다.

```rust
pub(crate) fn report_conflict(
    &mut self,
    ident: IdentKey,
    ns: Namespace,
    old_binding: Decl<'ra>,
    new_binding: Decl<'ra>,
) {
    if old_binding.span.lo() > new_binding.span.lo() {
        return self.report_conflict(ident, ns, new_binding, old_binding);
    }

    let container = match old_binding.parent_module.unwrap().expect_local().kind {
        ModuleKind::Def(kind, def_id, _, _) => kind.descr(def_id),
        ModuleKind::Block => \"block\",
    };

    let (name, span) =
        (ident.name, self.tcx.sess.source_map().guess_head_span(new_binding.span));

    if self.name_already_seen.get(&name) == Some(&span) {
        return;
    }

    let old_kind = match (ns, old_binding.res()) {
        (ValueNS, _) => \"value\",
        (MacroNS, _) => \"macro\",
        (TypeNS, _) if old_binding.is_extern_crate() => \"extern crate\",
        (TypeNS, Res::Def(DefKind::Mod, _)) => \"module\",
        (TypeNS, Res::Def(DefKind::Trait, _)) => \"trait\",
        (TypeNS, _) => \"type\",
    };

    let code = match (old_binding.is_extern_crate(), new_binding.is_extern_crate()) {
        (true, true) => E0259,
        (true, _) | (_, true) => match new_binding.is_import() && old_binding.is_import() {
            true => E0254,
            false => E0260,
        },
        _ => match (old_binding.is_import_user_facing(), new_binding.is_import_user_facing()) {
            (false, false) => E0428,
            (true, true) => E0252,
            _ => E0255,
        },
    };

    let label = match new_binding.is_import_user_facing() {
        true => diagnostics::NameDefinedMultipleTimeLabel::Reimported { span, name },
        false => diagnostics::NameDefinedMultipleTimeLabel::Redefined { span, name },
    };

    let old_binding_label =
        (!old_binding.span.is_dummy() && old_binding.span != span).then(|| {
            let span = self.tcx.sess.source_map().guess_head_span(old_binding.span);
            match old_binding.is_import_user_facing() {
                true => diagnostics::NameDefinedMultipleTimeOldBindingLabel::Import {
                    span,
                    old_kind,
                    name,
                },
                false => diagnostics::NameDefinedMultipleTimeOldBindingLabel::Definition {
                    span,
                    old_kind,
                    name,
                },
            }
        });

    let mut err = self
        .dcx()
        .create_err(diagnostics::NameDefinedMultipleTime {
            span,
            name,
            descr: ns.descr(),
            container,
            label,
            old_binding_label,
        })
        .with_code(code);
    let _ = &mut err;
}
```

## 8단계: 건드릴 표 고르기 얹기

```rust
pub(crate) fn report_conflict(
    &mut self,
    ident: IdentKey,
    ns: Namespace,
    old_binding: Decl<'ra>,
    new_binding: Decl<'ra>,
) {
    if old_binding.span.lo() > new_binding.span.lo() {
        return self.report_conflict(ident, ns, new_binding, old_binding);
    }

    let container = match old_binding.parent_module.unwrap().expect_local().kind {
        ModuleKind::Def(kind, def_id, _, _) => kind.descr(def_id),
        ModuleKind::Block => \"block\",
    };

    let (name, span) =
        (ident.name, self.tcx.sess.source_map().guess_head_span(new_binding.span));

    if self.name_already_seen.get(&name) == Some(&span) {
        return;
    }

    let old_kind = match (ns, old_binding.res()) {
        (ValueNS, _) => \"value\",
        (MacroNS, _) => \"macro\",
        (TypeNS, _) if old_binding.is_extern_crate() => \"extern crate\",
        (TypeNS, Res::Def(DefKind::Mod, _)) => \"module\",
        (TypeNS, Res::Def(DefKind::Trait, _)) => \"trait\",
        (TypeNS, _) => \"type\",
    };

    let code = match (old_binding.is_extern_crate(), new_binding.is_extern_crate()) {
        (true, true) => E0259,
        (true, _) | (_, true) => match new_binding.is_import() && old_binding.is_import() {
            true => E0254,
            false => E0260,
        },
        _ => match (old_binding.is_import_user_facing(), new_binding.is_import_user_facing()) {
            (false, false) => E0428,
            (true, true) => E0252,
            _ => E0255,
        },
    };

    let label = match new_binding.is_import_user_facing() {
        true => diagnostics::NameDefinedMultipleTimeLabel::Reimported { span, name },
        false => diagnostics::NameDefinedMultipleTimeLabel::Redefined { span, name },
    };

    let old_binding_label =
        (!old_binding.span.is_dummy() && old_binding.span != span).then(|| {
            let span = self.tcx.sess.source_map().guess_head_span(old_binding.span);
            match old_binding.is_import_user_facing() {
                true => diagnostics::NameDefinedMultipleTimeOldBindingLabel::Import {
                    span,
                    old_kind,
                    name,
                },
                false => diagnostics::NameDefinedMultipleTimeOldBindingLabel::Definition {
                    span,
                    old_kind,
                    name,
                },
            }
        });

    let mut err = self
        .dcx()
        .create_err(diagnostics::NameDefinedMultipleTime {
            span,
            name,
            descr: ns.descr(),
            container,
            label,
            old_binding_label,
        })
        .with_code(code);

    use DeclKind::Import;
    let can_suggest = |binding: Decl<'_>, import: self::Import<'_>| {
        !binding.span.is_dummy()
            && !matches!(import.kind, ImportKind::MacroUse { .. } | ImportKind::MacroExport)
    };
    let import = match (&new_binding.kind, &old_binding.kind) {
        (Import { import: new, .. }, Import { import: old, .. })
            if {
                (new.has_attributes || old.has_attributes)
                    && can_suggest(old_binding, old)
                    && can_suggest(new_binding, new)
            } =>
        {
            if old.has_attributes {
                Some((new, new_binding.span, true))
            } else {
                Some((old, old_binding.span, true))
            }
        }
        (Import { import, .. }, other) if can_suggest(new_binding, import) => {
            Some((import, new_binding.span, other.is_import()))
        }
        (other, Import { import, .. }) if can_suggest(old_binding, import) => {
            Some((import, old_binding.span, other.is_import()))
        }
        _ => None,
    };
    let _ = (&mut err, import);
}
```

## 9단계: 진짜 중복 가리기 얹기

```rust
pub(crate) fn report_conflict(
    &mut self,
    ident: IdentKey,
    ns: Namespace,
    old_binding: Decl<'ra>,
    new_binding: Decl<'ra>,
) {
    if old_binding.span.lo() > new_binding.span.lo() {
        return self.report_conflict(ident, ns, new_binding, old_binding);
    }

    let container = match old_binding.parent_module.unwrap().expect_local().kind {
        ModuleKind::Def(kind, def_id, _, _) => kind.descr(def_id),
        ModuleKind::Block => \"block\",
    };

    let (name, span) =
        (ident.name, self.tcx.sess.source_map().guess_head_span(new_binding.span));

    if self.name_already_seen.get(&name) == Some(&span) {
        return;
    }

    let old_kind = match (ns, old_binding.res()) {
        (ValueNS, _) => \"value\",
        (MacroNS, _) => \"macro\",
        (TypeNS, _) if old_binding.is_extern_crate() => \"extern crate\",
        (TypeNS, Res::Def(DefKind::Mod, _)) => \"module\",
        (TypeNS, Res::Def(DefKind::Trait, _)) => \"trait\",
        (TypeNS, _) => \"type\",
    };

    let code = match (old_binding.is_extern_crate(), new_binding.is_extern_crate()) {
        (true, true) => E0259,
        (true, _) | (_, true) => match new_binding.is_import() && old_binding.is_import() {
            true => E0254,
            false => E0260,
        },
        _ => match (old_binding.is_import_user_facing(), new_binding.is_import_user_facing()) {
            (false, false) => E0428,
            (true, true) => E0252,
            _ => E0255,
        },
    };

    let label = match new_binding.is_import_user_facing() {
        true => diagnostics::NameDefinedMultipleTimeLabel::Reimported { span, name },
        false => diagnostics::NameDefinedMultipleTimeLabel::Redefined { span, name },
    };

    let old_binding_label =
        (!old_binding.span.is_dummy() && old_binding.span != span).then(|| {
            let span = self.tcx.sess.source_map().guess_head_span(old_binding.span);
            match old_binding.is_import_user_facing() {
                true => diagnostics::NameDefinedMultipleTimeOldBindingLabel::Import {
                    span,
                    old_kind,
                    name,
                },
                false => diagnostics::NameDefinedMultipleTimeOldBindingLabel::Definition {
                    span,
                    old_kind,
                    name,
                },
            }
        });

    let mut err = self
        .dcx()
        .create_err(diagnostics::NameDefinedMultipleTime {
            span,
            name,
            descr: ns.descr(),
            container,
            label,
            old_binding_label,
        })
        .with_code(code);

    use DeclKind::Import;
    let can_suggest = |binding: Decl<'_>, import: self::Import<'_>| {
        !binding.span.is_dummy()
            && !matches!(import.kind, ImportKind::MacroUse { .. } | ImportKind::MacroExport)
    };
    let import = match (&new_binding.kind, &old_binding.kind) {
        (Import { import: new, .. }, Import { import: old, .. })
            if {
                (new.has_attributes || old.has_attributes)
                    && can_suggest(old_binding, old)
                    && can_suggest(new_binding, new)
            } =>
        {
            if old.has_attributes {
                Some((new, new_binding.span, true))
            } else {
                Some((old, old_binding.span, true))
            }
        }
        (Import { import, .. }, other) if can_suggest(new_binding, import) => {
            Some((import, new_binding.span, other.is_import()))
        }
        (other, Import { import, .. }) if can_suggest(old_binding, import) => {
            Some((import, old_binding.span, other.is_import()))
        }
        _ => None,
    };

    let duplicate = new_binding.res().opt_def_id() == old_binding.res().opt_def_id();
    let has_dummy_span = new_binding.span.is_dummy() || old_binding.span.is_dummy();
    let from_item =
        self.extern_prelude.get(&ident).is_none_or(|entry| entry.introduced_by_item());
    let should_remove_import = duplicate
        && !has_dummy_span
        && ((new_binding.is_extern_crate() || old_binding.is_extern_crate()) || from_item);
    let _ = (&mut err, import, should_remove_import);
}
```

## 10단계: 원본과 같은 최종형

9단계 뒤에 권유와 쏘기를 덧붙이면 원본이다.

```rust
pub(crate) fn report_conflict(
    &mut self,
    ident: IdentKey,
    ns: Namespace,
    old_binding: Decl<'ra>,
    new_binding: Decl<'ra>,
) {
    if old_binding.span.lo() > new_binding.span.lo() {
        return self.report_conflict(ident, ns, new_binding, old_binding);
    }

    let container = match old_binding.parent_module.unwrap().expect_local().kind {
        ModuleKind::Def(kind, def_id, _, _) => kind.descr(def_id),
        ModuleKind::Block => \"block\",
    };

    let (name, span) =
        (ident.name, self.tcx.sess.source_map().guess_head_span(new_binding.span));

    if self.name_already_seen.get(&name) == Some(&span) {
        return;
    }

    let old_kind = match (ns, old_binding.res()) {
        (ValueNS, _) => \"value\",
        (MacroNS, _) => \"macro\",
        (TypeNS, _) if old_binding.is_extern_crate() => \"extern crate\",
        (TypeNS, Res::Def(DefKind::Mod, _)) => \"module\",
        (TypeNS, Res::Def(DefKind::Trait, _)) => \"trait\",
        (TypeNS, _) => \"type\",
    };

    let code = match (old_binding.is_extern_crate(), new_binding.is_extern_crate()) {
        (true, true) => E0259,
        (true, _) | (_, true) => match new_binding.is_import() && old_binding.is_import() {
            true => E0254,
            false => E0260,
        },
        _ => match (old_binding.is_import_user_facing(), new_binding.is_import_user_facing()) {
            (false, false) => E0428,
            (true, true) => E0252,
            _ => E0255,
        },
    };

    let label = match new_binding.is_import_user_facing() {
        true => diagnostics::NameDefinedMultipleTimeLabel::Reimported { span, name },
        false => diagnostics::NameDefinedMultipleTimeLabel::Redefined { span, name },
    };

    let old_binding_label =
        (!old_binding.span.is_dummy() && old_binding.span != span).then(|| {
            let span = self.tcx.sess.source_map().guess_head_span(old_binding.span);
            match old_binding.is_import_user_facing() {
                true => diagnostics::NameDefinedMultipleTimeOldBindingLabel::Import {
                    span,
                    old_kind,
                    name,
                },
                false => diagnostics::NameDefinedMultipleTimeOldBindingLabel::Definition {
                    span,
                    old_kind,
                    name,
                },
            }
        });

    let mut err = self
        .dcx()
        .create_err(diagnostics::NameDefinedMultipleTime {
            span,
            name,
            descr: ns.descr(),
            container,
            label,
            old_binding_label,
        })
        .with_code(code);

    use DeclKind::Import;
    let can_suggest = |binding: Decl<'_>, import: self::Import<'_>| {
        !binding.span.is_dummy()
            && !matches!(import.kind, ImportKind::MacroUse { .. } | ImportKind::MacroExport)
    };
    let import = match (&new_binding.kind, &old_binding.kind) {
        (Import { import: new, .. }, Import { import: old, .. })
            if {
                (new.has_attributes || old.has_attributes)
                    && can_suggest(old_binding, old)
                    && can_suggest(new_binding, new)
            } =>
        {
            if old.has_attributes {
                Some((new, new_binding.span, true))
            } else {
                Some((old, old_binding.span, true))
            }
        }
        (Import { import, .. }, other) if can_suggest(new_binding, import) => {
            Some((import, new_binding.span, other.is_import()))
        }
        (other, Import { import, .. }) if can_suggest(old_binding, import) => {
            Some((import, old_binding.span, other.is_import()))
        }
        _ => None,
    };

    let duplicate = new_binding.res().opt_def_id() == old_binding.res().opt_def_id();
    let has_dummy_span = new_binding.span.is_dummy() || old_binding.span.is_dummy();
    let from_item =
        self.extern_prelude.get(&ident).is_none_or(|entry| entry.introduced_by_item());
    let should_remove_import = duplicate
        && !has_dummy_span
        && ((new_binding.is_extern_crate() || old_binding.is_extern_crate()) || from_item);

    match import {
        Some((import, span, true)) if should_remove_import && import.is_nested() => {
            self.add_suggestion_for_duplicate_nested_use(&mut err, import, span);
        }
        Some((import, _, true)) if should_remove_import && !import.is_glob() => {
            err.subdiagnostic(diagnostics::ToolOnlyRemoveUnnecessaryImport {
                span: import.use_span_with_attributes,
            });
        }
        Some((import, span, _)) => {
            self.add_suggestion_for_rename_of_use(&mut err, name, import, span);
        }
        _ => {}
    }

    err.emit();
    self.name_already_seen.insert(name, span);
}
```

1단계 코드가 10단계 안에 그대로 있고 뒤에만 자라 원본이 된다.

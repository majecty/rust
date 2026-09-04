# `NodeId`의 수명(Lifetime) 보고서

`NodeId`는 Rust 컴파일러(rustc)가 AST(추상 구문 트리) 노드를 식별하는 데 사용하는 crate-로컬 정수 ID입니다.
이 보고서는 `NodeId`가 **언제 생성되고, 어디서 할당되며, 어디까지 살아남아 결국 무엇으로 대체되는지** — 즉 컴파일 파이프라인 상에서의 수명을 정리합니다.

---

## 1. 개요: `NodeId`란 무엇인가

정의 위치: `compiler/rustc_ast/src/node_id.rs`

```rust
rustc_index::newtype_index! {
    /// Identifies an AST node.
    ///
    /// This identifies top-level definitions, expressions, and everything in between.
    /// This is later turned into [`DefId`] and `HirId` for the HIR.
    pub struct NodeId {
        const CRATE_NODE_ID = 0;
    }
}
```

- `NodeId`는 **AST 노드**를 가리키는 식별자입니다.
- crate 내에서만 유효한 **로컬 ID**입니다. 다른 crate와는 공유되지 않으며, cross-crate 식별은 `DefId`가 담당합니다.
- 파싱 → 매크로 확장 → 이름 해석(name resolution) → HIR 하강(lowering) 단계를 거치며,
  점차 `HirId`(HIR 노드)와 `DefId`(정의)로 변환됩니다.
- `CRATE_NODE_ID = 0` 이 crate의 루트 노드입니다.

---

## 2. 수명의 단계

```
[파싱] ── DUMMY_NODE_ID ─▶ [매크로 확장] ── next_node_id() 할당 ─▶ [이름 해석]
       ── placeholder(NodeId) 로 매크로 호출 표시 ─▶ [HIR 하강] ── lower_node_id()로 변환
       ──▶ HirId / DefId (NodeId 소멸)
```

### 2.1 탄생 — 파싱 단계: `DUMMY_NODE_ID`

`compiler/rustc_ast/src/node_id.rs:36`

```rust
/// When parsing and at the beginning of doing expansions, we initially give all AST nodes
/// this dummy AST [`NodeId`]. Then, during a later phase of expansion, we renumber them
/// to have small, positive IDs.
pub const DUMMY_NODE_ID: NodeId = NodeId::MAX;
```

- 파서가 AST를 만들 때 모든 노드의 `NodeId`는 `DUMMY_NODE_ID`(`NodeId::MAX`)로 초기화됩니다.
- 아직 어떤 유효한 id도 부여되지 않은 상태입니다.
- 매크로 확장 단계에서 "작고 양의 ID"로 재번호화(renumber)됩니다.

### 2.2 할당 — 매크로 확장 단계 (`rustc_expand`)

실제 `NodeId` 번호는 **확장(expansion) 단계**에서 `Resolver`를 통해 할당됩니다.

`compiler/rustc_resolve/src/lib.rs:1909` — 번호 발급기

```rust
fn next_node_id(&mut self) -> NodeId {
    let start = self.next_node_id;
    let next = start.as_u32().checked_add(1).expect("input too large; ran out of NodeIds");
    self.next_node_id = ast::NodeId::from_u32(next);
    start
}
```

- `Resolver`는 `next_node_id` 카운터(`= CRATE_NODE_ID`)를 들고 있으며,
  `next_node_id()` / `next_node_ids(count)`로 순차적으로 id를 발급합니다.
- `rustc_expand`의 `InvocationCollector`가 AST를 순회하면서 노드에 id를 부여합니다.

`compiler/rustc_expand/src/expand.rs:2600` — `visit_id`

```rust
fn visit_id(&mut self, id: &mut NodeId) {
    // We may have already assigned a `NodeId` by calling `assign_id`
    if self.monotonic && *id == ast::DUMMY_NODE_ID {
        *id = self.cx.resolver.next_node_id();
    }
}
```

`compiler/rustc_expand/src/expand.rs:1210` — `assign_id!` 매크로

- `assign_id!`는 AST 노드(항목, 표현식 등)에 `NodeId`를 부여하고,
  그 id를 해당 노드의 **lint node id**로 설정합니다.
- 매크로 호출 노드(`ExprKind::MacCall` 등)에는 호출하지 않습니다 — 매크로는 확장 후 사라지기 때문입니다.

### 2.3 매크로 placeholder — `placeholder_from_expn_id`

매크로 확장 중 아직 결과가 알려지지 않은 호출은 **placeholder 노드**로 남깁니다.

`compiler/rustc_ast/src/node_id.rs:38`

```rust
impl NodeId {
    pub fn placeholder_from_expn_id(expn_id: LocalExpnId) -> Self {
        NodeId::from_u32(expn_id.as_u32())
    }
    pub fn placeholder_to_expn_id(self) -> LocalExpnId {
        LocalExpnId::from_u32(self.as_u32())
    }
}
```

- 매크로 호출의 placeholder `NodeId`는 실제로 `LocalExpnId`를 담고 있습니다.
- 확장이 완료되면 placeholder는 실제 확장된 fragment의 노드들로 교체됩니다
  (`compiler/rustc_expand/src/placeholders.rs`의 `FxHashMap<NodeId, AstFragment>`).
- 즉 `NodeId`는 **확장(expansion) 도중에만** 이렇게 "가짜 의미"로 잠시 쓰입니다.

### 2.4 이름 해석 단계 — `NodeId` ↔ `DefId` 매핑 축적

이름 해석기(resolver)는 각 `NodeId`가 어떤 정의(`LocalDefId`)에 대응하는지 기록합니다.

`compiler/rustc_ast_lowering/src/lib.rs:764`

```rust
debug!("create_def: def_id_to_node_id[{:?}] <-> {:?}", def_id, node_id);
self.node_id_to_def_id.insert(node_id, def_id);
```

- 매핑은 `Resolver::owners`(`NodeMap<PerOwnerResolverData>`)에 per-owner로 보관됩니다.
- 이 정보는 이후 HIR 하강 때 `NodeId` → `LocalDefId` 변환에 사용됩니다.

### 2.5 종말 — HIR 하강(lowering) 단계

`NodeId`의 실제 수명이 끝나는 지점입니다. AST → HIR로 변환될 때
`NodeId`는 `HirId`로 바뀝니다.

`compiler/rustc_ast_lowering/src/lib.rs:948` — `lower_node_id`

```rust
fn lower_node_id(&mut self, ast_node_id: NodeId) -> HirId {
    assert_ne!(ast_node_id, DUMMY_NODE_ID);

    let owner = self.current_hir_id_owner;
    let local_id = self.item_local_id_counter;
    assert_ne!(local_id, hir::ItemLocalId::ZERO);
    self.item_local_id_counter.increment_by(1);
    let hir_id = HirId { owner, local_id };

    if let Some(def_id) = self.opt_local_def_id(ast_node_id) {
        self.children.insert(def_id, hir::MaybeOwner::NonOwner(hir_id));
    }
    ...
    hir_id
}
```

- `lower_node_id()`는 AST의 `NodeId`를 받아 HIR의 `HirId { owner, local_id }`로 변환합니다.
- 정의 노드라면 `node_id_to_def_id`에서 찾은 `DefId`와 연결됩니다.
- HIR이 만들어진 이후부터는 **`HirId`와 `DefId`가 식별자 역할**을 하며, `NodeId`는 더 이상 사용되지 않습니다.

하강 단계에서도 `next_node_id`가 계속 유지되어(`rustc_ast_lowering/src/lib.rs:769`의 `next_node_id()`),
하강 중 생성되는 새로운 노드들(예: desugar된 노드)에 id가 부여됩니다.

---

## 3. `NodeId` 수명의 핵심 특성

| 특성 | 설명 |
|------|------|
| **범위(Scope)** | 단일 crate 컴파일 내부. cross-crate가 아님 (`DefId`가 담당) |
| **탄생** | 파싱 시 `DUMMY_NODE_ID`(`NodeId::MAX`)로 시작 |
| **실제 할당** | 매크로 확장 단계에서 `Resolver::next_node_id()`로 순차 발급 |
| **특수 용도** | 확장 도중 placeholder로 `LocalExpnId`를 임시 보관 |
| **매핑 축적** | `node_id_to_def_id`에 `NodeId` → `LocalDefId` 연결 정보 저장 |
| **종말(변환)** | HIR 하강에서 `lower_node_id()`로 `HirId`로 변환됨 |
| **이후 식별자** | HIR은 `HirId`, 정의는 `DefId`/`LocalDefId` 사용 |

### 3.1 증분 컴파일(incremental)에서의 수명

`compiler/rustc_ast/src/node_id.rs:22`

```rust
impl StableHash for NodeId {
    fn stable_hash<Hcx: StableHashCtxt>(&self, _: &mut Hcx, _: &mut StableHasher) {
        panic!("Node IDs should not appear in incremental state");
    }
}
```

- `NodeId`는 **증분 컴파일 상태(incr cache)에 절대 들어가면 안 됩니다.**
- id 번호는 매 컴파일마다 달라질 수 있으므로, 캐시 키나 해시 대상이 되면 안 됩니다.
- `MainDefinition`, `DocLinkResMap`처럼 `StableHash`를 구현해야 하는 타입에 포함될 때만
  이 빈 impl이 요구되며, 실제로 호출되면 panic합니다.

---

## 4. 요약

1. `NodeId`는 **AST 전용, crate-로컬** 식별자다.
2. 파싱 때는 모두 `DUMMY_NODE_ID`로 태어나고, 확장 단계에서 `Resolver`가 번호를 매긴다.
3. 확장 도중 매크로 호출은 `placeholder_from_expn_id`로 `LocalExpnId`를 잠시 담는다.
4. 이름 해석은 `NodeId` → `DefId` 매핑을 쌓는다.
5. HIR 하강에서 `lower_node_id()`를 거쳐 `HirId`로 변환되고, 정의는 `DefId`로 굳어진다.
6. HIR 이후로는 `NodeId`가 쓰이지 않으며, 증분 컴파일 상태에도 포함될 수 없다.

> 한 줄 요약: **`NodeId`의 수명은 파싱(=DUMMY)에서 시작해 매크로 확장·이름해석을 거쳐 HIR 하강(=HirId/DefId 변환)에서 끝나는, crate 내 AST 수명주기 식별자다.**

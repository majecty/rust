# Lexer — rustc_lexer 대비

## 1. rustc 방식 (2025-09-05 조사)
- `Cursor::advance_token()` pull 방식
- Token은 `{kind, len}`만 보유, 위치는 파서가 추적
- 참고: `compiler/rustc_lexer/src/lib.rs:59`(Token), `:533`(advance_token)

## 2. 우리 방식
- `Vec<Token>` push 방식, char_indices 기반
- 토큰: Ident/Int/Whitespace/Punct
- 테스트 1개 통과 중

## 3. 차이·전환 검토
- 주석/리터럴/raw-ident 확장 시 pull 전환 검토
- 미구현 항목은 아래 목록 참조

## 4. 미구현 목록
- LineComment / BlockComment 토큰화
- 키워드 판별
- 숫자 접미사·float
- raw 문자열 `r#".."#`
- shebang 처리
- DocStyle (`///`)

use proc_macro::TokenStream;

#[proc_macro]
pub fn make_x(_input: TokenStream) -> TokenStream {
    //Transparent: 호출부 ctxt를 쓴다 (기본 동작)
    //ctxt가 잠깐 생겼다가 정규화에서 지워진다
    "let x = 1;".parse().unwrap()
}

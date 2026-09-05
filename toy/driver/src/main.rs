//! rtoy driver — minimal counterpart to `compiler/rustc_driver`.
//! 흐름: main -> run(args) -> read file -> lex -> (stub) parse/lower.

use std::process::ExitCode;

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;

fn print_help() {
    println!("rtoy <file.rs> — minimal rustc_driver toy");
    println!("  passes (stub): lex -> parse -> ast_lower -> hir -> done");
}

fn run(args: &[String]) -> i32 {
    if args.len() != 2 {
        print_help();
        return EXIT_FAILURE;
    }
    let path = &args[1];
    if path == "--help" || path == "-h" {
        print_help();
        return EXIT_SUCCESS;
    }
    match std::fs::read_to_string(path) {
        Ok(src) => {
            let tokens = rtoy_lexer::tokenize(&src);
            let code: Vec<_> = tokens
                .iter()
                .filter(|t| t.kind != rtoy_lexer::TokenKind::Whitespace)
                .map(|t| &src[t.start..t.end])
                .collect();
            println!("ok: {} tokens, first 10: {:?}", code.len(), &code[..code.len().min(10)]);
            println!("ast: {:?}", rtoy_ast::dummy_crate());
            // TODO: parser -> ast_lowering (stub for now)
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("error: cannot read {path}: {e}");
            EXIT_FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    ExitCode::from(run(&args) as u8)
}

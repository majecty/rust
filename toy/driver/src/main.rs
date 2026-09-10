//! rtoy driver — minimal counterpart to `compiler/rustc_driver`.
//! 흐름: main -> pipeline::run(args) -> read file -> lex -> (stub) parse/lower.

mod args;
mod diagnostics;
mod pipeline;
mod trace;

use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    ExitCode::from(pipeline::run(&argv) as u8)
}

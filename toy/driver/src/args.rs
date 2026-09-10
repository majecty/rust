//! CLI 인자 파싱 (`--lex/--ast/--trace/--sample <name> <file>`).

/// `Ok(Some(args))` = 실행, `Ok(None)` = `--help`, `Err(msg)` = 실패.
/// `msg`가 비어 있으면 help만 출력(위치 인자 초과·입력 없음), 아니면 `eprintln` 후 help.
pub fn parse(args: &[String]) -> Result<Option<CliArgs>, String> {
    let mut lex_only = false;
    let mut ast_only = false;
    let mut trace = false;
    let mut sample: Option<String> = None;
    let mut path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--lex" {
            lex_only = true;
        } else if arg == "--ast" {
            ast_only = true;
        } else if arg == "--trace" {
            trace = true;
        } else if arg == "--sample" {
            i += 1;
            match args.get(i) {
                Some(name) => sample = Some(name.clone()),
                None => return Err("--sample 뒤에 이름이 필요함".to_string()),
            }
        } else if arg == "--help" || arg == "-h" {
            return Ok(None);
        } else if path.is_none() {
            path = Some(arg.clone());
        } else {
            return Err(String::new());
        }
        i += 1;
    }
    if lex_only && ast_only {
        return Err("--lex와 --ast는 함께 쓸 수 없음".to_string());
    }
    if sample.is_some() && path.is_some() {
        return Err("--sample과 파일 인자는 함께 쓸 수 없음".to_string());
    }
    if sample.is_none() && path.is_none() {
        return Err(String::new());
    }
    Ok(Some(CliArgs {
        lex_only,
        ast_only,
        trace,
        sample,
        path,
    }))
}

/// 파싱된 CLI 옵션.
pub struct CliArgs {
    pub lex_only: bool,
    pub ast_only: bool,
    pub trace: bool,
    pub sample: Option<String>,
    pub path: Option<String>,
}

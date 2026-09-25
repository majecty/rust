//! CLI 인자 파싱 (`--lex/--ast/--trace/--sample <name> <file>`).

/// `Ok(Some(args))` = 실행, `Ok(None)` = `--help`, `Err(msg)` = 실패.
/// `msg`가 비어 있으면 help만 출력(위치 인자 초과·입력 없음), 아니면 `eprintln` 후 help.
pub fn parse(args: &[String]) -> Result<Option<CliArgs>, String> {
    let mut lex_only = false;
    let mut ast_only = false;
    let mut hir_only = false;
    let mut thir_only = false;
    let mut thir_tree = false;
    let mut mir_only = false;
    let mut mir_eval = false;
    let mut mir_steps: Option<u64> = None;
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
        } else if arg == "--hir" {
            hir_only = true;
        } else if arg == "--thir" {
            thir_only = true;
        } else if arg == "--thir-tree" {
            thir_tree = true;
        } else if arg == "--mir" {
            mir_only = true;
        } else if arg == "--mir-eval" {
            mir_eval = true;
        } else if let Some(raw) = arg.strip_prefix("--mir-steps=") {
            match raw.parse::<u64>() {
                Ok(n) => mir_steps = Some(n),
                Err(e) => return Err(format!("--mir-steps 값이 올바르지 않음: {raw} ({e})")),
            }
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
    let dumps = [lex_only, ast_only, hir_only, thir_only, thir_tree, mir_only, mir_eval];
    if dumps.iter().filter(|x| **x).count() > 1
        || (mir_steps.is_some() && (lex_only || ast_only || hir_only || thir_only || thir_tree || mir_only))
    {
        return Err("--lex/--ast/--hir/--thir/--thir-tree/--mir/--mir-eval/--mir-steps는 함께 쓸 수 없음".to_string());
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
        hir_only,
        thir_only,
        thir_tree,
        mir_only,
        mir_eval,
        mir_steps,
        trace,
        sample,
        path,
    }))
}

/// 파싱된 CLI 옵션.
pub struct CliArgs {
    pub lex_only: bool,
    pub ast_only: bool,
    /// `--hir`/`--thir`/`--thir-tree`: 해석·desugar된 트리(rustc -Zunpretty=hir/thir 쪽).
    pub hir_only: bool,
    pub thir_only: bool,
    /// `--thir-tree`: THIR을 평탄 arena 대신 트리로 전개해 출력 (rustc thir-tree).
    pub thir_tree: bool,
    pub mir_only: bool,
    pub mir_eval: bool,
    /// `--mir-steps=N`: MIR을 N스텝만 실행하고 실행 경로·현재 위치를 출력 (웹 한 줄 실행용).
    pub mir_steps: Option<u64>,
    pub trace: bool,
    pub sample: Option<String>,
    pub path: Option<String>,
}

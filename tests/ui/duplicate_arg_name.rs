use macro_kwargs::MacroKeywordArgs;

// Two fields cannot share an argument name, including via `rename`.
#[derive(MacroKeywordArgs)]
struct DuplicateNames {
    a: u32,
    #[kwarg(rename = "a")]
    b: u32,
    c: u32,
    #[kwarg(rename = "c")]
    d: u32,
}

fn main() {}

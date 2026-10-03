//! Function-like macros used by the `trybuild` tests of `macro-kwargs`.
//!
//! Each macro parses its input and expands to nothing on success,
//! or to the resulting `compile_error!` on failure.
use macro_kwargs::MacroKeywordArgs;
use macro_kwargs::parse::NestedList;
use proc_macro::TokenStream as RawTokenStream;
use syn::parse_macro_input;

#[derive(MacroKeywordArgs)]
#[allow(dead_code)]
struct ExampleArgs {
    first: usize,
    #[kwarg(rename = "foo")]
    second: String,
    #[kwarg(optional)]
    opt: bool,
}

#[derive(MacroKeywordArgs)]
#[allow(dead_code)]
struct NestedArgs {
    one: bool,
    #[kwarg(optional, with_wrapper = "NestedList<u32>")]
    list: Vec<u32>,
    #[kwarg(optional)]
    nested: Option<ExampleArgs>,
}

#[proc_macro]
pub fn nested_args(input: RawTokenStream) -> RawTokenStream {
    let _args = parse_macro_input!(input as NestedArgs);
    RawTokenStream::new()
}

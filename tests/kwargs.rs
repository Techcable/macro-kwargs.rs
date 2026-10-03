use macro_kwargs::MacroKeywordArgs;
use macro_kwargs::parse::NestedList;

#[derive(MacroKeywordArgs, Debug, PartialEq)]
pub struct ExampleArgs {
    first: usize,
    #[kwarg(rename = "foo")]
    second: String,
    #[kwarg(optional)]
    opt: bool,
}

#[test]
fn basic_example() {
    assert_eq!(
        syn::parse_str::<ExampleArgs>(
            r##"first => 1,
        foo => "str"
    "##
        )
        .unwrap(),
        ExampleArgs {
            first: 1,
            second: "str".into(),
            opt: false
        }
    );
    assert_eq!(
        syn::parse_str::<ExampleArgs>(
            r##"first => 17,
        foo => "str2",
        opt => true,
    "##
        )
        .unwrap(),
        ExampleArgs {
            first: 17,
            second: "str2".into(),
            opt: true
        }
    );
}

#[derive(MacroKeywordArgs, Debug, PartialEq, Default)]
struct NestedArgs {
    one: bool,
    #[kwarg(optional, with_wrapper = "NestedList<u32>")]
    list: Vec<u32>,
    #[kwarg(optional)]
    nested: Option<ExampleArgs>,
}

#[test]
fn nesting() {
    assert_eq!(
        syn::parse_str::<NestedArgs>(
            r##"one => false,
    "##
        )
        .unwrap(),
        Default::default()
    );
    assert_eq!(
        syn::parse_str::<NestedArgs>(
            r##"one => true,
        list => [1, 4, 7]
    "##
        )
        .unwrap(),
        NestedArgs {
            one: true,
            list: vec![1, 4, 7],
            nested: None
        }
    );
    assert_eq!(
        syn::parse_str::<NestedArgs>(
            r##"one => true,
        nested => {
            first => 12,
            foo => "nested-str",
        }
    "##
        )
        .unwrap(),
        NestedArgs {
            one: true,
            list: Default::default(),
            nested: Some(ExampleArgs {
                first: 12,
                second: "nested-str".into(),
                opt: false
            })
        }
    );
}

#[derive(MacroKeywordArgs, Debug, PartialEq)]
pub struct SynExprArgs {
    #[kwarg(syn)]
    expr1: syn::Expr,
    #[kwarg(with_syn, optional)]
    keyword: syn::Token![self],
}

#[test]
fn syn_expr() {
    assert_eq!(
        syn::parse_str::<SynExprArgs>(
            r##"
            expr1 => 3 + 7,
            keyword => self,
            "##
        )
        .unwrap(),
        SynExprArgs {
            expr1: syn::parse_quote!(3 + 7),
            keyword: syn::parse_quote!(self),
        }
    );
    assert_eq!(
        syn::parse_str::<SynExprArgs>(
            r##"
            expr1 => 3 + 7,
            "##
        )
        .unwrap(),
        SynExprArgs {
            expr1: syn::parse_quote!(3 + 7),
            // keyword can be missing since it is optional
            keyword: Default::default(),
        }
    );
}

mod custom_parse {
    use syn::parse::ParseStream;

    /// Parses a sequence of identifiers, unlike the default `Vec` handling.
    pub fn idents(stream: ParseStream) -> syn::Result<Vec<String>> {
        let mut res = Vec::new();
        while stream.peek(syn::Ident) {
            res.push(stream.parse::<syn::Ident>()?.to_string());
        }
        Ok(res)
    }
}

/// Parses an integer literal and doubles it,
/// so the test can tell this function was actually used.
fn doubled(stream: syn::parse::ParseStream) -> syn::Result<u32> {
    Ok(stream.parse::<syn::LitInt>()?.base10_parse::<u32>()? * 2)
}

#[derive(MacroKeywordArgs, Debug, PartialEq)]
pub struct WithFuncArgs {
    #[kwarg(with_func = "doubled")]
    num: u32,
    #[kwarg(optional, with_func = "custom_parse::idents")]
    names: Vec<String>,
}

#[test]
fn with_func() {
    assert_eq!(
        syn::parse_str::<WithFuncArgs>("num => 21, names => foo bar baz").unwrap(),
        WithFuncArgs {
            num: 42,
            names: vec!["foo".into(), "bar".into(), "baz".into()],
        }
    );
    assert_eq!(
        syn::parse_str::<WithFuncArgs>("num => 3").unwrap(),
        WithFuncArgs {
            num: 6,
            names: Vec::new(),
        }
    );
    let err = syn::parse_str::<WithFuncArgs>(r#"num => "not a number""#).unwrap_err();
    assert_eq!(err.to_string(), "expected integer literal");
}

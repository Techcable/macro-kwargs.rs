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

/// Generated code must not depend on `syn` or `proc_macro2` being in scope,
/// since users of the derive may not depend on those crates directly.
///
/// The empty modules here shadow those crate names,
/// breaking any generated paths that refer to them.
mod without_syn_in_scope {
    #![allow(dead_code)]
    use macro_kwargs::MacroKeywordArgs;

    mod syn {}
    mod proc_macro2 {}

    #[derive(MacroKeywordArgs, Debug, PartialEq)]
    pub struct ShadowedArgs {
        pub num: u32,
        #[kwarg(optional)]
        pub name: String,
    }
}

#[test]
fn without_syn_in_scope() {
    use without_syn_in_scope::ShadowedArgs;
    assert_eq!(
        syn::parse_str::<ShadowedArgs>(r#"num => 7, name => "x""#).unwrap(),
        ShadowedArgs {
            num: 7,
            name: "x".into(),
        }
    );
}

/// `parse_macro_arg_via_syn!` must work when invoked by its full path,
/// without importing it or having `syn` in scope.
mod via_syn_macro {
    mod syn {}

    pub struct Wrapped(pub ::syn::Ident);
    impl ::syn::parse::Parse for Wrapped {
        fn parse(stream: ::syn::parse::ParseStream) -> ::syn::Result<Self> {
            Ok(Wrapped(stream.parse()?))
        }
    }
    macro_kwargs::parse_macro_arg_via_syn!(Wrapped);
}

#[test]
fn via_syn_macro() {
    let wrapped = macro_kwargs::parse::parse_str::<via_syn_macro::Wrapped>("foo").unwrap();
    assert_eq!(wrapped.0, "foo");
}

/// Field names must not clash with locals in the generated code.
#[derive(MacroKeywordArgs, Debug, PartialEq)]
pub struct ClashingFieldNames {
    argument_list: u32,
    #[kwarg(optional)]
    missing_argument_errors: u32,
    other: u32,
}

#[test]
fn clashing_field_names() {
    assert_eq!(
        syn::parse_str::<ClashingFieldNames>(
            "argument_list => 1, missing_argument_errors => 2, other => 3"
        )
        .unwrap(),
        ClashingFieldNames {
            argument_list: 1,
            missing_argument_errors: 2,
            other: 3,
        }
    );
    let err = syn::parse_str::<ClashingFieldNames>("argument_list => 1").unwrap_err();
    assert_eq!(err.to_string(), "Missing required argument `other`");
}

#[derive(MacroKeywordArgs, Debug, PartialEq)]
pub struct RawFieldNames {
    r#type: syn::Ident,
    #[kwarg(optional)]
    r#match: u32,
}

#[test]
fn raw_field_names() {
    let expected = RawFieldNames {
        r#type: syn::parse_quote!(foo),
        r#match: 3,
    };
    assert_eq!(
        syn::parse_str::<RawFieldNames>("type => foo, match => 3").unwrap(),
        expected
    );
    // the raw form is also accepted
    assert_eq!(
        syn::parse_str::<RawFieldNames>("r#type => foo, r#match => 3").unwrap(),
        expected
    );
    let err = syn::parse_str::<RawFieldNames>("match => 3").unwrap_err();
    assert_eq!(err.to_string(), "Missing required argument `type`");
}

/// `ParsedKeywordArguments::iter` yields arguments in the order the user wrote them,
/// not the order the fields are declared in.
#[test]
fn iter_order_matches_input() {
    use macro_kwargs::args::ParsedKeywordArguments;
    // the reverse of the field declaration order in `ExampleArgs`
    let args = syn::parse_str::<ParsedKeywordArguments<ExampleArgs>>(
        r#"opt => true, foo => "str", first => 1"#,
    )
    .unwrap();
    let names = args
        .iter()
        .map(|arg| arg.name.to_string())
        .collect::<Vec<_>>();
    assert_eq!(names, ["opt", "foo", "first"]);
}

/// Trailing commas are allowed inside `#[kwarg(...)]`.
#[derive(MacroKeywordArgs, Debug, PartialEq)]
#[rustfmt::skip] // rustfmt would remove the trailing commas being tested
pub struct TrailingCommaAttrs {
    #[kwarg(optional,)]
    a: u32,
    #[kwarg(rename = "renamed", optional,)]
    b: u32,
}

#[test]
fn trailing_comma_attrs() {
    assert_eq!(
        syn::parse_str::<TrailingCommaAttrs>("renamed => 2").unwrap(),
        TrailingCommaAttrs { a: 0, b: 2 }
    );
}

/// Distinct field names must produce distinct generated enum variants,
/// even if they would be the same when converted to `UpperCamelCase`.
#[derive(MacroKeywordArgs, Debug, PartialEq)]
#[allow(non_snake_case)]
pub struct SimilarFieldNames {
    foo_bar: u32,
    foo__bar: u32,
    fooBar: u32,
    r#type: u32,
    type_: u32,
}

#[test]
fn similar_field_names() {
    assert_eq!(
        syn::parse_str::<SimilarFieldNames>(
            "foo_bar => 1, foo__bar => 2, fooBar => 3, type => 4, type_ => 5"
        )
        .unwrap(),
        SimilarFieldNames {
            foo_bar: 1,
            foo__bar: 2,
            fooBar: 3,
            r#type: 4,
            type_: 5,
        }
    );
}

use macro_kwargs_test_macros::nested_args;

// The nested `ExampleArgs` is missing its required `foo` argument.
// The error should point at the nested braces, not the whole macro invocation.
nested_args! {
    one => true,
    nested => { first => 12 },
}

fn main() {}

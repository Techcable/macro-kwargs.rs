use macro_kwargs::{MacroArg, MacroKeywordArgs};

// Generic structs are rejected with a clear error, pointing at the generics.
#[derive(MacroKeywordArgs)]
struct TypeParam<T: MacroArg> {
    value: T,
}

#[derive(MacroKeywordArgs)]
struct LifetimeParam<'a> {
    value: u32,
    #[kwarg(optional)]
    marker: std::marker::PhantomData<&'a ()>,
}

fn main() {}

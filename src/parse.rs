//! Utilities for parsing
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use indexmap::map::Entry;
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Token, braced, bracketed, parenthesized};

/// A type that can be parsed as an argument
/// to a macro.
///
/// This is often implemented by delegating to syn's [Parse].
///
/// However, sometimes it can behave differently.
/// For example, nested [`MacroKeywordArgs`](crate::MacroKeywordArgs)
/// require surrounding braces `{}` when parsed as a `MacroArg`,
/// but not when parsed via syn's [`Parse`].
///
/// This gives the effect of requiring braces when nested (as a [`MacroArg`]),
/// but not at the top level (via syn's Parse).
pub trait MacroArg: Sized {
    /// Parse the argument to the macro
    ///
    /// # Errors
    /// Returns an error if the argument fails to parse.
    fn parse_macro_arg(stream: ParseStream) -> syn::Result<Self>;
}

/// Parses an optional [`MacroArg]`,
/// always returning the `Some` variant
///
/// The `None` variant will only be generated
/// if the argument is missing
impl<T: MacroArg> MacroArg for Option<T> {
    fn parse_macro_arg(stream: ParseStream) -> syn::Result<Self> {
        Ok(Some(T::parse_macro_arg(stream)?))
    }
}

/// Internal utility for parsing
macro_rules! macro_arg_parse_map {
    ($target:ty; via $delegate:ty, |$src:ident| $transform:expr) => {
        impl MacroArg for $target {
            fn parse_macro_arg(stream: ParseStream) -> syn::Result<Self> {
                let $src: $delegate = stream.parse()?;
                Ok($transform)
            }
        }
    };
}
fn require_matching_suffix(
    span: Span,
    actual_suffix: &str,
    expected_type: &str,
) -> syn::Result<()> {
    if actual_suffix.is_empty() {
        // with no suffix, only fail if parse does
        return Ok(());
    }
    if expected_type == actual_suffix {
        Ok(())
    } else {
        Err(syn::Error::new(
            span,
            format!("Cannot parse a `{actual_suffix}` as a `{expected_type}`"),
        ))
    }
}
macro_rules! macro_arg_parse_num {
    ($p:path => $($target:ty),*) => {
        $(macro_arg_parse_map!($target; via $p, |i| {
            require_matching_suffix(i.span(), i.suffix(), stringify!($target))?;
            i.base10_parse::<$target>()?
        });)*
    };
}

/// Implements [`MacroArg`] via syn's [`Parse`] trait
#[macro_export]
macro_rules! parse_macro_arg_via_syn {
    ($target:path) => ($crate::parse_macro_arg_via_syn!($target; for <>););
    ($target:path; for <$($lt:lifetime,)* $($param:ident),*> $(where $($where_tks:tt)*)?) => {
        impl<$($lt,)* $($param),*> $crate::parse::MacroArg for $target $(where $($where_tks)* )* {
            fn parse_macro_arg(
                stream: $crate::__private::syn::parse::ParseStream,
            ) -> $crate::__private::syn::Result<Self> {
                stream.parse()
            }
        }
    };
}
macro_arg_parse_num!(syn::LitInt => u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);
macro_arg_parse_num!(syn::LitFloat => f32, f64);
macro_arg_parse_map!(String; via syn::LitStr, |s| s.value());
macro_arg_parse_map!(bool; via syn::LitBool, |s| s.value());
macro_arg_parse_map!(char; via syn::LitChar, |c| c.value());

/// The key in a [`NestedDict`]
///
/// This is supposed to be a trait alias,
/// but those are not yet stable.
pub trait MacroDictKey: MacroArg + Eq + Hash + Spanned {}
impl<T: MacroArg + Eq + Hash + Spanned> MacroDictKey for T {}

/// A pair of values in a [`NestedDict`]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeyValuePair<K: MacroDictKey, V: MacroArg> {
    /// The key
    pub key: K,
    /// The value
    pub value: V,
}
impl<K: MacroDictKey, V: MacroArg> Parse for KeyValuePair<K, V> {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        let key = K::parse_macro_arg(stream)?;
        stream.parse::<Token![=>]>()?;
        let value = V::parse_macro_arg(stream)?;
        Ok(KeyValuePair { key, value })
    }
}
parse_macro_arg_via_syn!(KeyValuePair::<K, V>; for <K, V> where K: MacroDictKey, V: MacroArg);

/// A nested dictionary mapping keys to values with `=>`,
/// and surrounded by braces
///
/// Duplicated keys are considered an error.
///
///
/// ## Example
/// `{ a => b, c => b }` is a `NestedDict<Ident, Ident>`
pub struct NestedDict<K: MacroDictKey, V: MacroArg> {
    /// The brace token
    pub braces: syn::token::Brace,
    /// The underlying map of keys to values
    pub elements: IndexMap<K, V>,
}
impl<K: MacroDictKey, V: MacroArg> Default for NestedDict<K, V> {
    fn default() -> Self {
        NestedDict {
            braces: Default::default(),
            elements: IndexMap::new(),
        }
    }
}
impl<K: MacroDictKey, V: MacroArg> NestedDict<K, V> {
    fn try_extend_pairs(
        &mut self,
        iter: impl Iterator<Item = KeyValuePair<K, V>>,
    ) -> Result<(), syn::Error> {
        // report every duplicate key, not just the first
        let mut errors = Vec::new();
        for pair in iter {
            // point at the duplicate, not the original key
            let key_span = pair.key.span();
            match self.elements.entry(pair.key) {
                Entry::Occupied(_) => {
                    errors.push(syn::Error::new(key_span, "Duplicate keys"));
                }
                Entry::Vacant(entry) => {
                    entry.insert(pair.value);
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(crate::combine_errors(errors))
        }
    }
}
impl<K: MacroDictKey, V: MacroArg> MacroArg for NestedDict<K, V> {
    fn parse_macro_arg(stream: ParseStream) -> syn::Result<Self> {
        let content;
        let braces = braced!(content in stream);
        let pairs = Punctuated::<KeyValuePair<K, V>, Token![,]>::parse_terminated(&content)?;
        let mut res = NestedDict {
            braces,
            elements: IndexMap::default(),
        };
        res.try_extend_pairs(pairs.into_iter())?;
        Ok(res)
    }
}
impl<K: MacroDictKey, V: MacroArg> Deref for NestedDict<K, V> {
    type Target = IndexMap<K, V>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}
impl<K: MacroDictKey, V: MacroArg> DerefMut for NestedDict<K, V> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elements
    }
}
impl<K: MacroDictKey + Debug, V: MacroArg + Debug> Debug for NestedDict<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("NestedDict").field(&self.elements).finish()
    }
}

/// A version of `Option` that is parsed with explicit `Some(<inner>)`
/// or `None` syntax
///
/// This is as opposed to a regular `Option`, which requires
/// no extra syntax and always parses to `Some`
/// (it's intended for use with optional args)
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ExplicitOption<T>(pub Option<T>);
impl<T> Deref for ExplicitOption<T> {
    type Target = Option<T>;
    #[inline]
    fn deref(&self) -> &Option<T> {
        &self.0
    }
}
impl<T> From<Option<T>> for ExplicitOption<T> {
    #[inline]
    fn from(opt: Option<T>) -> Self {
        ExplicitOption(opt)
    }
}
impl<T> From<ExplicitOption<T>> for Option<T> {
    #[inline]
    fn from(explicit: ExplicitOption<T>) -> Option<T> {
        explicit.0
    }
}
impl<T: MacroArg> MacroArg for ExplicitOption<T> {
    fn parse_macro_arg(stream: ParseStream) -> syn::Result<Self> {
        const EXPECTED: &str = "Expected either `Some` or `None`";
        if !stream.peek(syn::Ident) {
            return Err(stream.error(EXPECTED));
        }
        let ident = stream.parse::<Ident>()?;
        if ident == "Some" {
            let content;
            parenthesized!(content in stream);
            Ok(ExplicitOption(Some(T::parse_macro_arg(&content)?)))
        } else if ident == "None" {
            Ok(ExplicitOption(None))
        } else {
            // point at the unexpected identifier, not whatever follows it
            Err(syn::Error::new(ident.span(), EXPECTED))
        }
    }
}

/// A nested list of [Punctuated] items,
/// surrounded by brackets (ex. `[1, 2, 3]`)
///
/// By default, the separator token is the comma `,`
#[derive(PartialEq, Eq, Debug, Hash)]
pub struct NestedList<T: MacroArg, P = Token![,]> {
    /// The brackets tokens
    pub brackets: syn::token::Bracket,
    /// The list of elements
    pub elements: Vec<T>,
    /// `PhantomData`, for the Token
    marker: PhantomData<P>,
}
impl<T: MacroArg, P: Default> From<Vec<T>> for NestedList<T, P> {
    #[inline]
    fn from(v: Vec<T>) -> Self {
        NestedList {
            brackets: Default::default(),
            elements: v,
            marker: PhantomData,
        }
    }
}
impl<T: MacroArg, P> From<NestedList<T, P>> for Vec<T> {
    #[inline]
    fn from(v: NestedList<T, P>) -> Vec<T> {
        v.elements
    }
}
impl<T: MacroArg, P: Default> FromIterator<T> for NestedList<T, P> {
    fn from_iter<A: IntoIterator<Item = T>>(iter: A) -> Self {
        NestedList {
            brackets: Default::default(),
            elements: iter.into_iter().collect(),
            marker: PhantomData,
        }
    }
}
impl<T: MacroArg, P> IntoIterator for NestedList<T, P> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}
impl<'a, T: MacroArg, P> IntoIterator for &'a NestedList<T, P> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}
// NOTE: Implemented manually to avoid bounds on `T`
impl<T: MacroArg, P> Default for NestedList<T, P> {
    fn default() -> Self {
        NestedList {
            brackets: Default::default(),
            elements: Default::default(),
            marker: PhantomData,
        }
    }
}
impl<T: MacroArg, P> Deref for NestedList<T, P> {
    type Target = Vec<T>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}
impl<T: MacroArg, P> DerefMut for NestedList<T, P> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elements
    }
}
impl<T: MacroArg, P: Parse> Parse for NestedList<T, P> {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(NestedList {
            brackets: bracketed!(content in stream),
            elements: Punctuated::<T, P>::parse_terminated_with(&content, T::parse_macro_arg)?
                .into_iter()
                .collect(),
            marker: PhantomData,
        })
    }
}
parse_macro_arg_via_syn!(NestedList::<T, P>; for <T, P> where T: MacroArg, P: Parse);

parse_macro_arg_via_syn!(Ident);
parse_macro_arg_via_syn!(syn::Path);
parse_macro_arg_via_syn!(syn::Type);
parse_macro_arg_via_syn!(syn::GenericParam);
parse_macro_arg_via_syn!(syn::Expr);
parse_macro_arg_via_syn!(syn::Lifetime);
parse_macro_arg_via_syn!(syn::LitStr);
parse_macro_arg_via_syn!(syn::LitInt);
parse_macro_arg_via_syn!(syn::LitFloat);
parse_macro_arg_via_syn!(syn::LitBool);
parse_macro_arg_via_syn!(syn::Lit);
parse_macro_arg_via_syn!(syn::Meta);
// TODO: What is the replacement for this in syn v2?
// parse_macro_arg_via_syn!(syn::NestedMeta);
parse_macro_arg_via_syn!(syn::Visibility);
/// Wrapper type that parses via syn's [`Parse`] trait
///
/// Through this wrapper, any AST node can be parsed
/// as a [`MacroArg`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Syn<T: Parse>(pub T);
impl<T: Parse> Syn<T> {
    /// Convert this type into its inner type
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}
impl<T: Parse> From<T> for Syn<T> {
    #[inline]
    fn from(t: T) -> Syn<T> {
        Syn(t)
    }
}
impl<T: Parse> Parse for Syn<T> {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        Ok(Syn(stream.parse()?))
    }
}
impl<T: Parse + ToTokens> ToTokens for Syn<T> {
    fn to_tokens(&self, stream: &mut TokenStream) {
        self.0.to_tokens(stream);
    }
    fn to_token_stream(&self) -> TokenStream {
        self.0.to_token_stream()
    }
    fn into_token_stream(self) -> TokenStream {
        self.0.into_token_stream()
    }
}
impl<T: Parse> Deref for Syn<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<T: Parse> DerefMut for Syn<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
parse_macro_arg_via_syn!(Syn::<T>; for <T> where T: Parse);

/// Parses a [`MacroArg`] type from a string
///
/// Analogous to [`syn::parse_str`].
///
/// # Errors
/// If the value fails to parse, returns an error.
pub fn parse_str<T: MacroArg>(s: &str) -> syn::Result<T> {
    struct ParseWrapper<T: MacroArg>(T);
    impl<T: MacroArg> Parse for ParseWrapper<T> {
        fn parse(stream: ParseStream) -> syn::Result<Self> {
            Ok(ParseWrapper(T::parse_macro_arg(stream)?))
        }
    }
    syn::parse_str::<ParseWrapper<T>>(s).map(|wrap| wrap.0)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn ints() {
        assert_eq!(parse_str::<i32>("5").unwrap(), 5);
        assert_eq!(parse_str::<i32>("8").unwrap(), 8);
    }
    #[test]
    fn number_suffixes() {
        // matching suffixes are accepted
        assert_eq!(parse_str::<u8>("5u8").unwrap(), 5);
        assert_eq!(parse_str::<i64>("-5i64").unwrap(), -5);
        assert_eq!(parse_str::<usize>("7usize").unwrap(), 7);
        assert_eq!(parse_str::<f32>("1.5f32").unwrap(), 1.5);
        assert_eq!(parse_str::<f64>("2.5f64").unwrap(), 2.5);
        // so are unsuffixed literals
        assert_eq!(parse_str::<u8>("5").unwrap(), 5);
        assert_eq!(parse_str::<f64>("1.5").unwrap(), 1.5);
        // but unsuffixed literals still need to fit in the type
        assert!(parse_str::<u8>("256").is_err());

        // mismatched suffixes are rejected
        let check = |err: syn::Error, expected_msg: &str, expected_column: usize| {
            assert_eq!(err.to_string(), expected_msg);
            // relies on the `span-locations` feature of proc-macro2 (enabled in dev-dependencies)
            assert_eq!(err.span().start().column, expected_column, "{expected_msg}");
        };
        check(
            parse_str::<u8>("5i64").unwrap_err(),
            "Cannot parse a `i64` as a `u8`",
            0,
        );
        check(
            parse_str::<u64>("  5usize").unwrap_err(),
            "Cannot parse a `usize` as a `u64`",
            2,
        );
        check(
            parse_str::<i32>("-5i8").unwrap_err(),
            "Cannot parse a `i8` as a `i32`",
            0,
        );
        check(
            parse_str::<f64>("1.5f32").unwrap_err(),
            "Cannot parse a `f32` as a `f64`",
            0,
        );
        check(
            parse_str::<f32>("2.5f64").unwrap_err(),
            "Cannot parse a `f64` as a `f32`",
            0,
        );
    }
    #[test]
    fn strs() {
        assert_eq!(
            parse_str::<String>(r##""foo""##).unwrap(),
            String::from("foo")
        );
    }
    #[test]
    fn explicit_option() {
        assert_eq!(
            parse_str::<ExplicitOption::<i32>>("None").unwrap(),
            ExplicitOption(None)
        );
        assert_eq!(
            parse_str::<ExplicitOption::<i32>>("Some(5)").unwrap(),
            ExplicitOption(Some(5))
        );
        assert_eq!(
            parse_str::<ExplicitOption::<String>>(r##"Some("foo")"##).unwrap(),
            ExplicitOption(Some(String::from("foo")))
        );
    }
    #[test]
    fn explicit_option_error_span() {
        // relies on the `span-locations` feature of proc-macro2 (enabled in dev-dependencies)
        let check = |input: &str, expected_column: usize| {
            let err = parse_str::<ExplicitOption<i32>>(input).unwrap_err();
            assert_eq!(
                err.to_string(),
                "Expected either `Some` or `None`",
                "{input:?}"
            );
            assert_eq!(err.span().start().column, expected_column, "{input:?}");
        };
        check("Foo", 0);
        check("  Foo(5)", 2);
        check("5", 0);
    }
    #[test]
    fn nested_dict_duplicate_keys() {
        let err = parse_str::<NestedDict<Ident, u32>>("{a => 1, a => 2, b => 3, b => 4, c => 5}")
            .unwrap_err();
        // relies on the `span-locations` feature of proc-macro2 (enabled in dev-dependencies)
        let columns = err
            .into_iter()
            .map(|e| {
                assert_eq!(e.to_string(), "Duplicate keys");
                e.span().start().column
            })
            .collect::<Vec<_>>();
        assert_eq!(columns, [9, 25]);
    }
}

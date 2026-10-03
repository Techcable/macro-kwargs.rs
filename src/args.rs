//! The main interface to parsing kwargs
//!
//! Generally speaking,
//! this should be considered an implementation detail of the macro.
use std::fmt::Debug;
use std::hash::Hash;

use indexmap::IndexMap;
use indexmap::map::Entry;
use proc_macro2::{Ident, Span};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Token, braced};

use crate::MacroArg;

/// The runtime value of a parsed argument
pub trait ParsedArgValue<Id: KeywordArgId>: Sized {
    /// Return the corresponding id
    fn id(&self) -> Id;
    /// Parse the argument with the specified id
    ///
    /// # Errors
    /// Returns an error if the value fails to parse.
    fn parse_with_id(id: Id, id_span: Span, stream: ParseStream) -> syn::Result<Self>;
}
/// The unique id for a single keyword argument
pub trait KeywordArgId: Copy + Eq + Hash + Debug {
    /// The name of this argument, as a string
    fn as_str(&self) -> &'_ str;
    /// Get the argument with the specified name,
    /// returning `None` if it's unknown
    fn from_name(name: &str) -> Option<Self>;
}

/// A parsed argument,
/// along with its original name and id.
pub struct KeywordArg<K: MacroKeywordArgs> {
    /// The name of this argument,
    /// as a `Ident`
    ///
    /// This is needed in addition to `id`,
    /// because it contains a `Span`
    pub name: Ident,
    /// The id of this argument
    pub id: K::ArgId,
    /// The parsed value of this argument
    pub value: K::ParsedArg,
}
impl<K: MacroKeywordArgs> Parse for KeywordArg<K> {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        let name = stream.call(Ident::parse_any)?;
        // accept both `type` and `r#type`
        let name_text = name.unraw().to_string();
        let id = K::ArgId::from_name(&name_text).ok_or_else(|| {
            syn::Error::new(name.span(), format!("Unknown argument name: {name_text}"))
        })?;
        stream.parse::<Token![=>]>()?;
        let value = K::ParsedArg::parse_with_id(id, name.span(), stream)?;
        Ok(KeywordArg { id, name, value })
    }
}
/// The list of parsed keyword arguments
pub struct ParsedKeywordArguments<K: MacroKeywordArgs> {
    /// A map of arguments, in the order the user wrote the arguments.
    by_name: IndexMap<K::ArgId, KeywordArg<K>>,
    /// The span of the whole argument list,
    /// used for errors about missing arguments.
    span: Span,
}
impl<K: MacroKeywordArgs> ParsedKeywordArguments<K> {
    /// The span of the whole argument list
    ///
    /// For nested arguments, this is the span of the surrounding braces.
    /// At the top level, this defaults to [`Span::call_site`].
    #[inline]
    #[must_use]
    pub fn span(&self) -> Span {
        self.span
    }
    /// Lookup a argument by its id,
    /// returning `None` if it's missing
    pub fn get(&self, id: K::ArgId) -> Option<&'_ KeywordArg<K>> {
        self.by_name.get(&id)
    }
    /// Consume the argument with the specified id,
    /// returning `None` if it's missing
    ///
    /// This changes the order of the remaining arguments:
    /// the last argument is moved into the removed argument's place.
    pub fn take(&mut self, id: K::ArgId) -> Option<KeywordArg<K>> {
        self.by_name.swap_remove(&id)
    }
    /// Require that the argument with the specified id exists,
    /// returning an error if it is missing
    ///
    /// Consumes the argument, as if calling `take`
    ///
    /// # Errors
    /// Returns an error if the specified argument is missing.
    pub fn require(&mut self, id: K::ArgId) -> syn::Result<KeywordArg<K>> {
        self.take(id).ok_or_else(|| {
            syn::Error::new(
                self.span,
                format!("Missing required argument `{}`", id.as_str()),
            )
        })
    }
    /// Iterate over the arguments,
    /// in the order the user wrote the arguments.
    ///
    /// This is the order they appear in the macro input,
    /// which may differ from the order the struct's fields are declared in.
    ///
    /// Arguments consumed by [`take`](Self::take) or [`require`](Self::require)
    /// are skipped, and consuming them changes the order of the rest.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &'_ KeywordArg<K>> + '_ {
        self.by_name.values()
    }
}
impl<K: MacroKeywordArgs> Parse for ParsedKeywordArguments<K> {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        let punct: Punctuated<KeywordArg<K>, Token![,]> =
            stream.call(Punctuated::parse_terminated)?;
        let mut by_name = IndexMap::with_capacity(punct.len());
        let mut errors = Vec::new();
        for arg in punct {
            match by_name.entry(arg.id) {
                Entry::Occupied(_entry) => {
                    errors.push(syn::Error::new(
                        arg.name.span(),
                        format!("Duplicate values for argument: {}", arg.name),
                    ));
                }
                Entry::Vacant(entry) => {
                    entry.insert(arg);
                }
            }
        }
        if !errors.is_empty() {
            return Err(crate::combine_errors(errors));
        }
        Ok(ParsedKeywordArguments {
            by_name,
            span: Span::call_site(),
        })
    }
}
impl<K: MacroKeywordArgs> MacroArg for ParsedKeywordArguments<K> {
    fn parse_macro_arg(stream: ParseStream) -> syn::Result<Self> {
        let content;
        let braces = braced!(content in stream);
        let mut res = content.parse::<Self>()?;
        res.span = braces.span.join();
        Ok(res)
    }
}

/// The whole point.
///
/// Defines the interface for parsing keyword args.
///
/// A set of argument can itself be nested as a `MacroArg`,
/// provided it is wrapped in braces `{ }`
pub trait MacroKeywordArgs: MacroArg + Parse {
    /// The id of an argument.
    type ArgId: KeywordArgId;
    /// The runtime value of a parsed argument
    type ParsedArg: ParsedArgValue<Self::ArgId>;
    /// Create the parsed arguments struct from
    /// its list of arguments
    ///
    /// # Errors
    /// Will error if any required arguments are missing,
    /// or if the arguments are invalid.
    fn from_keyword_args(kwargs: ParsedKeywordArguments<Self>) -> Result<Self, syn::Error>;
}

//! Keyword argument parsing for function-like procedural macros (Rust).
//!
//! ## Example
//! ```
//! # macro_rules! example_macro { ($($x:tt)*) => {} };
//! example_macro!(
//!    name => bar,
//!    foo => i32
//! );
//! ```
//!
//! And here is the corresponding code in the proc macro:
//!
//! ```
//! # use macro_kwargs::MacroKeywordArgs;
//! # use proc_macro2::Ident;
//! #[derive(MacroKeywordArgs)]
//! struct MacroArgs {
//!     name: Ident,
//!     #[kwarg(optional)]
//!     optional: Option<syn::Expr>,
//!     #[kwarg(rename = "foo")]
//!     tp: syn::Type
//! }
//! ```
//!
//! See [`tests/kwargs.rs`] for more detailed examples.
//!
//! [`tests/kwargs.rs`]: https://github.com/Techcable/macro-kwargs.rs/blob/master/tests/kwargs.rs
#![deny(missing_docs)]
pub use macro_kwargs_derive::MacroKeywordArgs;

pub mod args;
pub mod parse;

pub use args::MacroKeywordArgs;
pub use parse::MacroArg;

/// Combine multiple `syn` errors into a single error struct
///
/// Panics if the specified vector is empty.
///
/// NOTE: This is an internal implementation detail
#[doc(hidden)]
#[allow(clippy::must_use_candidate)] // could cause warnings in generated code
pub fn combine_errors(errors: Vec<syn::Error>) -> syn::Error {
    let mut iter = errors.into_iter();
    let mut error = iter.next().expect("empty Vec");
    for extra in iter {
        error.combine(extra);
    }
    error
}

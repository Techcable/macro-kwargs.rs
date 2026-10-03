# Changelog

Notable changes to this project should be documented in this file.
Make sure it is up to date before performing a release.

This project follows the [Keep a Changelog](https://keepachangelog.com/en/2.0.0/) format wherever that is reasonable.

The "title" of each release should be its first line.
A title is required for publishing a github release, so all versions should have one.

Most newer changes include the relevant [jj](https://jj-vcs.dev) change ids in parens. An example of a change id is wuoxvnsw.

## Unreleased

### Added
- Implement `Debug` for `NestedDict` (kwsztwtm)

### Changed
- Declare minimum supported rust version of 1.71 (mrvqwsux)
  - This is the oldest version that syn v3 supports.
- Relicense from MIT to [`MIT OR Apache-2.0`](https://spdx.dev/ids) (kxrrqpts)
  - Since the MIT license is still an option, this change actually makes licensing more permissive.
- Require the exact matching version of `macro-kwargs-derive` (ultltztn)
  - Generated code relies on internal APIs, so mismatched versions could fail to compile.

### Fixed
- Fix span of missing argument errors in nested structs (pnrqmzup)
- Avoid redundant_field_names lint in generated code (nvqqkunl)
- Fix `#[kwarg(with_func = "...")]`, which previously failed to compile as the function wasn't properly called (movmzuok)
- Generated code no longer requires a direct dependency on `syn` or `proc-macro2` (vlkusror)
- Fix `parse_macro_arg_via_syn!` when invoked by path or without `syn` in scope (ossnuvpr)
- Point `ExplicitOption` parse errors at the unexpected identifier instead of the token after it (sxzzzyzv)
- Allow fields named `argument_list` or `missing_argument_errors`, which clashed with locals in generated code (vrtqvqnq)
- Reject fields with duplicate argument names (via `rename`), which previously made one of them impossible to set (owonzqoy)
- Fields with raw names like `r#type` now take the argument `type` instead of `r#type`; both spellings are accepted (zunuotso)

## v0.3.3 - 2026-10-03
Rename crate to `macro-kwargs`.

### Changes
- *BIG*: Rename crate from `proc-macro-kwargs` to `macro-kwargs`. (ltmmqrrp)
  - Crate with old name has been marked `#[deprecated(...)]` in v0.3.1 and will receive no further updates.
  - This crate's versions start off where `proc-macro-kwargs` ended.

*NOTE*: The crate has no v0.3.2 version, neither under the old crate name nor the new one.

I originally meant to publish change rmnzyzlo (commit 4b51166f) as v0.3.2,
but I accidentally published it as v0.3.3 on crates.io and messed up the CHANGELOG.
Since crates.io releases are immutable, I stuck with v0.3.3 as the version number for the release.

## v0.3.1 - 2026-10-03
Warn about impending rename to `macro-kwargs`.

### Changes
- *WARNING*: Impending rename from `proc-macro-kwargs` to `macro-kwargs`.

The crate has been marked deprecated in favor of its new name.
The crate under the old name will receive no further updates.

## v0.3.0 - 2026-10-03
Update to syn v3

### Changes
- *BREAKING*: Update to syn v3 (nzzxxmyz)

## v0.2.1 - 2026-10-03
Add a `#[kwarg(with_syn)]` attribute to `#[derive(MacroKeywordArguments)]`.

### Added
- Support a `#[kwarg(with_syn)]` shorthand to parse via [`syn::Parse`] instead of [`MacroArg`] (smwonvvz)
  - More concise than using the [`macro_kwargs::parse::Syn`] wrapper type (which doesn't even support `#[kwarg(with_wrapper)]`)
  - Also available through the `#[kwarg(syn)]` shorthand

[`MacroArg`]: https://docs.rs/proc-macro-kwargs/0.2/proc_macro_kwargs/parse/trait.MacroArg.html
[`syn::Parse`]: https://docs.rs/syn/latest/syn/parse/trait.Parse.html
[`proc_macro_kwargs::parse::Syn`]:  https://docs.rs/proc-macro-kwargs/0.2/proc_macro_kwargs/parse/struct.Syn.html

## v0.2.0 - 2024-04-12
Support stable rust.

Removes use of `#[feature(trait_alias)]`,
allowing program to compile on stable rust in addition to nightly.

Upgrade heck from 0.4 to 0.5

## v0.2.0-alpha.1 - 2024-01-15
Upgrade to syn v2

Requires minor version bump for semver compatibility.

This is an alpha version because it removes `impl MacroArg for NestedMeta`,
At the time, I didn't know what the syn v2 replacement for `NestedMeta` is.

## v0.1.5 - 2026-08-07
Fix "useless question mark" warning in derived code.

Add `#[automatically_derived]` to the generated trait impls.

Implement `From<T>` for [`Syn<T>`].
Cannot add `From<Syn<T>> for T` due to coherence rules.

[`Syn<T>`]: https://docs.rs/proc-macro-kwargs/0.1/proc_macro_kwargs/parse/struct.Syn.html

## v0.1.4 - 2026-08-07
Add `with_func`, `with_wrapper` attribute options.

Add an `ExplicitOption` wrapper to parse `Some`/`None` explicitly.

## v0.1.3 - 2026-08-07
Switch `NestedList` to use `Vec` instead of [`Punctuated`].

Fixes [issue #1], allowing easy conversion from a `NestedList` to a slice.

This is a breaking change.
At the time I considered it acceptable since the crate had only been released for an hour,
and an unreleased version of zerogc was the only consumer.

[issue #1]: https://github.com/Techcable/proc-macro-kwargs.rust/issues/1
[`Punctuated`]: https://docs.rs/syn/1/syn/punctuated/struct.Punctuated.html

## v0.1.2 - 2021-08-07
Implement `IntoIterator` for `NestedList`

## v0.1.1 - 2021-08-07
Implement `MacroArg` for `syn::GenericParam`.

## v0.1.0 - 2021-08-07
Initial release.


# Changelog

Notable changes to this project should be documented in this file.
Make sure it is up to date before performing a release.

This project follows the [Keep a Changelog](https://keepachangelog.com/en/2.0.0/) format wherever that is reasonable.

The "title" of each release should be its first line.
A title is required for publishing a github release, so all versions should have one.

Most newer changes include the relevant [jj](https://jj-vcs.dev) change ids in parens. An example of a change id is wuoxvnsw.

## Unreleased

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


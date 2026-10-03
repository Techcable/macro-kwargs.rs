//! Checks the error messages (and their spans) produced by macros using `macro-kwargs`
#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

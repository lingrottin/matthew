//! Matthew is a library that counts various code elements created by Rust developers. It provides
//! detailed statistics about function definitions, structs, enums, control flow structures,
//! expressions, and other syntactic elements in Rust source code.
//!
//! # Usage
//! ```
//! use matthew::{count_file, count_str, Counts};
//! use std::path::PathBuf;
//!
//! // Count code elements from a file
//! let counts = count_file(PathBuf::from("src/lib.rs")).unwrap();
//!
//! // Count code elements from a string
//! let source_code = "fn main() { println!(\"Hello, world!\"); }".to_string();
//! let counts = count_str(source_code).unwrap();
//!
//! // Access count results
//! println!("Number of function definitions: {}", counts.def_fn);
//! println!("Number of macro invocations: {}", counts.call_macro);
//! ```
//!
//! # Features
//! The library supports the following optional features:
//! - `syn` *(enabled by default)*: Provides code parsing functionality using [syn](https://crates.io/crates/syn)
//! - `serde`: Enables serialization and deserialization support for the `Counts` struct

use std::path::PathBuf;

pub mod types;

pub use anyhow::Result;
pub use types::Counts;

/// Counts various code elements from a Rust source file.
///
/// This function reads the file at the specified path, parses it using the syn crate (if the
/// `syn` feature is enabled), and counts various code elements in the file.
///
/// # Arguments
/// * `path` - The path to the Rust source file to analyze
///
/// # Returns
/// A `Result` containing the `Counts` struct with the statistics of the code elements,
/// or an error if the file cannot be read or parsed.
///
/// # Panics
/// This function panics if no parser feature is enabled. The `syn` feature is enabled by default.
#[expect(unreachable_code)]
pub fn count_file(path: PathBuf) -> Result<Counts> {
    #[cfg(feature = "syn")]
    {
        use crate::types::SynCounter;
        use std::fs::read_to_string;
        use syn::visit::Visit;

        let f = read_to_string(path)?;
        let syntax_tree: syn::File = syn::parse_file(&f)?;
        let mut counter = SynCounter::default();
        counter.visit_file(&syntax_tree);
        return Ok(counter.into_counts());
    }
    panic!("Please enable at least one parser feature!");
}

/// Counts various code elements from a Rust source code string.
///
/// This function parses the provided string using the syn crate (if the `syn` feature is enabled),
/// and counts various code elements in the code.
///
/// # Arguments
/// * `content` - The Rust source code string to analyze
///
/// # Returns
/// A `Result` containing the `Counts` struct with the statistics of the code elements,
/// or an error if the code cannot be parsed.
///
/// # Panics
/// This function panics if no parser feature is enabled. The `syn` feature is enabled by default.
#[expect(unreachable_code)]
pub fn count_str(content: String) -> Result<Counts> {
    #[cfg(feature = "syn")]
    {
        use crate::types::SynCounter;
        use syn::visit::Visit;

        let syntax_tree: syn::File = syn::parse_file(&content)?;
        let mut counter = SynCounter::default();
        counter.visit_file(&syntax_tree);
        return Ok(counter.into_counts());
    }
    panic!("Please enable at least one parser feature!");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a small snippet and return its counts conveniently.
    fn counts(src: &str) -> Counts {
        count_str(src.to_string()).expect("count_str failed")
    }

    #[test]
    fn counts_basic_functions() {
        let c = counts(
            r#"
            fn f1() {}
            async fn f2() {}
            #[test] fn f3() {}
        "#,
        );
        assert_eq!(c.def_fn, 2, "normal fn");
        assert_eq!(c.def_async_fn, 1, "async fn");
        assert_eq!(c.def_test_fn, 1, "#[test] fn");
    }

    #[test]
    fn counts_structs_traits_and_impls() {
        let c = counts(
            r#"
            struct S;
            enum E { A, B }
            trait T { fn x(&self); }
            impl T for S { fn x(&self) {} }
            impl S { fn y(&self) {} }
        "#,
        );
        assert_eq!(c.def_struct, 1);
        assert_eq!(c.def_enum, 1);
        assert_eq!(c.def_trait, 1);
        assert_eq!(c.trait_impl, 1, "impl T for S");
        assert_eq!(c.def_impl, 1, "inherent impl");
        assert_eq!(c.def_trait_members, 1, "trait fn");
        assert_eq!(c.def_method, 2, "impl fn");
    }

    #[test]
    fn counts_macros() {
        let c = counts(
            r#"
            macro_rules! my_macro { () => {} }
            my_macro!();
            println!("hi");
        "#,
        );
        assert_eq!(c.def_macro_rules, 1);
        assert_eq!(c.call_macro, 2);
    }

    #[test]
    fn counts_control_flow() {
        let c = counts(
            r#"
            fn main() {
                if true {} else {}
                match 3 { 1 => {}, _ => {} }
                for _ in 0..3 {}
                while false {}
                loop { break }
            }
        "#,
        );
        assert_eq!(c.block_if, 1);
        assert_eq!(c.block_else, 1);
        assert_eq!(c.block_match, 1);
        assert_eq!(c.block_match_arms, 2);
        assert_eq!(c.block_for, 1);
        assert_eq!(c.block_while, 1);
        assert_eq!(c.block_loop, 1);
    }

    #[test]
    fn counts_exprs_and_vars() {
        let c = counts(
            r#"
            fn main() {
                let a = 1;
                let mut b = 2;
                a + b;
                -b;
                async { a }.await;
                try { Ok::<_, ()>(())? };
                return;
            }
        "#,
        );
        assert_eq!(c.def_var, 1);
        assert_eq!(c.def_var_mut, 1);
        assert_eq!(c.expr_binop, 1);
        assert_eq!(c.expr_unop, 1);
        assert_eq!(c.expr_await, 1);
        assert_eq!(c.expr_try, 1);
        assert_eq!(c.expr_return, 1);
    }

    #[test]
    #[should_panic]
    fn not_rust_code() {
        let _ = counts("not a valid rust code,,,....");
    }
}

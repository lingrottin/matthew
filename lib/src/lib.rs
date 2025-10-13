use std::path::PathBuf;

pub mod types;

pub use types::Counts;

#[expect(unreachable_code)]
pub fn count_file(path: PathBuf) -> anyhow::Result<Counts> {
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

#[expect(unreachable_code)]
pub fn count_str(content: String) -> anyhow::Result<Counts> {
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
        assert!(c.call_macro >= 2, "macro_rules and println!");
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
}

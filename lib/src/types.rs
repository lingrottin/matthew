use std::ops::Add;

/// A comprehensive collection of code element counts in a file or crate.
///
/// This struct stores statistics about various code elements found in Rust source code.
/// It categorizes definitions, control structures, expressions, and other syntactic elements
/// that might reflect a developer's style or project complexity.
///
/// The struct implements the `Add` trait, allowing you to combine counts from multiple sources.
/// It also supports `PartialEq` for comparing count results.
///
/// When the `serde` feature is enabled, it also implements `Serialize` and `Deserialize` traits.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone)]
pub struct Counts {
    // ===== Function and call-related =====
    /// Number of function calls (e.g., `foo()`).
    pub call_fn: u64,
    /// Number of method calls (e.g., `obj.method()`).
    pub call_method: u64,
    /// Number of macro invocations (e.g., `println!()`).
    pub call_macro: u64,

    // ===== Control flow structures =====
    /// Number of `if` blocks.
    pub block_if: u64,
    /// Number of `else` blocks.
    pub block_else: u64,
    /// Number of `match` expressions.
    pub block_match: u64,
    /// Number of `match` arms.
    pub block_match_arms: u64,
    /// Number of `for` loops.
    pub block_for: u64,
    /// Number of `while` loops.
    pub block_while: u64,
    /// Number of `loop` expressions.
    pub block_loop: u64,
    /// Number of `unsafe` blocks.
    pub block_unsafe: u64,

    // ===== Definitions =====
    /// Number of immutable variable bindings (`let`).
    pub def_var: u64,
    /// Number of mutable variable bindings (`let mut`).
    pub def_var_mut: u64,
    /// Number of function definitions (`fn`).
    pub def_fn: u64,
    /// Number of associated method definitions (all functions in an `impl` block, which includes trait implementation).
    pub def_method: u64,
    /// Number of async function definitions (`async fn`).
    pub def_async_fn: u64,
    /// Number of async method definitions (all async functions in an `impl` block, which includes trait implementation)
    pub def_async_method: u64,
    /// Number of trait definitions
    pub def_trait: u64,
    /// Number of type alias definitions (`type`).
    pub def_type: u64,
    /// Number of enum definitions.
    pub def_enum: u64,
    /// Number of struct definitions.
    pub def_struct: u64,
    /// Number of union definitions.
    pub def_union: u64,
    /// Number of module definitions (`mod`).
    pub def_mod: u64,
    /// Number of constant definitions (`const`).
    pub def_const: u64,
    /// Number of static item definitions (`static`).
    pub def_static: u64,
    /// Number of trait implementation blocks (`impl Trait for Type`).
    pub trait_impl: u64,
    /// Number of inherent implementation blocks (`impl Type`).
    pub def_impl: u64,
    /// Number of `macro_rules!` definitions.
    pub def_macro_rules: u64,

    // ===== Expressions and operators =====
    /// Number of closure expressions (`|x| x + 1`).
    pub expr_closure: u64,
    /// Number of await expressions (`.await`).
    pub expr_await: u64,
    /// Number of try expressions (`?` operator).
    pub expr_try: u64,
    /// Number of return expressions.
    pub expr_return: u64,
    /// Number of yield expressions (generators).
    pub expr_yield: u64,
    /// Number of binary operations (`+`, `-`, `*`, etc.).
    pub expr_binop: u64,
    /// Number of unary operations (`!`, `-x`, etc.).
    pub expr_unop: u64,

    // ===== Imports and modules =====
    /// Number of `use` statements.
    pub use_stmt: u64,
    /// Number of `extern crate` declarations.
    pub extern_crate: u64,

    // ===== Structural metrics =====
    /// Number of struct fields across all structs.
    pub def_struct_fields: u64,
    /// Number of enum variants across all enums.
    pub def_enum_variants: u64,
    /// Number of items inside `trait` definitions.
    pub def_trait_members: u64,
    /// Number of items inside `impl` blocks.
    pub def_impl_members: u64,

    // ===== Miscellaneous =====
    /// Number of `#[test]` functions.
    pub def_test_fn: u64,
}

/// Auto-implements `PartialEq`, and `Add` for Counts
macro_rules! impl_counts_traits {
    ($( $field:ident ),+ $(,)?) => {
        impl PartialEq for Counts {
            fn eq(&self, other: &Self) -> bool {
                true $( && self.$field == other.$field )+
            }
        }

        impl Add for Counts {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self {
                    $( $field: self.$field + rhs.$field, )+
                }
            }
        }
    };
}

impl_counts_traits!(
    call_fn,
    call_method,
    call_macro,
    block_if,
    block_else,
    block_match,
    block_match_arms,
    block_for,
    block_while,
    block_loop,
    block_unsafe,
    def_var,
    def_var_mut,
    def_fn,
    def_method,
    def_async_fn,
    def_async_method,
    def_trait,
    def_type,
    def_enum,
    def_struct,
    def_union,
    def_mod,
    def_const,
    def_static,
    trait_impl,
    def_impl,
    def_macro_rules,
    expr_closure,
    expr_await,
    expr_try,
    expr_return,
    expr_yield,
    expr_binop,
    expr_unop,
    use_stmt,
    extern_crate,
    def_struct_fields,
    def_enum_variants,
    def_trait_members,
    def_impl_members,
    def_test_fn
);

#[cfg(feature = "syn")]
#[derive(Default)]
pub struct SynCounter {
    counts: Counts,
}

/// A visitor that counts various code elements in a Rust AST.
///
/// This struct is used internally by the library to traverse a Rust abstract syntax tree (AST)
/// and count various code elements. It implements the `syn::visit::Visit` trait to visit
/// different nodes in the AST and update the counts accordingly.
///
/// This struct is only available when the `syn` feature is enabled.
impl SynCounter {
    /// Extracts the collected counts from the counter.
    ///
    /// # Returns
    /// The `Counts` struct containing the statistics of the code elements counted during the visit.
    pub fn into_counts(self) -> Counts {
        self.counts
    }
}

#[cfg(feature = "syn")]
impl<'ast> syn::visit::Visit<'ast> for SynCounter {
    fn visit_arm(&mut self, i: &'ast syn::Arm) {
        self.counts.block_match_arms += 1;
        syn::visit::visit_arm(self, i);
    }

    fn visit_expr_await(&mut self, i: &'ast syn::ExprAwait) {
        self.counts.expr_await += 1;
        syn::visit::visit_expr_await(self, i);
    }

    fn visit_expr_binary(&mut self, i: &'ast syn::ExprBinary) {
        self.counts.expr_binop += 1;
        syn::visit::visit_expr_binary(self, i);
    }

    fn visit_expr_call(&mut self, i: &'ast syn::ExprCall) {
        self.counts.call_fn += 1;
        syn::visit::visit_expr_call(self, i);
    }

    fn visit_expr_closure(&mut self, i: &'ast syn::ExprClosure) {
        self.counts.expr_closure += 1;
        syn::visit::visit_expr_closure(self, i);
    }

    fn visit_expr_for_loop(&mut self, i: &'ast syn::ExprForLoop) {
        self.counts.block_for += 1;
        syn::visit::visit_expr_for_loop(self, i);
    }

    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        self.counts.block_if += 1;
        if i.else_branch.is_some() {
            self.counts.block_else += 1;
        }
        syn::visit::visit_expr_if(self, i);
    }

    fn visit_expr_loop(&mut self, i: &'ast syn::ExprLoop) {
        self.counts.block_loop += 1;
        syn::visit::visit_expr_loop(self, i);
    }

    fn visit_expr_macro(&mut self, i: &'ast syn::ExprMacro) {
        self.counts.call_macro += 1;
        syn::visit::visit_expr_macro(self, i);
    }

    fn visit_expr_match(&mut self, i: &'ast syn::ExprMatch) {
        self.counts.block_match += 1;
        syn::visit::visit_expr_match(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast syn::ExprMethodCall) {
        self.counts.call_method += 1;
        syn::visit::visit_expr_method_call(self, i);
    }

    fn visit_expr_return(&mut self, i: &'ast syn::ExprReturn) {
        self.counts.expr_return += 1;
        syn::visit::visit_expr_return(self, i);
    }

    fn visit_expr_try(&mut self, i: &'ast syn::ExprTry) {
        self.counts.expr_try += 1;
        syn::visit::visit_expr_try(self, i);
    }

    fn visit_expr_unary(&mut self, i: &'ast syn::ExprUnary) {
        self.counts.expr_unop += 1;
        syn::visit::visit_expr_unary(self, i);
    }

    fn visit_expr_unsafe(&mut self, i: &'ast syn::ExprUnsafe) {
        self.counts.block_unsafe += 1;
        syn::visit::visit_expr_unsafe(self, i);
    }

    fn visit_expr_while(&mut self, i: &'ast syn::ExprWhile) {
        self.counts.block_while += 1;
        syn::visit::visit_expr_while(self, i);
    }

    fn visit_expr_yield(&mut self, i: &'ast syn::ExprYield) {
        self.counts.expr_yield += 1;
        syn::visit::visit_expr_yield(self, i);
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        if i.attrs.iter().any(|attr| attr.path().is_ident("test")) {
            self.counts.def_test_fn += 1;
        }
        if i.sig.asyncness.is_some() {
            self.counts.def_async_method += 1;
        } else {
            self.counts.def_method += 1;
        }
        syn::visit::visit_impl_item_fn(self, i);
    }

    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        self.counts.def_const += 1;
        syn::visit::visit_item_const(self, i);
    }

    fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
        self.counts.def_enum += 1;
        syn::visit::visit_item_enum(self, i);
    }

    fn visit_item_extern_crate(&mut self, i: &'ast syn::ItemExternCrate) {
        self.counts.extern_crate += 1;
        syn::visit::visit_item_extern_crate(self, i);
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if i.attrs.iter().any(|attr| attr.path().is_ident("test")) {
            self.counts.def_test_fn += 1;
        }
        if i.sig.asyncness.is_some() {
            self.counts.def_async_fn += 1;
        } else {
            self.counts.def_fn += 1;
        }
        syn::visit::visit_item_fn(self, i);
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        if i.trait_.is_some() {
            self.counts.trait_impl += 1;
        } else {
            self.counts.def_impl += 1;
        }
        syn::visit::visit_item_impl(self, i);
    }

    fn visit_item_macro(&mut self, i: &'ast syn::ItemMacro) {
        if i.mac.path.is_ident("macro_rules") {
            self.counts.def_macro_rules += 1;
        } else {
            self.counts.call_macro += 1;
        }
        syn::visit::visit_item_macro(self, i);
    }

    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        self.counts.def_mod += 1;
        syn::visit::visit_item_mod(self, i);
    }

    fn visit_item_static(&mut self, i: &'ast syn::ItemStatic) {
        self.counts.def_static += 1;
        syn::visit::visit_item_static(self, i);
    }

    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        self.counts.def_struct += 1;
        syn::visit::visit_item_struct(self, i);
    }

    fn visit_item_trait(&mut self, i: &'ast syn::ItemTrait) {
        self.counts.def_trait += 1;
        syn::visit::visit_item_trait(self, i);
    }

    fn visit_item_type(&mut self, i: &'ast syn::ItemType) {
        self.counts.def_type += 1;
        syn::visit::visit_item_type(self, i);
    }

    fn visit_item_union(&mut self, i: &'ast syn::ItemUnion) {
        self.counts.def_union += 1;
        syn::visit::visit_item_union(self, i);
    }

    fn visit_item_use(&mut self, i: &'ast syn::ItemUse) {
        self.counts.use_stmt += 1;
        syn::visit::visit_item_use(self, i);
    }

    fn visit_pat_ident(&mut self, i: &'ast syn::PatIdent) {
        if i.mutability.is_some() {
            self.counts.def_var_mut += 1;
        } else {
            self.counts.def_var += 1;
        }
        syn::visit::visit_pat_ident(self, i);
    }

    fn visit_trait_item(&mut self, i: &'ast syn::TraitItem) {
        self.counts.def_trait_members += 1;
        syn::visit::visit_trait_item(self, i);
    }
}

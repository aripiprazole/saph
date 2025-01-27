//! Defines a module for walking throughout the AST, searching all fields for
//! a given pattern.

use std::collections::HashSet;

use fxhash::FxBuildHasher;

use crate::{solver::Reference, source::*};

pub trait Walker {
    fn accept<T: HirListener>(self, db: &dyn crate::HirDb, listener: &mut T);
}

impl<T: Walker> Walker for Vec<T> {
    fn accept<U: HirListener>(self, db: &dyn crate::HirDb, listener: &mut U) {
        for item in self {
            item.accept(db, listener);
        }
    }
}

impl<T: Walker> Walker for Option<T> {
    fn accept<U: HirListener>(self, db: &dyn crate::HirDb, listener: &mut U) {
        if let Some(item) = self {
            item.accept(db, listener);
        }
    }
}

impl<T: Walker> Walker for HashSet<T, FxBuildHasher> {
    fn accept<U: HirListener>(self, db: &dyn crate::HirDb, listener: &mut U) {
        for item in self {
            item.accept(db, listener);
        }
    }
}

impl<T: Walker> Walker for Box<T> {
    fn accept<U: HirListener>(self, db: &dyn crate::HirDb, listener: &mut U) {
        (*self).accept(db, listener)
    }
}

/// A listener that is called when a node is visited. It does have two methods:
/// `enter` and `exit`. The `enter` method is called when the node is visited
/// and the `exit` method is called when the node is left.
///
/// All functions in this trait have a default implementation that does nothing,
/// so all the functions have the `()` return type.
#[allow(dead_code, unused_variables, clippy::boxed_local)]
#[rustfmt::skip]
pub trait HirListener {
    // SECTION: visitors
    fn visit_reference(&mut self, reference: Reference) {}

    // SECTION: expr
    fn visit_type(&mut self, definition: expr::Type, location: Location) {}
    fn visit_hole(&mut self, location: Location) {}
    fn visit_empty_expr(&mut self) {}
    fn enter_path_expr(&mut self, definition: Reference) {}
    fn enter_literal_expr(&mut self, literal: Spanned<literal::Literal>) {}
    fn enter_call_expr(&mut self, call_expr: expr::CallExpr) {}
    fn enter_ann_expr(&mut self, call_expr: expr::AnnExpr) {}
    fn enter_lam_expr(&mut self, call_expr: expr::LamExpr) {}
    fn enter_match_expr(&mut self, match_expr: expr::MatchExpr) {}
    fn enter_upgrade_expr(&mut self, type_rep: Box<type_rep::TypeRep>) {}
    fn enter_pi(&mut self, type_rep: expr::Pi) {}
    fn enter_sigma(&mut self, type_rep: expr::Pi) {}
    fn enter_fun(&mut self, type_rep: expr::Pi) {}

    // SECTION: stmt
    fn visit_empty_stmt(&mut self) {}
    fn enter_let_stmt(&mut self, let_stmt: stmt::LetStmt) {}
    fn enter_ask_stmt(&mut self, ask_stmt: stmt::AskStmt) {}
    fn enter_downgrade_stmt(&mut self, expr: expr::Expr) {}
    fn enter_block(&mut self, block: stmt::Block) {}

    // SECTION: pattern
    fn visit_empty_pattern(&mut self) {}
    fn enter_literal_pattern(&mut self, literal: Spanned<literal::Literal>) {}
    fn enter_wildcard_pattern(&mut self, location: Location) {}
    fn enter_rest_pattern(&mut self, location: Location) {}
    fn enter_constructor_pattern(&mut self, constructor: pattern::ConstructorPattern) {}
    fn enter_binding_pattern(&mut self, binding: pattern::BindingPattern) {}

    // SECTION: top_level
    fn enter_using_top_level(&mut self, using: top_level::UsingTopLevel) {}
    fn enter_binding_top_level(&mut self, binding: top_level::BindingGroup) {}
    fn enter_command_top_level(&mut self, command: top_level::CommandTopLevel) {}
    fn enter_inductive_top_level(&mut self, inductive: top_level::Inductive) {}

    // SECTION: type_rep

    // SECTION: expr
    fn exit_path_expr(&mut self, definition: Reference) {}
    fn exit_literal_expr(&mut self, literal: Spanned<literal::Literal>) {}
    fn exit_call_expr(&mut self, call_expr: expr::CallExpr) {}
    fn exit_ann_expr(&mut self, call_expr: expr::AnnExpr) {}
    fn exit_lam_expr(&mut self, call_expr: expr::LamExpr) {}
    fn exit_match_expr(&mut self, match_expr: expr::MatchExpr) {}
    fn exit_pi(&mut self, type_rep: expr::Pi) {}
    fn exit_sigma(&mut self, type_rep: expr::Pi) {}
    fn exit_fun(&mut self, type_rep: expr::Pi) {}

    // SECTION: stmt
    fn exit_let_stmt(&mut self, let_stmt: stmt::LetStmt) {}
    fn exit_ask_stmt(&mut self, ask_stmt: stmt::AskStmt) {}
    fn exit_downgrade_stmt(&mut self, expr: expr::Expr) {}
    fn exit_block(&mut self, block: stmt::Block) {}

    // SECTION: pattern
    fn exit_literal_pattern(&mut self, literal: Spanned<literal::Literal>) {}
    fn exit_wildcard_pattern(&mut self, location: Location) {}
    fn exit_rest_pattern(&mut self, location: Location) {}
    fn exit_constructor_pattern(&mut self, constructor: pattern::ConstructorPattern) {}
    fn exit_binding_pattern(&mut self, binding: pattern::BindingPattern) {}

    // SECTION: top_level
    fn exit_using_top_level(&mut self, using: top_level::UsingTopLevel) {}
    fn exit_binding_top_level(&mut self, binding: top_level::BindingGroup) {}
    fn exit_command_top_level(&mut self, command: top_level::CommandTopLevel) {}
    fn exit_inductive_top_level(&mut self, inductive: top_level::Inductive) {}
}

//! Defines a module for resolving language "terms", that are the expressions, types, and other
//! things that are used in the language, like primaries. It will be used in the resolution, and
//! it's a helper module for the [`LowerHir`] struct.
//!
//! It's only a module, to organization purposes.

use sol_diagnostic::report_error;
use sol_hir::{
    errors::{HirError, HirErrorKind},
    solver::HirLevel,
    source::{
        expr::{MatchArm, MatchExpr, MatchKind, Pi, Type},
        literal::Literal,
        pattern::Pattern,
        HirElement,
    },
};

use super::*;

#[rustfmt::skip]
type SyntaxExpr<'tree> = sol_syntax::anon_unions::AnnExpr_AppExpr_BinaryExpr_LamExpr_MatchExpr_PiExpr_Primary_SigmaExpr<'tree>;

#[rustfmt::skip]
type SyntaxTypeRep<'tree> = sol_syntax::anon_unions::AnnExpr_BinaryExpr_LamExpr_MatchExpr_PiExpr_Primary_SigmaExpr_TypeAppExpr<'tree>;

impl HirLowering<'_, '_> {
    /// Resolves a type level expression.
    ///
    /// It does use the type level of expressions to resolve syntax
    /// expressions into high-level type representations.
    pub fn type_expr(&mut self, tree: SyntaxTypeRep) -> TypeRep {
        use sol_syntax::anon_unions::AnnExpr_BinaryExpr_LamExpr_MatchExpr_PiExpr_Primary_SigmaExpr_TypeAppExpr::*;

        match tree {
            // SECTION: type_expr
            //
            // Upgrades the expressions to type level expressions, to be easier to handle errors
            // in the type system, and still keeps the diagnostics in the IDE.
            Primary(primary) => self.primary(primary, HirLevel::Type).upgrade(self.db),
            AnnExpr(ann_expr) => self.ann_expr(ann_expr, HirLevel::Type).upgrade(self.db),
            LamExpr(lam_expr) => self.lam_expr(lam_expr, HirLevel::Type).upgrade(self.db),
            MatchExpr(match_expr) => self.match_expr(match_expr, HirLevel::Type).upgrade(self.db),
            BinaryExpr(binary_expr) => self
                .binary_expr(binary_expr, HirLevel::Type)
                .upgrade(self.db),

            // Type level expressions
            PiExpr(pi) => self.pi_expr(pi),
            SigmaExpr(sigma) => self.sigma_expr(sigma),
            TypeAppExpr(type_app) => self.type_app_expr(type_app),
        }
    }

    /// Resolves an expression.
    ///
    /// It does use the expression level of expressions to resolve syntax
    /// expressions into high-level expressions.
    pub fn expr(&mut self, tree: SyntaxExpr, level: HirLevel) -> Expr {
        use sol_syntax::anon_unions::AnnExpr_AppExpr_BinaryExpr_LamExpr_MatchExpr_PiExpr_Primary_SigmaExpr::*;

        match tree {
            // SECTION: expr
            Primary(primary) => self.primary(primary, level),
            AnnExpr(ann_expr) => self.ann_expr(ann_expr, level),
            LamExpr(lam_expr) => self.lam_expr(lam_expr, level),
            MatchExpr(match_expr) => self.match_expr(match_expr, level),
            BinaryExpr(binary_expr) => self.binary_expr(binary_expr, level),
            AppExpr(app) => self.app_expr(app, level),

            // Type level expressions
            PiExpr(pi) => self.pi_expr(pi).downgrade(),
            SigmaExpr(sigma) => self.sigma_expr(sigma).downgrade(),
        }
    }

    /// Resolves an annotation expression.
    ///
    /// It does translate the syntax annotation expression
    /// into a high-level annotation.
    pub fn ann_expr(&mut self, tree: sol_syntax::AnnExpr, level: HirLevel) -> Expr {
        let value = tree
            .value()
            .solve(self, |this, node| this.expr(node, level));
        let type_rep = tree
            .against()
            .solve(self, |this, expr| this.type_expr(expr));
        let location = self.range(tree.range());

        Expr::Ann(AnnExpr {
            value: Box::new(value),
            type_rep,
            location,
        })
    }

    /// Resolves a binary expression.
    ///
    /// It does translate the syntax binary expression
    /// into a high-level binary expression.
    pub fn binary_expr(&mut self, tree: sol_syntax::BinaryExpr, level: HirLevel) -> Expr {
        let lhs = tree.lhs().solve(self, |this, node| this.expr(node, level));
        let rhs = tree.rhs().solve(self, |this, node| {
            use sol_syntax::anon_unions::BinaryExpr_Primary::*;

            match node {
                BinaryExpr(binary_expr) => this.binary_expr(binary_expr, level),
                Primary(primary) => this.primary(primary, level),
            }
        });
        let op = tree.op().solve(self, |this, node| {
            let location = this.range(node.range());
            let identifier = node
                .utf8_text(this.src.source_text(this.db).as_bytes())
                .unwrap_or_default();

            let identifier = Identifier::symbol(this.db, identifier, location.clone());

            HirPath::new(this.db, location, vec![identifier])
        });
        let location = self.range(tree.range());

        let op = self.qualify(op, DefinitionKind::Function);

        let reference = self.scope.using(self.db, op, location.clone());

        Expr::Call(CallExpr {
            kind: CallKind::Infix,
            callee: Callee::Reference(reference),
            arguments: vec![lhs, rhs],
            do_notation: None,
            location,
        })
    }

    /// Resolves a lambda expression.
    ///
    /// It does translate the syntax lambda expression
    /// into a high-level lambda expression.
    pub fn lam_expr(&mut self, tree: sol_syntax::LamExpr, level: HirLevel) -> Expr {
        self.scope = self.scope.fork(ScopeKind::Lambda);

        let parameters = tree
            .parameters(&mut tree.walk())
            .flatten()
            .filter_map(|node| node.regular())
            .map(|node| self.pattern(node))
            .collect::<Vec<_>>();

        let value = tree
            .value()
            .solve(self, |this, node| this.expr(node, level));
        let location = self.range(tree.range());

        let scope = self.pop_scope();

        Expr::Lam(LamExpr {
            parameters,
            value: Box::new(value),
            location,
            scope,
        })
    }

    /// Resolves a call expression.
    ///
    /// It does translate the syntax call expression
    /// into a high-level call expression.
    pub fn app_expr(&mut self, tree: sol_syntax::AppExpr, level: HirLevel) -> Expr {
        let callee = tree
            .callee()
            .solve(self, |this, node| this.primary(node, level));

        let arguments = tree
            .arguments(&mut tree.walk())
            .flatten()
            .flat_map(|node| node.regular())
            .map(|node| self.primary(node, level))
            .collect::<Vec<_>>();

        let do_notation = tree
            .children(&mut tree.walk())
            .flatten()
            .filter_map(|node| node.regular())
            .filter_map(|node| node.block())
            .map(|node| self.block(node, level))
            .last();

        let location = self.range(tree.range());

        Expr::Call(CallExpr {
            kind: CallKind::Infix,
            callee: Callee::Expr(callee.into()),
            arguments,
            do_notation,
            location,
        })
    }

    /// Resolves a type level application expression.
    ///
    /// It does translate the syntax type level application expression
    /// into a high-level type level application expression.
    pub fn type_app_expr(&mut self, tree: sol_syntax::TypeAppExpr) -> TypeRep {
        let callee = tree.callee().solve(self, |this, node| {
            this.primary(node, HirLevel::Type).upgrade(this.db)
        });

        let arguments = tree
            .arguments(&mut tree.walk())
            .flatten()
            .flat_map(|node| node.regular())
            .map(|node| self.primary(node, HirLevel::Type).upgrade(self.db))
            .collect::<Vec<_>>();

        let location = self.range(tree.range());

        TypeRep {
            expr: Box::new(Expr::Call(CallExpr {
                kind: CallKind::Prefix,
                callee: Callee::Expr(callee.downgrade().into()),
                do_notation: None,
                arguments: arguments.into_iter().map(|expr| expr.downgrade()).collect(),
                location,
            })),
        }
    }

    /// Resolves a pi type expression.
    ///
    /// It does translate the syntax pi type expression
    /// into a high-level pi type expression.
    pub fn pi_expr(&mut self, tree: sol_syntax::PiExpr) -> TypeRep {
        use sol_syntax::anon_unions::AnnExpr_BinaryExpr_ForallParameters_LamExpr_MatchExpr_PiExpr_PiParameters_Primary_SigmaExpr_TypeAppExpr::*;

        self.scope = self.scope.fork(ScopeKind::Pi);

        let parameters = tree.parameter().solve(self, |this, node| {
            let type_rep = match node {
                PiParameters(tree) => {
                    return tree
                        .parameters(&mut tree.walk())
                        .flatten()
                        .filter_map(|node| node.regular())
                        .filter_map(|node| node.parameter())
                        .map(|parameter| this.parameter(false, true, parameter))
                        .collect::<Vec<_>>();
                }
                ForallParameters(tree) => {
                    return tree
                        .parameters(&mut tree.walk())
                        .flatten()
                        .filter_map(|node| node.regular())
                        .filter_map(|node| node.parameter())
                        .map(|parameter| this.parameter(false, true, parameter))
                        .collect::<Vec<_>>();
                }
                _ => this.type_expr(node.into_node().try_into().unwrap()),
            };

            // This handles the case where the parameter is unnamed, and only haves a type. The name
            // should not be shown in the IDE in this case.
            vec![Parameter::unnamed(this.db, type_rep)]
        });

        let value = tree.value().solve(self, |this, expr| this.type_expr(expr));
        let _ = self.pop_scope();

        TypeRep {
            expr: Box::new(Expr::Pi(Pi {
                parameters,
                value: Box::new(value),
                location: self.range(tree.range()),
            })),
        }
    }

    /// Resolves a sigma type expression.
    ///
    /// It does translate the syntax sigma type expression
    /// into a high-level sigma type expression.
    pub fn sigma_expr(&mut self, tree: sol_syntax::SigmaExpr) -> TypeRep {
        self.scope = self.scope.fork(ScopeKind::Sigma);

        let parameters = tree
            .parameters(&mut tree.walk())
            .flatten()
            .filter_map(|parameter| parameter.regular())
            .map(|parameter| {
                use sol_syntax::anon_unions::Comma_ConsPattern_GroupPattern_Literal_Parameter_RestPattern::*;

                match parameter {
                    Parameter(parameter) => self.parameter(true, true, parameter),
                    _ => {
                        let parameter = parameter.into_node().try_into().unwrap();

                        // This handles the case where the parameter is unnamed, and only haves a
                        // pattern. The name should not be shown in the IDE in this case.
                        let pattern = self.trait_pattern(parameter);

                        // The location of the parameter is the location of the pattern
                        let location = pattern.location(self.db);

                        // Creates a new parameter, with no type, but a pattern
                        self::Parameter::new(
                            self.db,
                            /* binding     = */ pattern,
                            /* type_rep    = */ TypeRep {
                                expr: Box::new(Expr::Hole(location.clone())),
                            },
                            /* is_implicit = */ true,
                            /* rigid       = */ false,
                            /* level       = */ HirLevel::Type,
                            /* location    = */ location,
                        )
                    }
                }
            })
            .collect::<Vec<_>>();

        let value = tree.value().solve(self, |this, expr| this.type_expr(expr));
        let _ = self.pop_scope();

        TypeRep {
            expr: Box::new(Expr::Sigma(Pi {
                parameters,
                value: Box::new(value),
                location: self.range(tree.range()),
            })),
        }
    }

    /// Resolves a match expression.
    ///
    /// It does translate the syntax match expression
    /// into a high-level match expression.
    pub fn match_expr(&mut self, tree: sol_syntax::MatchExpr, level: HirLevel) -> Expr {
        let scrutinee = tree
            .scrutinee()
            .solve(self, |this, node| this.expr(node, level));

        let location = self.range(tree.range());

        let clauses = tree.arms(&mut tree.walk())
      .flatten()
      .filter_map(|node| node.regular())
      .map(|node| {
        let pattern = node.pattern().solve(self, |this, pattern| this.pattern(pattern));
        let body = node.body().solve(self, |this, node| {
          use sol_syntax::anon_unions::AnnExpr_AppExpr_BinaryExpr_Block_LamExpr_MatchExpr_PiExpr_Primary_SigmaExpr::*;

          match node {
            Block(block) => Expr::block(this.db, this.block(block, level)),
            _ => this.expr(node.into_node().try_into().unwrap(), level)
          }
        });

        MatchArm {
          pattern,
          value: body,
          location: self.range(node.range()),
        }
      })
      .collect();

        Expr::Match(MatchExpr {
            kind: MatchKind::Match,
            scrutinee: Box::new(scrutinee),
            clauses,
            location,
        })
    }

    /// Resolves a if expression.
    ///
    /// It does translate the syntax if expression
    /// into a high-level if expression.
    pub fn if_expr(&mut self, tree: sol_syntax::IfExpr, level: HirLevel) -> Expr {
        let scrutinee = tree
            .condition()
            .solve(self, |this, node| this.expr(node, level));

        let then = tree.then().solve(self, |this, node| {
        use sol_syntax::anon_unions::AnnExpr_AppExpr_BinaryExpr_Block_LamExpr_MatchExpr_PiExpr_Primary_SigmaExpr::*;

        node.child().solve(this, |this, node| match node {
          Block(block) => Expr::block(this.db, this.block(block, level)),
          _ => this.expr(node.into_node().try_into().unwrap(), level),
        })
      });

        let otherwise = tree.otherwise().solve(self, |this, node| {
        use sol_syntax::anon_unions::AnnExpr_AppExpr_BinaryExpr_Block_LamExpr_MatchExpr_PiExpr_Primary_SigmaExpr::*;

        node.value().solve(this, |this, node| match node {
          Block(block) => Expr::block(this.db, this.block(block, level)),
          _ => this.expr(node.into_node().try_into().unwrap(), level),
        })
      });

        let clauses = vec![
            MatchArm {
                pattern: Pattern::Literal(Spanned::on_call_site(Literal::TRUE)),
                location: then.location(self.db),
                value: then,
            },
            MatchArm {
                pattern: Pattern::Literal(Spanned::on_call_site(Literal::FALSE)),
                location: otherwise.location(self.db),
                value: otherwise,
            },
        ];

        let location = self.range(tree.range());

        Expr::Match(MatchExpr {
            kind: MatchKind::If,
            scrutinee: Box::new(scrutinee),
            clauses,
            location,
        })
    }

    /// Resolves an array expression.
    ///
    /// It does translate the syntax array expression
    /// into a high-level array expression.
    pub fn array_expr(&mut self, tree: sol_syntax::ArrayExpr, level: HirLevel) -> Expr {
        let location = self.range(tree.range());

        let items = tree
            .items(&mut tree.walk())
            .map(|item| item.solve(self, |this, node| this.expr(node, level)))
            .collect::<Vec<_>>();

        Expr::Call(CallExpr {
            kind: CallKind::Prefix,
            callee: Callee::Array,
            arguments: items,
            do_notation: None,
            location,
        })
    }

    /// Resolves a tuple expression.
    ///
    /// It does translate the syntax tuple expression
    /// into a high-level tuple expression.
    pub fn tuple_expr(&mut self, tree: sol_syntax::TupleExpr, level: HirLevel) -> Expr {
        let location = self.range(tree.range());

        let arguments = tree
            .children(&mut tree.walk())
            .map(|item| item.solve(self, |this, node| this.expr(node, level)))
            .collect::<Vec<_>>();

        Expr::Call(CallExpr {
            kind: CallKind::Prefix,
            callee: Callee::Tuple,
            arguments,
            do_notation: None,
            location,
        })
    }

    /// Resolves a return expression.
    ///
    /// It does translate the syntax return expression
    /// into a high-level return expression.
    pub fn return_expr(&mut self, tree: sol_syntax::ReturnExpr, level: HirLevel) -> Expr {
        let location = self.range(tree.range());

        // Reports errors, because "return expression" is equivalent
        // to pure expression, and it's only allowed inside do notation.
        //
        // Or in other words, it's only allowed inside a do notation scope.
        if !self.scope.is_do_notation_scope() {
            report_error(self.db, HirError {
                label: location.clone(),
                kind: HirErrorKind::ReturnOutsideDoNotation,
            })
        }

        // If it's a return expression, it will return the value of the expression, otherwise it
        // will return a default value.
        let value = tree
            .value()
            .map(|node| node.solve(self, |this, node| this.expr(node, level)))
            .unwrap_or_else(|| Expr::call_unit_expr(location.clone()));

        Expr::Call(CallExpr {
            kind: CallKind::Prefix,
            callee: Callee::Pure,
            arguments: vec![value],
            do_notation: None,
            location,
        })
    }

    /// Resolves a primary expression.
    ///
    /// It does translate the syntax primary expression
    /// using the level supplied.
    pub fn primary(&mut self, tree: sol_syntax::Primary, level: HirLevel) -> Expr {
        use sol_syntax::anon_unions::ArrayExpr_FreeVariable_IfExpr_Literal_MatchExpr_Path_ReturnExpr_TupleExpr_UniverseExpr::*;

        let location = self.range(tree.range());

        tree.child().solve(self, |this, node| match node {
            // SECTION: primary
            ArrayExpr(array_expr) => this.array_expr(array_expr, level),
            IfExpr(if_expr) => this.if_expr(if_expr, level),
            Literal(literal) => this.literal(literal).upgrade_expr(location, this.db),
            MatchExpr(match_expr) => this.match_expr(match_expr, level),
            ReturnExpr(return_expr) => this.return_expr(return_expr, level),
            TupleExpr(tuple_expr) => this.tuple_expr(tuple_expr, level),

            // SECTION: identifier
            // It will match agains't the identifier, and it will create a new [`Expr::Path`]
            // expression, with the [`Definition`] as the callee, from the given identifier.
            //
            // It will search for the definition in the scope, and if it is not present in the
            // it will query the compiler.
            Path(identifier) => {
                let location = this.range(identifier.range());

                // Create a new path with the identifier, and search for the definition in the
                // scope, and if it is not present in the scope, it will invoke a compiler query
                // to search in the entire package.
                let path = this.path(identifier);

                let def = match level {
                    HirLevel::Expr => this.qualify(path, DefinitionKind::Function),
                    HirLevel::Type => this.qualify(path, DefinitionKind::Type),
                };

                // Creates a new [`Reference`] from the [`Definition`] and the location.
                let reference = this.scope.using(this.db, def, location);

                // Creates a new [`Expr`] with the [`Definition`] as the callee.
                Expr::Path(reference)
            }
            // Free variables are variables that aren't bound in the context,
            // and it's only allowed in the type level.
            //
            // TODO: add to a list of free-variables, to build the forall type
            FreeVariable(identifier) => {
                let location = this.range(identifier.range());

                // Create a new path with the identifier, and search for the definition in the
                // scope, and if it is not present in the scope, it will invoke a compiler query
                // to search in the entire package.
                //
                // NOTE: We need to remove the first character, because it's a `^` character. So we can
                // get the identifier without the `^` character.
                let text = identifier
                    .utf8_text(this.src.source_text(this.db).as_bytes())
                    .unwrap_or_default();
                let identifier =
                    Identifier::symbol(this.db, &text[1..text.len()], location.clone());
                let path = HirPath::new(this.db, location.clone(), vec![identifier]);

                // Creates a new [`Expr`] with the [`Definition`] as the callee.
                Expr::Path(this.scope.insert_free_variable(this.db, path))
            }
            UniverseExpr(_) => Expr::Type(Type::Universe, location),
        })
    }
}

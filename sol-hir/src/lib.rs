#![feature(fn_traits)]
#![feature(unboxed_closures)]
#![feature(trait_upcasting)]

use lowering::HirLowering;
use package::HasManifest;
use salsa::DbWithJar;
use sol_diagnostic::DiagnosticDb;
use sol_syntax::ParseDb;

use crate::primitives::PrimitiveProvider;

extern crate salsa_2022 as salsa;

#[salsa::jar(db = HirDb)]
pub struct Jar(
    package::Package,
    package::Package_all_files,
    solver::Definition,
    solver::DefinitionId,
    solver::Definition_location,
    solver::Definition_to_string,
    solver::Reference,
    solver::Reference_is_type_level,
    solver::Reference_name,
    solver::unresolved,
    solver::find_constructor,
    solver::find_function,
    solver::find_type,
    solver::find_trait,
    solver::query_module,
    solver::references,
    source::HirSource,
    source::HirPath,
    source::HirLocation,
    source::VirtualPath,
    source::new_path,
    source::HirPath_to_string,
    source::Identifier,
    source::top_level::Clause,
    source::top_level::Signature,
    source::top_level::UsingTopLevel,
    source::top_level::Inductive,
    source::top_level::Constructor,
    source::top_level::BindingGroup,
    source::top_level::CommandTopLevel,
    source::declaration::Parameter,
    source::declaration::DocString,
    source::declaration::Attribute,
    completions::completions,
    reparse::reparse_hir_path,
    primitives::new_type_rep,
    primitives::primitive_type_rep,
    primitives::primitive_type_definition,
);

/// The database that stores all the information about the source code. It is
/// implemented using the `salsa` crate.
///
/// The `salsa` crate is a crate that provides an incremental and parallel
/// recomputation library. It is used to implement the incremental and parallel
/// compilation of Sol.
pub trait HirDb:
    PrimitiveProvider + HirLowering + HasManifest + ParseDb + DiagnosticDb + DbWithJar<Jar>
{
}

impl<DB: HasManifest + HirLowering + PrimitiveProvider> HirDb for DB where
    DB: ?Sized + ParseDb + DiagnosticDb + salsa::DbWithJar<Jar>
{
}

pub mod completions;
pub mod debug;
pub mod errors;
pub mod fmt;
pub mod lowering;
pub mod package;
pub mod primitives;
pub mod reference;
pub mod reparse;
pub mod scope;
pub mod solver;
pub mod source;
pub mod walking;

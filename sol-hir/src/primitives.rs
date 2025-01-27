use std::sync::Arc;

use dashmap::DashMap;
use fxhash::FxBuildHasher;

use crate::{
    solver::{Definition, DefinitionId, DefinitionKind},
    source::{
        expr::{Expr, Type},
        type_rep::TypeRep,
        HirPath, Location,
    },
};

/// The primitive map, that stores all the primitives that are available in the
/// language.
pub type PrimitiveMap = im::HashMap<String, Definition, FxBuildHasher>;

/// The primitive bag, that stores all the primitive maps.
#[derive(Default, Clone)]
pub struct PrimitiveBag {
    type_representations: DashMap<String, Definition>,
    type_definitions: DashMap<Definition, TypeRep>,
}

pub trait PrimitiveProvider {
    /// Gets a primitive map for the given definition kind.
    ///
    /// It does it lazily
    fn primitives(&self) -> Arc<PrimitiveBag>;
}

/// Defines the [`initialize_primitive_bag`] query.
///
/// Initializes the primitive bag, with the default builtins.
pub fn initialize_primitive_bag(db: &dyn crate::HirDb) {
    /// Overrides the [`self::new_type_rep`] query, to make it simplier to
    /// use.
    pub fn new_type_rep(db: &dyn crate::HirDb, name: &str, refr: Type) {
        self::new_type_rep(db, HirPath::create(db, name), TypeRep {
            expr: Expr::Type(refr, Location::CallSite).into(),
        });
    }

    // Defines string types
    new_type_rep(db, "String", Type::String);
    new_type_rep(db, "Unit", Type::Unit);

    // Defines bool types
    new_type_rep(db, "Bool", Type::Bool);

    // Defines integer types
    new_type_rep(db, "Int", Type::Int32);
    new_type_rep(db, "Int8", Type::Int8);
    new_type_rep(db, "UInt8", Type::UInt8);
    new_type_rep(db, "Int16", Type::Int16);
    new_type_rep(db, "UInt16", Type::UInt16);
    new_type_rep(db, "Int32", Type::Int32);
    new_type_rep(db, "UInt32", Type::UInt32);
    new_type_rep(db, "Int64", Type::Int64);
    new_type_rep(db, "UInt64", Type::UInt64);
    new_type_rep(db, "Nat", Type::Nat);
}

/// Defines the [`new_type_rep`] query.
///
/// Creates a new type representation primitive in the current context.
///
/// # Parameters
/// - `db`: The database
/// - `path`: The path of the type representation
#[salsa::tracked]
pub fn new_type_rep(db: &dyn crate::HirDb, path: HirPath, repr: TypeRep) {
    // Get the database for primitives
    let primitives = db.primitives();

    // Create a definition
    let text = path.to_string(db);
    let definition = *primitives
        .type_representations
        .entry(text.clone().unwrap())
        .or_insert_with(move || {
            let id = DefinitionId::new(db, Location::CallSite, text.clone());
            Definition::new(db, id, DefinitionKind::Type, path)
        });

    // Define the type if it is not defined
    if !primitives.type_definitions.contains_key(&definition) {
        primitives.type_definitions.insert(definition, repr);
    }
}

/// Defines the [`primitive_type_rep`] query.
///
/// Gets the type representation of a primitive type.
#[salsa::tracked]
pub fn primitive_type_rep(db: &dyn crate::HirDb, path: HirPath) -> Option<TypeRep> {
    let primitives = db.primitives();
    let definition = primitives.type_representations.get(&path.to_string(db)?)?;
    primitives
        .type_definitions
        .get(&definition)
        .map(|value| value.clone())
}

/// Defines the [`primitive_type_definition`] query.
///
/// Gets the type definition of a primitive type.
#[salsa::tracked]
pub fn primitive_type_definition(db: &dyn crate::HirDb, path: HirPath) -> Option<Definition> {
    let primitives = db.primitives();
    let definition = primitives.type_representations.get(&path.to_string(db)?)?;

    Some(*definition)
}

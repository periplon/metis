//! Schema-Driven Form Generation for Mock Strategies
//!
//! Generates dynamic forms from JSON Schema definitions for:
//! - Static strategy: Enter concrete values per schema property
//! - Faker strategy: Select faker generators + constraints per property

pub mod types;
pub mod resolver;
pub mod generator;
pub mod fields;
pub mod faker_selector;
pub mod array_field;
pub mod variant_selector;

pub use types::*;
pub use generator::*;

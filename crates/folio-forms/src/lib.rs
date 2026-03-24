//! PDF interactive forms (AcroForms).
//!
//! Provides access to form fields: reading/writing values, iterating
//! the field tree, and querying field properties.

mod field;
mod form;

pub use field::{Field, FieldFlags, FieldType};
pub use form::AcroForm;

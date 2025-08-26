//! This module contains a Hugr extension for _jeff_ types and operations that don't have a direct
//! mapping to Hugr-native types and operations.

mod jeff_op;
mod jeff_type;

use hugr::types::{Term, TypeBound};
pub use jeff_op::{JeffOp, JeffOpDef};
pub use jeff_type::{
    ConstIntReg, FLOATREG_TYPE_ID, INTREG_TYPE_ID, QUREG_TYPE_ID, floatreg_custom_type,
    floatreg_type, intreg_custom_type, intreg_parametric_custom_type, intreg_parametric_type,
    intreg_type, qureg_custom_type, qureg_type,
};

use hugr::Extension;
use hugr::extension::simple_op::MakeOpDef;
use hugr::extension::{ExtensionId, Version};
use hugr::hugr::IdentList;
use lazy_static::lazy_static;
use std::sync::Arc;

/// The ID of the hugr-jeff extension.
pub const JEFF_EXTENSION_ID: ExtensionId = IdentList::new_unchecked("jeff");

/// Current version of the TKET 1 extension
pub const JEFF_EXTENSION_VERSION: Version = Version::new(0, 1, 0);

lazy_static! {
    /// The extension definition for TKET ops and types.
    pub static ref JEFF_EXTENSION: Arc<Extension> = {
        Extension::new_arc(JEFF_EXTENSION_ID, JEFF_EXTENSION_VERSION, |extension, extension_ref| {
            JeffOpDef::load_all_ops(extension, extension_ref).expect("add_fail");

            extension
            .add_type(
                QUREG_TYPE_ID,
                vec![],
                "jeff quantum register".to_owned(),
                TypeBound::Linear.into(),
                extension_ref,
            )
            .unwrap();

            extension
            .add_type(
                INTREG_TYPE_ID,
                vec![Term::max_nat_type()],
                "jeff integer register".to_owned(),
                TypeBound::Copyable.into(),
                extension_ref,
            ).unwrap();

            extension
            .add_type(
                FLOATREG_TYPE_ID,
                vec![Term::max_nat_type()],
                "jeff floating-point register".to_owned(),
                TypeBound::Copyable.into(),
                extension_ref,
            ).unwrap();
        })
    };
}

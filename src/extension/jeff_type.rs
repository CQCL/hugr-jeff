//! Basic floating-point types

use std::sync::{Arc, Weak};

use hugr::Extension;
use hugr::ops::constant::{CustomConst, TryHash, ValueName};
use hugr::types::{CustomType, Term, Type, TypeArg, TypeBound, TypeName};
use itertools::Itertools;
use jeff::types::FloatPrecision;

use super::{JEFF_EXTENSION, JEFF_EXTENSION_ID};

/// Identifier for the _jeff_ quantum register type
pub const QUREG_TYPE_ID: TypeName = TypeName::new_inline("qureg");

/// Identifier for the _jeff_ integer register type
///
/// Parameterized by the bitwidth of the integers in the array.
pub const INTREG_TYPE_ID: TypeName = TypeName::new_inline("intArray");

/// Identifier for the _jeff_ floating-point register type
pub const FLOATREG_TYPE_ID: TypeName = TypeName::new_inline("floatArray");

/// _jeff_ quantum register type (as [CustomType])
pub fn qureg_custom_type(extension_ref: &Weak<Extension>) -> CustomType {
    CustomType::new(
        QUREG_TYPE_ID,
        vec![],
        JEFF_EXTENSION_ID,
        TypeBound::Linear,
        extension_ref,
    )
}

/// _jeff_ quantum register type (as [Type])
pub fn qureg_type() -> Type {
    qureg_custom_type(&Arc::downgrade(&JEFF_EXTENSION)).into()
}

/// _jeff_ integer register type (as [CustomType])
///
/// The integer bitwidth is passed as an argument.
pub fn intreg_custom_type(extension_ref: &Weak<Extension>, bitwidth: u8) -> CustomType {
    CustomType::new(
        INTREG_TYPE_ID,
        vec![TypeArg::BoundedNat(bitwidth as u64)],
        JEFF_EXTENSION_ID,
        TypeBound::Copyable,
        extension_ref,
    )
}

/// _jeff_ integer register type (as [CustomType])
///
/// The integer bitwidth is passed as an argument.
pub fn intreg_parametric_custom_type(
    extension_ref: &Weak<Extension>,
    bitwidth_arg: TypeArg,
) -> CustomType {
    CustomType::new(
        INTREG_TYPE_ID,
        vec![bitwidth_arg],
        JEFF_EXTENSION_ID,
        TypeBound::Copyable,
        extension_ref,
    )
}

/// _jeff_ integer register type (as [Type])
///
/// The integer bitwidth is passed as a generic argument.
pub fn intreg_type(bitwidth: u8) -> Type {
    intreg_custom_type(&Arc::downgrade(&JEFF_EXTENSION), bitwidth).into()
}

/// _jeff_ integer register type (as [Type])
///
/// The integer bitwidth is passed as a generic argument.
pub fn intreg_parametric_type(bitwidth_arg: TypeArg) -> Type {
    intreg_parametric_custom_type(&Arc::downgrade(&JEFF_EXTENSION), bitwidth_arg).into()
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// A constant array value.
pub struct ConstIntReg {
    /// The bitwidth of the integers in the array.
    bits: u8,
    /// The values, stored as u64s.
    values: Vec<u64>,
}

impl ConstIntReg {
    /// Name of the constructor for creating constant 64bit floats.
    pub const CTR_NAME: &'static str = "jeff.const-intreg";

    /// Create a new [`ConstIntReg`]
    pub fn new(values: impl IntoIterator<Item = u64>, bits: u8) -> Self {
        Self {
            bits,
            values: values.into_iter().collect_vec(),
        }
    }

    /// Returns the value of the constant
    pub fn values(&self) -> &[u64] {
        &self.values
    }

    /// Returns the bitwidth of the constant
    pub fn bits(&self) -> u8 {
        self.bits
    }
}

impl TryHash for ConstIntReg {}

#[typetag::serde]
impl CustomConst for ConstIntReg {
    fn name(&self) -> ValueName {
        format!("[{}]", self.values.iter().join(", ")).into()
    }

    fn get_type(&self) -> Type {
        intreg_type(self.bits)
    }

    fn equal_consts(&self, other: &dyn CustomConst) -> bool {
        hugr::ops::constant::downcast_equal_consts(self, other)
    }
}

/// _jeff_ floating-point register type (as [CustomType])
///
/// The floating-point precision is either 32 or 64 bits.
pub fn floatreg_custom_type(
    extension_ref: &Weak<Extension>,
    precision: FloatPrecision,
) -> CustomType {
    let precision = match precision {
        FloatPrecision::Float32 => 32,
        FloatPrecision::Float64 => 64,
    };
    CustomType::new(
        FLOATREG_TYPE_ID,
        vec![Term::BoundedNat(precision)],
        JEFF_EXTENSION_ID,
        TypeBound::Copyable,
        extension_ref,
    )
}

/// _jeff_ floating-point register type (as [Type])
///
/// The floating-point precision is either 32 or 64 bits.
pub fn floatreg_type(precision: FloatPrecision) -> Type {
    floatreg_custom_type(&Arc::downgrade(&JEFF_EXTENSION), precision).into()
}

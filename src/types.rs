//! Translation between _jeff_ and HUGR types

use hugr::extension::prelude::qb_t;
use hugr::extension::{ExtensionId, prelude as hugr_prelude};
use hugr::std_extensions::arithmetic::{
    float_types as hugr_float_types, int_types as hugr_int_types,
};
use hugr::types::{Signature as HugrSignature, Term, Type as HugrType, TypeArg, TypeName};
use itertools::Itertools;
use jeff::types::{FloatPrecision, Type as JeffType};

use crate::HugrToJeffError;
use crate::extension::{
    FLOATREG_TYPE_ID, INTREG_TYPE_ID, JEFF_EXTENSION_ID, QUREG_TYPE_ID, floatreg_type, intreg_type,
    qureg_type,
};

/// Translate a _jeff_ type to a HUGR type.
///
/// Integer widths are extended to the next power of 2, as HUGR only supports
/// integer widths of the form 2^n.
///
/// Float types are translated to 64-bit floats, regardless of the precision
/// specified in the _jeff_ type.
///
/// Qubit arrays are translated into `qureg` types from the _jeff_ extension.
pub fn jeff_to_hugr(jeff_type: JeffType) -> HugrType {
    match jeff_type {
        JeffType::Qubit => qb_t(),
        JeffType::Int { bits } => {
            if bits == 1 {
                return hugr_prelude::bool_t();
            }
            let log_width = jeff_int_width_to_hugr_arg(bits);
            hugr_int_types::int_type(log_width)
        }
        JeffType::Float { .. } => hugr_float_types::float64_type(),
        // List types
        JeffType::QubitRegister => qureg_type(),
        JeffType::IntArray { bits } => intreg_type(bits),
        JeffType::FloatArray { precision } => floatreg_type(precision),
    }
}

/// Translate a _jeff_ signature into a HUGR signature.
pub fn jeff_signature_to_hugr(
    inputs: impl IntoIterator<Item = JeffType>,
    outputs: impl IntoIterator<Item = JeffType>,
) -> HugrSignature {
    let inputs = inputs.into_iter().map(jeff_to_hugr).collect_vec();
    let outputs = outputs.into_iter().map(jeff_to_hugr).collect_vec();
    HugrSignature::new(inputs, outputs)
}

/// Translate a HUGR type to a _jeff_ type.
///
/// # Errors
///
/// - [`HugrToJeffError::UnsupportedType`] if the HUGR type is not supported by _jeff_.
pub fn hugr_to_jeff(hugr_type: &HugrType) -> Result<JeffType, HugrToJeffError> {
    // Error to return when the HUGR type is unsupported
    let unsupported_err = || HugrToJeffError::UnsupportedType {
        hugr_type: hugr_type.to_string(),
    };

    // Boolean types are the only ones not represented by custom types.
    if &hugr_prelude::bool_t() == hugr_type {
        return Ok(JeffType::Int { bits: 1 });
    }

    // Otherwise, we can assume the type is a custom type.
    let hugr::types::TypeEnum::Extension(custom) = hugr_type.as_type_enum() else {
        return Err(unsupported_err());
    };
    let extension_name: &ExtensionId = custom.extension();
    let type_name: &TypeName = custom.name();

    if extension_name == &hugr_prelude::PRELUDE_ID && type_name == "qubit" {
        // TODO: Hugr doesn't export the qubit type name to match against, so we have to hardcode it.
        Ok(JeffType::Qubit)
    } else if extension_name == &hugr_int_types::EXTENSION_ID
        && type_name == &hugr_int_types::INT_TYPE_ID
    {
        let log_width = custom.args()[0].as_nat().expect("Hugr should be valid");
        let bits = 1 << log_width as u8;
        Ok(JeffType::Int { bits })
    } else if extension_name == &hugr_float_types::EXTENSION_ID
        && type_name == &hugr_float_types::FLOAT_TYPE_ID
    {
        Ok(JeffType::Float {
            precision: FloatPrecision::Float64,
        })
    } else if extension_name == &JEFF_EXTENSION_ID {
        if type_name == &QUREG_TYPE_ID {
            Ok(JeffType::QubitRegister)
        } else if type_name == &INTREG_TYPE_ID {
            let bitwidth = custom.args()[0].as_nat().expect("Hugr should be valid") as u8;
            Ok(JeffType::IntArray { bits: bitwidth })
        } else if type_name == &FLOATREG_TYPE_ID {
            let precision = custom.args()[0].as_nat().expect("Hugr should be valid");
            match precision {
                32 => Ok(JeffType::FloatArray {
                    precision: FloatPrecision::Float32,
                }),
                64 => Ok(JeffType::FloatArray {
                    precision: FloatPrecision::Float64,
                }),
                _ => Err(unsupported_err()),
            }
        } else {
            Err(unsupported_err())
        }
    } else {
        Err(unsupported_err())
    }
}

/// Translate a HUGR signature into a _jeff_ signature.
///
/// # Errors
///
/// - [`HugrToJeffError::UnsupportedType`] if a HUGR type in the signature is not supported by _jeff_.
pub fn hugr_signature_to_jeff(
    hugr_signature: &HugrSignature,
) -> Result<(Vec<JeffType>, Vec<JeffType>), HugrToJeffError> {
    let (inputs, outputs) = hugr_signature.io();
    let inputs = inputs
        .iter()
        .map(hugr_to_jeff)
        .collect::<Result<Vec<_>, _>>()?;
    let outputs = outputs
        .iter()
        .map(hugr_to_jeff)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((inputs, outputs))
}

/// Rounds a _jeff_ integer width to the next power of 2 and returns it as a hugr
/// type argument.
///
/// Hugr only supports int widths of the form 2^n, so we extend the int width to
/// the next power of 2.
fn jeff_int_width_to_hugr_arg(bits: u8) -> TypeArg {
    let log_width = bits.next_power_of_two().trailing_zeros();
    Term::BoundedNat(log_width as u64)
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    /// Test the _jeff_->Hugr->_jeff_ type roundtrip.
    ///
    /// For some types the roundtrip is not perfect, as Hugr does not support
    /// f32 nor integer widths that are not powers of 2.
    #[rstest]
    #[case::qubit(JeffType::Qubit, JeffType::Qubit)]
    #[case::qureg(JeffType::QubitRegister, JeffType::QubitRegister)]
    #[case::bit(JeffType::Int { bits: 1 }, JeffType::Int { bits: 1 })]
    #[case::int8(JeffType::Int { bits: 8 }, JeffType::Int { bits: 8 })]
    #[case::int7(JeffType::Int { bits: 7 }, JeffType::Int { bits: 8 })]
    #[case::f32(JeffType::Float { precision: FloatPrecision::Float32 }, JeffType::Float { precision: FloatPrecision::Float64 })]
    #[case::f64(JeffType::Float { precision: FloatPrecision::Float64 }, JeffType::Float { precision: FloatPrecision::Float64 })]
    #[case::bit_arr(JeffType::IntArray { bits: 1 }, JeffType::IntArray { bits: 1 })]
    #[case::int8_arr(JeffType::IntArray { bits: 8 }, JeffType::IntArray { bits: 8 })]
    #[case::int7_arr(JeffType::IntArray { bits: 7 }, JeffType::IntArray { bits: 7 })]
    #[case::f32_arr(JeffType::FloatArray { precision: FloatPrecision::Float32 }, JeffType::FloatArray { precision: FloatPrecision::Float32 })]
    #[case::f64_arr(JeffType::FloatArray { precision: FloatPrecision::Float64 }, JeffType::FloatArray { precision: FloatPrecision::Float64 })]
    fn jeff_type_roundtrip(#[case] initial: JeffType, #[case] expected: JeffType) {
        let hugr_type = jeff_to_hugr(initial);
        let roundtripped = hugr_to_jeff(&hugr_type).unwrap();
        assert_eq!(roundtripped, expected);
    }

    #[rstest]
    fn jeff_signature_roundtrip() {
        let inputs = vec![
            JeffType::Qubit,
            JeffType::Int { bits: 8 },
            JeffType::Float {
                precision: FloatPrecision::Float64,
            },
        ];
        let outputs = vec![
            JeffType::QubitRegister,
            JeffType::IntArray { bits: 8 },
            JeffType::FloatArray {
                precision: FloatPrecision::Float64,
            },
        ];
        let hugr_signature =
            jeff_signature_to_hugr(inputs.iter().copied(), outputs.iter().copied());
        let (roundtripped_inputs, roundtripped_outputs) =
            hugr_signature_to_jeff(&hugr_signature).unwrap();
        assert_eq!(roundtripped_inputs, inputs);
        assert_eq!(roundtripped_outputs, outputs);
    }
}

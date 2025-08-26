//! Quantum gate in the _jeff_ hugr extension.

use std::num::NonZero;
use std::sync::{Arc, Weak};

use hugr::Extension;
use hugr::extension::prelude::qb_t;
use hugr::extension::simple_op::{
    HasConcrete, HasDef, MakeExtensionOp, MakeOpDef, MakeRegisteredOp, OpLoadError, try_from_name,
};
use hugr::extension::{CustomSignatureFunc, ExtensionId, OpDef, SignatureError, SignatureFunc};
use hugr::ops::ExtensionOp;
use hugr::std_extensions::arithmetic::float_types::float64_type;
use hugr::types::{PolyFuncType, PolyFuncTypeRV, Signature, Term};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};

use super::{
    JEFF_EXTENSION, JEFF_EXTENSION_ID, intreg_parametric_custom_type, intreg_type,
    qureg_custom_type,
};

#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumIter,
    EnumString,
)]
#[non_exhaustive]
/// _jeff_ operations with no direct equivalent in `tket2.quantum`.
pub enum JeffOpDef {
    /// Quantum gate with an arbitrary number of qubits and parameters.
    ///
    /// Operation arguments:
    /// - The operation name (as a string)
    /// - The number of qubits
    /// - The number of parameters (floating point numbers)
    /// - The number of control qubits
    /// - Whether the gate is adjoint
    /// - A power value (how many times to apply it in sequence)
    QGate,
    /// Allocate a new qubit register with a size parameter.
    QuregAlloc,
    /// Free a qubit register.
    QuregFree,
    /// Extract a qubit at the given index from a register.
    QuregExtractIndex,
    /// Insert a qubit at the given index into a register.
    QuregInsertIndex,
    /// Create a register of qubits from a variable number of input qubits.
    QuregCreate,
    /// Extract a slice of qubits from a register.
    QuregExtractSlice,
    /// Insert a slice of qubits into a register at a given index.
    ///
    /// Shifts the qubits in the register to the right.
    QuregInsertSlice,
    /// Split a register of qubits into two registers.
    QuregSplit,
    /// Join two registers of qubits into a single register.
    QuregJoin,
    /// Returns the length of a qubit register.
    QuregLength,

    /// Allocate a new IntArray with the given length.
    IntArrayCreate,
    /// Return the length of an IntArray.
    IntArrayLength,
    /// Get the value at a given index in an IntArray.
    IntArrayGet,
    /// Set the value at a given index in an IntArray.
    IntArraySet,
    /// Create a zeroed integer array of a given bitwidth with dynamic length.
    IntArrayZero,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
/// A concrete _jeff_ operations with no direct equivalent in `tket2.quantum`.
pub enum JeffOp {
    /// Quantum gate with an arbitrary number of qubits and parameters.
    ///
    /// It also stores flags for controlling, taking the adjoint, and applying a power of the gate.
    QGate {
        /// The name of the gate.
        name: String,
        /// The number of qubits.
        qubits: usize,
        /// Number of floating point parameter inputs after the qubit inputs.
        params: usize,
        /// The number of control qubits.
        control: usize,
        /// Whether the gate is adjoint.
        adjoint: bool,
        /// How many times in a row to apply the gate.
        power: usize,
    },

    /// Allocate a new qubit register with a size parameter.
    QuregAlloc,
    /// Free a qubit register.
    QuregFree,
    /// Extract a qubit at the given index from a register.
    QuregExtractIndex,
    /// Insert a qubit at the given index into a register.
    QuregInsertIndex,
    /// Create a register of qubits from a variable number of input qubits.
    QuregCreate {
        /// The number of qubits in the register.
        qubits: usize,
    },
    /// Extract a slice of qubits from a register.
    QuregExtractSlice,
    /// Insert a slice of qubits into a register at a given index.
    ///
    /// Shifts the qubits in the register to the right.
    QuregInsertSlice,
    /// Split a register of qubits into two registers.
    QuregSplit,
    /// Join two registers of qubits into a single register.
    QuregJoin,
    /// Returns the length of a qubit register.
    QuregLength,

    /// Allocate a new IntArray with the given length.
    IntArrayCreate {
        /// The bitwidth of the integers in the array.
        bits: u8,
        /// The number of input integers.
        inputs: usize,
    },
    /// Return the length of an IntArray.
    IntArrayLength {
        /// The bitwidth of the integers in the array.
        bits: u8,
    },
    /// Get the value at a given index in an IntArray.
    IntArrayGet {
        /// The bitwidth of the integers in the array.
        bits: u8,
    },
    /// Set the value at a given index in an IntArray.
    IntArraySet {
        /// The bitwidth of the integers in the array.
        bits: u8,
    },
    /// Create a zeroed integer array of a given bitwidth with dynamic length.
    IntArrayZero {
        /// The bitwidth of the integers in the array.
        bits: u8,
    },
}

impl JeffOp {
    /// Returns an [`JeffOp::QGate`] for a named quantum gate.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the gate.
    /// * `n` - The number of qubits.
    /// * `params` - The number of floating point parameters.
    /// * `control` - The number of control qubits (not included in `n`).
    /// * `adjoint` - Whether the gate is adjoint.
    /// * `power` - How many times to apply the gate in a row.
    pub fn quantum_gate(
        name: String,
        n: usize,
        params: usize,
        control: usize,
        adjoint: bool,
        power: usize,
    ) -> JeffOp {
        JeffOp::QGate {
            name,
            qubits: n,
            params,
            control,
            adjoint,
            power,
        }
    }

    /// Returns a [`JeffOp::QGate`] for a _jeff_ quantum gate.
    pub fn jeff_gate_op(name: impl ToString, jeff_op: &jeff::reader::optype::GateOp<'_>) -> Self {
        let base_qubits = jeff_op.num_qubits() - jeff_op.control_qubits as usize;
        Self::quantum_gate(
            name.to_string(),
            base_qubits,
            jeff_op.num_params(),
            jeff_op.control_qubits as usize,
            jeff_op.adjoint,
            jeff_op.power as usize,
        )
    }

    /// Returns the non-instantiated [`JeffOpDef`] for this operation.
    pub fn opdef(&self) -> JeffOpDef {
        match self {
            JeffOp::QGate { .. } => JeffOpDef::QGate,
            JeffOp::QuregAlloc => JeffOpDef::QuregAlloc,
            JeffOp::QuregFree => JeffOpDef::QuregFree,
            JeffOp::QuregExtractIndex => JeffOpDef::QuregExtractIndex,
            JeffOp::QuregInsertIndex => JeffOpDef::QuregInsertIndex,
            JeffOp::QuregCreate { .. } => JeffOpDef::QuregCreate,
            JeffOp::QuregExtractSlice => JeffOpDef::QuregExtractSlice,
            JeffOp::QuregInsertSlice => JeffOpDef::QuregInsertSlice,
            JeffOp::QuregSplit => JeffOpDef::QuregSplit,
            JeffOp::QuregJoin => JeffOpDef::QuregJoin,
            JeffOp::QuregLength => JeffOpDef::QuregLength,
            JeffOp::IntArrayCreate { .. } => JeffOpDef::IntArrayCreate,
            JeffOp::IntArrayLength { .. } => JeffOpDef::IntArrayLength,
            JeffOp::IntArrayGet { .. } => JeffOpDef::IntArrayGet,
            JeffOp::IntArraySet { .. } => JeffOpDef::IntArraySet,
            JeffOp::IntArrayZero { .. } => JeffOpDef::IntArrayZero,
        }
    }

    /// Wraps the operation in an [`ExtensionOp`]
    pub fn into_extension_op(self) -> ExtensionOp {
        <Self as MakeRegisteredOp>::to_extension_op(self)
            .expect("Failed to convert to extension op.")
    }
}

impl MakeOpDef for JeffOpDef {
    fn init_signature(&self, extension_ref: &std::sync::Weak<hugr::Extension>) -> SignatureFunc {
        let qreg_t = || qureg_custom_type(extension_ref).into();
        let int32_t = || crate::types::jeff_to_hugr(jeff::types::Type::Int { bits: 32 });

        let intarr_type = |var_idx| {
            intreg_parametric_custom_type(
                extension_ref,
                Term::new_var_use(var_idx, Term::max_nat_type()),
            )
            .into()
        };

        match self {
            JeffOpDef::QGate => JeffGateNSignature.into(),
            // Registers
            JeffOpDef::QuregAlloc => {
                PolyFuncType::new(vec![], Signature::new(vec![int32_t()], vec![qreg_t()])).into()
            }
            JeffOpDef::QuregFree => {
                PolyFuncType::new(vec![], Signature::new(vec![qreg_t()], vec![])).into()
            }
            JeffOpDef::QuregExtractIndex => PolyFuncType::new(
                vec![],
                Signature::new(vec![qreg_t(), int32_t()], vec![qreg_t(), qb_t()]),
            )
            .into(),
            JeffOpDef::QuregInsertIndex => PolyFuncType::new(
                vec![],
                Signature::new(vec![qreg_t(), qb_t(), int32_t()], vec![qreg_t()]),
            )
            .into(),
            JeffOpDef::QuregCreate => JeffQuregCreateSignature.into(),
            JeffOpDef::QuregExtractSlice => PolyFuncType::new(
                vec![],
                Signature::new(
                    vec![qreg_t(), int32_t(), int32_t()],
                    vec![qreg_t(), qreg_t()],
                ),
            )
            .into(),
            JeffOpDef::QuregInsertSlice => PolyFuncType::new(
                vec![],
                Signature::new(vec![qreg_t(), qreg_t(), int32_t()], vec![qreg_t()]),
            )
            .into(),
            JeffOpDef::QuregSplit => PolyFuncType::new(
                vec![],
                Signature::new(vec![qreg_t(), int32_t()], vec![qreg_t(), qreg_t()]),
            )
            .into(),
            JeffOpDef::QuregJoin => PolyFuncType::new(
                vec![],
                Signature::new(vec![qreg_t(), qreg_t()], vec![qreg_t()]),
            )
            .into(),
            JeffOpDef::QuregLength => {
                PolyFuncType::new(vec![], Signature::new(vec![qreg_t()], vec![int32_t()])).into()
            }
            // IntArrays
            JeffOpDef::IntArrayCreate => JeffIntArrayCreateSignature.into(),
            JeffOpDef::IntArrayLength => PolyFuncType::new(
                vec![Term::max_nat_type()],
                Signature::new(vec![intarr_type(0)], vec![int32_t()]),
            )
            .into(),
            JeffOpDef::IntArrayGet => JeffIntArrayGetSignature.into(),
            JeffOpDef::IntArraySet => JeffIntArraySetSignature.into(),
            JeffOpDef::IntArrayZero => PolyFuncType::new(
                vec![Term::max_nat_type()],
                Signature::new(vec![int32_t()], vec![intarr_type(0)]),
            )
            .into(),
        }
    }

    fn opdef_id(&self) -> hugr::ops::OpName {
        match self {
            JeffOpDef::QGate => "QGateN".into(),
            JeffOpDef::QuregAlloc => "QuregAlloc".into(),
            JeffOpDef::QuregFree => "QuregFree".into(),
            JeffOpDef::QuregExtractIndex => "QuregExtractIndex".into(),
            JeffOpDef::QuregInsertIndex => "QuregInsertIndex".into(),
            JeffOpDef::QuregCreate => "QuregCreate".into(),
            JeffOpDef::QuregExtractSlice => "QuregExtractSlice".into(),
            JeffOpDef::QuregInsertSlice => "QuregInsertSlice".into(),
            JeffOpDef::QuregSplit => "QuregSplit".into(),
            JeffOpDef::QuregJoin => "QuregJoin".into(),
            JeffOpDef::QuregLength => "QuregLength".into(),
            JeffOpDef::IntArrayCreate => "IntArrayCreate".into(),
            JeffOpDef::IntArrayLength => "IntArrayLength".into(),
            JeffOpDef::IntArrayGet => "IntArrayGet".into(),
            JeffOpDef::IntArraySet => "IntArraySet".into(),
            JeffOpDef::IntArrayZero => "IntArrayZero".into(),
        }
    }

    fn description(&self) -> String {
        match self {
            JeffOpDef::QGate => "A jeff n-qubit gate.".to_string(),
            JeffOpDef::QuregAlloc => "Allocate a new qubit register.".to_string(),
            JeffOpDef::QuregFree => "Free a qubit register.".to_string(),
            JeffOpDef::QuregExtractIndex => "Extract a qubit from a register.".to_string(),
            JeffOpDef::QuregInsertIndex => "Insert a qubit into a register.".to_string(),
            JeffOpDef::QuregCreate => "Create a register of qubits.".to_string(),
            JeffOpDef::QuregExtractSlice => {
                "Extract a slice of qubits from a register.".to_string()
            }
            JeffOpDef::QuregInsertSlice => "Insert a slice of qubits into a register.".to_string(),
            JeffOpDef::QuregSplit => "Split a register of qubits.".to_string(),
            JeffOpDef::QuregJoin => "Join two registers of qubits.".to_string(),
            JeffOpDef::QuregLength => "Get the length of a qubit register.".to_string(),
            JeffOpDef::IntArrayCreate => "Create a new IntArray.".to_string(),
            JeffOpDef::IntArrayLength => "Get the length of an IntArray.".to_string(),
            JeffOpDef::IntArrayGet => "Get the value at an index in an IntArray.".to_string(),
            JeffOpDef::IntArraySet => "Set the value at an index in an IntArray.".to_string(),
            JeffOpDef::IntArrayZero => "Create a zeroed IntArray.".to_string(),
        }
    }

    fn from_def(op_def: &OpDef) -> Result<Self, hugr::extension::simple_op::OpLoadError> {
        try_from_name(op_def.name(), op_def.extension_id())
    }

    fn extension(&self) -> ExtensionId {
        JEFF_EXTENSION_ID.to_owned()
    }

    fn extension_ref(&self) -> Weak<hugr::Extension> {
        Arc::downgrade(&JEFF_EXTENSION)
    }
}

/// A signature computation function for [`JeffOp::QGateN`].
#[derive(Debug, Clone, Copy)]
pub struct JeffGateNSignature;

impl CustomSignatureFunc for JeffGateNSignature {
    fn compute_signature<'o, 'a: 'o>(
        &'a self,
        arg_values: &[Term],
        _def: &'o OpDef,
    ) -> Result<PolyFuncTypeRV, SignatureError> {
        let [
            Term::String(_name),
            Term::BoundedNat(num_qubits),
            Term::BoundedNat(num_params),
            Term::BoundedNat(num_controls),
            Term::BoundedNat(_adjoint),
            Term::BoundedNat(_power),
        ] = arg_values
        else {
            return Err(SignatureError::InvalidTypeArgs);
        };

        let qubits = itertools::repeat_n(qb_t(), *num_qubits as usize);
        let controls = itertools::repeat_n(qb_t(), *num_controls as usize);
        let params = itertools::repeat_n(float64_type(), *num_params as usize);

        let sig: PolyFuncType = Signature::new(
            qubits
                .clone()
                .chain(controls.clone())
                .chain(params)
                .collect_vec(),
            qubits.chain(controls).collect_vec(),
        )
        .into();
        Ok(sig.into())
    }

    fn static_params(&self) -> &[Term] {
        static PARAMS: [Term; 6] = [
            Term::StringType,
            Term::max_nat_type(),
            Term::max_nat_type(),
            Term::max_nat_type(),
            Term::bounded_nat_type(NonZero::new(2).unwrap()),
            Term::max_nat_type(),
        ];
        &PARAMS
    }
}

/// A signature computation function for [`JeffOp::QuregCreate`].
#[derive(Debug, Clone, Copy)]
pub struct JeffQuregCreateSignature;

impl CustomSignatureFunc for JeffQuregCreateSignature {
    fn compute_signature<'o, 'a: 'o>(
        &'a self,
        arg_values: &[Term],
        _def: &'o OpDef,
    ) -> Result<PolyFuncTypeRV, SignatureError> {
        let qubits = arg_values[0].as_nat().expect("JeffOp arg should be a nat") as usize;

        let inputs = vec![qb_t(); qubits];
        let outputs = vec![crate::types::jeff_to_hugr(jeff::types::Type::QubitRegister)];
        let sig: PolyFuncType = Signature::new(inputs, outputs).into();
        Ok(sig.into())
    }

    fn static_params(&self) -> &[Term] {
        static PARAMS: [Term; 1] = [Term::max_nat_type()];
        &PARAMS
    }
}

/// A signature computation function for [`JeffOp::IntArrayCreate`].
#[derive(Debug, Clone, Copy)]
pub struct JeffIntArrayCreateSignature;

impl CustomSignatureFunc for JeffIntArrayCreateSignature {
    fn compute_signature<'o, 'a: 'o>(
        &'a self,
        arg_values: &[Term],
        _def: &'o OpDef,
    ) -> Result<PolyFuncTypeRV, SignatureError> {
        let bits = arg_values[0].as_nat().expect("JeffOp arg should be a nat") as u8;
        let input_count = arg_values[1].as_nat().expect("JeffOp arg should be a nat") as usize;

        let int_type = crate::types::jeff_to_hugr(jeff::types::Type::Int { bits });
        let inputs = vec![int_type; input_count];
        let outputs = vec![intreg_type(bits)];
        let sig: PolyFuncType = Signature::new(inputs, outputs).into();
        Ok(sig.into())
    }

    fn static_params(&self) -> &[Term] {
        static PARAMS: [Term; 2] = [Term::max_nat_type(), Term::max_nat_type()];
        &PARAMS
    }
}

/// A signature computation function for [`JeffOp::IntArrayGet`].
#[derive(Debug, Clone, Copy)]
pub struct JeffIntArrayGetSignature;

impl CustomSignatureFunc for JeffIntArrayGetSignature {
    fn compute_signature<'o, 'a: 'o>(
        &'a self,
        arg_values: &[Term],
        _def: &'o OpDef,
    ) -> Result<PolyFuncTypeRV, SignatureError> {
        let bits = arg_values[0].as_nat().expect("JeffOp arg should be a nat") as u8;

        let int_type = crate::types::jeff_to_hugr(jeff::types::Type::Int { bits });
        let int32_t = crate::types::jeff_to_hugr(jeff::types::Type::Int { bits: 32 });

        let inputs = vec![intreg_type(bits), int32_t];
        let outputs = vec![int_type];
        let sig: PolyFuncType = Signature::new(inputs, outputs).into();
        Ok(sig.into())
    }

    fn static_params(&self) -> &[Term] {
        static PARAMS: [Term; 1] = [Term::max_nat_type()];
        &PARAMS
    }
}

/// A signature computation function for [`JeffOp::IntArraySet`].
#[derive(Debug, Clone, Copy)]
pub struct JeffIntArraySetSignature;

impl CustomSignatureFunc for JeffIntArraySetSignature {
    fn compute_signature<'o, 'a: 'o>(
        &'a self,
        arg_values: &[Term],
        _def: &'o OpDef,
    ) -> Result<PolyFuncTypeRV, SignatureError> {
        let bits = arg_values[0].as_nat().expect("JeffOp arg should be a nat") as u8;

        let int_type = crate::types::jeff_to_hugr(jeff::types::Type::Int { bits });
        let int32_t = crate::types::jeff_to_hugr(jeff::types::Type::Int { bits: 32 });

        let inputs = vec![intreg_type(bits), int32_t, int_type];
        let outputs = vec![intreg_type(bits)];
        let sig: PolyFuncType = Signature::new(inputs, outputs).into();
        Ok(sig.into())
    }

    fn static_params(&self) -> &[Term] {
        static PARAMS: [Term; 1] = [Term::max_nat_type()];
        &PARAMS
    }
}

impl MakeExtensionOp for JeffOp {
    fn from_extension_op(ext_op: &ExtensionOp) -> Result<Self, OpLoadError> {
        let def = JeffOpDef::from_def(ext_op.def())?;
        def.instantiate(ext_op.args())
    }

    fn type_args(&self) -> Vec<Term> {
        match self {
            JeffOp::QGate {
                name,
                qubits,
                params,
                control,
                adjoint,
                power,
            } => vec![
                Term::String(name.clone()),
                Term::BoundedNat(*qubits as u64),
                Term::BoundedNat(*params as u64),
                Term::BoundedNat(*control as u64),
                Term::BoundedNat(*adjoint as u64),
                Term::BoundedNat(*power as u64),
            ],
            JeffOp::QuregAlloc => vec![],
            JeffOp::QuregFree => vec![],
            JeffOp::QuregExtractIndex => vec![],
            JeffOp::QuregInsertIndex => vec![],
            JeffOp::QuregCreate { qubits } => vec![Term::BoundedNat(*qubits as u64)],
            JeffOp::QuregExtractSlice => vec![],
            JeffOp::QuregInsertSlice => vec![],
            JeffOp::QuregSplit => vec![],
            JeffOp::QuregJoin => vec![],
            JeffOp::QuregLength => vec![],
            JeffOp::IntArrayCreate { bits, inputs } => vec![
                Term::BoundedNat(*bits as u64),
                Term::BoundedNat(*inputs as u64),
            ],
            JeffOp::IntArrayLength { bits } => vec![Term::BoundedNat(*bits as u64)],
            JeffOp::IntArrayGet { bits } => vec![Term::BoundedNat(*bits as u64)],
            JeffOp::IntArraySet { bits } => vec![Term::BoundedNat(*bits as u64)],
            JeffOp::IntArrayZero { bits } => vec![Term::BoundedNat(*bits as u64)],
        }
    }

    fn op_id(&self) -> hugr::ops::OpName {
        self.opdef().opdef_id()
    }
}

impl MakeRegisteredOp for JeffOp {
    fn extension_id(&self) -> ExtensionId {
        JEFF_EXTENSION_ID.to_owned()
    }

    fn extension_ref(&self) -> Weak<Extension> {
        Arc::downgrade(&JEFF_EXTENSION)
    }
}

impl HasConcrete for JeffOpDef {
    type Concrete = JeffOp;

    fn instantiate(&self, type_args: &[Term]) -> Result<Self::Concrete, OpLoadError> {
        match (self, type_args) {
            (
                JeffOpDef::QGate,
                [
                    Term::String(name),
                    Term::BoundedNat(num_qubits),
                    Term::BoundedNat(num_params),
                    Term::BoundedNat(num_controls),
                    Term::BoundedNat(adjoint),
                    Term::BoundedNat(power),
                ],
            ) => Ok(JeffOp::quantum_gate(
                name.clone(),
                *num_qubits as usize,
                *num_params as usize,
                *num_controls as usize,
                *adjoint != 0,
                *power as usize,
            )),
            (JeffOpDef::QuregAlloc, []) => Ok(JeffOp::QuregAlloc),
            (JeffOpDef::QuregFree, []) => Ok(JeffOp::QuregFree),
            (JeffOpDef::QuregExtractIndex, []) => Ok(JeffOp::QuregExtractIndex),
            (JeffOpDef::QuregInsertIndex, []) => Ok(JeffOp::QuregInsertIndex),
            (JeffOpDef::QuregCreate, [Term::BoundedNat(num_qubits)]) => Ok(JeffOp::QuregCreate {
                qubits: *num_qubits as usize,
            }),
            (JeffOpDef::QuregExtractSlice, []) => Ok(JeffOp::QuregExtractSlice),
            (JeffOpDef::QuregInsertSlice, []) => Ok(JeffOp::QuregInsertSlice),
            (JeffOpDef::QuregSplit, []) => Ok(JeffOp::QuregSplit),
            (JeffOpDef::QuregJoin, []) => Ok(JeffOp::QuregJoin),
            (JeffOpDef::QuregLength, []) => Ok(JeffOp::QuregLength),
            (JeffOpDef::IntArrayCreate, [Term::BoundedNat(bits), Term::BoundedNat(inputs)]) => {
                Ok(JeffOp::IntArrayCreate {
                    bits: *bits as u8,
                    inputs: *inputs as usize,
                })
            }
            (JeffOpDef::IntArrayLength, [Term::BoundedNat(bits)]) => {
                Ok(JeffOp::IntArrayLength { bits: *bits as u8 })
            }
            (JeffOpDef::IntArrayGet, [Term::BoundedNat(bits)]) => {
                Ok(JeffOp::IntArrayGet { bits: *bits as u8 })
            }
            (JeffOpDef::IntArraySet, [Term::BoundedNat(bits)]) => {
                Ok(JeffOp::IntArraySet { bits: *bits as u8 })
            }
            (JeffOpDef::IntArrayZero, [Term::BoundedNat(bits)]) => {
                Ok(JeffOp::IntArrayZero { bits: *bits as u8 })
            }
            _ => Err(SignatureError::InvalidTypeArgs.into()),
        }
    }
}

impl HasDef for JeffOp {
    type Def = JeffOpDef;
}

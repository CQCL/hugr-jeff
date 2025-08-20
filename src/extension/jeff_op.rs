//! Quantum gate in the _jeff_ hugr extension.

use std::sync::{Arc, Weak};

use hugr::Extension;
use hugr::extension::prelude::qb_t;
use hugr::extension::simple_op::{
    HasConcrete, HasDef, MakeExtensionOp, MakeOpDef, MakeRegisteredOp, OpLoadError, try_from_name,
};
use hugr::extension::{CustomSignatureFunc, ExtensionId, OpDef, SignatureError, SignatureFunc};
use hugr::ops::ExtensionOp;
use hugr::std_extensions::arithmetic::float_types::float64_type;
use hugr::types::type_param::TypeParam;
use hugr::types::{PolyFuncType, PolyFuncTypeRV, Signature, TypeArg};
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
    /// One qubit gate.
    QGate1,
    /// Two qubit gate.
    QGate2,
    /// Quantum gate with an arbitrary number of qubits.
    QGateN,

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
    /// One qubit gate.
    QGate1 {
        /// The name of the gate.
        name: String,
    },
    /// Two qubit gate.
    QGate2 {
        /// The name of the gate.
        name: String,
    },
    /// Quantum gate with an arbitrary number of qubits.
    QGateN {
        /// The name of the gate.
        name: String,
        /// The number of qubits.
        qubits: usize,
        /// Number of floating point parameter inputs after the qubit inputs.
        params: usize,
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
    /// Returns a JeffOp for a named quantum gate with `n` qubits.
    pub fn q_gate(name: String, n: usize) -> JeffOp {
        Self::parametric_gate(name, n, 0)
    }

    /// Returns an Optype for a named quantum gate with `n` qubits
    /// and `params` f64 parameters.
    pub fn parametric_gate(name: String, n: usize, params: usize) -> JeffOp {
        match (n, params) {
            (1, 0) => JeffOp::QGate1 { name },
            (2, 0) => JeffOp::QGate2 { name },
            _ => JeffOp::QGateN {
                name,
                qubits: n,
                params,
            },
        }
    }

    /// Returns the non-instantiated [`JeffOpDef`] for this operation.
    pub fn opdef(&self) -> JeffOpDef {
        match self {
            JeffOp::QGate1 { .. } => JeffOpDef::QGate1,
            JeffOp::QGate2 { .. } => JeffOpDef::QGate2,
            JeffOp::QGateN { .. } => JeffOpDef::QGateN,
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
                TypeArg::new_var_use(var_idx, TypeParam::max_nat()),
            )
            .into()
        };

        match self {
            JeffOpDef::QGate1 => {
                PolyFuncType::new(vec![TypeParam::String], Signature::new_endo(vec![qb_t()])).into()
            }
            JeffOpDef::QGate2 => PolyFuncType::new(
                vec![TypeParam::String],
                Signature::new_endo(vec![qb_t(), qb_t()]),
            )
            .into(),
            JeffOpDef::QGateN => JeffGateNSignature.into(),
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
                vec![TypeParam::max_nat()],
                Signature::new(vec![intarr_type(0)], vec![int32_t()]),
            )
            .into(),
            JeffOpDef::IntArrayGet => JeffIntArrayGetSignature.into(),
            JeffOpDef::IntArraySet => JeffIntArraySetSignature.into(),
            JeffOpDef::IntArrayZero => PolyFuncType::new(
                vec![TypeParam::max_nat()],
                Signature::new(vec![int32_t()], vec![intarr_type(0)]),
            )
            .into(),
        }
    }

    fn opdef_id(&self) -> hugr::ops::OpName {
        match self {
            JeffOpDef::QGate1 => "QGate1".into(),
            JeffOpDef::QGate2 => "QGate2".into(),
            JeffOpDef::QGateN => "QGateN".into(),
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
            JeffOpDef::QGate1 => "A jeff 1-qubit gate.".to_string(),
            JeffOpDef::QGate2 => "A jeff 2-qubit gate.".to_string(),
            JeffOpDef::QGateN => "A jeff n-qubit gate.".to_string(),
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
        arg_values: &[TypeArg],
        _def: &'o OpDef,
    ) -> Result<PolyFuncTypeRV, SignatureError> {
        let qubits = arg_values[1].as_nat().expect("JeffOp arg should be a nat") as usize;
        let params = arg_values[2].as_nat().unwrap_or_default() as usize;

        let mut inputs = vec![qb_t(); qubits];
        inputs.extend(vec![float64_type(); params]);
        let outputs = vec![qb_t(); qubits];
        let sig: PolyFuncType = Signature::new(inputs, outputs).into();
        Ok(sig.into())
    }

    fn static_params(&self) -> &[TypeParam] {
        static PARAMS: [TypeParam; 3] = [
            TypeParam::String,
            TypeParam::max_nat(),
            TypeParam::max_nat(),
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
        arg_values: &[TypeArg],
        _def: &'o OpDef,
    ) -> Result<PolyFuncTypeRV, SignatureError> {
        let qubits = arg_values[0].as_nat().expect("JeffOp arg should be a nat") as usize;

        let inputs = vec![qb_t(); qubits];
        let outputs = vec![crate::types::jeff_to_hugr(jeff::types::Type::QubitRegister)];
        let sig: PolyFuncType = Signature::new(inputs, outputs).into();
        Ok(sig.into())
    }

    fn static_params(&self) -> &[TypeParam] {
        static PARAMS: [TypeParam; 1] = [TypeParam::max_nat()];
        &PARAMS
    }
}

/// A signature computation function for [`JeffOp::IntArrayCreate`].
#[derive(Debug, Clone, Copy)]
pub struct JeffIntArrayCreateSignature;

impl CustomSignatureFunc for JeffIntArrayCreateSignature {
    fn compute_signature<'o, 'a: 'o>(
        &'a self,
        arg_values: &[TypeArg],
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

    fn static_params(&self) -> &[TypeParam] {
        static PARAMS: [TypeParam; 2] = [TypeParam::max_nat(), TypeParam::max_nat()];
        &PARAMS
    }
}

/// A signature computation function for [`JeffOp::IntArrayGet`].
#[derive(Debug, Clone, Copy)]
pub struct JeffIntArrayGetSignature;

impl CustomSignatureFunc for JeffIntArrayGetSignature {
    fn compute_signature<'o, 'a: 'o>(
        &'a self,
        arg_values: &[TypeArg],
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

    fn static_params(&self) -> &[TypeParam] {
        static PARAMS: [TypeParam; 1] = [TypeParam::max_nat()];
        &PARAMS
    }
}

/// A signature computation function for [`JeffOp::IntArraySet`].
#[derive(Debug, Clone, Copy)]
pub struct JeffIntArraySetSignature;

impl CustomSignatureFunc for JeffIntArraySetSignature {
    fn compute_signature<'o, 'a: 'o>(
        &'a self,
        arg_values: &[TypeArg],
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

    fn static_params(&self) -> &[TypeParam] {
        static PARAMS: [TypeParam; 1] = [TypeParam::max_nat()];
        &PARAMS
    }
}

impl MakeExtensionOp for JeffOp {
    fn from_extension_op(ext_op: &ExtensionOp) -> Result<Self, OpLoadError> {
        let def = JeffOpDef::from_def(ext_op.def())?;
        def.instantiate(ext_op.args())
    }

    fn type_args(&self) -> Vec<TypeArg> {
        match self {
            JeffOp::QGate1 { name } => vec![TypeArg::String { arg: name.clone() }],
            JeffOp::QGate2 { name } => vec![TypeArg::String { arg: name.clone() }],
            JeffOp::QGateN {
                name,
                qubits,
                params,
            } => vec![
                TypeArg::String { arg: name.clone() },
                TypeArg::BoundedNat { n: *qubits as u64 },
                TypeArg::BoundedNat { n: *params as u64 },
            ],
            JeffOp::QuregAlloc => vec![],
            JeffOp::QuregFree => vec![],
            JeffOp::QuregExtractIndex => vec![],
            JeffOp::QuregInsertIndex => vec![],
            JeffOp::QuregCreate { qubits } => vec![TypeArg::BoundedNat { n: *qubits as u64 }],
            JeffOp::QuregExtractSlice => vec![],
            JeffOp::QuregInsertSlice => vec![],
            JeffOp::QuregSplit => vec![],
            JeffOp::QuregJoin => vec![],
            JeffOp::QuregLength => vec![],
            JeffOp::IntArrayCreate { bits, inputs } => vec![
                TypeArg::BoundedNat { n: *bits as u64 },
                TypeArg::BoundedNat { n: *inputs as u64 },
            ],
            JeffOp::IntArrayLength { bits } => vec![TypeArg::BoundedNat { n: *bits as u64 }],
            JeffOp::IntArrayGet { bits } => vec![TypeArg::BoundedNat { n: *bits as u64 }],
            JeffOp::IntArraySet { bits } => vec![TypeArg::BoundedNat { n: *bits as u64 }],
            JeffOp::IntArrayZero { bits } => vec![TypeArg::BoundedNat { n: *bits as u64 }],
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

    fn instantiate(&self, type_args: &[TypeArg]) -> Result<Self::Concrete, OpLoadError> {
        let (name, qubits, params) = match (self, type_args) {
            (JeffOpDef::QGate1, [TypeArg::String { arg }]) => (arg.clone(), 1, 0),
            (JeffOpDef::QGate2, [TypeArg::String { arg }]) => (arg.clone(), 2, 0),
            (
                JeffOpDef::QGateN,
                [
                    TypeArg::String { arg },
                    TypeArg::BoundedNat { n: qubits },
                    TypeArg::BoundedNat { n: params },
                ],
            ) => (arg.clone(), *qubits, *params),
            _ => return Err(SignatureError::InvalidTypeArgs.into()),
        };

        Ok(JeffOp::parametric_gate(
            name,
            qubits as usize,
            params as usize,
        ))
    }
}

impl HasDef for JeffOp {
    type Def = JeffOpDef;
}

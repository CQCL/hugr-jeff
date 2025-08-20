use hugr::extension::prelude::qb_t;
use hugr::hugr::hugrmut::HugrMut;
use hugr::ops::OpTrait;
use hugr::{HugrView, IncomingPort, OutgoingPort};
use itertools::Itertools;
use jeff::reader::optype as jeff_optype;
use tket::extension::rotation::RotationOp;

use crate::JeffToHugrError;
use crate::extension::JeffOp;
use crate::to_hugr::BuildContext;

use super::JeffToHugrOp;
use super::to_hugr::build_single_op;

/// Translation for _jeff_ quantum ops
impl JeffToHugrOp for jeff_optype::QubitOp<'_> {
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError> {
        match self {
            jeff_optype::QubitOp::Alloc => build_single_op(tket::TketOp::QAlloc, op, builder, ctx)?,
            jeff_optype::QubitOp::Free => build_single_op(tket::TketOp::QFree, op, builder, ctx)?,
            // TODO: Define a custom op for freeing qubits that are known to be in the |0> state.
            jeff_optype::QubitOp::FreeZero => {
                build_single_op(tket::TketOp::QFree, op, builder, ctx)?
            }
            jeff_optype::QubitOp::Measure => {
                build_single_op(tket::TketOp::MeasureFree, op, builder, ctx)?
            }
            jeff_optype::QubitOp::MeasureNd => {
                build_single_op(tket::TketOp::Measure, op, builder, ctx)?
            }
            jeff_optype::QubitOp::Reset => build_single_op(tket::TketOp::Reset, op, builder, ctx)?,
            jeff_optype::QubitOp::Gate(gate_op) => gate_op.build_hugr_op(op, builder, ctx)?,
            _ => return Err(JeffToHugrError::unsupported_op(self)),
        };
        Ok(())
    }
}

impl JeffToHugrOp for jeff_optype::GateOp<'_> {
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError> {
        let name = self.name;
        let qubits = self.num_qubits as usize;
        let params = self.num_params as usize;

        match (name.to_lowercase().as_str(), qubits, params) {
            ("h", 1, 0) | ("hadamard", 1, 0) => build_single_op(tket::TketOp::H, op, builder, ctx)?,
            ("cx", 2, 0) | ("cnot", 2, 0) => build_single_op(tket::TketOp::CX, op, builder, ctx)?,
            ("cy", 2, 0) => build_single_op(tket::TketOp::CY, op, builder, ctx)?,
            ("cz", 2, 0) => build_single_op(tket::TketOp::CZ, op, builder, ctx)?,
            ("crz", 2, 1) => build_parametric_gate(tket::TketOp::CRz, op, builder, ctx)?,
            ("t", 1, 0) => build_single_op(tket::TketOp::T, op, builder, ctx)?,
            ("tdg", 1, 0) => build_single_op(tket::TketOp::Tdg, op, builder, ctx)?,
            ("s", 1, 0) => build_single_op(tket::TketOp::S, op, builder, ctx)?,
            ("sdg", 1, 0) => build_single_op(tket::TketOp::Sdg, op, builder, ctx)?,
            ("x", 1, 0) => build_single_op(tket::TketOp::X, op, builder, ctx)?,
            ("y", 1, 0) => build_single_op(tket::TketOp::Y, op, builder, ctx)?,
            ("z", 1, 0) => build_single_op(tket::TketOp::Z, op, builder, ctx)?,
            ("rx", 1, 1) => build_parametric_gate(tket::TketOp::Rx, op, builder, ctx)?,
            ("ry", 1, 1) => build_parametric_gate(tket::TketOp::Ry, op, builder, ctx)?,
            ("rz", 1, 1) => build_parametric_gate(tket::TketOp::Rz, op, builder, ctx)?,
            ("toffoli", 3, 0) => build_single_op(tket::TketOp::Toffoli, op, builder, ctx)?,
            ("swap", 2, 0) => {
                let inputs = op.inputs().collect::<Result<Vec<_>, _>>()?;
                let outputs = op.outputs().collect::<Result<Vec<_>, _>>()?;
                ctx.merge_with_earlier(outputs[0].id(), inputs[1].id());
                ctx.merge_with_earlier(outputs[1].id(), inputs[0].id());
            }
            _ => build_parametric_gate(
                JeffOp::parametric_gate(name.to_string(), qubits, params),
                op,
                builder,
                ctx,
            )?,
        };
        Ok(())
    }
}

/// Build a quantum operation with some input f64 parameters.
///
/// The parameters have to be converted to "rotation"s before being passed to the tket operation.
fn build_parametric_gate(
    op: impl Into<hugr::ops::OpType>,
    jeff_op: &jeff::reader::Operation<'_>,
    builder: &mut impl hugr::builder::Dataflow,
    ctx: &mut BuildContext,
) -> Result<(), JeffToHugrError> {
    let op: hugr::ops::OpType = op.into();

    // Compute the number of qubits and parameters
    let sig = op.dataflow_signature().unwrap();
    let qubit_type = qb_t();
    let rotation_type = tket::extension::rotation::rotation_type();
    let mut input_types = sig.input_types().iter();
    let qubits = input_types.take_while_ref(|t| t == &&qubit_type).count();
    let params = input_types.take_while_ref(|t| t == &&rotation_type).count();
    debug_assert!(input_types.next().is_none());

    // We first need to convert the f64 parameters to "rotation"s.
    //
    // TODO: Do we need to convert the value? HUGR expects half-turns.
    let mut rotations = Vec::with_capacity(params);
    for _ in 0..params {
        let node = builder.add_child_node(RotationOp::from_halfturns_unchecked);
        rotations.push(node);
    }

    let op_node = builder.add_child_node(op);

    // Internal connections between the f64 conversion nodes and the operation node.
    for (i, rot_node) in rotations.iter().enumerate() {
        let rot_port = OutgoingPort::from(0);
        builder
            .hugr_mut()
            .connect(*rot_node, rot_port, op_node, IncomingPort::from(i + qubits));
    }

    for (i, value) in jeff_op.inputs().enumerate() {
        let value_id = value?.id();
        if i < qubits {
            // Boundary connection into the quantum operation
            let port = IncomingPort::from(i);
            ctx.register_input(value_id, op_node, port);
        } else {
            // Connection to the f64 conversion nodes
            let port = IncomingPort::from(0);
            ctx.register_input(value_id, rotations[i - qubits], port);
        }
    }
    for (port, value) in builder.hugr().node_outputs(op_node).zip(jeff_op.outputs()) {
        let value = value?;
        ctx.register_output(value.id(), op_node, port);
    }

    Ok(())
}

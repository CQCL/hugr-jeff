use hugr::hugr::hugrmut::HugrMut;
use hugr::ops::OpTrait;
use hugr::ops::handle::NodeHandle;
use hugr::std_extensions::arithmetic::float_ops::FloatOps;
use hugr::std_extensions::arithmetic::float_types::ConstF64;
use hugr::{HugrView, Wire};
use itertools::Itertools;
use jeff::reader::optype as jeff_optype;
use tket::extension::rotation::{RotationOp, rotation_type};

use crate::JeffToHugrError;
use crate::extension::JeffOp;
use crate::to_hugr::BuildContext;

use super::JeffToHugrOp;

/// Translation for _jeff_ quantum ops
impl JeffToHugrOp for jeff_optype::QubitOp<'_> {
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError> {
        match self {
            jeff_optype::QubitOp::Alloc => {
                ctx.build_single_op(tket::TketOp::QAlloc, op, builder)?
            }
            jeff_optype::QubitOp::Free => ctx.build_single_op(tket::TketOp::QFree, op, builder)?,
            // TODO: Define a custom op for freeing qubits that are known to be in the |0> state.
            jeff_optype::QubitOp::FreeZero => {
                ctx.build_single_op(tket::TketOp::QFree, op, builder)?
            }
            jeff_optype::QubitOp::Measure => {
                ctx.build_single_op(tket::TketOp::MeasureFree, op, builder)?
            }
            jeff_optype::QubitOp::MeasureNd => {
                ctx.build_single_op(tket::TketOp::Measure, op, builder)?
            }
            jeff_optype::QubitOp::Reset => ctx.build_single_op(tket::TketOp::Reset, op, builder)?,
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
        let gate = self.normalize();
        match gate.gate_type {
            jeff_optype::GateOpType::WellKnown(well_known) => {
                build_well_known_gate(well_known, gate, op, builder, ctx)
            }
            jeff_optype::GateOpType::PauliProdRotation { pauli_string } => {
                ctx.build_single_op(JeffOp::jeff_gate_op(pauli_string, gate), op, builder)
            }
            jeff_optype::GateOpType::Custom { name, .. } => {
                ctx.build_single_op(JeffOp::jeff_gate_op(name, gate), op, builder)
            }
        }
    }
}

/// Adds a well-known gate to the HUGR.
///
/// Reads the extra parameters from the gate operation if any.
fn build_well_known_gate(
    wk_gate: jeff_optype::WellKnownGate,
    gate_op: jeff_optype::GateOp<'_>,
    op: &jeff::reader::Operation<'_>,
    builder: &mut impl hugr::builder::Dataflow,
    ctx: &mut BuildContext,
) -> Result<(), JeffToHugrError> {
    use jeff_optype::WellKnownGate::*;

    let mut build_self_inverse = |tket_op, pwr| match pwr % 2 == 0 {
        true => ctx.build_transparent_op(op),
        false => ctx.build_single_op(tket_op, op, builder),
    };

    match (
        wk_gate,
        gate_op.adjoint,
        gate_op.control_qubits,
        gate_op.power,
    ) {
        // Any operation with power 0 is a no-op.
        (I, _, _, _) => ctx.build_transparent_op(op),
        (H, _, 0, pwr) => build_self_inverse(tket::TketOp::H, pwr),
        (X, _, 0, pwr) => build_self_inverse(tket::TketOp::X, pwr),
        (X, _, 1, pwr) => build_self_inverse(tket::TketOp::CX, pwr),
        (Y, _, 0, pwr) => build_self_inverse(tket::TketOp::Y, pwr),
        (Y, _, 1, pwr) => build_self_inverse(tket::TketOp::CY, pwr),
        (Z, _, 0, pwr) => build_self_inverse(tket::TketOp::Z, pwr),
        (Z, _, 1, pwr) => build_self_inverse(tket::TketOp::CZ, pwr),
        (S, false, 0, 1) => ctx.build_single_op(tket::TketOp::S, op, builder),
        (S, true, 0, 1) => ctx.build_single_op(tket::TketOp::Sdg, op, builder),
        (T, false, 0, 1) => ctx.build_single_op(tket::TketOp::T, op, builder),
        (T, true, 0, 1) => ctx.build_single_op(tket::TketOp::Tdg, op, builder),
        (Rx, false, 0, 1) => build_parametric_tket_op(ctx, tket::TketOp::Rx, op, builder),
        (Ry, false, 0, 1) => build_parametric_tket_op(ctx, tket::TketOp::Ry, op, builder),
        (Rz, false, 0, 1) => build_parametric_tket_op(ctx, tket::TketOp::Rz, op, builder),
        (Swap, _, 0, pwr) => match pwr % 2 == 0 {
            true => ctx.build_transparent_op(op),
            false => {
                let mut inputs = op.inputs();
                let mut outputs = op.outputs();
                let a_in = inputs.next().unwrap().unwrap().id();
                let b_in = inputs.next().unwrap().unwrap().id();
                let a_out = outputs.next().unwrap().unwrap().id();
                let b_out = outputs.next().unwrap().unwrap().id();
                ctx.merge_with_earlier(a_out, b_in);
                ctx.merge_with_earlier(b_out, a_in);
                Ok(())
            }
        },
        _ => ctx.build_single_op(JeffOp::jeff_gate_op(wk_gate, gate_op), op, builder),
    }
}

/// Emit a single HUGR operation that expects rotation-type parameters.
///
/// Jeff operations work on radians, so we need to convert the inputs to half-turn rotations here.
pub fn build_parametric_tket_op(
    ctx: &mut BuildContext,
    op: impl Into<hugr::ops::OpType>,
    jeff_op: &jeff::reader::Operation<'_>,
    builder: &mut impl hugr::builder::Dataflow,
) -> Result<(), JeffToHugrError> {
    let op: hugr::ops::OpType = op.into();
    let sig = op.dataflow_signature().unwrap().into_owned();
    let node = builder.add_child_node(op);
    let rotation_t = rotation_type();

    // A loaded pi constant, used for converting radians to half-turns.
    let mut pi: Option<Wire> = None;

    let input_ports = builder.hugr().node_inputs(node).collect_vec();
    for (&port, value) in input_ports.iter().zip(jeff_op.inputs()) {
        if sig.in_port_type(port).unwrap() == &rotation_t {
            let pi = *pi
                .get_or_insert_with(|| builder.add_load_value(ConstF64::new(std::f64::consts::PI)));
            let div = builder.add_child_node(FloatOps::fdiv);
            let rot = builder
                .add_dataflow_op(RotationOp::from_halfturns_unchecked, [Wire::new(div, 0)])?;

            builder.hugr_mut().connect(pi.node(), pi.source(), div, 1);
            builder.hugr_mut().connect(rot.node(), 0, node, port);
            ctx.register_input(value?.id(), div, 0.into());
        } else {
            ctx.register_input(value?.id(), node, port);
        }
    }
    for (port, value) in builder.hugr().node_outputs(node).zip(jeff_op.outputs()) {
        ctx.register_output(value?.id(), node, port);
    }

    Ok(())
}

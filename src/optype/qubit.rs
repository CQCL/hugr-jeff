use itertools::Itertools;
use jeff::reader::optype as jeff_optype;

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
        match self.gate_type {
            jeff_optype::GateOpType::WellKnown(well_known) => {
                build_well_known_gate(well_known, *self, op, builder, ctx)
            }
            jeff_optype::GateOpType::PauliProdRotation { pauli_string } => {
                ctx.build_single_op(JeffOp::jeff_gate_op(pauli_string, self), op, builder)
            }
            jeff_optype::GateOpType::Custom { name, .. } => {
                ctx.build_single_op(JeffOp::jeff_gate_op(name, self), op, builder)
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
        (_, _, _, 0) => {
            let qubits = gate_op.num_qubits();
            for (input, output) in op.inputs().zip(op.outputs()).take(qubits) {
                let input = input?.id();
                let output = output?.id();
                ctx.merge_with_earlier(output, input);
            }
            Ok(())
        }
        (I, _, _, _) => ctx.build_transparent_op(op),
        (H, _, 0, pwr) => build_self_inverse(tket::TketOp::H, pwr),
        (X, _, 0, pwr) => build_self_inverse(tket::TketOp::X, pwr),
        (Y, _, 0, pwr) => build_self_inverse(tket::TketOp::Y, pwr),
        (Z, _, 0, pwr) => build_self_inverse(tket::TketOp::Z, pwr),
        (S, false, 0, 1) => ctx.build_single_op(tket::TketOp::S, op, builder),
        (S, true, 0, 1) => ctx.build_single_op(tket::TketOp::Sdg, op, builder),
        (T, false, 0, 1) => ctx.build_single_op(tket::TketOp::T, op, builder),
        (T, true, 0, 1) => ctx.build_single_op(tket::TketOp::Tdg, op, builder),
        (Rx, false, 0, 1) => ctx.build_single_op(tket::TketOp::Rx, op, builder),
        (Ry, false, 0, 1) => ctx.build_single_op(tket::TketOp::Ry, op, builder),
        (Rz, false, 0, 1) => ctx.build_single_op(tket::TketOp::Rz, op, builder),
        (Swap, _, 0, pwr) => match pwr % 2 == 0 {
            true => ctx.build_transparent_op(op),
            false => {
                let [a, b] = op
                    .inputs()
                    .map(|v| v.unwrap().id())
                    .collect_array()
                    .expect("2 inputs");
                ctx.merge_with_earlier(b, a);
                ctx.merge_with_earlier(a, b);
                Ok(())
            }
        },
        _ => ctx.build_single_op(JeffOp::jeff_gate_op(wk_gate, &gate_op), op, builder),
    }
}

//! Translation between _jeff_ and HUGR operation types

use jeff::reader::optype as jeff_optype;

use crate::JeffToHugrError;
use crate::to_hugr::BuildContext;

mod control_flow;
mod float;
mod function;
mod int;
mod int_array;
mod qubit;
mod qubit_array;

/// Internal utility trait to convert jeff optypes.
pub(crate) trait JeffToHugrOp {
    /// Given a _jeff_ operation type and a HUGR dataflow builder, build the corresponding HUGR operation.
    ///
    /// Returns the ports corresponding to the _jeff_ operation I/O.
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError>;
}

impl JeffToHugrOp for jeff_optype::OpType<'_> {
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError> {
        match self {
            jeff_optype::OpType::FloatOp(optype) => optype.build_hugr_op(op, builder, ctx),
            jeff_optype::OpType::FuncOp(optype) => optype.build_hugr_op(op, builder, ctx),
            jeff_optype::OpType::IntOp(optype) => optype.build_hugr_op(op, builder, ctx),
            jeff_optype::OpType::IntArrayOp(optype) => optype.build_hugr_op(op, builder, ctx),
            jeff_optype::OpType::QubitOp(optype) => optype.build_hugr_op(op, builder, ctx),
            jeff_optype::OpType::QubitRegisterOp(optype) => optype.build_hugr_op(op, builder, ctx),
            jeff_optype::OpType::ControlFlowOp(cfop) => cfop.build_hugr_op(op, builder, ctx),
            _ => Err(JeffToHugrError::unsupported_op(self)),
        }
    }
}

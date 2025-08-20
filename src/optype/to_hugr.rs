//! Convert _jeff_ operation types to HUGR operation types.
use hugr::HugrView;
use jeff::reader::optype as jeff_optype;

use crate::JeffToHugrError;
use crate::to_hugr::BuildContext;

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

/// Helper function to convert _jeff_ operations that map to a single HUGR operation.
pub(super) fn build_single_op(
    op: impl Into<hugr::ops::OpType>,
    jeff_op: &jeff::reader::Operation<'_>,
    builder: &mut impl hugr::builder::Dataflow,
    ctx: &mut BuildContext,
) -> Result<(), JeffToHugrError> {
    let node = builder.add_child_node(op.into());

    for (port, value) in builder.hugr().node_inputs(node).zip(jeff_op.inputs()) {
        let value = value?;
        ctx.register_input(value.id(), node, port);
    }
    for (port, value) in builder.hugr().node_outputs(node).zip(jeff_op.outputs()) {
        let value = value?;
        ctx.register_output(value.id(), node, port);
    }

    Ok(())
}

/// Helper function to convert _jeff_ constant ops into HUGR constant / loadConstant pairs.
pub(super) fn build_constant_op(
    op: impl Into<hugr::ops::Value>,
    jeff_op: &jeff::reader::Operation<'_>,
    builder: &mut impl hugr::builder::Dataflow,
    ctx: &mut BuildContext,
) -> Result<(), JeffToHugrError> {
    let wire = builder.add_load_value(op.into());

    // Constant ops in _jeff_ have no inputs and a single output.
    if jeff_op.input_count() != 0 || jeff_op.output_count() != 1 {
        return Err(JeffToHugrError::unsupported_op(jeff_op));
    }
    let value = jeff_op.output(0).unwrap()?;

    ctx.register_output(value.id(), wire.node(), wire.source());
    Ok(())
}

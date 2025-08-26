use jeff::reader::optype as jeff_optype;

use crate::JeffToHugrError;
use crate::extension::JeffOp;
use crate::to_hugr::BuildContext;

use super::JeffToHugrOp;

/// Translation for _jeff_ quantum ops
impl JeffToHugrOp for jeff_optype::QubitRegisterOp {
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError> {
        match self {
            jeff_optype::QubitRegisterOp::Alloc => {
                ctx.build_single_op(JeffOp::QuregAlloc, op, builder)?
            }
            jeff_optype::QubitRegisterOp::Free => {
                ctx.build_single_op(JeffOp::QuregFree, op, builder)?
            }
            jeff_optype::QubitRegisterOp::FreeZero => {
                ctx.build_single_op(JeffOp::QuregFree, op, builder)?
            }
            jeff_optype::QubitRegisterOp::ExtractIndex => {
                ctx.build_single_op(JeffOp::QuregExtractIndex, op, builder)?
            }
            jeff_optype::QubitRegisterOp::InsertIndex => {
                ctx.build_single_op(JeffOp::QuregInsertIndex, op, builder)?
            }
            jeff_optype::QubitRegisterOp::ExtractSlice => {
                ctx.build_single_op(JeffOp::QuregExtractSlice, op, builder)?
            }
            jeff_optype::QubitRegisterOp::InsertSlice => {
                ctx.build_single_op(JeffOp::QuregInsertSlice, op, builder)?
            }
            jeff_optype::QubitRegisterOp::Length => {
                ctx.build_single_op(JeffOp::QuregLength, op, builder)?
            }
            jeff_optype::QubitRegisterOp::Split => {
                ctx.build_single_op(JeffOp::QuregSplit, op, builder)?
            }
            jeff_optype::QubitRegisterOp::Join => {
                ctx.build_single_op(JeffOp::QuregJoin, op, builder)?
            }
            jeff_optype::QubitRegisterOp::Create => {
                let qubits = op.input_count();
                ctx.build_single_op(JeffOp::QuregCreate { qubits }, op, builder)?
            }
            _ => return Err(JeffToHugrError::unsupported_op(self)),
        };
        Ok(())
    }
}

use jeff::reader::optype as jeff_optype;

use crate::JeffToHugrError;
use crate::extension::JeffOp;
use crate::to_hugr::BuildContext;

use super::JeffToHugrOp;
use super::to_hugr::build_single_op;

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
                build_single_op(JeffOp::QuregAlloc, op, builder, ctx)?
            }
            jeff_optype::QubitRegisterOp::Free => {
                build_single_op(JeffOp::QuregFree, op, builder, ctx)?
            }
            jeff_optype::QubitRegisterOp::FreeZero => {
                build_single_op(JeffOp::QuregFree, op, builder, ctx)?
            }
            jeff_optype::QubitRegisterOp::ExtractIndex => {
                build_single_op(JeffOp::QuregExtractIndex, op, builder, ctx)?
            }
            jeff_optype::QubitRegisterOp::InsertIndex => {
                build_single_op(JeffOp::QuregInsertIndex, op, builder, ctx)?
            }
            jeff_optype::QubitRegisterOp::ExtractSlice => {
                build_single_op(JeffOp::QuregExtractSlice, op, builder, ctx)?
            }
            jeff_optype::QubitRegisterOp::InsertSlice => {
                build_single_op(JeffOp::QuregInsertSlice, op, builder, ctx)?
            }
            jeff_optype::QubitRegisterOp::Length => {
                build_single_op(JeffOp::QuregLength, op, builder, ctx)?
            }
            jeff_optype::QubitRegisterOp::Split => {
                build_single_op(JeffOp::QuregSplit, op, builder, ctx)?
            }
            jeff_optype::QubitRegisterOp::Join => {
                build_single_op(JeffOp::QuregJoin, op, builder, ctx)?
            }
            jeff_optype::QubitRegisterOp::Create => {
                let qubits = op.input_count();
                build_single_op(JeffOp::QuregCreate { qubits }, op, builder, ctx)?
            }
            _ => return Err(JeffToHugrError::unsupported_op(self)),
        };
        Ok(())
    }
}

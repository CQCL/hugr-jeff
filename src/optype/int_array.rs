use jeff::reader::optype as jeff_optype;

use crate::JeffToHugrError;
use crate::extension::{ConstIntReg, JeffOp};
use crate::to_hugr::BuildContext;

use super::JeffToHugrOp;
use super::to_hugr::{build_constant_op, build_single_op};

/// Translation for _jeff_ quantum ops
impl JeffToHugrOp for jeff_optype::IntArrayOp<'_> {
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError> {
        // Get the number of bits in an integer input.
        let input_bits = |idx| {
            let value = op.input(idx).unwrap()?;
            match value.ty() {
                jeff::types::Type::Int { bits } => Ok(bits),
                jeff::types::Type::IntArray { bits } => Ok(bits),
                _ => Err(JeffToHugrError::unsupported_op(self)),
            }
        };

        match self {
            jeff_optype::IntArrayOp::Create => {
                let bits = input_bits(0)?;
                let inputs = op.input_count();
                build_single_op(JeffOp::IntArrayCreate { bits, inputs }, op, builder, ctx)?
            }
            jeff_optype::IntArrayOp::GetIndex => {
                let bits = input_bits(0)?;
                build_single_op(JeffOp::IntArrayGet { bits }, op, builder, ctx)?
            }
            jeff_optype::IntArrayOp::SetIndex => {
                let bits = input_bits(0)?;
                build_single_op(JeffOp::IntArraySet { bits }, op, builder, ctx)?
            }
            jeff_optype::IntArrayOp::Zero { bits } => {
                build_single_op(JeffOp::IntArrayZero { bits: *bits }, op, builder, ctx)?
            }
            jeff_optype::IntArrayOp::ConstArray8(array) => {
                let bits = 3;
                let const_val = ConstIntReg::new(array.values().map(|v| v as u64), bits);
                build_constant_op(const_val, op, builder, ctx)?
            }
            jeff_optype::IntArrayOp::ConstArray16(array) => {
                let bits = 4;
                let const_val = ConstIntReg::new(array.values().map(|v| v as u64), bits);
                build_constant_op(const_val, op, builder, ctx)?
            }
            jeff_optype::IntArrayOp::ConstArray32(array) => {
                let bits = 5;
                let const_val = ConstIntReg::new(array.values().map(|v| v as u64), bits);
                build_constant_op(const_val, op, builder, ctx)?
            }
            jeff_optype::IntArrayOp::ConstArray64(array) => {
                let bits = 6;
                let const_val = ConstIntReg::new(array.values(), bits);
                build_constant_op(const_val, op, builder, ctx)?
            }
            // TODO: jeff_optype::IntArrayOp::ConstArray1(array)
            _ => return Err(JeffToHugrError::unsupported_op(self)),
        };
        Ok(())
    }
}

use hugr::std_extensions::arithmetic::float_ops::FloatOps;
use hugr::std_extensions::arithmetic::float_types::ConstF64;
use jeff::reader::optype as jeff_optype;

use crate::JeffToHugrError;
use crate::to_hugr::BuildContext;

use super::JeffToHugrOp;
use super::to_hugr::{build_constant_op, build_single_op};

/// Translation for _jeff_ quantum ops
impl JeffToHugrOp for jeff_optype::FloatOp {
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError> {
        match self {
            jeff_optype::FloatOp::Const32(f) => {
                build_constant_op(ConstF64::new(*f as f64), op, builder, ctx)?
            }
            jeff_optype::FloatOp::Const64(f) => {
                build_constant_op(ConstF64::new(*f), op, builder, ctx)?
            }
            jeff_optype::FloatOp::Add => build_single_op(FloatOps::fadd, op, builder, ctx)?,
            jeff_optype::FloatOp::Sub => build_single_op(FloatOps::fsub, op, builder, ctx)?,
            jeff_optype::FloatOp::Mul => build_single_op(FloatOps::fmul, op, builder, ctx)?,
            jeff_optype::FloatOp::Pow => build_single_op(FloatOps::fpow, op, builder, ctx)?,
            jeff_optype::FloatOp::Eq => build_single_op(FloatOps::feq, op, builder, ctx)?,
            jeff_optype::FloatOp::Lt => build_single_op(FloatOps::flt, op, builder, ctx)?,
            jeff_optype::FloatOp::Lte => build_single_op(FloatOps::fle, op, builder, ctx)?,
            jeff_optype::FloatOp::Abs => build_single_op(FloatOps::fabs, op, builder, ctx)?,
            jeff_optype::FloatOp::Ceil => build_single_op(FloatOps::fceil, op, builder, ctx)?,
            jeff_optype::FloatOp::Floor => build_single_op(FloatOps::ffloor, op, builder, ctx)?,
            jeff_optype::FloatOp::Exp => build_single_op(FloatOps::fpow, op, builder, ctx)?,
            jeff_optype::FloatOp::Max => build_single_op(FloatOps::fmax, op, builder, ctx)?,
            jeff_optype::FloatOp::Min => build_single_op(FloatOps::fmin, op, builder, ctx)?,
            // Unsupported _jeff_ float ops
            jeff_optype::FloatOp::Sqrt
            | jeff_optype::FloatOp::IsNan
            | jeff_optype::FloatOp::IsInf
            | jeff_optype::FloatOp::Log
            | jeff_optype::FloatOp::Sin
            | jeff_optype::FloatOp::Cos
            | jeff_optype::FloatOp::Tan
            | jeff_optype::FloatOp::Asin
            | jeff_optype::FloatOp::Acos
            | jeff_optype::FloatOp::Atan
            | jeff_optype::FloatOp::Atan2
            | jeff_optype::FloatOp::Sinh
            | jeff_optype::FloatOp::Cosh
            | jeff_optype::FloatOp::Tanh
            | jeff_optype::FloatOp::Asinh
            | jeff_optype::FloatOp::Acosh
            | jeff_optype::FloatOp::Atanh
            | _ => return Err(JeffToHugrError::unsupported_op(self)),
        };
        Ok(())
    }
}

use hugr::std_extensions::arithmetic::float_ops::FloatOps;
use hugr::std_extensions::arithmetic::float_types::ConstF64;
use jeff::reader::optype as jeff_optype;

use crate::JeffToHugrError;
use crate::to_hugr::BuildContext;

use super::JeffToHugrOp;

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
                ctx.build_constant_value(ConstF64::new(*f as f64), op, builder)?
            }
            jeff_optype::FloatOp::Const64(f) => {
                ctx.build_constant_value(ConstF64::new(*f), op, builder)?
            }
            jeff_optype::FloatOp::Add => ctx.build_single_op(FloatOps::fadd, op, builder)?,
            jeff_optype::FloatOp::Sub => ctx.build_single_op(FloatOps::fsub, op, builder)?,
            jeff_optype::FloatOp::Mul => ctx.build_single_op(FloatOps::fmul, op, builder)?,
            jeff_optype::FloatOp::Pow => ctx.build_single_op(FloatOps::fpow, op, builder)?,
            jeff_optype::FloatOp::Eq => ctx.build_single_op(FloatOps::feq, op, builder)?,
            jeff_optype::FloatOp::Lt => ctx.build_single_op(FloatOps::flt, op, builder)?,
            jeff_optype::FloatOp::Lte => ctx.build_single_op(FloatOps::fle, op, builder)?,
            jeff_optype::FloatOp::Abs => ctx.build_single_op(FloatOps::fabs, op, builder)?,
            jeff_optype::FloatOp::Ceil => ctx.build_single_op(FloatOps::fceil, op, builder)?,
            jeff_optype::FloatOp::Floor => ctx.build_single_op(FloatOps::ffloor, op, builder)?,
            jeff_optype::FloatOp::Exp => ctx.build_single_op(FloatOps::fpow, op, builder)?,
            jeff_optype::FloatOp::Max => ctx.build_single_op(FloatOps::fmax, op, builder)?,
            jeff_optype::FloatOp::Min => ctx.build_single_op(FloatOps::fmin, op, builder)?,
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

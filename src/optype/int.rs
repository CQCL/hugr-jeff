use hugr::ops::Value;
use hugr::std_extensions::arithmetic::int_types::ConstInt;
use jeff::reader::optype as jeff_optype;

use crate::JeffToHugrError;
use crate::to_hugr::BuildContext;

use super::JeffToHugrOp;

/// Translation for _jeff_ quantum ops
impl JeffToHugrOp for jeff_optype::IntOp {
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError> {
        match self {
            jeff_optype::IntOp::Const1(b) => {
                ctx.build_constant_value(Value::from_bool(*b), op, builder)?
            }
            jeff_optype::IntOp::Const8(n) => {
                ctx.build_constant_value(ConstInt::new_u(3, *n as u64).unwrap(), op, builder)?
            }
            jeff_optype::IntOp::Const16(n) => {
                ctx.build_constant_value(ConstInt::new_u(4, *n as u64).unwrap(), op, builder)?
            }
            jeff_optype::IntOp::Const32(n) => {
                ctx.build_constant_value(ConstInt::new_u(5, *n as u64).unwrap(), op, builder)?
            }
            jeff_optype::IntOp::Const64(n) => {
                ctx.build_constant_value(ConstInt::new_u(6, *n).unwrap(), op, builder)?
            }

            // TODO: Int operations require querying the jeff value type to determine the correct
            // integer width.
            _ => return Err(JeffToHugrError::unsupported_op(self)),
        };
        Ok(())
    }
}

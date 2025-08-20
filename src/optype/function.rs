use hugr::{HugrView, IncomingPort};
use jeff::reader::{FunctionId, optype as jeff_optype};

use crate::JeffToHugrError;
use crate::to_hugr::BuildContext;
use crate::types::jeff_signature_to_hugr;

use super::JeffToHugrOp;

/// Translation for _jeff_ quantum ops
impl JeffToHugrOp for jeff_optype::FuncOp {
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError> {
        let fn_inputs = op.input_types().collect::<Result<Vec<_>, _>>()?;
        let fn_outputs = op.output_types().collect::<Result<Vec<_>, _>>()?;
        let call_signature = jeff_signature_to_hugr(fn_inputs, fn_outputs);

        let call = hugr::ops::Call::try_new(call_signature.into(), vec![]).unwrap();
        let node = builder.add_child_node(call);

        // Note: the `zip` will stop when the _jeff_ operation inputs are
        // exhausted, so it won't register the static function parameters of the
        // call.
        for (port, value) in builder.hugr().node_inputs(node).zip(op.inputs()) {
            let value = value?;
            ctx.register_input(value.id(), node, port);
        }
        for (port, value) in builder.hugr().node_outputs(node).zip(op.outputs()) {
            let value = value?;
            ctx.register_output(value.id(), node, port);
        }

        let static_inp = IncomingPort::from(op.input_count());
        ctx.register_function_call(self.func_idx as FunctionId, node, static_inp);

        Ok(())
    }
}

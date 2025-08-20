use std::iter::once;

use hugr::builder::{
    BuildHandle, ConditionalBuilder, Container as _, DFGWrapper, Dataflow, SubContainer,
    TailLoopBuilder,
};
use hugr::hugr::hugrmut::HugrMut;
use hugr::ops::handle::NodeHandle;
use hugr::std_extensions::arithmetic::int_ops::IntOpDef;
use hugr::std_extensions::arithmetic::int_types::ConstInt;
use hugr::{Hugr, HugrView as _};
use jeff::reader::Region;
use jeff::reader::optype::{self as jeff_optype, ControlFlowOp};

use crate::to_hugr::{BuildContext, build_region};
use crate::{JeffToHugrError, types};

use super::JeffToHugrOp;
use jeff::types::Type as JeffType;

/// Translation for _jeff_ quantum ops
impl JeffToHugrOp for jeff_optype::ControlFlowOp<'_> {
    fn build_hugr_op(
        &self,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
        ctx: &mut BuildContext,
    ) -> Result<(), JeffToHugrError> {
        let input_types = op
            .input_types()
            .map(|ty| {
                let ty = ty?;
                Ok(types::jeff_to_hugr(ty))
            })
            .collect::<Result<Vec<_>, JeffToHugrError>>()?;
        let output_types = op
            .output_types()
            .map(|ty| {
                let ty = ty?;
                Ok(types::jeff_to_hugr(ty))
            })
            .collect::<Result<Vec<_>, JeffToHugrError>>()?;

        match self {
            ControlFlowOp::Switch(switch_op) => {
                // For now, we only support an i1 switch
                let Ok(JeffType::Int { bits: 1 }) = op.input_types().next().unwrap() else {
                    todo!("Lower switches with more branches")
                };
                let mut cond_builder = ConditionalBuilder::new(
                    vec![vec![].into(), vec![].into()],
                    input_types,
                    output_types,
                )?;
                let mut case0 = cond_builder.case_builder(0)?;
                build_nested(&mut case0, &switch_op.branch(0))?;
                let mut case1 = cond_builder.case_builder(1)?;
                if switch_op.branch_count() > 1 {
                    build_nested(&mut case1, &switch_op.branch(1))?;
                } else if let Some(default_branch) = switch_op.default_branch() {
                    build_nested(&mut case1, &default_branch)?;
                } else {
                    case1.set_outputs(case1.input_wires())?;
                }
                // Insert into the current Hugr and update context
                let cond_node = builder
                    .add_hugr(cond_builder.hugr().clone())
                    .inserted_entrypoint;
                for (port, value) in builder.hugr().node_inputs(cond_node).zip(op.inputs()) {
                    ctx.register_input(value?.id(), cond_node, port);
                }
                for (port, value) in builder.hugr().node_outputs(cond_node).zip(op.outputs()) {
                    ctx.register_output(value?.id(), cond_node, port);
                }
            }
            ControlFlowOp::Loop { region } => {
                let mut loop_builder = TailLoopBuilder::new(vec![], input_types, vec![])?;
                build_nested(&mut loop_builder, region)?;
                // Insert into the current Hugr and update context
                let loop_node = builder
                    .add_hugr(loop_builder.hugr().clone())
                    .inserted_entrypoint;
                for (port, value) in builder.hugr().node_inputs(loop_node).zip(op.inputs()) {
                    ctx.register_input(value?.id(), loop_node, port);
                }
                for (port, value) in builder.hugr().node_outputs(loop_node).zip(op.outputs()) {
                    ctx.register_output(value?.id(), loop_node, port);
                }
            }
            ControlFlowOp::For { region } => {
                let Ok(JeffType::Int { bits }) = op.input_types().next().unwrap() else {
                    panic!("Bad input to for loop")
                };
                let log_width = bits.next_power_of_two().trailing_zeros() as u8;
                let mut loop_builder = TailLoopBuilder::new(vec![], input_types.clone(), vec![])?;
                // Emit check if current iter is less than the bound
                let counter = loop_builder.input_wires().next().unwrap();
                let test = loop_builder
                    .add_dataflow_op(IntOpDef::ile_s.with_log_width(log_width), [counter])?;
                let mut cond = loop_builder.conditional_builder(
                    (vec![vec![].into(), vec![].into()], test.out_wire(0)),
                    input_types.into_iter().zip(loop_builder.input_wires()),
                    output_types.into(),
                )?;
                // Emit loop body conditioned on the test being true
                let mut ok_case = cond.case_builder(0)?;
                build_nested(&mut ok_case, region)?;
                // Otherwise, the break case is just identity
                let mut break_case = cond.case_builder(1)?;
                break_case.set_outputs(break_case.input_wires().skip(1))?;
                let cond = cond.finish_sub_container()?;
                // Increment counter
                let one = loop_builder.add_load_value(ConstInt::new_u(log_width, 1).unwrap());
                let counter = loop_builder
                    .add_dataflow_op(IntOpDef::iadd.with_log_width(log_width), [counter, one])?
                    .out_wire(0);
                loop_builder.set_outputs(test.out_wire(0), once(counter).chain(cond.outputs()))?;

                // Insert into the current hugr and update context
                let res = builder.add_hugr(loop_builder.hugr().clone());
                let loop_node = res.inserted_entrypoint;
                let test_node = res.node_map.get(&test.node()).unwrap();
                ctx.register_input(
                    op.input(0).unwrap()?.id(),
                    *test_node,
                    builder.hugr().node_inputs(*test_node).nth(1).unwrap(),
                );
                for (port, value) in builder
                    .hugr()
                    .node_inputs(loop_node)
                    .skip(1)
                    .zip(op.inputs().skip(1))
                {
                    ctx.register_input(value?.id(), loop_node, port);
                }
                for (port, value) in builder
                    .hugr()
                    .node_outputs(loop_node)
                    .skip(1)
                    .zip(op.outputs())
                {
                    ctx.register_output(value?.id(), loop_node, port);
                }
                // Insert zero as the intial counter
                let zero = builder.add_load_value(ConstInt::new_u(log_width, 0).unwrap());
                let counter_port = builder.hugr().node_inputs(loop_node).next().unwrap();
                builder
                    .hugr_mut()
                    .connect(zero.node(), zero.source(), loop_node, counter_port);
            }
        }
        Ok(())
    }
}

fn build_nested<H, T>(
    builder: &mut DFGWrapper<H, BuildHandle<T>>,
    region: &Region,
) -> Result<(), JeffToHugrError>
where
    H: AsMut<Hugr>,
    H: AsRef<Hugr>,
{
    let inp_node = builder.input().node();
    let out_node = builder.output().node();
    let mut ctx = BuildContext::default();
    for (port, value) in builder.hugr().node_outputs(inp_node).zip(region.sources()) {
        ctx.register_output(value?.id(), inp_node, port);
    }
    for (port, value) in builder.hugr().node_inputs(out_node).zip(region.targets()) {
        ctx.register_input(value?.id(), out_node, port);
    }
    build_region(*region, builder, &mut ctx)?;
    Ok(())
}

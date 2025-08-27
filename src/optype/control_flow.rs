use hugr::builder::{
    ConditionalBuilder, Container as _, Dataflow, DataflowSubContainer, SubContainer,
    TailLoopBuilder,
};
use hugr::extension::prelude::bool_t;
use hugr::ops::handle::NodeHandle;
use hugr::std_extensions::arithmetic::int_ops::IntOpDef;
use hugr::std_extensions::arithmetic::int_types::int_type;
use hugr::types::{Signature, SumType, TypeRow};
use hugr::{HugrView as _, type_row};
use itertools::Itertools;
use jeff::reader::Region;
use jeff::reader::optype::{self as jeff_optype, ControlFlowOp};

use crate::to_hugr::BuildContext;
use crate::types::{jeff_int_width_to_hugr_arg, jeff_int_width_to_hugr_width};
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
            ControlFlowOp::DoWhile { body, condition } => {
                if !itertools::equal(
                    op.input_types().map(|ty| ty.unwrap()),
                    op.output_types().map(|ty| ty.unwrap()),
                ) {
                    return Err(JeffToHugrError::invalid_op_io("DoWhile", op));
                }
                let state_types = op
                    .input_types()
                    .map(|ty| types::jeff_to_hugr(ty.unwrap()))
                    .collect_vec();

                let mut loop_builder = TailLoopBuilder::new(vec![], state_types.clone(), vec![])?;

                let body_dfg = {
                    let mut body_builder = loop_builder.dfg_builder(
                        Signature::new_endo(state_types.clone()),
                        loop_builder.input_wires(),
                    )?;
                    build_nested(&mut body_builder, body)?;
                    body_builder.finish_sub_container()?
                };

                let condition_dfg = {
                    let mut condition_builder = loop_builder.dfg_builder(
                        Signature::new(state_types, vec![bool_t()]),
                        body_dfg.outputs(),
                    )?;
                    build_nested(&mut condition_builder, condition)?;
                    condition_builder.finish_sub_container()?
                };
                let conditional_result = condition_dfg.out_wire(0);

                // TODO: This assumes that the state returned by the body is copyable.
                //
                // See <https://github.com/unitaryfoundation/jeff/issues/4>
                loop_builder.set_outputs(conditional_result, body_dfg.outputs())?;

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
            ControlFlowOp::While { body, condition } => {
                if !itertools::equal(
                    op.input_types().map(|ty| ty.unwrap()),
                    op.output_types().map(|ty| ty.unwrap()),
                ) {
                    return Err(JeffToHugrError::invalid_op_io("DoWhile", op));
                }
                let state_types = op
                    .input_types()
                    .map(|ty| types::jeff_to_hugr(ty.unwrap()))
                    .collect_vec();

                let mut loop_builder = TailLoopBuilder::new(vec![], state_types.clone(), vec![])?;

                let condition_dfg = {
                    let mut condition_builder = loop_builder.dfg_builder(
                        Signature::new(state_types.clone(), vec![bool_t()]),
                        loop_builder.input_wires(),
                    )?;
                    build_nested(&mut condition_builder, condition)?;
                    condition_builder.finish_sub_container()?
                };
                let conditional_result = condition_dfg.out_wire(0);

                let body_conditional = {
                    // TODO: This assumes that the state at the loop_builder input is copyable.
                    //
                    // See <https://github.com/unitaryfoundation/jeff/issues/4>
                    let mut conditional_builder = loop_builder.conditional_builder(
                        ([type_row!(), type_row!()], conditional_result),
                        state_types
                            .clone()
                            .into_iter()
                            .zip(loop_builder.input_wires()),
                        state_types.clone().into(),
                    )?;

                    // False branch
                    {
                        let false_case = conditional_builder.case_builder(0)?;
                        let inputs = false_case.input_wires();
                        false_case.finish_with_outputs(inputs)?;
                    }

                    // True branch
                    {
                        let mut body_builder = conditional_builder.case_builder(1)?;
                        build_nested(&mut body_builder, body)?;
                        body_builder.finish_sub_container()?;
                    }

                    conditional_builder.finish_sub_container()?
                };

                loop_builder.set_outputs(conditional_result, body_conditional.outputs())?;

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
                // Region inputs:
                // - `int(N)`: The (signed) start value.
                // - `int(N)`: The (signed) stop value (exclusive).
                // - `int(N)`: The (signed) step value.
                // - `... state`: Any number of values that are passed to the loop body.
                let Ok(JeffType::Int { bits }) = op.input_types().next().unwrap() else {
                    return Err(JeffToHugrError::invalid_op_io("For", op));
                };
                let log_width = jeff_int_width_to_hugr_width(bits);
                let int_t = || int_type(jeff_int_width_to_hugr_arg(bits));
                let state_types = output_types;

                // Construct a loop that takes
                // - An integer counter
                // - The loop body inputs
                // And then checks if the counter is zero.
                // - If yes, the loop is done.
                // - If no, decrease the counter and run the loop body.
                let loop_hugr = {
                    let mut loop_builder = TailLoopBuilder::new(
                        vec![int_t(), int_t(), int_t()],
                        state_types.clone(),
                        vec![],
                    )?;

                    // Emit check if current iteration is less than the bound
                    let mut input_wires = loop_builder.input_wires();
                    let start_value = input_wires.next().unwrap();
                    let stop_value = input_wires.next().unwrap();
                    let step_value = input_wires.next().unwrap();
                    let state_inputs = input_wires;

                    // Test if the counter is less than the stop value
                    let less_than_stop = loop_builder.add_dataflow_op(
                        IntOpDef::ilt_s.with_log_width(log_width),
                        [start_value, stop_value],
                    )?;

                    // Now branch into two cases, depending on whether the counter is less than the stop value.
                    let condition = {
                        let conditional_sum_type: SumType =
                            SumType::new([vec![int_t(), int_t(), int_t()], vec![]]);
                        let conditional_outputs: TypeRow =
                            std::iter::once(conditional_sum_type.clone().into())
                                .chain(state_types.clone())
                                .collect_vec()
                                .into();
                        let mut cond = loop_builder.conditional_builder(
                            ([type_row![], type_row!()], less_than_stop.out_wire(0)),
                            [
                                (int_t(), start_value),
                                (int_t(), stop_value),
                                (int_t(), step_value),
                            ]
                            .into_iter()
                            .chain(state_types.clone().into_iter().zip(state_inputs)),
                            conditional_outputs,
                        )?;

                        // If the counter is less than the stop value, run the loop body, decrement the counter and return a continue signal.
                        {
                            let mut continue_case = cond.case_builder(1)?;
                            let mut input_wires = continue_case.input_wires();
                            let start_value = input_wires.next().unwrap();
                            let stop_value = input_wires.next().unwrap();
                            let step_value = input_wires.next().unwrap();
                            let state_inputs = input_wires;

                            // Add a DFG region with the loop's body.
                            let body = {
                                let body_inputs = std::iter::once(int_t())
                                    .chain(state_types.clone())
                                    .collect_vec();
                                let body_outputs = state_types.clone();
                                let mut body = continue_case.dfg_builder(
                                    Signature::new(body_inputs, body_outputs),
                                    std::iter::once(start_value).chain(state_inputs),
                                )?;
                                build_nested(&mut body, region)?;
                                body.finish_sub_container()?
                            };

                            // Increment the counter by `step_value`
                            let start_value = continue_case
                                .add_dataflow_op(
                                    IntOpDef::iadd.with_log_width(log_width),
                                    [start_value, step_value],
                                )?
                                .out_wire(0);

                            // Return the new counter value and the continue signal
                            let continue_flag = continue_case.make_sum(
                                0,
                                [vec![int_t(), int_t(), int_t()].into(), type_row![]],
                                [start_value, stop_value, step_value],
                            )?;

                            continue_case.set_outputs(
                                std::iter::once(continue_flag).chain(body.outputs()),
                            )?;
                        }

                        // Otherwise, if the counter is greater than or equal to the stop value, return a break signal.
                        {
                            let mut break_case = cond.case_builder(0)?;
                            let mut input_wires = break_case.input_wires();
                            let _start_value = input_wires.next().unwrap();
                            let _stop_value = input_wires.next().unwrap();
                            let _step_value = input_wires.next().unwrap();
                            let state_inputs = input_wires;

                            // Return the break signal
                            let break_flag = break_case.make_sum(
                                1,
                                [vec![int_t(), int_t(), int_t()].into(), type_row![]],
                                [],
                            )?;

                            break_case
                                .set_outputs(std::iter::once(break_flag).chain(state_inputs))?;
                        }

                        cond.finish_sub_container()?
                    };

                    let mut condition_outputs = condition.outputs();
                    let continue_flag = condition_outputs.next().unwrap();
                    let rest = condition_outputs;
                    loop_builder.set_outputs(continue_flag, rest)?;

                    // Avoid validating the resulting hugr, as it may contain unconnected wires in the loop body.
                    // (The build context will connect them at a later stage.)
                    std::mem::take(loop_builder.hugr_mut())
                };

                // Insert into the current hugr and update context
                let res = builder.add_hugr(loop_hugr);
                let loop_node = res.inserted_entrypoint;
                for (port, value) in builder.hugr().node_inputs(loop_node).zip(op.inputs()) {
                    ctx.register_input(value?.id(), loop_node, port);
                }
                for (port, value) in builder.hugr().node_outputs(loop_node).zip(op.outputs()) {
                    ctx.register_output(value?.id(), loop_node, port);
                }
            }
        }
        Ok(())
    }
}

/// Build a region nested inside a builder.
///
/// Uses the builder's input and output nodes for the new `BuildContext` input and output wires.
fn build_nested(
    builder: &mut impl hugr::builder::Dataflow,
    region: &Region,
) -> Result<(), JeffToHugrError> {
    let inp_node = builder.input().node();
    let out_node = builder.output().node();
    let mut ctx = BuildContext::default();
    for (port, value) in builder.hugr().node_outputs(inp_node).zip(region.sources()) {
        ctx.register_output(value?.id(), inp_node, port);
    }
    for (port, value) in builder.hugr().node_inputs(out_node).zip(region.targets()) {
        ctx.register_input(value?.id(), out_node, port);
    }
    ctx.build_region(*region, builder)?;
    Ok(())
}

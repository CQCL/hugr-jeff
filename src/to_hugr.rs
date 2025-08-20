//! _jeff_ to HUGR Translation

use std::collections::BTreeMap;
use std::mem;

use derive_more::{Display, Error, From};
use hugr::builder::{Container, HugrBuilder, ModuleBuilder, SubContainer};
use hugr::hugr::hugrmut::HugrMut;
use hugr::ops::handle::{self, NodeHandle};
use hugr::{Hugr, HugrView, IncomingPort, Node, OutgoingPort};
use jeff::Jeff;
use jeff::reader::ReadJeff;

use crate::optype::JeffToHugrOp;
use crate::types::jeff_signature_to_hugr;

/// Translate a _jeff_ program into a HUGR program.
pub fn jeff_to_hugr(jeff: &Jeff) -> Result<Hugr, JeffToHugrError> {
    build_module(jeff.module())
}

/// Error type for the _jeff_ to HUGR translation.
#[derive(Debug, Display, From, Error)]
#[non_exhaustive]
pub enum JeffToHugrError {
    /// The _jeff_ program is invalid.
    InvalidJeffProgram(jeff::reader::ReadError),
    /// We tried to generate an invalid HUGR program.
    InvalidHugrProgram(Box<hugr::hugr::ValidationError<Node>>),
    /// Internal error while building the HUGR program.
    BuildError(Box<hugr::builder::BuildError>),
    /// The _jeff_ operation is not supported.
    #[display("Unsupported operation: {}", op_name)]
    UnsupportedOperation {
        /// The operation name.
        op_name: String,
    },
}

impl JeffToHugrError {
    /// New [`JeffToHugrError::UnsupportedOperation`] error.
    pub fn unsupported_op(op: &impl std::fmt::Debug) -> Self {
        Self::UnsupportedOperation {
            op_name: format!("{op:?}"),
        }
    }
}

impl From<hugr::hugr::ValidationError<Node>> for JeffToHugrError {
    fn from(err: hugr::hugr::ValidationError<Node>) -> Self {
        Self::InvalidHugrProgram(Box::new(err))
    }
}

impl From<hugr::builder::BuildError> for JeffToHugrError {
    fn from(err: hugr::builder::BuildError) -> Self {
        Self::BuildError(Box::new(err))
    }
}

/// Internal context used while building a HUGR program.
#[derive(Debug, Default, Clone)]
pub(crate) struct BuildContext {
    /// Map from _jeff_ (hyperedge) values to incoming node ports.
    ///
    /// This is used to defer the HUGR node connection until all nodes are created.
    input_edges: BTreeMap<jeff::reader::ValueId, Vec<(Node, IncomingPort)>>,
    /// Map from _jeff_ (hyperedge) values to outgoing node ports.
    ///
    /// This is used to defer the HUGR node connection until all nodes are created.
    output_edges: BTreeMap<jeff::reader::ValueId, Vec<(Node, OutgoingPort)>>,
    /// Map of values that should be merged into other values appearing earlier in the _jeff_.
    ///
    /// This is used to elide swap operations or other no-op ops.
    merged_values: BTreeMap<jeff::reader::ValueId, jeff::reader::ValueId>,
    /// Map from function IDs to HUGR call node inputs ports.
    ///
    /// This is used to defer the HUGR node connection until all functions have been defined.
    function_calls: BTreeMap<jeff::reader::FunctionId, Vec<(Node, IncomingPort)>>,
    /// Register of auxiliary functions that have been added to the HUGR program.
    ///
    /// This is used to re-use the same function node on multiple calls.
    utility_functions: BTreeMap<String, handle::FuncID<true>>,
}

impl BuildContext {
    /// Register an incoming node port to a _jeff_ value.
    pub fn register_input(
        &mut self,
        value_id: jeff::reader::ValueId,
        node: Node,
        port: IncomingPort,
    ) {
        let value_id = self.earliest_id(value_id);
        self.input_edges
            .entry(value_id)
            .or_default()
            .push((node, port));
    }

    /// Register an outgoing node port to a _jeff_ value.
    pub fn register_output(
        &mut self,
        value_id: jeff::reader::ValueId,
        node: Node,
        port: OutgoingPort,
    ) {
        let value_id = self.earliest_id(value_id);
        self.output_edges
            .entry(value_id)
            .or_default()
            .push((node, port));
    }

    /// Register an input port to a function call id.
    pub fn register_function_call(
        &mut self,
        function_id: jeff::reader::FunctionId,
        node: Node,
        port: IncomingPort,
    ) {
        self.function_calls
            .entry(function_id)
            .or_default()
            .push((node, port));
    }

    /// Signal that a value should be merged into another appearing earlier in the region.
    ///
    /// This is used to elide no-op operations.
    pub fn merge_with_earlier(
        &mut self,
        value_id: jeff::reader::ValueId,
        earlier_id: jeff::reader::ValueId,
    ) {
        self.merged_values.insert(value_id, earlier_id);
        if let Some(edges) = self.input_edges.remove(&value_id) {
            self.input_edges
                .entry(earlier_id)
                .or_default()
                .extend(edges);
        }
        if let Some(edges) = self.output_edges.remove(&value_id) {
            self.output_edges
                .entry(earlier_id)
                .or_default()
                .extend(edges);
        }
    }

    /// Returns the earliest value id that should be used for a given value.
    ///
    /// Follows the list of merged values until it reaches the earliest one.
    fn earliest_id(&self, value_id: jeff::reader::ValueId) -> jeff::reader::ValueId {
        let mut value_id = value_id;
        while let Some(earlier_id) = self.merged_values.get(&value_id) {
            value_id = *earlier_id;
        }
        value_id
    }
}

/// Build the HUGR program by traversing the _jeff_.
fn build_module(module: jeff::reader::Module<'_>) -> Result<Hugr, JeffToHugrError> {
    let mut builder = ModuleBuilder::new();
    let mut ctx = BuildContext::default();

    // A map between _jeff_ (sequential) function IDs and HUGR function nodes.
    let mut function_nodes: Vec<Node> = vec![];

    for func in module.functions() {
        let name = func.name();
        let body = func.body();

        let fn_inputs = func.input_types().collect::<Result<Vec<_>, _>>()?;
        let fn_outputs = func.output_types().collect::<Result<Vec<_>, _>>()?;
        let signature = jeff_signature_to_hugr(fn_inputs, fn_outputs);
        let mut fn_builder = builder.define_function(name, signature)?;

        build_region(body, &mut fn_builder, &mut ctx)?;

        let fn_node = fn_builder.finish_sub_container()?.node();
        function_nodes.push(fn_node);
    }

    // Connect the function calls.
    for (func_id, inputs) in ctx.function_calls {
        let fn_node = function_nodes[func_id as usize];
        for (node, port) in inputs {
            builder
                .hugr_mut()
                .connect(fn_node, OutgoingPort::from(0), node, port);
        }
    }

    let hugr = builder.hugr().clone();
    if let Err(e) = builder.finish_hugr() {
        eprintln!("Failed to build HUGR program: {e}");
    };
    Ok(hugr)
}

/// Build a HUGR dataflow graph from a _jeff_ region.
pub fn build_region(
    region: jeff::reader::Region<'_>,
    builder: &mut impl hugr::builder::Dataflow,
    ctx: &mut BuildContext,
) -> Result<(), JeffToHugrError> {
    // Each function keeps a separate list of values, while sharing the function table from the module.
    ctx.input_edges.clear();
    ctx.output_edges.clear();

    // Start by adding the input and output connections to the maps.
    let [in_node, out_node] = builder.io();
    for (output_port, value) in region.sources().enumerate() {
        let value = value?;
        let hugr_port = OutgoingPort::from(output_port);
        ctx.register_output(value.id(), in_node, hugr_port);
    }
    for (input_port, value) in region.targets().enumerate() {
        let value = value?;
        let hugr_port = IncomingPort::from(input_port);
        ctx.register_input(value.id(), out_node, hugr_port);
    }

    // Add all the nodes to the dataflow region,
    // and register the ports that will need to be connected later.
    for op in region.operations() {
        op.op_type().build_hugr_op(&op, builder, ctx)?;
    }

    // Add all the missing edges.
    let output_edges = mem::take(&mut ctx.output_edges);
    for (value_id, outputs) in output_edges {
        let Some(inputs) = ctx.input_edges.get(&value_id) else {
            continue;
        };
        for (out_node, out_port) in outputs {
            for (in_node, in_port) in inputs {
                builder
                    .hugr_mut()
                    .connect(out_node, out_port, *in_node, *in_port);

                // Insert an order edge if it was a non-local edge
                let in_parent = builder.hugr().get_parent(*in_node);
                let out_parent = builder.hugr().get_parent(out_node);
                if out_parent != in_parent {
                    let mut curr_node = *in_node;
                    let mut curr_parent = in_parent;
                    while curr_parent != out_parent {
                        let Some(p) = curr_parent else {
                            panic!("Bad nonlocal edge");
                        };
                        curr_parent = builder.hugr().get_parent(p);
                        curr_node = p;
                    }
                    builder.hugr_mut().add_other_edge(out_node, curr_node);
                }
            }
        }
    }

    Ok(())
}

/// Define and call an utility function.
///
/// Stores the function node in the context so it can be reused.
#[expect(unused)]
pub(crate) fn build_helper_call(
    fn_name: &str,
    fn_builder: impl FnOnce(
        &str,
        ModuleBuilder<&mut Hugr>,
    ) -> Result<handle::FuncID<true>, JeffToHugrError>,
    op: &jeff::reader::Operation<'_>,
    builder: &mut impl hugr::builder::Dataflow,
    ctx: &mut BuildContext,
) -> Result<(), JeffToHugrError> {
    let func_node = match ctx.utility_functions.contains_key(fn_name) {
        true => ctx.utility_functions[fn_name],
        false => {
            let old_entrypoint = builder.hugr().entrypoint();
            let module_node = builder.hugr().module_root();
            builder.hugr_mut().set_entrypoint(module_node);
            let module_builder = ModuleBuilder::with_hugr(builder.hugr_mut());
            let node = fn_builder(fn_name, module_builder)?;
            builder.hugr_mut().set_entrypoint(old_entrypoint);
            ctx.utility_functions.insert(fn_name.to_string(), node);
            node
        }
    };
    let node = builder.call(&func_node, &[], [])?.node();

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

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::{catalyst_simple, qarray, qubits};
    use hugr::HugrView;
    use rstest::rstest;

    #[rstest]
    #[case::qubits(qubits())]
    #[case::catalyst_simple(catalyst_simple())]
    // #[case::catalyst_simple(catalyst_tket_opt())]
    #[case::qarray(qarray())]
    fn test_to_hugr_qubits(#[case] jeff: Jeff<'static>) {
        let hugr = jeff_to_hugr(&jeff).unwrap();

        println!("{}", hugr.mermaid_string());
    }
}

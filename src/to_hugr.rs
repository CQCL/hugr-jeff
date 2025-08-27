//! _jeff_ to HUGR Translation

use std::collections::BTreeMap;
use std::mem;

use derive_more::{Display, Error, From};
use hugr::builder::{Container, HugrBuilder, ModuleBuilder, SubContainer};
use hugr::hugr::hugrmut::HugrMut;
use hugr::ops::handle::{self, NodeHandle};
use hugr::{Hugr, HugrView, IncomingPort, Node, OutgoingPort};
use itertools::Itertools;
use jeff::Jeff;
use jeff::reader::ReadJeff;

use crate::optype::JeffToHugrOp;
use crate::types::jeff_signature_to_hugr;

/// Translate a _jeff_ program into a HUGR program.
pub fn jeff_to_hugr(jeff: &Jeff) -> Result<Hugr, JeffToHugrError> {
    BuildContext::build_module(jeff.module())
}

/// Error type for the _jeff_ to HUGR translation.
#[derive(Debug, Display, From, Error)]
#[non_exhaustive]
pub enum JeffToHugrError {
    /// The input/outputs to a jeff operation are not compatible with the
    /// operation type.
    #[display(
        "Invalid operation I/O. {op} had input types {input_types} and output types {output_types}",
        input_types = input_types.iter().join(", "),
        output_types = output_types.iter().join(", "),
    )]
    InvalidOperationIO {
        /// The operation name.
        op: String,
        /// The input types.
        input_types: Vec<String>,
        /// The output types.
        output_types: Vec<String>,
    },
    /// The _jeff_ file is invalid.
    MalformedJeffFile(jeff::reader::ReadError),
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

    /// New [`JeffToHugrError::InvalidOperationIO`] error.
    pub fn invalid_op_io(name: impl ToString, op: &jeff::reader::Operation<'_>) -> Self {
        let input_types = match op
            .input_types()
            .map(|ty| ty.map(|t| t.to_string()))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(input_types) => input_types,
            Err(e) => {
                return Self::MalformedJeffFile(e);
            }
        };
        let output_types = match op
            .output_types()
            .map(|ty| ty.map(|t| t.to_string()))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(output_types) => output_types,
            Err(e) => {
                return Self::MalformedJeffFile(e);
            }
        };
        Self::InvalidOperationIO {
            op: name.to_string(),
            input_types,
            output_types,
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
    input_edges: BTreeMap<jeff::reader::value::ValueId, Vec<(Node, IncomingPort)>>,
    /// Map from _jeff_ (hyperedge) values to outgoing node ports.
    ///
    /// This is used to defer the HUGR node connection until all nodes are created.
    output_edges: BTreeMap<jeff::reader::value::ValueId, Vec<(Node, OutgoingPort)>>,
    /// Map of values that should be merged into other values appearing earlier in the _jeff_.
    ///
    /// This is used to elide swap operations or other no-op ops.
    merged_values: BTreeMap<jeff::reader::value::ValueId, jeff::reader::value::ValueId>,
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
        value_id: jeff::reader::value::ValueId,
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
        value_id: jeff::reader::value::ValueId,
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
        value_id: jeff::reader::value::ValueId,
        earlier_id: jeff::reader::value::ValueId,
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
    fn earliest_id(&self, value_id: jeff::reader::value::ValueId) -> jeff::reader::value::ValueId {
        let mut value_id = value_id;
        while let Some(earlier_id) = self.merged_values.get(&value_id) {
            value_id = *earlier_id;
        }
        value_id
    }

    /// Build the HUGR program by traversing the _jeff_.
    fn build_module(module: jeff::reader::Module<'_>) -> Result<Hugr, JeffToHugrError> {
        let mut builder = ModuleBuilder::new();
        let mut ctx = BuildContext::default();

        // A map between _jeff_ (sequential) function IDs and HUGR function nodes.
        let mut function_nodes: Vec<Node> = vec![];

        for func in module.functions() {
            let name = func.name();
            let fn_inputs = func
                .input_types()
                .map(|port| Ok(port?.ty()))
                .collect::<Result<Vec<_>, JeffToHugrError>>()?;
            let fn_outputs = func
                .output_types()
                .map(|port| Ok(port?.ty()))
                .collect::<Result<Vec<_>, JeffToHugrError>>()?;
            let signature = jeff_signature_to_hugr(fn_inputs, fn_outputs);

            match func {
                jeff::reader::Function::Definition(def) => {
                    let body = def.body();
                    let mut fn_builder = builder.define_function(name, signature)?;

                    ctx.build_region(body, &mut fn_builder)?;

                    let fn_node = fn_builder.finish_sub_container()?.node();
                    function_nodes.push(fn_node);
                }
                jeff::reader::Function::Declaration(_) => {
                    let fn_decl = builder.declare(name, signature.into())?;
                    function_nodes.push(fn_decl.node());
                }
            }
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
        &mut self,
        region: jeff::reader::Region<'_>,
        builder: &mut impl hugr::builder::Dataflow,
    ) -> Result<(), JeffToHugrError> {
        // Each function keeps a separate list of values, while sharing the function table from the module.
        self.input_edges.clear();
        self.output_edges.clear();

        // Start by adding the input and output connections to the maps.
        let [in_node, out_node] = builder.io();
        for (output_port, value) in region.sources().enumerate() {
            let value = value?;
            let hugr_port = OutgoingPort::from(output_port);
            self.register_output(value.id(), in_node, hugr_port);
        }
        for (input_port, value) in region.targets().enumerate() {
            let value = value?;
            let hugr_port = IncomingPort::from(input_port);
            self.register_input(value.id(), out_node, hugr_port);
        }

        // Add all the nodes to the dataflow region,
        // and register the ports that will need to be connected later.
        for op in region.operations() {
            op.op_type().build_hugr_op(&op, builder, self)?;
        }

        // Add all the missing edges.
        self.connect_hyperedges(builder)?;

        Ok(())
    }

    /// Connect all the hyperedges between inputs and outputs with the same value id.
    ///
    /// See [`BuildContext::register_input`] and [`BuildContext::register_output`] for more details.
    fn connect_hyperedges(
        &mut self,
        builder: &mut impl hugr::builder::Dataflow,
    ) -> Result<(), JeffToHugrError> {
        let output_edges = mem::take(&mut self.output_edges);
        for (value_id, outputs) in output_edges {
            let Some(inputs) = self.input_edges.get(&value_id) else {
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
        &mut self,
        fn_name: &str,
        fn_builder: impl FnOnce(
            &str,
            ModuleBuilder<&mut Hugr>,
        ) -> Result<handle::FuncID<true>, JeffToHugrError>,
        op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
    ) -> Result<(), JeffToHugrError> {
        let func_node = match self.utility_functions.contains_key(fn_name) {
            true => self.utility_functions[fn_name],
            false => {
                let old_entrypoint = builder.hugr().entrypoint();
                let module_node = builder.hugr().module_root();
                builder.hugr_mut().set_entrypoint(module_node);
                let module_builder = ModuleBuilder::with_hugr(builder.hugr_mut());
                let node = fn_builder(fn_name, module_builder)?;
                builder.hugr_mut().set_entrypoint(old_entrypoint);
                self.utility_functions.insert(fn_name.to_string(), node);
                node
            }
        };
        let node = builder.call(&func_node, &[], [])?.node();

        // Note: the `zip` will stop when the _jeff_ operation inputs are
        // exhausted, so it won't register the static function parameters of the
        // call.
        for (port, value) in builder.hugr().node_inputs(node).zip(op.inputs()) {
            let value = value?;
            self.register_input(value.id(), node, port);
        }
        for (port, value) in builder.hugr().node_outputs(node).zip(op.outputs()) {
            let value = value?;
            self.register_output(value.id(), node, port);
        }

        Ok(())
    }

    /// Emit a single HUGR operation in the node, and register its inputs and outputs.
    pub fn build_single_op(
        &mut self,
        op: impl Into<hugr::ops::OpType>,
        jeff_op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
    ) -> Result<(), JeffToHugrError> {
        let node = builder.add_child_node(op.into());

        for (port, value) in builder.hugr().node_inputs(node).zip(jeff_op.inputs()) {
            self.register_input(value?.id(), node, port);
        }
        for (port, value) in builder.hugr().node_outputs(node).zip(jeff_op.outputs()) {
            self.register_output(value?.id(), node, port);
        }

        Ok(())
    }

    /// Mark a jeff operation that does not produce any HUGR output values.
    ///
    /// Merges the input values with its outputs in the context.
    /// Ignores extra parameters in the input if possible.
    ///
    /// # Errors
    ///
    /// If the operation outputs cannot be matched to the inputs.
    pub fn build_transparent_op(
        &mut self,
        jeff_op: &jeff::reader::Operation<'_>,
    ) -> Result<(), JeffToHugrError> {
        for (input, output) in jeff_op.inputs().zip(jeff_op.outputs()) {
            let input = input?;
            let output = output?;

            if input.ty() != output.ty() {
                return Err(JeffToHugrError::unsupported_op(jeff_op));
            }

            self.merge_with_earlier(output.id(), input.id());
        }
        Ok(())
    }

    /// Helper function to convert _jeff_ constant values into HUGR constant / loadConstant pairs.
    pub fn build_constant_value(
        &mut self,
        value: impl Into<hugr::ops::Value>,
        jeff_op: &jeff::reader::Operation<'_>,
        builder: &mut impl hugr::builder::Dataflow,
    ) -> Result<(), JeffToHugrError> {
        let wire = builder.add_load_value(value.into());

        // Constant ops in _jeff_ have no inputs and a single output.
        if jeff_op.input_count() != 0 || jeff_op.output_count() != 1 {
            return Err(JeffToHugrError::unsupported_op(jeff_op));
        }
        let value = jeff_op.output(0).unwrap()?;

        self.register_output(value.id(), wire.node(), wire.source());
        Ok(())
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::test::{catalyst_simple, catalyst_tket_opt, qubits};
    use hugr::HugrView;
    use rstest::rstest;

    #[rstest]
    #[case::qubits(qubits())]
    #[case::catalyst_simple(catalyst_simple())]
    #[case::catalyst_tket(catalyst_tket_opt())]
    fn test_to_hugr_qubits(#[case] jeff: Jeff<'static>) {
        let hugr = jeff_to_hugr(&jeff).unwrap();

        hugr.validate().unwrap_or_else(|e| panic!("{e}"));
    }
}

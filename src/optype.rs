//! Translation between _jeff_ and HUGR operation types

mod control_flow;
mod float;
mod function;
mod int;
mod int_array;
mod qubit;
mod qubit_array;
mod to_hugr;

pub(crate) use to_hugr::JeffToHugrOp;

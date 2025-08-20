//! # hugr-jeff
//!
//! `hugr-jeff` is a translation layer between Quantinuum's HUGR IR and the
//! _jeff_ exchange format.
//!
//! See
//! - hugr: github.com/cqcl/hugr
//! - _jeff_: github.com/jeff-org/jeff

mod to_hugr;
mod to_jeff;

#[cfg(test)]
mod test;

pub mod extension;
pub mod optype;
pub mod types;

pub use to_hugr::{JeffToHugrError, jeff_to_hugr};
pub use to_jeff::HugrToJeffError;

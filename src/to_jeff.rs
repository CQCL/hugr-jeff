//! HUGR to _jeff_ Translation

use derive_more::{Display, Error, From};

/// Error type for the HUGR to _jeff_ translation.
#[derive(Debug, Display, From, Error)]
#[non_exhaustive]
pub enum HugrToJeffError {
    /// The HUGR type cannot be converted to _jeff_.
    #[display("HUGR type '{hugr_type}' cannot be converted to jeff")]
    UnsupportedType {
        /// The HUGR type that cannot be converted.
        hugr_type: String,
    },
}

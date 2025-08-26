//! Utility generator functions for testing.

use core::panic;
use std::path::PathBuf;

use jeff::Jeff;

const TEST_PROGRAMS_DIR: &str = "test_files/";

/// Simple catalyst program with qubit arrays
#[rstest::fixture]
pub fn catalyst_simple() -> Jeff<'static> {
    load_example_program("catalyst_simple")
}

/// Simple _jeff_ quantum circuit program using only qubit types.
#[rstest::fixture]
pub fn qubits() -> Jeff<'static> {
    load_example_program("qubits")
}

/// An example of a very simple kernel, with no inputs and no outputs.
///
/// It allocates 5 qubits and fully entangles them, performing a measurement into a classical int array.
#[rstest::fixture]
pub fn entangled_qs() -> Jeff<'static> {
    load_example_program("entangled_qs")
}

/// An example of a more complex set of functions, directly translated from C++.
///
/// The main function allocates 5 qubits and fully entangles them, performing a measurement into a classical int array. It then collects all the measurements into a single int by shl+adding them and returns the result. The wrapping function simply calls the main one.
#[rstest::fixture]
pub fn entangled_calls() -> Jeff<'static> {
    load_example_program("entangled_calls")
}

/// A Catalyst example using a for loop.
///
/// The tket phase folding pass should be able to cancel two T gates in this program.
#[rstest::fixture]
pub fn catalyst_tket_opt() -> Jeff<'static> {
    load_example_program("catalyst_tket_opt")
}

/// Load the example program by copying the file to an internal buffer.
fn load_example_program(name: &str) -> Jeff<'static> {
    let filename = format!("{name}.jeff");
    let path = PathBuf::from(TEST_PROGRAMS_DIR).join(name).join(filename);

    let file = std::fs::File::open(&path).unwrap();
    let buffer = std::io::BufReader::new(file);
    Jeff::read(buffer).unwrap_or_else(|e| panic!("Failed to read example program: {}", e))
}

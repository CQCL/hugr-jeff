//! Convert the jeff file passed as parameter into HUGR and print it as mermaid.
//!
//! Usage: jeff_to_hugr <jeff_file>

use clap::Parser;
use core::panic;
use hugr::envelope::EnvelopeConfig;
use std::path::PathBuf;

use hugr::HugrView;
use hugr_jeff::jeff_to_hugr;
use jeff::Jeff;

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The _jeff_ file to convert
    file: String,

    /// Sets an optional output file for HUGR JSON
    #[arg(short, long)]
    output: Option<String>,

    /// Print the hugr as mermaid.
    #[arg(short, long)]
    mermaid: bool,
}

fn main() {
    // Parse command-line arguments
    let args = Args::parse();

    // Read _jeff_ file
    let path = PathBuf::from(args.file);
    let file = std::fs::File::open(&path).unwrap();
    let buffer = std::io::BufReader::new(file);
    let jeff =
        Jeff::read(buffer).unwrap_or_else(|e| panic!("Failed to read example program:\n {}", e));

    // Convert _jeff_ to HUGR
    let hugr =
        jeff_to_hugr(&jeff).unwrap_or_else(|e| panic!("Failed to convert jeff to HUGR:\n {}", e));

    // Print HUGR as mermaid
    if args.mermaid || args.output.is_none() {
        println!("{}", hugr.mermaid_string());
    }

    // Optionally write HUGR JSON to output file
    if let Some(output) = args.output {
        let json = hugr.store_str(EnvelopeConfig::text()).unwrap_or_else(|e| {
            panic!("Failed to serialize HUGR:\n {}", e);
        });
        std::fs::write(output, json).unwrap();
    }
}

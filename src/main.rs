use ltmatrix::cli::Args;

fn main() {
    println!("ltmatrix - Long-Time Agent Orchestrator");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    // TODO: Initialize and run CLI
    let args = Args::parse();
    if let Err(e) = ltmatrix::run(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

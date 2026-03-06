use std::path::PathBuf;

fn main() {
    let output_dir = PathBuf::from("./target/man_pages");
    println!("Generating man pages to: {:?}", output_dir);

    match ltmatrix::man::generate_man_pages(&output_dir) {
        Ok(_) => println!("✓ Man pages generated successfully"),
        Err(e) => {
            eprintln!("✗ Failed to generate man pages: {:?}", e);
            std::process::exit(1);
        }
    }

    // Read and print the first part of the main man page
    let main_man = output_dir.join("ltmatrix.1");
    match std::fs::read_to_string(&main_man) {
        Ok(content) => {
            let preview: String = content.chars().take(500).collect();
            println!("\n=== Preview of ltmatrix.1 ===");
            println!("{}", preview);
            println!("=== End preview ===\n");

            // Check what's in it
            println!("Contains .TH: {}", content.contains(".TH"));
            println!("Contains .SH: {}", content.contains(".SH"));
            println!("Contains .PP: {}", content.contains(".PP"));
            println!("Contains 'ltmatrix': {}", content.contains("ltmatrix"));
        }
        Err(e) => {
            eprintln!("Failed to read man page: {:?}", e);
        }
    }
}

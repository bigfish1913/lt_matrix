use std::fs;
use std::path::PathBuf;

#[test]
fn debug_man_page_content() {
    let output_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("debug_man");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    ltmatrix::man::generate_man_pages(&output_dir).expect("Man page generation should succeed");

    let main_man_page = output_dir.join("ltmatrix.1");
    let content = fs::read_to_string(&main_man_page).expect("Failed to read man page");
    
    println!("=== Man page content (first 1000 chars) ===");
    println!("{}", &content.chars().take(1000).collect::<String>());
    println!("=== End of preview ===");
    
    println!("\n=== Checking for macros ===");
    println!("Contains .TH: {}", content.contains(".TH"));
    println!("Contains .SH: {}", content.contains(".SH"));
    println!("Contains .PP: {}", content.contains(".PP"));
}

//! Example: Demonstrate framework detection capabilities
use std::path::Path;
use ltmatrix::pipeline::test::{detect_test_framework, TestFramework};

fn main() {
    println!("Testing framework detection on ltmatrix project...");
    
    let project_dir = Path::new(".");
    match detect_test_framework(project_dir) {
        Ok(detection) => {
            println!("Detected Framework: {}", detection.framework.display_name());
            println!("Confidence: {:.1}%", detection.confidence * 100.0);
            println!("Test Command: {}", detection.framework.test_command());
            
            if !detection.config_files.is_empty() {
                println!("Config Files:");
                for file in &detection.config_files {
                    println!("  - {}", file.display());
                }
            }
            
            if !detection.test_paths.is_empty() {
                println!("Test Paths:");
                for path in &detection.test_paths {
                    println!("  - {}", path.display());
                }
            }
        }
        Err(e) => {
            eprintln!("Error detecting framework: {}", e);
        }
    }
}

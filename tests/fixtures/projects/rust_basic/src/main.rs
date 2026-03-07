//! Basic Rust project fixture for testing
//!
//! This file represents a simple Rust project structure for use in tests.

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        assert!(true);
    }
}

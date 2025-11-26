//! Build script to generate Clorinde code from SQL queries.
//!
//! This runs during `cargo build` and generates type-safe Rust code
//! from the SQL queries in the `queries/` directory.

fn main() {
    // Rerun if queries change
    println!("cargo:rerun-if-changed=queries/");
    println!("cargo:rerun-if-changed=migrations/");
    println!("cargo:rerun-if-changed=clorinde.toml");

    // Note: Clorinde code generation requires a running database.
    // For CI/CD, we pre-generate the code and commit it.
    // For local dev, run: `clorinde generate` manually or via Tilt.
    //
    // The generated code lives in src/generated/ and is checked into git.
}

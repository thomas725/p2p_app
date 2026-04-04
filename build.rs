// https://docs.rs/diesel_migrations/latest/diesel_migrations/macro.embed_migrations.html
// Due to limitations in rusts proc-macro API there is currently no way to signal that a
// specific proc macro should be rerun if some external file changes/is added. This implies
// that embed_migrations! cannot regenerate the list of embedded migrations if only the
// migrations are changed. This limitation can be solved by adding a custom build.rs file
// to your crate, such that the crate is rebuild if the migration directory changes.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // println!("cargo:rerun-if-changed=path/to/your/migration/dir/relative/to/your/Cargo.toml");
    println!("cargo:rerun-if-changed=migrations");
    Ok(())
}

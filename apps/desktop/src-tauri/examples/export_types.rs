fn main() -> Result<(), Box<dyn std::error::Error>> {
    pc_care::domain::bindings::export(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/generated"),
    )?;
    Ok(())
}

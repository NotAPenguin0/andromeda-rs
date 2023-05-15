fn main() -> std::io::Result<()> {
    // We cannot commit empty directories on git, so create the output
    // directory for shaders here
    std::fs::create_dir("../../shaders/src/out")?;
    Ok(())
}

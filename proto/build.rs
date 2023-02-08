fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../proto/scheduler.proto")?;
    tonic_build::compile_protos("../proto/dispatcher.proto")?;
    Ok(())
}

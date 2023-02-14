use std::{path::PathBuf, env};

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .file_descriptor_set_path(out_dir.join("file_descriptor.bin"))
        .compile(
            &[
                "../proto/scheduler.proto",
                "../proto/dispatcher.proto",
                "../proto/trigger.proto",
                "../proto/event.proto",
            ],
            &["../proto"],
        )?;
    Ok(())
}

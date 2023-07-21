use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let descriptor_path = out_dir.join("file_descriptor.bin");

    tonic_build::configure()
        .message_attribute(".", "#[derive(::dto::ProstMessageExt)]")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_well_known_types(true)
        .extern_path(".google.protobuf", "::pbjson_types")
        .file_descriptor_set_path(descriptor_path.clone())
        .compile(
            &[
                "./common.proto",
                "./scheduler.proto",
                "./dispatcher.proto",
                "./project_svc.proto",
                "./trigger.proto",
                "./run.proto",
                "./attempt.proto",
                "./events.proto",
            ],
            &["../proto"],
        )?;

    let descriptor_set = std::fs::read(descriptor_path)?;
    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)?
        // Add more packages here as needed.
        .build(&[
            ".events",
            ".common",
            ".trigger_proto",
            ".attempt_proto",
            ".project_svc_proto",
        ])?;

    Ok(())
}

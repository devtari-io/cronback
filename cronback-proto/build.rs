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
                "./attempts.proto",
                "./common.proto",
                "./dispatcher_svc.proto",
                "./events.proto",
                "./metadata_svc.proto",
                "./projects.proto",
                "./runs.proto",
                "./scheduler_svc.proto",
                "./triggers.proto",
            ],
            &["../proto"],
        )?;

    let descriptor_set = std::fs::read(descriptor_path)?;
    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)?
        // Add more packages here as needed.
        .build(&[
            ".attempts",
            ".common",
            ".events",
            ".projects",
            ".triggers",
        ])?;

    Ok(())
}

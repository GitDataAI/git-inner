use std::path::PathBuf;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_files: Vec<PathBuf> = WalkDir::new("./proto")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "proto")
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    let proto_includes = &[PathBuf::from("./proto")];

    println!("cargo:rerun-if-changed=proto");

    tonic_prost_build::configure()
        .build_server(true)
        .out_dir("./src/rpc")
        .compile_protos(&proto_files, proto_includes)?;

    Ok(())
}

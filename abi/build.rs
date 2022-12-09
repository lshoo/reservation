fn main() {
    tonic_build::configure()
        .out_dir("src/pb")
        .compile(&["protos/reservation.proto"], &["protos"])
        .unwrap();

    // fs::remove_file("src/pb/google.protobuf.rs").unwarp();

    println!("cargo:rerun-if-changed=protos/reservation.proto");
}

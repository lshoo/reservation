use std::process::Command;

use proto_builder_trait::tonic::BuilderAttributes;

fn main() {
    tonic_build::configure()
        .out_dir("src/pb")
        .with_sqlx_type(&["reservation.ReservationStatus"])
        .with_derive_builder(&[
            "reservation.ReservationQuery",
            "reservation.ReservationFilter",
        ])
        .with_derive_builder_into(
            "reservation.ReservationQuery",
            &["resource_id", "user_id", "status", "page", "desc"],
        )
        .with_derive_builder_into(
            "reservation.ReservationFilter",
            &["resource_id", "user_id", "status", "page", "desc"],
        )
        .with_derive_builder_option("reservation.ReservationFilter", &["cursor"])
        .with_derive_builder_option("reservation.ReservationQuery", &["start", "end"])
        .with_field_attributes(
            &["page_size"],
            &["#[builder(setter(into), default = \"10\")]"],
        )
        .compile(&["protos/reservation.proto"], &["protos"])
        .unwrap();

    Command::new("cargo").args(["fmt"]).output().unwrap();
    // fs::remove_file("src/pb/google.protobuf.rs").unwarp();

    println!("cargo:rerun-if-changed=protos/reservation.proto");
}

// trait BuilderExt {
//     fn with_sql_type(self, paths: &[&str]) -> Self;
//     fn with_builder(self, paths: &[&str]) -> Self;
//     fn with_builder_into(self, path: &str, fields: &[&str]) -> Self;
//     fn with_builder_option(self, path: &str, fields: &[&str]) -> Self;
// }

// impl BuilderExt for Builder {
//     fn with_sql_type(self, paths: &[&str]) -> Self {
//         paths.iter().fold(self, |acc, path| {
//             acc.type_attribute(path, "#[derive(sqlx::Type)]")
//         })
//     }

//     fn with_builder(self, paths: &[&str]) -> Self {
//         paths.iter().fold(self, |acc, path| {
//             acc.type_attribute(path, "#[derive(derive_builder::Builder)]")
//         })
//     }

//     fn with_builder_into(self, path: &str, fields: &[&str]) -> Self {
//         fields.iter().fold(self, |acc, field| {
//             acc.field_attribute(
//                 format!("{}.{}", path, field),
//                 "#[builder(setter(into), default)]",
//             )
//         })
//     }

//     fn with_builder_option(self, path: &str, fields: &[&str]) -> Self {
//         fields.iter().fold(self, |acc, field| {
//             acc.field_attribute(
//                 format!("{}.{}", path, field),
//                 "#[builder(setter(into, strip_option))]",
//             )
//         })
//     }
// }

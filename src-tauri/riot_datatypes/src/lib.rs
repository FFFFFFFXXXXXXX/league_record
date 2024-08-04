mod common;
pub mod lcu;
pub mod riot_api;

pub use common::*;
// use specta::Language;

// #[test]
// fn generate_type_bindings() {
//     use specta_typescript::BigIntExportBehavior;

//     specta::function::
//     let types = specta_typescript::Typescript::new()
//         .bigint(BigIntExportBehavior::Number)
//         .export(specta::datatype::TypeMap)
//         .unwrap();
//     specta_typescript::export::ts_with_cfg(
//         "index.d.ts",
//         &ExportConfiguration::new().bigint(BigIntExportBehavior::Number),
//     )
//     .unwrap();
// }

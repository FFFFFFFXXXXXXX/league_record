mod common;
pub mod lcu;
pub mod riot_api;

pub use common::*;

#[test]
fn generate_type_bindings() {
    use specta::ts::{BigIntExportBehavior, ExportConfiguration};

    specta::export::ts_with_cfg(
        "index.d.ts",
        &ExportConfiguration::new().bigint(BigIntExportBehavior::Number),
    )
    .unwrap();
}

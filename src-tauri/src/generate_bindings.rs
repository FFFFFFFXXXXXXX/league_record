#[test]
fn generate_bindings() -> anyhow::Result<()> {
    use crate::{app::AppEvent, commands};
    use specta_typescript::Typescript;
    use tauri_specta::{collect_commands, collect_events};

    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::get_marker_flags,
            commands::set_marker_flags,
            commands::get_recordings_path,
            commands::get_recordings_size,
            commands::get_recordings_list,
            commands::open_recordings_folder,
            commands::delete_video,
            commands::rename_video,
            commands::get_metadata,
            commands::toggle_favorite
        ])
        .events(collect_events![AppEvent])
        .export(
            Typescript::new().bigint(specta_typescript::BigIntExportBehavior::Number),
            "../src/bindings.ts",
        )?;

    Ok(())
}

#[test]
fn generate_type_bindings() -> anyhow::Result<()> {
    use crate::{recorder::MetadataFile, state::Settings};
    use specta::{function::fn_datatype, TypeMap};
    use specta_typescript::{export_named_datatype, BigIntExportBehavior, Typescript};

    #[specta::specta]
    fn _tmp(_types: (Settings, MetadataFile)) {}

    let mut type_map = TypeMap::default();
    _ = fn_datatype!(_tmp)(&mut type_map);

    let exports = type_map
        .iter()
        .map(|(_sid, ndt)| {
            export_named_datatype(
                &Typescript::default().bigint(BigIntExportBehavior::Number),
                ndt,
                &type_map,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.join("\n"))?;

    std::fs::write("../league_record_types/index.d.ts", exports)?;

    Ok(())
}

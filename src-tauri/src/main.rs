#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! ValSync - Valorant settings transfer between accounts.
//!
//! Module map:
//! - `model`     accounts, setting values, bundles, snapshots
//! - `crosshair` crosshair profile code parser / encoder
//! - `diff`      per-setting diff and cherry-pick apply
//! - `vault`     versioned snapshots in %APPDATA%\ValSync
//! - `transfer`  one-to-many queue, auto-backup, restore
//! - `riot`      Riot Client lockfile + process detection (API is a TODO trait)
//! - `sample`    demo data until real import exists
//! - `commands`  Tauri IPC surface

mod commands;
mod crosshair;
mod diff;
mod error;
mod model;
mod riot;
mod sample;
mod state;
mod transfer;
mod vault;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn main() {
    let state = match state::AppState::init() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ValSync failed to open its vault: {e}");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_overview,
            commands::category_counts,
            commands::add_account,
            commands::compute_diff,
            commands::apply_selected,
            commands::start_transfer,
            commands::recent_transfers,
            commands::list_backups,
            commands::create_snapshot,
            commands::restore_backup,
            commands::list_crosshairs,
            commands::parse_crosshair,
            commands::encode_crosshair,
            commands::riot_status,
            commands::riot_probe,
            commands::import_from_client,
            commands::push_to_client,
            commands::set_config,
        ])
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Open ValSync", true, None::<&str>)?;
            let backup = MenuItem::with_id(app, "backup", "Back up main account now", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &backup, &quit])?;
            let mut tray = TrayIconBuilder::with_id("valsync").tooltip("ValSync").menu(&menu);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "open" => show_main(app),
                "backup" => {
                    let state = app.state::<state::AppState>();
                    if let Some(main) = state.main_account() {
                        let bundle = state.vault.latest(&main.id).ok().flatten().map(|s| s.bundle).unwrap_or_default();
                        if let Err(e) = state.vault.snapshot(&main.id, "Manual snapshot from tray", model::SnapshotKind::Manual, bundle) {
                            log::error!("tray backup failed: {e}");
                        }
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ValSync");
}

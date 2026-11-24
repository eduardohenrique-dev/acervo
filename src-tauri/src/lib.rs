mod commands;
mod drives;
mod elevation;
mod items;
mod live;
mod operations;
mod queue;
mod scan;
mod secure;
mod state;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};

use acervo_classify::{discover, Classifier};
use acervo_index::ScanMode;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

use state::{AppState, DriveSlot, DriveStatus};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let elevated = elevation::is_elevated();
            let known = discover();
            let classifier = Arc::new(Classifier::standard(&known));
            // Snapshots só numa pasta que apenas SYSTEM e Administradores podem alterar.
            let dir = known.program_data.join("Acervo");
            let snapshot_dir = (elevated && secure::secure_dir(&dir).is_ok()).then_some(dir);
            let mode = if elevated { ScanMode::Mft } else { ScanMode::Walk };
            let drives: BTreeMap<char, DriveSlot> = drives::fixed_drives()
                .into_iter()
                .map(|info| {
                    let slot = DriveSlot {
                        info: info.clone(),
                        status: DriveStatus::Scanning { fraction: 0.0, mode },
                        tree: None,
                        busy: false,
                        live: None,
                    };
                    (info.letter, slot)
                })
                .collect();
            let letters: Vec<char> = drives.keys().copied().collect();
            // Nenhuma operação toca a pasta do Acervo nem a dos dados dele.
            let mut app_dirs = vec![known.program_data.join("Acervo")];
            if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
                app_dirs.push(exe_dir);
            }
            let handle = app.handle().clone();
            let check_handle = app.handle().clone();
            let queue = queue::OpQueue::new(
                Arc::new(|kind, items, progress| acervo_ops::run_operation(kind, items, progress)),
                Arc::new(move |item| operations::precheck(&check_handle.state::<AppState>(), item)),
                Arc::new(move |ev| operations::on_event(&handle, ev)),
            );
            app.manage(AppState {
                drives: RwLock::new(drives),
                classifier,
                elevated,
                snapshot_dir,
                rules_hash: known.fingerprint(),
                queue,
                caches: Mutex::new(HashMap::new()),
                protected: acervo_ops::Protected { app_dirs },
            });

            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Acervo")
                .inner_size(1200.0, 780.0)
                .min_inner_size(960.0, 640.0)
                .decorations(false)
                .transparent(true)
                .effects(EffectsBuilder::new().effect(Effect::Acrylic).build())
                .visible(false)
                .build()?;
            // Diálogos do Windows durante uma operação (ex.: "não cabe na Lixeira, apagar de vez?") na frente do Acervo.
            if let Ok(hwnd) = window.hwnd() {
                acervo_ops::set_owner_window(hwnd.0 as isize);
            }

            for letter in letters {
                scan::start_drive(app.handle().clone(), letter, false);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_drives,
            commands::drive_summary,
            commands::system_breakdown,
            commands::rescan_drive,
            commands::app_is_elevated,
            commands::category_children,
            commands::query_files,
            commands::check_items,
            commands::start_operation,
            commands::reveal_item,
            commands::list_operations,
        ])
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o Acervo")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                scan::shutdown(app);
            }
        });
}

pub mod commands;
pub mod database;
pub mod errors;
pub mod models;
pub mod restore;
pub mod scanner;
pub mod snapshot;
pub mod storage;
pub mod watcher;

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::Manager;

use crate::scanner::Scanner;
use crate::storage::AppStorage;
use crate::watcher::FolderWatcher;

pub struct AppState {
    pub storage: AppStorage,
    pub watchers: Mutex<HashMap<String, FolderWatcher>>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;

            let storage = AppStorage::initialize(app_data_dir)
                .map_err(|error| std::io::Error::other(error.to_string()))?;

            let scanner = Arc::new(Scanner::new(
                storage.database.clone(),
                storage.snapshots.clone(),
            ));

            let mut watchers = HashMap::new();

            // Start monitoring all previously registered folders.
            let folders = storage.database.list_protected_folders()
                .map_err(|error| std::io::Error::other(error.to_string()))?;

            for folder in folders {
                if !folder.enabled {
                    continue;
                }

                match FolderWatcher::start(
                    scanner.clone(),
                    std::path::PathBuf::from(&folder.root_path),
                ) {
                    Ok(watcher) => {
                        watchers.insert(folder.id.clone(), watcher);
                    }
                    Err(error) => {
                        eprintln!(
                            "Could not start watcher for {}: {}",
                            folder.root_path,
                            error
                        );
                    }
                }
            }

            app.manage(AppState {
                storage,
                watchers: Mutex::new(watchers),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_folders,
            commands::scan_folder,
            commands::list_versions,
            commands::restore_version,
            commands::add_folder,
            commands::list_files,
            commands::enable_folder,
            commands::disable_folder,
            commands::remove_folder,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Rewind");
}


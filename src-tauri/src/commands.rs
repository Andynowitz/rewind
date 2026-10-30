use tauri::State;

use crate::errors::AppResult;
use crate::models::{
    FileDto,
    FolderDto,
    RestoreResultDto,
    ScanResultDto,
    VersionDto,
};
use crate::restore::restore_version as perform_restore;
use crate::scanner::Scanner;
use crate::AppState;
use crate::watcher::FolderWatcher;

use std::sync::Arc;
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn list_files(
    state: State<'_, AppState>,
) -> AppResult<Vec<FileDto>> {
    let files = state.storage.database.list_tracked_files()?;

    Ok(files.into_iter().map(FileDto::from).collect())
}

#[tauri::command]
pub fn scan_folder(
    folder_id: String,
    state: State<'_, AppState>,
) -> AppResult<ScanResultDto> {
    let scanner = Scanner::new(
        state.storage.database.clone(),
        state.storage.snapshots.clone(),
    );

    scanner.scan_folder(&folder_id)
}

#[tauri::command]
pub fn list_versions(
    file_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<VersionDto>> {
    let versions = state.storage.database.list_versions(&file_id)?;

    Ok(versions.into_iter().map(VersionDto::from).collect())
}

#[tauri::command]
pub fn restore_version(
    version_id: String,
    state: State<'_, AppState>,
) -> AppResult<RestoreResultDto> {
    let result = perform_restore(
        &state.storage.database,
        &state.storage.snapshots,
        &version_id,
    )?;

    Ok(RestoreResultDto::from(result))
}

#[tauri::command]
pub fn add_folder(
    path: String,
    state: State<'_, AppState>,
) -> AppResult<FolderDto> {
    let folder = state.storage.database.add_protected_folder(
        Path::new(&path),
    )?;

    let scanner = Arc::new(Scanner::new(
        state.storage.database.clone(),
        state.storage.snapshots.clone(),
    ));

    // Capture the folder's existing files.
    if let Err(error) = scanner.scan_folder(&folder.id) {
        eprintln!("Initial scan failed: {error}");
    }

    // Start monitoring future changes.
    match FolderWatcher::start(
        scanner,
        PathBuf::from(&folder.root_path),
    ) {
        Ok(watcher) => {
            state.watchers.lock().insert(folder.id.clone(), watcher);
        }
        Err(error) => {
            eprintln!("Could not start folder watcher: {error}");
        }
    }

    Ok(FolderDto::from(folder))
}

#[tauri::command]
pub fn list_folders(
    state: State<'_, AppState>,
) -> AppResult<Vec<FolderDto>> {
    let folders = state.storage.database.list_protected_folders()?;

    Ok(folders.into_iter().map(FolderDto::from).collect())
}

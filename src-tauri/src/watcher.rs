
use crate::errors::{AppError, AppResult};
use crate::scanner::Scanner;

use notify_debouncer_mini::{
    new_debouncer,
    notify::{RecursiveMode, RecommendedWatcher, Watcher},
    DebounceEventResult,
    Debouncer,
};

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub struct FolderWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
}

impl FolderWatcher {
    pub fn start(
        scanner: Arc<Scanner>,
        root: impl AsRef<Path>,
    ) -> AppResult<Self> {
        let root = root.as_ref().to_path_buf();

        let folder_root = root.clone();

        let mut debouncer = new_debouncer(
            Duration::from_millis(700),
            move |result: DebounceEventResult| {
                Self::handle_events(
                    &scanner,
                    &folder_root,
                    result,
                );
            },
        )
        .map_err(|error| {
            AppError::Message(format!(
                "Unable to initialize filesystem watcher: {error}"
            ))
        })?;

        debouncer
            .watcher()
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|error| {
                AppError::Message(format!(
                    "Unable to watch {}: {error}",
                    root.display()
                ))
            })?;

        Ok(Self {
            _debouncer: debouncer,
        })
    }

    fn handle_events(
        scanner: &Scanner,
        root: &Path,
        result: DebounceEventResult,
    ) {
        match result {
            Ok(events) => {
                for event in events {
                    let path = event.path;

                    if !path.starts_with(root) {
                        continue;
                    }

                    if Self::is_internal_path(&path) {
                        continue;
                    }

                    match std::fs::symlink_metadata(&path) {
                        Ok(metadata) if metadata.is_dir() => {
                            // Directory changes can introduce new files.
                            // A full scan reconciles all accessible children.
                            if let Err(error) = scanner.scan_folder_from_root(root) {
                                eprintln!(
                                    "Rewind directory reconciliation failed: {error}"
                                );
                            }
                        }

                        Ok(metadata) if metadata.is_file() => {
                            if let Err(error) =
                                scanner.scan_changed_path_for_root(root, &path)
                            {
                                eprintln!(
                                    "Rewind file snapshot failed for {}: {}",
                                    path.display(),
                                    error
                                );
                            }
                        }

                        Ok(_) => {}

                        Err(error)
                            if error.kind()
                                == std::io::ErrorKind::NotFound =>
                        {
                            // Deletion handling is performed by the
                            // complete reconciliation implementation.
                        }

                        Err(error) => {
                            eprintln!(
                                "Rewind could not inspect {}: {}",
                                path.display(),
                                error
                            );
                        }
                    }
                }
            }

            Err(error) => {
                eprintln!(
                    "Rewind filesystem watcher error: {error}"
                );
            }
        }
    }

    fn is_internal_path(path: &Path) -> bool {
        path.components().any(|component| {
            let name = component.as_os_str().to_string_lossy();

            name.starts_with(".rewind-restore-")
                || name == "Thumbs.db"
        })
    }
}

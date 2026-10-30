import { invoke } from "@tauri-apps/api/core";
import type {
  Folder,
  FileRecord,
  ScanResult,
  Version,
  RestoreResult
} from "./types";

export const commands = {
  listFolders: () =>
    invoke<Folder[]>("list_folders"),

  addFolder: (path: string) =>
    invoke<Folder>("add_folder", { path }),

  scanFolder: (folderId: string) =>
    invoke<ScanResult>("scan_folder", { folderId }),

  listVersions: (fileId: string) =>
    invoke<Version[]>("list_versions", { fileId }),
  listFiles: () =>
    invoke<FileRecord[]>("list_files"),

  restoreVersion: (versionId: string) =>
    invoke<RestoreResult>("restore_version", { versionId })
};
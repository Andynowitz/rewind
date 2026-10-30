export interface Folder {
  id: string;
  path: string;
  enabled: boolean;
  createdAt: string;
}

export interface FileRecord {
  id: string;
  folderId: string;
  relativePath: string;
  lastHash: string | null;
  lastSeenAt: string | null;
  state: string;
}

export interface Version {
  id: string;
  fileId: string;
  hash: string;
  createdAt: string;
  sizeBytes: number;
  reason: string;
  rootPath: string;
  relativePath: string;
}

export interface ScanResult {
  folderId: string;
  filesScanned: number;
  snapshotsCreated: number;
  unchangedFiles: number;
  errors: string[];
}

export interface RestoreResult {
  operationId: string;
  restoredVersionId: string;
  restoredHash: string;
  previousHash: string | null;
  targetPath: string;
  restoredSize: number;
}

import { useCallback, useEffect, useState } from "react";
import {
  AlertCircle,
  File,
  History,
  RefreshCw
} from "lucide-react";

import VersionTimeline from "../../components/VersionTimeline";
import RestoreDialog from "../../components/RestoreDialog";

import { commands } from "../../lib/commands";
import { formatDate } from "../../lib/formatters";

import type {
  FileRecord,
  Version
} from "../../lib/types";

export default function HistoryPage() {
  const [files, setFiles] = useState<FileRecord[]>([]);
  const [selectedFileId, setSelectedFileId] = useState<string | null>(null);
  const [versions, setVersions] = useState<Version[]>([]);
  const [selectedVersion, setSelectedVersion] = useState<Version | null>(null);

  const [loadingFiles, setLoadingFiles] = useState(true);
  const [loadingVersions, setLoadingVersions] = useState(false);
  const [restoring, setRestoring] = useState(false);

  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  const loadFiles = useCallback(async () => {
    setLoadingFiles(true);
    setError("");

    try {
      const result = await commands.listFiles();
      setFiles(result);

      setSelectedFileId(current => {
        if (current && result.some(file => file.id === current)) {
          return current;
        }

        return result[0]?.id ?? null;
      });
    } catch (err) {
      setError(`Could not load files: ${String(err)}`);
    } finally {
      setLoadingFiles(false);
    }
  }, []);

  const loadVersions = useCallback(async (fileId: string) => {
    setLoadingVersions(true);
    setError("");

    try {
      const result = await commands.listVersions(fileId);
      setVersions(result);
    } catch (err) {
      setError(`Could not load versions: ${String(err)}`);
      setVersions([]);
    } finally {
      setLoadingVersions(false);
    }
  }, []);

  useEffect(() => {
    void loadFiles();
  }, [loadFiles]);

  useEffect(() => {
    if (selectedFileId) {
      void loadVersions(selectedFileId);
    } else {
      setVersions([]);
    }
  }, [selectedFileId, loadVersions]);

  async function handleRestore() {
    if (!selectedVersion) return;

    setRestoring(true);
    setError("");
    setNotice("");

    try {
      const result = await commands.restoreVersion(selectedVersion.id);

      setNotice(`Version restored successfully: ${result.targetPath}`);
      setSelectedVersion(null);

      await loadVersions(selectedVersion.fileId);
      await loadFiles();
    } catch (err) {
      setError(`Restore failed: ${String(err)}`);
    } finally {
      setRestoring(false);
    }
  }

  const selectedFile = files.find(file => file.id === selectedFileId);

  return (
    <div>
      <div className="page-heading">
        <div>
          <div className="eyebrow">VERSION CONTROL</div>
          <h1>File history</h1>
          <p>Explore previous versions and restore earlier files.</p>
        </div>

        <button
          className="secondary-button"
          onClick={() => void loadFiles()}
          disabled={loadingFiles}
        >
          <RefreshCw size={16} />
          Refresh
        </button>
      </div>

      {error && (
        <div className="notice error-notice">
          <AlertCircle size={17} />
          {error}
        </div>
      )}

      {notice && (
        <div className="notice success-notice">
          {notice}
        </div>
      )}

      {loadingFiles ? (
        <section className="panel">
          <div className="empty-state">Loading tracked files...</div>
        </section>
      ) : files.length === 0 ? (
        <section className="panel">
          <div className="empty-state">
            <History size={30} />
            <strong>No tracked files yet</strong>
            <span>
              Add a protected folder and scan it to create your first snapshots.
            </span>
          </div>
        </section>
      ) : (
        <div className="history-layout">
          <section className="panel history-file-panel">
            <div className="panel-heading">
              <strong>Tracked files</strong>
              <span>{files.length} files</span>
            </div>

            {files.map(file => (
              <button
                key={file.id}
                className={`history-file-item ${
                  selectedFileId === file.id ? "selected" : ""
                }`}
                onClick={() => setSelectedFileId(file.id)}
              >
                <File size={18} />

                <span className="history-file-name">
                  <strong>
                    {file.relativePath.split(/[\\/]/).pop()}
                  </strong>
                  <small>{file.relativePath}</small>
                </span>
              </button>
            ))}
          </section>

          <section className="panel history-version-panel">
            {selectedFile ? (
              <>
                <div className="panel-heading">
                  <div>
                    <strong>
                      {selectedFile.relativePath.split(/[\\/]/).pop()}
                    </strong>
                    <p>{selectedFile.relativePath}</p>
                  </div>

                  <span>{versions.length} versions</span>
                </div>

                {loadingVersions ? (
                  <div className="empty-state">
                    Loading version history...
                  </div>
                ) : (
                  <VersionTimeline
                    versions={versions}
                    onRestore={(versionId) => {
                      const version = versions.find(
                        item => item.id === versionId
                      );

                      if (version) {
                        setSelectedVersion(version);
                      }
                    }}
                  />
                )}
              </>
            ) : (
              <div className="empty-state">
                <History size={30} />
                <strong>Select a file</strong>
                <span>Choose a tracked file to see its saved versions.</span>
              </div>
            )}
          </section>
        </div>
      )}

      <RestoreDialog
        open={selectedVersion !== null}
        versionLabel={
          selectedVersion
            ? formatDate(selectedVersion.createdAt)
            : ""
        }
        onCancel={() => setSelectedVersion(null)}
        onConfirm={() => void handleRestore()}
        loading={restoring}
      />
    </div>
  );
}
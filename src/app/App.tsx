import { useCallback, useEffect, useState } from "react";
import { AlertCircle, CheckCircle2, RefreshCw } from "lucide-react";

import Sidebar from "../components/Sidebar";
import DashboardPage from "../features/dashboard/DashboardPage";
import FoldersPage from "../features/folders/FoldersPage";
import HistoryPage from "../features/history/HistoryPage";
import SettingsPage from "../features/settings/SettingsPage";

import { commands } from "../lib/commands";
import type { Folder } from "../lib/types";
import type { AppRoute } from "./routes";
import { open } from "@tauri-apps/plugin-dialog";

export default function App() {
  const [active, setActive] = useState<AppRoute>("dashboard");
  const [folders, setFolders] = useState<Folder[]>([]);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState<string | null>(null);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [addingFolder, setAddingFolder] = useState(false);

  const loadFolders = useCallback(async () => {
    setLoading(true);
    setError("");

    try {
      setFolders(await commands.listFolders());
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadFolders();
  }, [loadFolders]);

  async function handleScan(folderId: string) {
    setScanning(folderId);
    setNotice("");
    setError("");

    try {
      const result = await commands.scanFolder(folderId);
      setNotice(
        `Scan complete: ${result.filesScanned} files scanned, ` +
        `${result.snapshotsCreated} new snapshots created.`
      );
      if (result.errors.length) {
        setError(result.errors.slice(0, 3).join("; "));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setScanning(null);
      await loadFolders();
    }
  }

  async function handleAddFolder() {
    setAddingFolder(true);
    setNotice("");
    setError("");

    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Choose a folder to protect"
      });

      if (!selected) return;

      const folder = await commands.addFolder(selected);

      setNotice(`Folder added successfully: ${folder.path}`);
      await loadFolders();
    } catch (err) {
      setError(`Could not add folder: ${String(err)}`);
    } finally {
      setAddingFolder(false);
    }
  }

  function renderPage() {
    switch (active) {
      case "dashboard":
        return (
          <DashboardPage
            folders={folders}
            loading={loading}
            onScan={handleScan}
            scanning={scanning}
          />
        );
      case "folders":
        return (
          <FoldersPage
            folders={folders}
            onAddFolder={handleAddFolder}
            adding={addingFolder}
          />
        );
      case "history":
        return <HistoryPage />;
      case "settings":
        return <SettingsPage />;
    }
  }

  return (
    <div className="app-shell">
      <Sidebar active={active} onNavigate={setActive} />

      <main className="main-area">
        <header className="topbar">
          <div className="breadcrumb">
            Workspace <span>/</span> Rewind
          </div>
          <button
            className="icon-button"
            onClick={() => void loadFolders()}
            title="Refresh folders"
          >
            <RefreshCw size={17} />
          </button>
        </header>

        <div className="page-content">
          {notice && (
            <div className="notice success-notice">
              <CheckCircle2 size={17} /> {notice}
              <button onClick={() => setNotice("")}>×</button>
            </div>
          )}

          {error && (
            <div className="notice error-notice">
              <AlertCircle size={17} /> {error}
              <button onClick={() => setError("")}>×</button>
            </div>
          )}

          {renderPage()}
        </div>

        <footer className="app-footer">
          <span><span className="status-dot" /> Local-first protection</span>
          <span>REWIND · VERSION 0.1.0</span>
        </footer>
      </main>
    </div>
  );
}

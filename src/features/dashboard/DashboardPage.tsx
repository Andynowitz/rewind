import { Folder, Camera, HardDrive, Activity } from "lucide-react";
import type { Folder as FolderType } from "../../lib/types";

interface Props {
  folders: FolderType[];
  loading: boolean;
  onScan: (id: string) => void;
  scanning: string | null;
}

export default function DashboardPage({
  folders,
  loading,
  onScan,
  scanning
}: Props) {
  const activeFolders = folders.filter(folder => folder.enabled).length;

  return (
    <div>
      <div className="page-heading">
        <div>
          <div className="eyebrow">YOUR WORKSPACE</div>
          <h1>Overview</h1>
          <p>Your files, protected and ready to rewind.</p>
        </div>
      </div>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-icon"><Folder /></div>
          <span>Protected folders</span>
          <strong>{folders.length}</strong>
          <small>{activeFolders} active</small>
        </div>

        <div className="stat-card">
          <div className="stat-icon"><Camera /></div>
          <span>Snapshots created</span>
          <strong>—</strong>
          <small>History statistics coming soon</small>
        </div>

        <div className="stat-card">
          <div className="stat-icon"><HardDrive /></div>
          <span>Storage used</span>
          <strong>—</strong>
          <small>Local snapshot storage</small>
        </div>

        <div className="stat-card">
          <div className="stat-icon"><Activity /></div>
          <span>Monitoring</span>
          <strong>{activeFolders > 0 ? "Active" : "Idle"}</strong>
          <small>Folder watcher status</small>
        </div>
      </div>

      <section className="panel">
        <div className="panel-heading">
          <div>
            <h2>Protected folders</h2>
            <p>Folders currently registered with Rewind.</p>
          </div>
        </div>

        {loading ? (
          <div className="empty-state">Loading folders...</div>
        ) : folders.length === 0 ? (
          <div className="empty-state">
            <Folder size={30} />
            <strong>No protected folders yet</strong>
            <span>Add a folder to start tracking file changes.</span>
          </div>
        ) : (
          <div className="folder-list">
            {folders.map(folder => (
              <div className="folder-row" key={folder.id}>
                <div className="folder-symbol"><Folder size={20} /></div>
                <div className="folder-details">
                  <strong>{folder.path.split(/[\\/]/).filter(Boolean).pop() || folder.path}</strong>
                  <span>{folder.path}</span>
                </div>
                <span className={`folder-status ${folder.enabled ? "enabled" : ""}`}>
                  {folder.enabled ? "Protected" : "Disabled"}
                </span>
                <button
                  className="secondary-button"
                  disabled={scanning === folder.id || !folder.enabled}
                  onClick={() => onScan(folder.id)}
                >
                  {scanning === folder.id ? "Scanning..." : "Scan now"}
                </button>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

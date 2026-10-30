import { HardDrive, Shield, Info } from "lucide-react";

export default function SettingsPage() {
  return (
    <div>
      <div className="page-heading">
        <div>
          <div className="eyebrow">PREFERENCES</div>
          <h1>Settings</h1>
          <p>Rewind application and storage information.</p>
        </div>
      </div>

      <section className="panel settings-panel">
        <div className="setting-row">
          <div className="setting-icon"><HardDrive /></div>
          <div>
            <strong>Local storage</strong>
            <p>Your snapshots are stored locally on this computer.</p>
          </div>
          <span className="folder-status enabled">Enabled</span>
        </div>

        <div className="setting-row">
          <div className="setting-icon"><Shield /></div>
          <div>
            <strong>Content verification</strong>
            <p>Snapshots use SHA-256 content hashes.</p>
          </div>
          <span className="folder-status enabled">Enabled</span>
        </div>

        <div className="setting-row">
          <div className="setting-icon"><Info /></div>
          <div>
            <strong>Rewind version</strong>
            <p>0.1.0 · Development build</p>
          </div>
        </div>
      </section>
    </div>
  );
}

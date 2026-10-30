import { FolderPlus, Folder } from "lucide-react";
import type { Folder as FolderType } from "../../lib/types";

interface Props {
  folders: FolderType[];
  onAddFolder: () => void;
  adding: boolean;
}

export default function FoldersPage({
  folders,
  onAddFolder,
  adding
}: Props) {
  return (
    <div>
      <div className="page-heading">
        <div>
          <div className="eyebrow">FILE PROTECTION</div>
          <h1>Protected folders</h1>
          <p>Manage the folders Rewind monitors.</p>
        </div>

        <button
          className="primary-button"
          onClick={onAddFolder}
          disabled={adding}
        >
          <FolderPlus size={17} />
          {adding ? "Adding folder..." : "Add folder"}
        </button>
      </div>

      <section className="panel">
        {folders.length === 0 ? (
          <div className="empty-state">
            <Folder size={30} />
            <strong>No folders registered</strong>
            <span>Your protected folders will appear here.</span>
          </div>
        ) : (
          folders.map(folder => (
            <div className="folder-row" key={folder.id}>
              <div className="folder-symbol">
                <Folder size={20} />
              </div>

              <div className="folder-details">
                <strong>
                  {folder.path.split(/[\\/]/).filter(Boolean).pop() || folder.path}
                </strong>
                <span>{folder.path}</span>
              </div>

              <span className={`folder-status ${folder.enabled ? "enabled" : ""}`}>
                {folder.enabled ? "Protected" : "Disabled"}
              </span>
            </div>
          ))
        )}
      </section>
    </div>
  );
}
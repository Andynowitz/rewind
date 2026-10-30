import { Clock, RotateCcw } from "lucide-react";
import type { Version } from "../lib/types";
import { formatBytes, formatDate } from "../lib/formatters";

interface Props {
  versions: Version[];
  onRestore: (id: string) => void;
}

export default function VersionTimeline({ versions, onRestore }: Props) {
  if (!versions.length) {
    return <div className="empty-state">No saved versions found.</div>;
  }

  return (
    <div className="timeline">
      {versions.map(version => (
        <div className="timeline-item" key={version.id}>
          <div className="timeline-dot"><Clock size={15} /></div>
          <div className="timeline-content">
            <strong>{formatDate(version.createdAt)}</strong>
            <span>{version.reason} · {formatBytes(version.sizeBytes)}</span>
            <code>{version.hash.slice(0, 16)}…</code>
          </div>
          <button className="secondary-button" onClick={() => onRestore(version.id)}>
            <RotateCcw size={15} /> Restore
          </button>
        </div>
      ))}
    </div>
  );
}

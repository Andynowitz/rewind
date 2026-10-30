interface Props {
  open: boolean;
  versionLabel: string;
  onCancel: () => void;
  onConfirm: () => void;
  loading?: boolean;
}

export default function RestoreDialog({
  open,
  versionLabel,
  onCancel,
  onConfirm,
  loading = false
}: Props) {
  if (!open) return null;

  return (
    <div className="dialog-backdrop">
      <div className="dialog">
        <div className="eyebrow">RESTORE VERSION</div>
        <h2>Restore this version?</h2>
        <p>
          Rewind will replace the current file with <strong>{versionLabel}</strong>.
          The current file should be preserved as a separate version first.
        </p>
        <div className="dialog-actions">
          <button className="secondary-button" onClick={onCancel} disabled={loading}>
            Cancel
          </button>
          <button className="primary-button" onClick={onConfirm} disabled={loading}>
            {loading ? "Restoring..." : "Restore version"}
          </button>
        </div>
      </div>
    </div>
  );
}

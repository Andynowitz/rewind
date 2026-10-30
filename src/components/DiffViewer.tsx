interface Props {
  oldText: string;
  newText: string;
}

export default function DiffViewer({ oldText, newText }: Props) {
  const oldLines = oldText.split("\n");
  const newLines = newText.split("\n");
  const count = Math.max(oldLines.length, newLines.length);

  return (
    <div className="diff-viewer">
      {Array.from({ length: count }, (_, index) => {
        const oldLine = oldLines[index];
        const newLine = newLines[index];

        if (oldLine === newLine) {
          return (
            <div className="diff-line unchanged" key={index}>
              <span>{index + 1}</span><code>{oldLine ?? ""}</code>
            </div>
          );
        }

        return (
          <div key={index}>
            {oldLine !== undefined && (
              <div className="diff-line removed">
                <span>−</span><code>{oldLine}</code>
              </div>
            )}
            {newLine !== undefined && (
              <div className="diff-line added">
                <span>+</span><code>{newLine}</code>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

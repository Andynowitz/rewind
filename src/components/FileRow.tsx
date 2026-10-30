import { File, FileText, Image, FileCode, FileArchive } from "lucide-react";
import type { ReactNode } from "react";

interface Props {
  name: string;
  location: string;
  modified?: string;
  onClick?: () => void;
}

function getIcon(name: string): ReactNode {
  const ext = name.split(".").pop()?.toLowerCase();

  if (["png", "jpg", "jpeg", "gif", "webp"].includes(ext || "")) {
    return <Image size={18} />;
  }

  if (["ts", "tsx", "js", "jsx", "rs", "py", "json"].includes(ext || "")) {
    return <FileCode size={18} />;
  }

  if (["zip", "rar", "7z"].includes(ext || "")) {
    return <FileArchive size={18} />;
  }

  if (["txt", "md", "doc", "docx"].includes(ext || "")) {
    return <FileText size={18} />;
  }

  return <File size={18} />;
}

export default function FileRow({ name, location, modified, onClick }: Props) {
  return (
    <button className="file-row" onClick={onClick}>
      <div className="file-icon">{getIcon(name)}</div>
      <div className="file-info">
        <strong>{name}</strong>
        <span>{location}</span>
      </div>
      <span className="file-modified">{modified || "—"}</span>
    </button>
  );
}

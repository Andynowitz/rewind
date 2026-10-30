import {
  LayoutDashboard,
  Folder,
  History,
  Settings,
  Rewind
} from "lucide-react";

import type { AppRoute } from "../app/routes";

interface Props {
  active: AppRoute;
  onNavigate: (route: AppRoute) => void;
}

const items = [
  { id: "dashboard" as const, label: "Overview", icon: LayoutDashboard },
  { id: "folders" as const, label: "Protected folders", icon: Folder },
  { id: "history" as const, label: "File history", icon: History },
  { id: "settings" as const, label: "Settings", icon: Settings }
];

export default function Sidebar({ active, onNavigate }: Props) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-icon"><Rewind size={21} /></div>
        <span>rewind</span>
        <span className="brand-version">BETA</span>
      </div>

      <div className="nav-label">WORKSPACE</div>

      <nav>
        {items.map(item => {
          const Icon = item.icon;

          return (
            <button
              key={item.id}
              className={`nav-item ${active === item.id ? "active" : ""}`}
              onClick={() => onNavigate(item.id)}
            >
              <Icon size={18} />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>

      <div className="sidebar-bottom">
        <div className="status-dot" />
        <span>Local storage active</span>
      </div>
    </aside>
  );
}

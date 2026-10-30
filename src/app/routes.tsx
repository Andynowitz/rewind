export type AppRoute = "dashboard" | "folders" | "history" | "settings";

export const routes: { id: AppRoute; label: string }[] = [
  { id: "dashboard", label: "Overview" },
  { id: "folders", label: "Protected folders" },
  { id: "history", label: "File history" },
  { id: "settings", label: "Settings" }
];

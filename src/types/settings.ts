/** Mirrors `Theme` in `src-tauri/src/models/settings.rs`. */
export type Theme = "system" | "light" | "dark";

/** Mirrors `Settings` in `src-tauri/src/models/settings.rs`. */
export interface Settings {
  theme: Theme;
  notificationsEnabled: boolean;
}

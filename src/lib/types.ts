export type NavigationItem = "home" | "mods" | "modpacks" | "instances" | "logs" | "settings";
export type Theme = "dark" | "light" | "system";

export interface LauncherSettings {
  allocatedMemoryMb: number;
  javaPath: string | null;
  downloadConcurrency: number;
  closeOnLaunch: boolean;
  reopenOnExit: boolean;
  showLogsOnLaunch: boolean;
  language: string;
  theme: Theme;
  checkForUpdates: boolean;
}

export interface LauncherOverview {
  appDataDirectory: string;
  instanceCount: number;
  version: string;
}

export interface SelectedMod {
  projectId: string;
  name: string;
  enabled: boolean;
  versionOverrides: Record<string, string>;
}

export interface LauncherInstance {
  id: string;
  name: string;
  minecraftVersion: string;
  loader: "fabric";
  loaderVersion: string;
  mods: SelectedMod[];
  modpack?: InstalledModpack;
}

export interface InstalledModpack {
  projectId: string;
  versionId: string;
  name: string;
  iconUrl: string | null;
  fileCount: number;
}

export interface SupportedVersion {
  minecraftVersion: string;
  fabricLoaderVersion: string;
}

export interface ModrinthHit {
  projectId: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  downloads: number;
  iconUrl: string | null;
  categories: string[];
  versions: string[];
}
export interface ModrinthProject {
  id: string;
  slug: string;
  title: string;
  iconUrl: string | null;
}
export interface ModrinthVersion {
  id: string;
  name: string;
  versionType: "release" | "beta" | "alpha";
}
export interface ModConflictReport {
  compatible: boolean;
  message: string | null;
  projectIds: string[];
}
export interface ModrinthSearchResponse {
  hits: ModrinthHit[];
  total_hits: number;
}
export interface Account {
  username: string;
  uuid: string;
}
export interface AccountState {
  accounts: Account[];
  activeUuid: string | null;
}
export interface DeviceCode {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
  message: string;
}
export interface GameLogEntry {
  stream: "stdout" | "stderr";
  line: string;
}

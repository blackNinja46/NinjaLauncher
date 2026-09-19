export type NavigationItem =
  "home" | "skins" | "mods" | "modpacks" | "instances" | "logs" | "settings";
export type Theme = "dark" | "light" | "system";

export interface LauncherSettings {
  allocatedMemoryMb: number;
  javaPath: string | null;
  downloadConcurrency: number;
  closeOnLaunch: boolean;
  reopenOnExit: boolean;
  showLogsOnLaunch: boolean;
  showSnapshots: boolean;
  language: string;
  theme: Theme;
  checkForUpdates: boolean;
  discordRichPresence: boolean;
  shareAnonymousUsageData: boolean;
}

export interface LauncherOverview {
  appDataDirectory: string;
  instanceCount: number;
  version: string;
}

export interface RemoteLauncherConfiguration {
  maintenanceEnabled: boolean;
  maintenanceActive: boolean;
  maintenanceMessage: string;
  maintenanceStartsAt: string | null;
  maintenanceEndsAt: string | null;
  registrationEnabled: boolean;
  recommendedMinecraftVersion: string | null;
  minimumSupportedLauncherVersion: string | null;
  latestLauncherVersion: string | null;
  launcherHomeAnnouncementsEnabled: boolean;
  launcherNotificationsEnabled: boolean;
  enabledFeatures: string[];
  updatedAt: string;
}

export interface ClientAnnouncement {
  id: string;
  title: string;
  message: string;
  type: "INFO" | "UPDATE" | "MAINTENANCE" | "WARNING";
  priority: "LOW" | "NORMAL" | "HIGH" | "CRITICAL";
  startsAt: string | null;
  endsAt: string | null;
  displayTypes: ("HOME_NEWS" | "NOTIFICATION" | "BANNER" | "MODAL" | "CRITICAL_ALERT")[];
}

export interface ClientMinecraftVersion {
  version: string;
  recommended: boolean;
  releaseType: "RELEASE" | "SNAPSHOT";
  loaders: { type: "FABRIC" | "NEOFORGE" | "FORGE" | "VANILLA"; version: string }[];
}

export interface ClientBootstrap {
  generatedAt: string;
  configuration: RemoteLauncherConfiguration;
  announcements: ClientAnnouncement[];
  minecraftVersions: ClientMinecraftVersion[];
  latestRelease: { version: string; buildNumber: number; mandatory: boolean } | null;
}

export interface SelectedMod {
  projectId: string;
  name: string;
  enabled: boolean;
  disabledVersions?: string[];
  enabledVersions?: string[];
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

export type ProfileContentKind = "resourcepacks" | "shaderpacks";

export interface ProfileContentFile {
  name: string;
  fileName: string;
  sizeBytes: number | null;
  directory: boolean;
  projectId: string | null;
  versionId: string | null;
  iconUrl: string | null;
  enabled: boolean | null;
}

export interface InstalledModpack {
  projectId: string;
  versionId: string;
  name: string;
  iconUrl: string | null;
  fileCount: number;
  packMods: PackMod[];
}

export interface PackMod {
  projectId: string | null;
  versionId: string | null;
  name: string;
  versionName: string;
  iconUrl: string | null;
}

export interface ModpackInstallation {
  projectId: string;
  name: string;
  iconUrl: string | null;
  phase: "installing" | "failed";
  error?: string;
}

export interface ModpackFileInspection {
  path: string;
  name: string;
  versionId: string;
  minecraftVersion: string;
  loaderVersion: string;
  fileCount: number;
  foreignMods: string[];
}

export interface ModpackExportReport {
  path: string;
  fileCount: number;
  foreignMods: string[];
}

export interface SupportedVersion {
  minecraftVersion: string;
  fabricLoaderVersion: string;
  versionType: "release" | "snapshot";
}

export interface FabricLoaderVersionOption {
  version: string;
  stable: boolean;
}

export interface ModrinthHit {
  projectId: string;
  projectType: "mod" | "resourcepack" | "shader";
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
  gameVersions: string[];
}
export interface ModConflictReport {
  compatible: boolean;
  message: string | null;
  projectIds: string[];
}
export interface ModRepairReport {
  resetVersions: number;
  disabledMods: string[];
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
export interface NetworkRank {
  name: string;
  displayName: string;
  prefix: string;
  icon: string | null;
  color: string | null;
  weight: number;
  primary: boolean;
}
export interface PlayerRanks {
  uuid: string;
  username: string | null;
  primaryGroup: string;
  ranks: NetworkRank[];
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

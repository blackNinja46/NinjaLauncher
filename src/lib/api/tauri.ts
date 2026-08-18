import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent } from "@tauri-apps/plugin-updater";
import type {
  Account,
  AccountState,
  DeviceCode,
  LauncherInstance,
  LauncherOverview,
  LauncherSettings,
  ModConflictReport,
  ModrinthProject,
  ModrinthSearchResponse,
  ModrinthVersion,
  SupportedVersion,
} from "$lib/types";

const defaultSettings: LauncherSettings = {
  allocatedMemoryMb: 4096,
  javaPath: null,
  downloadConcurrency: 4,
  closeOnLaunch: false,
  reopenOnExit: true,
  showLogsOnLaunch: true,
  showSnapshots: false,
  language: "en",
  theme: "dark",
  checkForUpdates: true,
  discordRichPresence: true,
};

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export interface AppUpdateProgress {
  phase: "checking" | "downloading" | "installing" | "restarting" | "current";
  version?: string;
  downloadedBytes?: number;
  totalBytes?: number;
}

export async function installAvailableUpdate(
  onprogress: (progress: AppUpdateProgress) => void,
): Promise<boolean> {
  if (!inTauri()) return false;
  onprogress({ phase: "checking" });
  const update = await check();
  if (!update) {
    onprogress({ phase: "current" });
    return false;
  }

  let downloadedBytes = 0;
  let totalBytes: number | undefined;
  await update.downloadAndInstall((event: DownloadEvent) => {
    if (event.event === "Started") {
      totalBytes = event.data.contentLength;
      onprogress({ phase: "downloading", version: update.version, downloadedBytes, totalBytes });
    } else if (event.event === "Progress") {
      downloadedBytes += event.data.chunkLength;
      onprogress({ phase: "downloading", version: update.version, downloadedBytes, totalBytes });
    } else {
      onprogress({ phase: "installing", version: update.version, downloadedBytes, totalBytes });
    }
  });
  onprogress({ phase: "restarting", version: update.version, downloadedBytes, totalBytes });
  await relaunch();
  return true;
}

export async function listInstances(): Promise<LauncherInstance[]> {
  if (!inTauri())
    return [
      {
        id: "default-fabric",
        name: "My Fabric World",
        minecraftVersion: "26.2",
        loader: "fabric",
        loaderVersion: "0.19.3",
        mods: [],
      },
    ];
  return invoke("list_instances");
}
export const listSupportedVersions = (): Promise<SupportedVersion[]> =>
  inTauri()
    ? invoke("list_supported_versions")
    : Promise.resolve(
        [
          { minecraftVersion: "26.3-snapshot-9", versionType: "snapshot" as const },
          ...[
            "26.2",
            "26.1.2",
            "26.1.1",
            "26.1",
            "1.21.11",
            "1.21.10",
            "1.21.9",
            "1.21.8",
            "1.21.7",
            "1.21.6",
            "1.21.5",
            "1.21.4",
            "1.21.3",
            "1.21.2",
            "1.21.1",
            "1.21",
          ].map((minecraftVersion) => ({ minecraftVersion, versionType: "release" as const })),
        ].map((version) => ({ ...version, fabricLoaderVersion: "0.19.3" })),
      );
export const saveInstance = (instance: LauncherInstance) =>
  invoke<LauncherInstance>("save_instance", { instance });
export const createInstance = (name: string, minecraftVersion: string, loaderVersion: string) =>
  invoke<LauncherInstance>("create_instance", { name, minecraftVersion, loaderVersion });
export const duplicateInstance = (id: string) =>
  invoke<LauncherInstance>("duplicate_instance", { id });
export const deleteInstance = (id: string) => invoke<void>("delete_instance", { id });
export const openInstanceFolder = (id: string) => invoke<void>("open_instance_folder", { id });
export const searchMods = (query: string, minecraftVersion: string, loader: string, offset = 0) =>
  inTauri()
    ? invoke<ModrinthSearchResponse>("search_mods", { query, minecraftVersion, loader, offset })
    : Promise.resolve({ hits: [], total_hits: 0 });
export const getModDetails = (projectIds: string[]) =>
  inTauri() ? invoke<ModrinthProject[]>("get_mod_details", { projectIds }) : Promise.resolve([]);
export const listModVersions = (projectId: string, minecraftVersion: string, loader: string) =>
  inTauri()
    ? invoke<ModrinthVersion[]>("list_mod_versions", { projectId, minecraftVersion, loader })
    : Promise.resolve([]);
export const analyzeModConflicts = (instanceId: string) =>
  inTauri()
    ? invoke<ModConflictReport>("analyze_mod_conflicts", { instanceId })
    : Promise.resolve({ compatible: true, message: null, projectIds: [] });
export const addMod = (
  instanceId: string,
  projectId: string,
  name: string,
  versionId: string | null = null,
) => invoke<void>("add_mod", { instanceId, projectId, name, versionId });
export const removeMod = (instanceId: string, projectId: string) =>
  invoke<void>("remove_mod", { instanceId, projectId });
export const setModEnabled = (instanceId: string, projectId: string, enabled: boolean) =>
  invoke<void>("set_mod_enabled", { instanceId, projectId, enabled });
export const setModVersion = (
  instanceId: string,
  projectId: string,
  minecraftVersion: string,
  versionId: string | null,
) => invoke<void>("set_mod_version", { instanceId, projectId, minecraftVersion, versionId });
export const searchModpacks = (query: string, offset = 0, loader = "fabric") =>
  inTauri()
    ? invoke<ModrinthSearchResponse>("search_modpacks", { query, loader, offset })
    : Promise.resolve({ hits: [], total_hits: 0 });
export const listModpackVersions = (projectId: string, loader = "fabric") =>
  inTauri()
    ? invoke<ModrinthVersion[]>("list_modpack_versions", {
        projectId,
        loader,
      })
    : Promise.resolve([]);
export const installModpack = (
  projectId: string,
  versionId: string | null,
  name: string,
  iconUrl: string | null,
) =>
  invoke<LauncherInstance>("install_modpack", {
    projectId,
    versionId,
    name,
    iconUrl,
  });
export const getAccount = () =>
  inTauri() ? invoke<Account | null>("get_account") : Promise.resolve(null);
export const getAccounts = () =>
  inTauri()
    ? invoke<AccountState>("get_accounts")
    : Promise.resolve({ accounts: [], activeUuid: null });
export const beginMicrosoftLogin = () => invoke<DeviceCode>("begin_microsoft_login");
export const completeMicrosoftLogin = (deviceCode: DeviceCode) =>
  invoke<Account>("complete_microsoft_login", { deviceCode });
export const switchMicrosoftAccount = (uuid: string) =>
  invoke<Account>("switch_microsoft_account", { uuid });
export const removeMicrosoftAccount = (uuid: string) =>
  invoke<AccountState>("remove_microsoft_account", { uuid });
export const logout = () => invoke<void>("logout");
export const launchGame = (instanceId: string, demo = false) =>
  invoke<void>("launch_game", { instanceId, demo });
export const openLauncherFolder = () => invoke<void>("open_launcher_folder");
export const openLogsFolder = () => invoke<void>("open_logs_folder");

export async function getSettings(): Promise<LauncherSettings> {
  if (!inTauri()) return { ...defaultSettings };
  return invoke<LauncherSettings>("get_settings");
}

export async function saveSettings(settings: LauncherSettings): Promise<LauncherSettings> {
  if (!inTauri()) return settings;
  return invoke<LauncherSettings>("save_settings", { settings });
}

export async function getLauncherOverview(): Promise<LauncherOverview> {
  if (!inTauri()) {
    return { appDataDirectory: "NinjaLauncher", instanceCount: 1, version: "0.1.0" };
  }
  return invoke<LauncherOverview>("get_launcher_overview");
}

export function messageFromError(error: unknown): string {
  if (typeof error === "string") return error;
  if (typeof error === "object" && error && "message" in error) return String(error.message);
  return "Something went wrong. Please try again.";
}

import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent } from "@tauri-apps/plugin-updater";
import type {
  Account,
  AccountState,
  ClientBootstrap,
  DeviceCode,
  FabricLoaderVersionOption,
  LauncherInstance,
  LauncherOverview,
  LauncherSettings,
  ModConflictReport,
  ModRepairReport,
  ModpackExportReport,
  ModpackFileInspection,
  ModrinthProject,
  ModrinthSearchResponse,
  ModrinthVersion,
  PlayerRanks,
  ProfileContentFile,
  ProfileContentKind,
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
  shareAnonymousUsageData: false,
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
export const listFabricLoaderVersions = (): Promise<FabricLoaderVersionOption[]> =>
  inTauri()
    ? invoke("list_fabric_loader_versions")
    : Promise.resolve([{ version: "0.19.3", stable: true }]);
export const saveInstance = (instance: LauncherInstance) =>
  invoke<LauncherInstance>("save_instance", { instance });
export const createInstance = (name: string, minecraftVersion: string, loaderVersion: string) =>
  invoke<LauncherInstance>("create_instance", { name, minecraftVersion, loaderVersion });
export const duplicateInstance = (id: string) =>
  invoke<LauncherInstance>("duplicate_instance", { id });
export const deleteInstance = (id: string) => invoke<void>("delete_instance", { id });
export const openInstanceFolder = (id: string) => invoke<void>("open_instance_folder", { id });
export const listProfileContent = (instanceId: string, kind: ProfileContentKind) =>
  inTauri()
    ? invoke<ProfileContentFile[]>("list_profile_content", { instanceId, kind })
    : Promise.resolve([]);
export async function chooseProfileContentFiles(): Promise<string[]> {
  if (!inTauri()) return [];
  const paths = await open({
    multiple: true,
    directory: false,
    filters: [{ name: "Minecraft pack", extensions: ["zip"] }],
  });
  return paths ? (Array.isArray(paths) ? paths : [paths]) : [];
}
export const addProfileContent = (
  instanceId: string,
  kind: ProfileContentKind,
  sourcePath: string,
) => invoke<ProfileContentFile>("add_profile_content", { instanceId, kind, sourcePath });
export const removeProfileContent = (
  instanceId: string,
  kind: ProfileContentKind,
  fileName: string,
) => invoke<void>("remove_profile_content", { instanceId, kind, fileName });
export const setResourcePackEnabled = (instanceId: string, fileName: string, enabled: boolean) =>
  invoke<void>("set_resource_pack_enabled", { instanceId, fileName, enabled });
export const openProfileContentFolder = (instanceId: string, kind: ProfileContentKind) =>
  invoke<void>("open_profile_content_folder", { instanceId, kind });
export const searchProfileContent = (
  query: string,
  minecraftVersion: string,
  kind: ProfileContentKind,
  offset = 0,
) =>
  inTauri()
    ? invoke<ModrinthSearchResponse>("search_profile_content", {
        query,
        minecraftVersion,
        kind,
        offset,
      })
    : Promise.resolve({ hits: [], total_hits: 0 });
export const listProfileContentVersions = (
  projectId: string,
  minecraftVersion: string,
  kind: ProfileContentKind,
) =>
  inTauri()
    ? invoke<ModrinthVersion[]>("list_profile_content_versions", {
        projectId,
        minecraftVersion,
        kind,
      })
    : Promise.resolve([]);
export const installProfileContent = (
  instanceId: string,
  kind: ProfileContentKind,
  projectId: string,
  versionId: string | null,
  name: string,
  iconUrl: string | null,
) =>
  invoke<ProfileContentFile>("install_profile_content", {
    instanceId,
    kind,
    projectId,
    versionId,
    name,
    iconUrl,
  });
export const searchMods = (query: string, minecraftVersion: string, loader: string, offset = 0) =>
  inTauri()
    ? invoke<ModrinthSearchResponse>("search_mods", { query, minecraftVersion, loader, offset })
    : Promise.resolve({ hits: [], total_hits: 0 });
export type ModrinthSearchIndex = "relevance" | "downloads" | "follows" | "newest" | "updated";

export const searchLauncherContent = (
  query: string,
  minecraftVersion: string,
  offset = 0,
  index: ModrinthSearchIndex = "relevance",
) =>
  inTauri()
    ? invoke<ModrinthSearchResponse>("search_launcher_content", {
        query,
        minecraftVersion,
        offset,
        index,
      })
    : Promise.resolve({ hits: [], total_hits: 0 });
export const getModDetails = (projectIds: string[]) =>
  inTauri() ? invoke<ModrinthProject[]>("get_mod_details", { projectIds }) : Promise.resolve([]);
export const listModVersions = (projectId: string, minecraftVersion: string, loader: string) =>
  inTauri()
    ? invoke<ModrinthVersion[]>("list_mod_versions", { projectId, minecraftVersion, loader })
    : Promise.resolve([]);
export const getInstalledModVersions = (instanceId: string) =>
  inTauri()
    ? invoke<Record<string, string>>("get_installed_mod_versions", { instanceId })
    : Promise.resolve({});
export const analyzeModConflicts = (instanceId: string) =>
  inTauri()
    ? invoke<ModConflictReport>("analyze_mod_conflicts", { instanceId })
    : Promise.resolve({ compatible: true, message: null, projectIds: [] });
export const repairModConfiguration = (instanceId: string) =>
  invoke<ModRepairReport>("repair_mod_configuration", { instanceId });
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
export async function chooseModpackFile(): Promise<ModpackFileInspection | null> {
  if (!inTauri()) return null;
  const path = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Modrinth Modpack", extensions: ["mrpack"] }],
  });
  return path ? invoke<ModpackFileInspection>("inspect_modpack_file", { path }) : null;
}
export const importModpackFile = (path: string, allowForeignMods: boolean) =>
  invoke<LauncherInstance>("import_modpack_file", { path, allowForeignMods });
export const refreshImportedModpackMods = (instanceId: string) =>
  invoke<LauncherInstance>("refresh_imported_modpack_mods", { instanceId });
export const listExportableFolders = (instanceId: string) =>
  invoke<string[]>("list_exportable_folders", { instanceId });
export async function exportModpackFile(
  instanceId: string,
  packName: string,
  versionId: string,
  includedFolders: string[],
): Promise<ModpackExportReport | null> {
  if (!inTauri()) return null;
  const safeName = packName.replace(/[^a-zA-Z0-9._-]+/g, "-").replace(/^-+|-+$/g, "");
  const destination = await save({
    defaultPath: `${safeName || "modpack"}.mrpack`,
    filters: [{ name: "Modrinth Modpack", extensions: ["mrpack"] }],
  });
  const path = destination?.toLowerCase().endsWith(".mrpack")
    ? destination
    : destination
      ? `${destination}.mrpack`
      : null;
  return path
    ? invoke<ModpackExportReport>("export_modpack", {
        instanceId,
        destination: path,
        packName,
        versionId,
        includedFolders,
      })
    : null;
}
export const getAccount = () =>
  inTauri() ? invoke<Account | null>("get_account") : Promise.resolve(null);
export const getAccounts = () =>
  inTauri()
    ? invoke<AccountState>("get_accounts")
    : Promise.resolve({ accounts: [], activeUuid: null });
export const getAccountCape = (uuid: string) =>
  inTauri() ? invoke<string | null>("get_account_cape", { uuid }) : Promise.resolve(null);
export const beginMicrosoftLogin = () => invoke<DeviceCode>("begin_microsoft_login");
export const completeMicrosoftLogin = (deviceCode: DeviceCode) =>
  invoke<Account>("complete_microsoft_login", { deviceCode });
export const switchMicrosoftAccount = (uuid: string) =>
  invoke<Account>("switch_microsoft_account", { uuid });
export const removeMicrosoftAccount = (uuid: string) =>
  invoke<AccountState>("remove_microsoft_account", { uuid });
export const logout = () => invoke<void>("logout");
export const launchGame = (instanceId: string, demo = false, serverAddress?: string) =>
  invoke<void>("launch_game", { instanceId, demo, serverAddress: serverAddress ?? null });
export const getServerIcon = (serverAddress: string) =>
  inTauri() ? invoke<string | null>("get_server_icon", { serverAddress }) : Promise.resolve(null);
export const getNetworkRanks = (uuid: string): Promise<PlayerRanks | null> =>
  inTauri()
    ? invoke("get_network_ranks", { uuid })
    : Promise.resolve({
        uuid,
        username: "PreviewPlayer",
        primaryGroup: "ninja-plus",
        ranks: [
          {
            name: "ninja-plus",
            displayName: "Ninja+",
            prefix: "[Ninja+]",
            icon: "ninja",
            color: "#3b9df5",
            weight: 50,
            primary: true,
          },
        ],
      });
export const listRunningInstances = () =>
  inTauri() ? invoke<string[]>("list_running_instances") : Promise.resolve([]);
export const stopGame = (instanceId: string) => invoke<void>("stop_game", { instanceId });
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

export async function getLauncherBootstrap(minecraftVersion?: string): Promise<ClientBootstrap> {
  if (!inTauri()) {
    return {
      generatedAt: new Date(0).toISOString(),
      configuration: {
        maintenanceEnabled: false,
        maintenanceActive: false,
        maintenanceMessage: "",
        maintenanceStartsAt: null,
        maintenanceEndsAt: null,
        registrationEnabled: true,
        recommendedMinecraftVersion: null,
        minimumSupportedLauncherVersion: null,
        latestLauncherVersion: null,
        launcherHomeAnnouncementsEnabled: true,
        launcherNotificationsEnabled: true,
        enabledFeatures: [],
        updatedAt: new Date(0).toISOString(),
      },
      announcements: [],
      minecraftVersions: [],
      latestRelease: null,
    };
  }
  return invoke<ClientBootstrap>("get_launcher_bootstrap", {
    minecraftVersion: minecraftVersion ?? null,
  });
}

export const recordClientEvent = (
  eventType: "LAUNCHER_START" | "GAME_START",
  minecraftVersion?: string,
) =>
  inTauri()
    ? invoke<void>("record_client_event", {
        eventType,
        minecraftVersion: minecraftVersion ?? null,
      })
    : Promise.resolve();

export function messageFromError(error: unknown): string {
  if (typeof error === "string") return error;
  if (typeof error === "object" && error && "message" in error) return String(error.message);
  return "Something went wrong. Please try again.";
}

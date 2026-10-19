<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import Sidebar from "$lib/components/layout/Sidebar.svelte";
  import NotificationCenter from "$lib/components/layout/NotificationCenter.svelte";
  import AnnouncementBanner from "$lib/components/layout/AnnouncementBanner.svelte";
  import ToastStack from "$lib/components/layout/ToastStack.svelte";
  import WindowTitlebar from "$lib/components/layout/WindowTitlebar.svelte";
  import SkinsPage from "$lib/components/skins/SkinsPage.svelte";
  import HomePage from "$lib/components/launcher/HomePage.svelte";
  import HalloweenIdleSounds from "$lib/components/launcher/HalloweenIdleSounds.svelte";
  import HalloweenIntroDialog from "$lib/components/launcher/HalloweenIntroDialog.svelte";
  import ModsPage from "$lib/components/mods/ModsPage.svelte";
  import ModpacksPage from "$lib/components/modpacks/ModpacksPage.svelte";
  import InstancesPage from "$lib/components/instances/InstancesPage.svelte";
  import SettingsPage from "$lib/components/settings/SettingsPage.svelte";
  import LogsPage from "$lib/components/logs/LogsPage.svelte";
  import CrashDialog from "$lib/components/logs/CrashDialog.svelte";
  import AccountDialog from "$lib/components/account/AccountDialog.svelte";
  import SetupWizard from "$lib/components/setup/SetupWizard.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import { compareMinecraftVersionsDescending, sortSupportedVersions } from "$lib/utils/version";
  import {
    getAccounts,
    getLauncherOverview,
    getNetworkRanks,
    getLauncherBootstrap,
    getSettings,
    installAvailableUpdate,
    importModpackFile,
    installModpack,
    launchGame,
    listInstances,
    listRunningInstances,
    listSupportedVersions,
    listVanillaVersions,
    messageFromError,
    repairModConfiguration,
    recordClientEvent,
    saveInstance,
    stopGame,
    uploadCrashReport,
    type AppUpdateProgress,
  } from "$lib/api/tauri";
  import type {
    Account,
    ClientBootstrap,
    ClientAnnouncement,
    GameCrashDetails,
    GameLogEntry,
    InstanceConsole,
    LauncherInstance,
    LauncherOverview,
    LauncherSettings,
    ModpackInstallation,
    ModpackFileInspection,
    ModrinthHit,
    NavigationItem,
    PlayerRanks,
    SupportedVersion,
  } from "$lib/types";

  let activePage = $state<NavigationItem>("home");
  let loading = $state(true);
  let startupError = $state("");
  let settings = $state<LauncherSettings>({
    setupComplete: false,
    installationPath: null,
    termsAcceptedAt: null,
    allocatedMemoryMb: 4096,
    javaPath: null,
    downloadConcurrency: 4,
    closeOnLaunch: false,
    reopenOnExit: true,
    showLogsOnLaunch: true,
    showSnapshots: false,
    language: "en",
    theme: "dark",
    animationsEnabled: true,
    parallaxEnabled: true,
    halloweenThemeEnabled: true,
    halloweenSoundsEnabled: true,
    checkForUpdates: true,
    discordRichPresence: true,
    shareAnonymousUsageData: false,
  });
  let overview = $state<LauncherOverview>({
    appDataDirectory: "NinjaLauncher",
    instanceCount: 1,
    version: "0.1.0",
  });
  let instances = $state<LauncherInstance[]>([]);
  let selectedId = $state("default-fabric");
  let account = $state<Account | null>(null);
  let accounts = $state<Account[]>([]);
  let networkRanks = $state<PlayerRanks | null>(null);
  let networkRankError = $state("");
  let clientBootstrap = $state<ClientBootstrap | null>(null);
  let halloweenThemeAvailable = $derived(
    clientBootstrap?.configuration.enabledFeatures.includes("halloween_theme") ?? false,
  );
  let halloweenThemeActive = $derived(halloweenThemeAvailable && settings.halloweenThemeEnabled);
  const halloweenIntroStorageKey = "ninja-launcher:halloween-event:2026:intro-seen";
  let halloweenIntroOpen = $state(false);
  let halloweenIntroChecked = $state(false);
  let dismissedAnnouncementIds = $state<string[]>([]);
  let controlServiceError = $state("");
  let clientRefreshInFlight = false;
  let accountOpen = $state(false);
  let allVersions = $state<SupportedVersion[]>([]);
  let vanillaVersions = $state<SupportedVersion[]>([]);
  let managedVersions = $derived.by(() => {
    const catalog = clientBootstrap?.minecraftVersions ?? [];
    if (!catalog.length) return sortSupportedVersions(allVersions);
    const managed = [...catalog]
      .sort((left, right) => compareMinecraftVersionsDescending(left.version, right.version))
      .flatMap((version) => {
        const fabric = version.loaders.find((loader) => loader.type === "FABRIC");
        if (!fabric) return [];
        return [
          {
            minecraftVersion: version.version,
            fabricLoaderVersion: fabric.version,
            versionType:
              version.releaseType === "SNAPSHOT" ? ("snapshot" as const) : ("release" as const),
          },
        ];
      });
    return managed.length ? sortSupportedVersions(managed) : sortSupportedVersions(allVersions);
  });
  let managedVanillaVersions = $derived.by(() => {
    const catalog = clientBootstrap?.minecraftVersions ?? [];
    if (!catalog.length) return sortSupportedVersions(vanillaVersions);
    const managed = [...catalog]
      .sort((left, right) => compareMinecraftVersionsDescending(left.version, right.version))
      .map((version) => ({
        minecraftVersion: version.version,
        fabricLoaderVersion: "",
        versionType:
          version.releaseType === "SNAPSHOT" ? ("snapshot" as const) : ("release" as const),
      }));
    return managed.length ? sortSupportedVersions(managed) : sortSupportedVersions(vanillaVersions);
  });
  let versions = $derived(
    managedVersions.filter(
      (version) => settings.showSnapshots || version.versionType === "release",
    ),
  );
  let gameLogs = $state<GameLogEntry[]>([]);
  let selectedConsoleId = $state("");
  let runningInstanceIds = $state<string[]>([]);
  let launchingInstanceIds = $state<string[]>([]);
  let stoppingInstanceIds = $state<string[]>([]);
  const requestedStopInstanceIds = new Set<string>();
  let launcherStatus = $state("Everything is ready");
  let launchProgress = $state<number | null>(null);
  let modpackInstallation = $state<ModpackInstallation | null>(null);
  let appUpdate = $state<AppUpdateProgress | null>(null);
  let updateError = $state("");
  let fabricIssue = $state("");
  let fabricIssueCandidate = $state("");
  let fabricDialogOpen = $state(false);
  let gameProcessExited = $state(false);
  let minecraftClientStarted = $state(false);
  let fabricRepairing = $state(false);
  let fabricRepairNotice = $state("");
  let fabricCrashReportId = $state("");
  let crashDetails = $state<GameCrashDetails | null>(null);
  let showCustomTitlebar = $state(false);
  let launcherAssembly = $state(false);
  let toastItems = $state<
    {
      id: string;
      title: string;
      message: string;
      tone: "info" | "success" | "warning" | "error";
    }[]
  >([]);

  $effect(() => {
    const systemDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    const resolvedTheme =
      settings.theme === "system" ? (systemDark ? "dark" : "light") : settings.theme;
    document.documentElement.dataset.theme = resolvedTheme;
    document.documentElement.classList.toggle("animations-disabled", !settings.animationsEnabled);
    document.documentElement.classList.toggle("parallax-disabled", !settings.parallaxEnabled);
    document.documentElement.classList.toggle("halloween-theme", halloweenThemeActive);
  });

  $effect(() => {
    if (loading || !settings.setupComplete || !halloweenThemeAvailable || halloweenIntroChecked)
      return;
    halloweenIntroChecked = true;
    halloweenIntroOpen = localStorage.getItem(halloweenIntroStorageKey) !== "seen";
  });

  function dismissHalloweenIntro(openSettings = false) {
    localStorage.setItem(halloweenIntroStorageKey, "seen");
    halloweenIntroOpen = false;
    if (openSettings) activePage = "settings";
  }
  let visibleSystemNotificationIds: string[] = [];
  let mainElement = $state<HTMLElement>();
  let instance = $derived(instances.find((item) => item.id === selectedId) ?? instances[0]);
  let fabricCrashDetails = $derived.by((): GameCrashDetails | null => {
    if (!instance || !fabricIssue) return null;
    const lines = fabricIssue
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
    return {
      reportId: fabricCrashReportId || "fabric-compatibility",
      instanceId: instance.id,
      profileName: instance.name,
      minecraftVersion: instance.minecraftVersion,
      exitCode: null,
      title: "Incompatible Fabric mods",
      message:
        "Fabric stopped Minecraft because one or more installed mods cannot be resolved together.",
      suggestion:
        "Automatic repair first restores compatible automatic versions. It only disables a personal mod when Fabric identifies exactly one unambiguous cause.",
      technicalDetail: fabricIssue,
      logExcerpt: lines.slice(-40).map((line) => `[fabric] ${line}`),
      uploadLogExcerpt: lines.slice(-200).map((line) => `[fabric] ${line}`),
      repairAvailable: !gameRunning && !launching,
    };
  });
  let selectedVersions = $derived(
    instance?.loader === "vanilla"
      ? managedVanillaVersions.filter(
          (version) => settings.showSnapshots || version.versionType !== "snapshot",
        )
      : versions,
  );
  let selectedManagedVersions = $derived(
    instance?.loader === "vanilla" ? managedVanillaVersions : managedVersions,
  );
  let currentVersionAvailable = $derived(
    !instance ||
      selectedManagedVersions.some(
        (version) => version.minecraftVersion === instance.minecraftVersion,
      ),
  );
  let gameRunning = $derived(Boolean(instance && runningInstanceIds.includes(instance.id)));
  let launching = $derived(Boolean(instance && launchingInstanceIds.includes(instance.id)));
  let stopping = $derived(Boolean(instance && stoppingInstanceIds.includes(instance.id)));
  let runningInstances = $derived(instances.filter((item) => runningInstanceIds.includes(item.id)));
  let instanceConsoles = $derived.by((): InstanceConsole[] => {
    const availableIds = new Set([
      ...runningInstanceIds,
      ...gameLogs.map((entry) => entry.instanceId),
    ]);
    return instances
      .filter((item) => availableIds.has(item.id))
      .sort(
        (left, right) =>
          Number(runningInstanceIds.includes(right.id)) -
          Number(runningInstanceIds.includes(left.id)),
      )
      .map((item) => ({
        id: item.id,
        name: item.name,
        minecraftVersion: item.minecraftVersion,
        loader: item.loader,
        logs: gameLogs.filter((entry) => entry.instanceId === item.id),
        running: runningInstanceIds.includes(item.id),
      }));
  });
  let updatePercent = $derived(
    appUpdate?.totalBytes && appUpdate.downloadedBytes
      ? Math.min(100, Math.round((appUpdate.downloadedBytes / appUpdate.totalBytes) * 100))
      : 0,
  );
  let notifications = $derived.by(() => {
    const items: {
      id: string;
      title: string;
      message: string;
      tone: "info" | "success" | "warning" | "error";
      actionLabel?: string;
    }[] = [];
    if (startupError) {
      items.push({ id: "startup", title: "Launcher error", message: startupError, tone: "error" });
    }
    if (updateError) {
      items.push({ id: "update", title: "Update failed", message: updateError, tone: "error" });
    }
    if (networkRankError) {
      items.push({
        id: "rank-sync",
        title: "Rank sync",
        message: networkRankError,
        tone: "warning",
      });
    }
    if (controlServiceError) {
      items.push({
        id: "control-service",
        title: "Online configuration",
        message: controlServiceError,
        tone: "warning",
      });
    }
    const configuration = clientBootstrap?.configuration;
    for (const announcement of clientBootstrap?.announcements ?? []) {
      if (dismissedAnnouncementIds.includes(announcement.id)) continue;
      const homeVisible =
        configuration?.launcherHomeAnnouncementsEnabled &&
        announcement.displayTypes.includes("HOME_NEWS");
      const notificationVisible =
        configuration?.launcherNotificationsEnabled &&
        announcement.displayTypes.some((type) => type !== "HOME_NEWS" && type !== "BANNER");
      if (!homeVisible && !notificationVisible) continue;
      items.push({
        id: `announcement:${announcement.id}`,
        title: announcement.title,
        message: announcement.message,
        tone:
          announcement.priority === "CRITICAL" || announcement.type === "WARNING"
            ? "warning"
            : "info",
      });
    }
    if (gameRunning && fabricIssueCandidate && !fabricDialogOpen) {
      items.push({
        id: "fabric",
        title: "Fabric warning detected",
        message: "Minecraft is still running. Review the detected compatibility issue.",
        tone: "warning",
        actionLabel: "View details",
      });
    }
    if (fabricRepairNotice) {
      items.push({
        id: "fabric-repair",
        title: "Fabric repaired",
        message: fabricRepairNotice,
        tone: "success",
      });
    }
    return items;
  });
  let bannerAnnouncements = $derived.by((): ClientAnnouncement[] => {
    if (!clientBootstrap?.configuration.launcherNotificationsEnabled) return [];
    return clientBootstrap.announcements.filter(
      (announcement) =>
        announcement.displayTypes.includes("BANNER") &&
        !dismissedAnnouncementIds.includes(announcement.id),
    );
  });

  function showToast(notification: {
    title: string;
    message: string;
    tone: "info" | "success" | "warning" | "error";
  }) {
    const duplicate = toastItems.find(
      (item) => item.title === notification.title && item.message === notification.message,
    );
    if (duplicate) return;
    const id = crypto.randomUUID();
    toastItems = [...toastItems, { id, ...notification }].slice(-4);
    window.setTimeout(
      () => {
        toastItems = toastItems.filter((item) => item.id !== id);
      },
      notification.tone === "error" ? 8_000 : 5_500,
    );
  }

  function dismissToast(id: string) {
    toastItems = toastItems.filter((item) => item.id !== id);
  }

  $effect(() => {
    const currentIds = notifications.map((item) => item.id);
    for (const item of notifications) {
      if (!visibleSystemNotificationIds.includes(item.id)) showToast(item);
    }
    visibleSystemNotificationIds = currentIds;
  });

  $effect(() => {
    activePage;
    requestAnimationFrame(() => {
      if (mainElement) mainElement.scrollTop = 0;
    });
  });

  $effect(() => {
    const consoleIds = instanceConsoles.map((item) => item.id);
    if (!consoleIds.includes(selectedConsoleId)) {
      selectedConsoleId = consoleIds.includes(selectedId) ? selectedId : (consoleIds[0] ?? "");
    }
  });

  $effect(() => {
    const uuid = account?.uuid;
    networkRanks = null;
    networkRankError = "";
    if (!uuid) return;
    let current = true;
    getNetworkRanks(uuid)
      .then((result) => {
        if (current && account?.uuid === uuid) networkRanks = result;
      })
      .catch((cause) => {
        if (current && account?.uuid === uuid) {
          networkRanks = null;
          const detail = messageFromError(cause);
          networkRankError = /get_network_ranks|command.*not found/i.test(detail)
            ? "Restart the launcher to enable rank sync."
            : "Rank sync is temporarily unavailable.";
        }
      });
    return () => {
      current = false;
    };
  });

  async function runAutomaticUpdate() {
    try {
      const installing = await installAvailableUpdate((progress) => (appUpdate = progress));
      if (!installing) appUpdate = null;
    } catch (cause) {
      appUpdate = null;
      updateError = `Automatic update failed: ${messageFromError(cause)}`;
    }
  }

  function dismissNotification(id: string) {
    if (id === "startup") startupError = "";
    if (id === "update") updateError = "";
    if (id === "rank-sync") networkRankError = "";
    if (id === "control-service") controlServiceError = "";
    if (id.startsWith("announcement:")) {
      dismissedAnnouncementIds = [...dismissedAnnouncementIds, id.slice("announcement:".length)];
    }
    if (id === "fabric") {
      fabricIssueCandidate = "";
      fabricIssue = "";
    }
    if (id === "fabric-repair") fabricRepairNotice = "";
  }

  function clearAllNotifications() {
    startupError = "";
    updateError = "";
    networkRankError = "";
    controlServiceError = "";
    dismissedAnnouncementIds = [
      ...new Set([
        ...dismissedAnnouncementIds,
        ...(clientBootstrap?.announcements.map((item) => item.id) ?? []),
      ]),
    ];
    fabricIssueCandidate = "";
    fabricIssue = "";
    fabricRepairNotice = "";
  }

  function openNotification(id: string) {
    if (id !== "fabric" || !fabricIssueCandidate) return;
    fabricIssue = fabricIssueCandidate;
    fabricCrashReportId = crypto.randomUUID();
    fabricDialogOpen = true;
  }

  async function refreshInstances() {
    instances = await listInstances();
    if (!instances.some((item) => item.id === selectedId)) selectedId = instances[0]?.id ?? "";
  }

  async function refreshAfterSetup() {
    await refreshInstances();
    overview = await getLauncherOverview();
  }

  async function refreshClientBootstrap() {
    if (clientRefreshInFlight) return;
    clientRefreshInFlight = true;
    try {
      const selectedInstance = instances.find((item) => item.id === selectedId) ?? instances[0];
      clientBootstrap = await getLauncherBootstrap(selectedInstance?.minecraftVersion);
      controlServiceError = "";
    } catch (error) {
      controlServiceError = `${messageFromError(error)} The last available configuration remains active.`;
    } finally {
      clientRefreshInFlight = false;
    }
  }

  function startsRepairableFabricLoaderFailure(message: string): boolean {
    return /incompatible mods? found|incompatible mod set|formattedexception:\s*some of your mods are incompatible/i.test(
      message,
    );
  }

  function collectFabricIssue(line: string) {
    if (
      /\[Render thread\/INFO\]|setting user:|backend library:\s*lwjgl|lwjgl version/i.test(line)
    ) {
      minecraftClientStarted = true;
    }
    const startsFailure = !fabricIssueCandidate && startsRepairableFabricLoaderFailure(line);
    if (!fabricIssueCandidate && !startsFailure) return;
    if (fabricIssueCandidate.length >= 16_000) return;
    fabricIssueCandidate = fabricIssueCandidate ? `${fabricIssueCandidate}\n${line}` : line;
    if (startsFailure && gameProcessExited) window.setTimeout(finalizeFabricFailure, 300);
  }

  function finalizeFabricFailure() {
    if (gameProcessExited && fabricIssueCandidate && !minecraftClientStarted) {
      fabricIssue = fabricIssueCandidate;
      fabricCrashReportId = crypto.randomUUID();
      fabricDialogOpen = true;
    } else if (gameProcessExited && minecraftClientStarted) {
      fabricIssueCandidate = "";
    }
  }

  function createCrashDetails(
    instanceId: string,
    exitCode: number | null,
    entries: GameLogEntry[],
  ): GameCrashDetails {
    const crashedProfile = instances.find((item) => item.id === instanceId);
    const output = entries.map((entry) => entry.line).join("\n");
    const technicalDetail =
      [...entries]
        .reverse()
        .find((entry) =>
          /exception|error|fatal|caused by|outofmemory|unsatisfiedlink|unsupportedclassversion|mixin|glfw/i.test(
            entry.line,
          ),
        )?.line ?? null;
    let title = "Minecraft crashed";
    let message = "Minecraft closed unexpectedly before it could shut down normally.";
    let suggestion =
      "Open the console to review the final messages. If the crash repeats, check this profile's mods and Java runtime.";

    if (/outofmemoryerror|could not reserve enough space|gc overhead limit/i.test(output)) {
      title = "Not enough memory";
      message = "Minecraft ran out of usable memory and had to close.";
      suggestion =
        "Increase the allocated memory in Settings or remove memory-heavy mods, shaders, and resource packs.";
    } else if (
      /incompatible mods?|modresolutionexception|mod resolution|mixin.*(?:failed|error)|could not execute entrypoint|nosuchmethoderror/i.test(
        output,
      )
    ) {
      title = "Mod compatibility problem";
      message = "One or more mods are incompatible with this Minecraft version or with each other.";
      suggestion =
        "Open the console to identify the affected mod, then update, disable, or remove it from this profile.";
    } else if (
      /unsatisfiedlinkerror|wrong architecture|bad cpu type|no lwjgl|failed to load.*(?:dylib|dll|so)|unsupportedclassversionerror/i.test(
        output,
      )
    ) {
      title = "Java or native runtime problem";
      message =
        "Minecraft could not load the Java or graphics components required by this version.";
      suggestion =
        "Use Automatic Java in Settings and launch the profile again. Older versions on Apple Silicon may require Rosetta 2.";
    } else if (
      /glfw error|failed to create window|opengl.*(?:error|failed)|pixel format|renderer.*failed/i.test(
        output,
      )
    ) {
      title = "Graphics initialization failed";
      message = "Minecraft could not initialize its game window or graphics backend.";
      suggestion =
        "Disable shaders and try the profile without graphics-related mods. Also check that the system graphics software is current.";
    }

    return {
      reportId: crypto.randomUUID(),
      instanceId,
      profileName: crashedProfile?.name ?? "Minecraft",
      minecraftVersion: crashedProfile?.minecraftVersion ?? "Unknown",
      exitCode,
      title,
      message,
      suggestion,
      technicalDetail,
      logExcerpt: entries
        .filter((entry) => entry.line.trim())
        .slice(-40)
        .map((entry) => `[${entry.stream}] ${entry.line}`),
      uploadLogExcerpt: entries
        .filter((entry) => entry.line.trim())
        .slice(-200)
        .map((entry) => `[${entry.stream}] ${entry.line}`),
    };
  }

  function createLaunchFailureDetails(instanceId: string, detail: string): GameCrashDetails {
    const failedProfile = instances.find((item) => item.id === instanceId);
    const javaProblem = /java|runtime|jdk|jre|architecture|rosetta/i.test(detail);
    const namedIncompatibleMod = detail.match(
      /No compatible version was found for (.+?) \(Modrinth project ([A-Za-z0-9_-]+)\)\.?/i,
    );
    const legacyIncompatibleMod = detail.match(
      /No compatible version was found for mod ([A-Za-z0-9_-]+)/i,
    );
    const incompatibleModId = namedIncompatibleMod?.[2] ?? legacyIncompatibleMod?.[1];
    const incompatibleMod = incompatibleModId
      ? failedProfile?.mods.find((item) => item.projectId === incompatibleModId)
      : null;
    const incompatibleModName = namedIncompatibleMod?.[1] ?? incompatibleMod?.name;
    const loaderName = failedProfile?.loader === "fabric" ? "Fabric" : failedProfile?.loader;
    const compatibilityProblem = Boolean(incompatibleModId);
    return {
      reportId: crypto.randomUUID(),
      instanceId,
      profileName: failedProfile?.name ?? "Minecraft",
      minecraftVersion: failedProfile?.minecraftVersion ?? "Unknown",
      exitCode: null,
      title: compatibilityProblem ? "Incompatible mod" : "Minecraft could not start",
      message: compatibilityProblem
        ? `${incompatibleModName ?? "A required mod"} is not available for Minecraft ${failedProfile?.minecraftVersion ?? "this version"}${loaderName ? ` with ${loaderName}` : ""}, so Minecraft was not started.`
        : "The launcher could not complete the startup process for this profile.",
      suggestion: compatibilityProblem
        ? "Repair the profile to disable the incompatible mod for this Minecraft version, then launch again. The mod will not be deleted."
        : javaProblem
          ? "Set Java Runtime to Automatic in Settings and try again. Older versions on Apple Silicon may require Rosetta 2."
          : "Open the console for the full error, then verify this profile's version, loader, and installed content.",
      technicalDetail: detail,
      logExcerpt: [`[launcher] ${detail}`],
      uploadLogExcerpt: [`[launcher] ${detail}`],
      repairAvailable: compatibilityProblem && failedProfile?.loader === "fabric",
    };
  }

  async function repairLaunchProfile(instanceId: string) {
    try {
      const report = await repairModConfiguration(instanceId);
      await refreshInstances();
      const disabled = report.disabledMods.length
        ? ` Disabled for this version: ${report.disabledMods.join(", ")}.`
        : "";
      crashDetails = null;
      showToast({
        title:
          report.resetVersions === 0 && report.disabledMods.length === 0
            ? "Profile already compatible"
            : "Profile repaired",
        message:
          report.resetVersions === 0 && report.disabledMods.length === 0
            ? "No changes were required. You can launch the profile again."
            : `${report.resetVersions} pinned mod version(s) reset.${disabled} You can launch the profile again.`,
        tone: "success",
      });
    } catch (cause) {
      showToast({
        title: "Profile repair failed",
        message: messageFromError(cause),
        tone: "error",
      });
      throw cause;
    }
  }

  async function uploadCrashToDashboard(crash: GameCrashDetails) {
    const crashedInstance = instances.find((item) => item.id === crash.instanceId);
    if (!crashedInstance) throw new Error("The crashed profile could not be found.");
    await uploadCrashReport({
      clientReportId: crash.reportId,
      profileId: crash.instanceId,
      profileName: crash.profileName,
      minecraftVersion: crash.minecraftVersion,
      loader: crashedInstance.loader,
      accountUuid: account?.uuid ?? null,
      accountName: account?.username ?? null,
      exitCode: crash.exitCode,
      title: crash.title,
      message: crash.message,
      suggestion: crash.suggestion,
      technicalDetail: crash.technicalDetail,
      logExcerpt: crash.uploadLogExcerpt ?? crash.logExcerpt,
    });
    showToast({
      title: "Crash report uploaded",
      message: "The report is now available in the Ninja Launcher dashboard.",
      tone: "success",
    });
  }

  async function repairFabric() {
    if (!instance || fabricRepairing) return;
    fabricRepairing = true;
    fabricRepairNotice = "";
    try {
      const report = await repairModConfiguration(instance.id);
      await refreshInstances();
      const disabled = report.disabledMods.length
        ? ` Disabled: ${report.disabledMods.join(", ")}.`
        : "";
      fabricRepairNotice =
        report.resetVersions === 0 && report.disabledMods.length === 0
          ? "The Fabric configuration is already compatible. Start the profile again."
          : `Fabric configuration repaired. ${report.resetVersions} pinned mod version(s) reset.${disabled} Start the profile again to apply the repair.`;
      fabricIssue = "";
      fabricIssueCandidate = "";
      fabricDialogOpen = false;
    } catch (cause) {
      fabricIssue = messageFromError(cause);
      throw cause;
    } finally {
      fabricRepairing = false;
    }
  }
  async function changeVersion(minecraftVersion: string) {
    if (!instance) return;
    const supported = selectedVersions.find((item) => item.minecraftVersion === minecraftVersion);
    if (!supported) return;
    try {
      await saveInstance({
        ...instance,
        minecraftVersion,
        loaderVersion:
          instance.loader === "fabric" ? supported.fabricLoaderVersion : instance.loaderVersion,
      });
      await refreshInstances();
      showToast({
        title: "Minecraft version changed",
        message: `${instance.name} now uses Minecraft ${minecraftVersion}.`,
        tone: "success",
      });
    } catch (error) {
      startupError = messageFromError(error);
    }
  }

  async function finishModpackInstall(created: LauncherInstance) {
    await refreshInstances();
    selectedId = created.id;
    activePage = "home";
  }

  async function installSelectedModpack(pack: ModrinthHit, versionId: string): Promise<void> {
    if (modpackInstallation?.phase === "installing") {
      throw new Error("Another modpack installation is already running.");
    }
    const pending: ModpackInstallation = {
      projectId: pack.projectId,
      name: pack.title,
      iconUrl: pack.iconUrl,
      source: "modrinth",
      phase: "installing",
    };
    modpackInstallation = pending;
    try {
      const created = await installModpack(pack.projectId, versionId, pack.title, pack.iconUrl);
      await finishModpackInstall(created);
      modpackInstallation = null;
    } catch (error) {
      modpackInstallation = {
        ...pending,
        phase: "failed",
        error: messageFromError(error),
      };
      throw error;
    }
  }

  async function importSelectedModpackFile(
    inspection: ModpackFileInspection,
    allowForeignMods: boolean,
  ): Promise<LauncherInstance> {
    if (modpackInstallation?.phase === "installing") {
      throw new Error("Another modpack installation is already running.");
    }
    const pending: ModpackInstallation = {
      projectId: `file:${inspection.path}`,
      name: inspection.name,
      iconUrl: null,
      source: "file",
      phase: "installing",
    };
    modpackInstallation = pending;
    try {
      const created = await importModpackFile(inspection.path, allowForeignMods);
      await refreshInstances();
      selectedId = created.id;
      modpackInstallation = { ...pending, phase: "complete" };
      showToast({
        title: "Modpack imported",
        message: `${inspection.name} is ready as a new profile.`,
        tone: "success",
      });
      window.setTimeout(() => {
        if (
          modpackInstallation?.projectId === pending.projectId &&
          modpackInstallation.phase === "complete"
        ) {
          modpackInstallation = null;
        }
      }, 3_000);
      return created;
    } catch (cause) {
      modpackInstallation = {
        ...pending,
        phase: "failed",
        error: messageFromError(cause),
      };
      throw cause;
    }
  }

  async function play(serverAddress?: string) {
    if (!instance || launching || gameRunning) return;
    if (clientBootstrap?.configuration.maintenanceActive) {
      showToast({
        title: "Maintenance",
        message:
          clientBootstrap.configuration.maintenanceMessage ||
          "The launcher is temporarily unavailable.",
        tone: "warning",
      });
      return;
    }
    if (!currentVersionAvailable) {
      showToast({
        title: "Version unavailable",
        message: `Minecraft ${instance.minecraftVersion} is no longer enabled in the Ninja Launcher dashboard. Choose another version before launching.`,
        tone: "warning",
      });
      return;
    }
    const launchingId = instance.id;
    requestedStopInstanceIds.delete(launchingId);
    fabricIssue = "";
    fabricIssueCandidate = "";
    fabricDialogOpen = false;
    crashDetails = null;
    gameProcessExited = false;
    minecraftClientStarted = false;
    fabricRepairNotice = "";
    launchingInstanceIds = [...launchingInstanceIds, launchingId];
    launchProgress = 3;
    launcherStatus = serverAddress
      ? `Preparing connection to ${serverAddress}…`
      : account
        ? "Preparing profile…"
        : "Preparing demo profile…";
    try {
      await launchGame(launchingId, account === null, serverAddress);
      if (launchingInstanceIds.includes(launchingId)) {
        launchingInstanceIds = launchingInstanceIds.filter((id) => id !== launchingId);
        launchProgress = 100;
        launcherStatus = "Minecraft is running";
      }
    } catch (error) {
      launchingInstanceIds = launchingInstanceIds.filter((id) => id !== launchingId);
      launchProgress = null;
      const message = messageFromError(error);
      launcherStatus = message;
      selectedConsoleId = launchingId;
      gameLogs = [
        ...gameLogs.filter((entry) => entry.instanceId !== launchingId),
        {
          instanceId: launchingId,
          stream: "stderr",
          line: `[Ninja Launcher] Launch failed: ${message}`,
        },
      ];
      const launchFailure = createLaunchFailureDetails(launchingId, message);
      crashDetails = launchFailure;
      showToast({ title: launchFailure.title, message: launchFailure.message, tone: "error" });
      if (settings.showLogsOnLaunch) activePage = "logs";
    }
  }

  async function stopInstance(instanceId: string) {
    if (stoppingInstanceIds.includes(instanceId)) return;
    const target = instances.find((item) => item.id === instanceId);
    const targetName = target?.name ?? "Minecraft";
    const stoppingId = instanceId;
    requestedStopInstanceIds.add(stoppingId);
    stoppingInstanceIds = [...stoppingInstanceIds, stoppingId];
    launcherStatus = `Stopping ${targetName}…`;
    try {
      await stopGame(stoppingId);
      runningInstanceIds = runningInstanceIds.filter((id) => id !== stoppingId);
      launcherStatus = `${targetName} stopped`;
    } catch (cause) {
      requestedStopInstanceIds.delete(stoppingId);
      launcherStatus = messageFromError(cause);
      runningInstanceIds = await listRunningInstances();
    } finally {
      stoppingInstanceIds = stoppingInstanceIds.filter((id) => id !== stoppingId);
    }
  }

  async function stopSelectedGame() {
    if (!instance) return;
    await stopInstance(instance.id);
  }

  function dismissStartScreen() {
    const startScreen = document.getElementById("app-start-screen");
    if (!startScreen) return;
    startScreen.classList.add("is-hidden");
    window.setTimeout(() => startScreen.remove(), 260);
  }

  onMount(() => {
    let unlisteners: (() => void)[] = [];
    const preventContextMenu = (event: MouseEvent) => event.preventDefault();
    const refreshWhenFocused = () => void refreshClientBootstrap();
    const refreshInterval = window.setInterval(() => void refreshClientBootstrap(), 60_000);
    document.addEventListener("contextmenu", preventContextMenu);
    window.addEventListener("focus", refreshWhenFocused);
    if ("__TAURI_INTERNALS__" in window) {
      showCustomTitlebar = true;
      Promise.all([
        listen<GameLogEntry>("launcher://log", (event) => {
          const otherLogs = gameLogs.filter(
            (entry) => entry.instanceId !== event.payload.instanceId,
          );
          const instanceLogs = gameLogs
            .filter((entry) => entry.instanceId === event.payload.instanceId)
            .slice(-1999);
          gameLogs = [...otherLogs, ...instanceLogs, event.payload];
          if (event.payload.instanceId === selectedId) collectFabricIssue(event.payload.line);
        }),
        listen<{ phase: string; message: string }>("launcher://status", (event) => {
          launcherStatus = event.payload.message;
          const progressByPhase: Record<string, number> = {
            validate: 8,
            mods: 24,
            minecraft: 42,
            java: 58,
            downloads: 74,
            launch: 94,
          };
          launchProgress = Math.max(
            launchProgress ?? 0,
            progressByPhase[event.payload.phase] ?? launchProgress ?? 3,
          );
        }),
        listen<string>("launcher://game-started", (event) => {
          gameLogs = gameLogs.filter((entry) => entry.instanceId !== event.payload);
          selectedConsoleId = event.payload;
          runningInstanceIds = [...new Set([...runningInstanceIds, event.payload])];
          gameProcessExited = false;
          minecraftClientStarted = false;
          launchingInstanceIds = launchingInstanceIds.filter((id) => id !== event.payload);
          launchProgress = 100;
          launcherStatus = "Minecraft is running";
          if (settings.shareAnonymousUsageData) {
            const startedInstance = instances.find((item) => item.id === event.payload);
            void recordClientEvent("GAME_START", startedInstance?.minecraftVersion).catch(() => {});
          }
          if (settings.showLogsOnLaunch) activePage = "logs";
        }),
        listen<{ instanceId: string; exitCode: number | null }>(
          "launcher://game-exited",
          (event) => {
            const exitedInstanceLogs = gameLogs.filter(
              (entry) => entry.instanceId === event.payload.instanceId,
            );
            const stopWasRequested = requestedStopInstanceIds.delete(event.payload.instanceId);
            const stoppedByUser =
              stopWasRequested || stoppingInstanceIds.includes(event.payload.instanceId);
            if (event.payload.exitCode !== 0 && !stoppedByUser) {
              gameLogs = [
                ...gameLogs,
                {
                  instanceId: event.payload.instanceId,
                  stream: "stderr",
                  line: `[Ninja Launcher] Minecraft exited unexpectedly with code ${event.payload.exitCode ?? "unknown"}.`,
                },
              ];
              const hasRepairableFabricFailure =
                !minecraftClientStarted &&
                exitedInstanceLogs.some((entry) => startsRepairableFabricLoaderFailure(entry.line));
              if (!hasRepairableFabricFailure) {
                crashDetails = createCrashDetails(
                  event.payload.instanceId,
                  event.payload.exitCode,
                  exitedInstanceLogs,
                );
              }
            }
            runningInstanceIds = runningInstanceIds.filter((id) => id !== event.payload.instanceId);
            launchingInstanceIds = launchingInstanceIds.filter(
              (id) => id !== event.payload.instanceId,
            );
            stoppingInstanceIds = stoppingInstanceIds.filter(
              (id) => id !== event.payload.instanceId,
            );
            gameProcessExited = true;
            launchProgress = null;
            launcherStatus = stoppedByUser
              ? "Minecraft stopped"
              : event.payload.exitCode === 0
                ? "Minecraft closed normally"
                : `Minecraft exited with code ${event.payload.exitCode ?? "unknown"}`;
            if (!stoppedByUser) window.setTimeout(finalizeFabricFailure, 300);
          },
        ),
      ]).then((listeners) => (unlisteners = listeners));
    }
    const instancesRequest = listInstances();
    Promise.all([
      getSettings(),
      getLauncherOverview(),
      instancesRequest,
      getAccounts(),
      listSupportedVersions(),
      listVanillaVersions(),
      listRunningInstances(),
      instancesRequest
        .then((loadedInstances) => {
          const selectedInstance =
            loadedInstances.find((item) => item.id === selectedId) ?? loadedInstances[0];
          return getLauncherBootstrap(selectedInstance?.minecraftVersion);
        })
        .catch((error) => {
          controlServiceError = `${messageFromError(error)} Local launcher features remain available.`;
          return null;
        }),
    ])
      .then(
        ([
          loadedSettings,
          loadedOverview,
          loadedInstances,
          loadedAccounts,
          loadedVersions,
          loadedVanillaVersions,
          loadedRunning,
          loadedClientBootstrap,
        ]) => {
          settings = loadedSettings;
          overview = loadedOverview;
          instances = loadedInstances;
          accounts = loadedAccounts.accounts;
          account =
            accounts.find((item) => item.uuid === loadedAccounts.activeUuid) ?? accounts[0] ?? null;
          allVersions = loadedVersions;
          vanillaVersions = loadedVanillaVersions;
          runningInstanceIds = loadedRunning;
          clientBootstrap = loadedClientBootstrap;
          if (loadedSettings.shareAnonymousUsageData) {
            void recordClientEvent("LAUNCHER_START").catch(() => {});
          }
          if (loadedSettings.checkForUpdates) void runAutomaticUpdate();
        },
      )
      .catch((error) => (startupError = messageFromError(error)))
      .finally(() => {
        loading = false;
        requestAnimationFrame(dismissStartScreen);
      });
    return () => {
      window.clearInterval(refreshInterval);
      window.removeEventListener("focus", refreshWhenFocused);
      document.removeEventListener("contextmenu", preventContextMenu);
      unlisteners.forEach((unlisten) => unlisten());
    };
  });
</script>

<svelte:head
  ><meta name="description" content="Ninja Launcher for Minecraft Java Edition" /></svelte:head
>

<div
  class="launcher-shell relative flex h-screen min-h-[640px] flex-col overflow-hidden text-[#f1f7ff] {launcherAssembly
    ? 'launcher-assembly'
    : ''} {showCustomTitlebar ? 'rounded-[12px] [.window-maximized_&]:rounded-none' : ''}"
>
  {#if showCustomTitlebar}<WindowTitlebar
      {notifications}
      {runningInstances}
      {stoppingInstanceIds}
      onsettings={() => (activePage = "settings")}
      onstopinstance={stopInstance}
      onopenconsole={(id) => {
        selectedConsoleId = id;
        activePage = "logs";
      }}
      ondismissnotification={dismissNotification}
      onclearnotifications={clearAllNotifications}
      onnotificationaction={openNotification}
    />{/if}
  {#if bannerAnnouncements[0]}
    <AnnouncementBanner
      announcement={bannerAnnouncements[0]}
      ondismiss={(id) => (dismissedAnnouncementIds = [...dismissedAnnouncementIds, id])}
    />
  {/if}
  <div class="launcher-body flex min-h-0 flex-1">
    <Sidebar
      active={activePage}
      {account}
      rank={networkRanks?.ranks.find((rank) => rank.primary) ?? networkRanks?.ranks[0] ?? null}
      {networkRanks}
      halloweenTheme={halloweenThemeActive}
      modCount={!instance || instance.loader === "vanilla"
        ? null
        : instance.mods.length + (instance.modpack?.packMods.length ?? 0)}
      onnavigate={(page) => (activePage = page)}
      onaccount={() => (accountOpen = true)}
    />
    <main
      bind:this={mainElement}
      class="launcher-main min-w-0 flex-1 overflow-x-hidden overscroll-contain {activePage ===
      'home'
        ? 'overflow-y-hidden'
        : 'overflow-y-auto'}"
    >
      <div class="mx-auto w-full max-w-[1120px] py-6 pl-8 pr-16">
        {#if loading}
          <div class="grid min-h-[500px] place-items-center">
            <div class="flex flex-col items-center gap-5">
              <img
                src="/brand/ninja-launcher-mark.png"
                alt="Ninja Launcher"
                class="h-20 w-24 animate-pulse object-contain"
              />
              <div class="h-2 w-36 overflow-hidden bg-blue-950">
                <div class="h-full w-2/3 animate-pulse bg-cyan-400"></div>
              </div>
            </div>
          </div>
        {:else}
          {#key activePage}
            <div class="page-stage">
              {#if activePage === "instances"}<InstancesPage
                  {instances}
                  {versions}
                  vanillaVersions={managedVanillaVersions.filter(
                    (version) => settings.showSnapshots || version.versionType !== "snapshot",
                  )}
                  {selectedId}
                  installation={modpackInstallation}
                  runningIds={runningInstanceIds}
                  onselect={(id) => (selectedId = id)}
                  onchanged={refreshInstances}
                  onimportfile={importSelectedModpackFile}
                />
              {:else if activePage === "skins"}{#key account?.uuid}<SkinsPage
                    {account}
                    onnotify={showToast}
                  />{/key}
              {:else if activePage === "logs"}<LogsPage
                  consoles={instanceConsoles}
                  selectedId={selectedConsoleId}
                  onselect={(id) => (selectedConsoleId = id)}
                  onclear={(id) => (gameLogs = gameLogs.filter((entry) => entry.instanceId !== id))}
                />
              {:else if activePage === "settings"}<SettingsPage
                  bind:settings
                  {overview}
                  halloweenAvailable={halloweenThemeAvailable}
                />
              {:else if activePage === "modpacks"}<ModpacksPage
                  {versions}
                  installation={modpackInstallation}
                  oninstall={installSelectedModpack}
                  onimportfile={importSelectedModpackFile}
                />
              {:else if !instance}<div class="mc-panel p-10 text-center">
                  <h2 class="block-title text-lg uppercase">No profile selected</h2>
                  <p class="mt-2 text-sm text-[#7595b7]">
                    Create a profile to start Minecraft or manage mods.
                  </p>
                  <button
                    onclick={() => (activePage = "instances")}
                    class="mc-primary mt-6 px-5 py-3 text-xs font-bold">Create profile</button
                  >
                </div>
              {:else if activePage === "home"}<HomePage
                  {instance}
                  {instances}
                  versions={selectedVersions}
                  {account}
                  signedIn={account !== null}
                  {networkRanks}
                  halloweenTheme={halloweenThemeActive}
                  halloweenSoundsEnabled={settings.halloweenSoundsEnabled}
                  animationsEnabled={settings.animationsEnabled}
                  {launching}
                  running={gameRunning}
                  {stopping}
                  progress={launchProgress}
                  status={launcherStatus}
                  maintenanceActive={clientBootstrap?.configuration.maintenanceActive ?? false}
                  maintenanceMessage={clientBootstrap?.configuration.maintenanceMessage ?? ""}
                  maintenanceEndsAt={clientBootstrap?.configuration.maintenanceEndsAt ?? null}
                  versionAvailable={currentVersionAvailable}
                  onplay={() => play()}
                  onconnect={play}
                  onstop={stopSelectedGame}
                  onopenmods={() => (activePage = "mods")}
                  onversionchange={changeVersion}
                  onprofilechange={(id) => (selectedId = id)}
                />
              {:else if instance.loader === "vanilla"}<div class="mc-panel p-8">
                  <div class="flex items-start gap-4">
                    <span
                      class="grid size-12 shrink-0 place-items-center text-[var(--color-primary)]"
                      ><Icon name="vanilla" size={24} /></span
                    >
                    <div>
                      <h2 class="text-lg font-bold">Vanilla profile</h2>
                      <p class="mt-2 max-w-xl text-xs leading-6 text-[var(--color-text-muted)]">
                        Vanilla profiles launch Minecraft without Fabric. Choose or create a Fabric
                        profile to install mods.
                      </p>
                    </div>
                  </div>
                </div>{:else}<ModsPage
                  {instance}
                  versions={selectedVersions}
                  onchanged={refreshInstances}
                  onversionchange={changeVersion}
                  onnotify={showToast}
                />{/if}
            </div>
          {/key}
        {/if}
      </div>
    </main>
  </div>
</div>
{#if halloweenThemeActive && settings.halloweenSoundsEnabled && !gameRunning && !launching}
  <HalloweenIdleSounds />
{/if}
{#if halloweenIntroOpen}
  <HalloweenIntroDialog
    onclose={() => dismissHalloweenIntro()}
    onopensettings={() => dismissHalloweenIntro(true)}
  />
{/if}
{#if modpackInstallation}
  <aside
    class="fixed bottom-5 right-5 z-[170] w-[330px] overflow-hidden rounded-[12px] border border-white/[.11] bg-[var(--color-section-background)] shadow-[0_22px_70px_rgba(0,0,0,.58)] backdrop-blur-xl"
    aria-live="polite"
    aria-atomic="true"
  >
    <button
      type="button"
      onclick={() => (activePage = "instances")}
      class="flex w-full items-center gap-3 p-4 text-left transition hover:bg-white/[.035]"
    >
      <span
        class="grid size-11 shrink-0 place-items-center overflow-hidden rounded-[8px] border border-white/[.09] bg-white/[.04]"
      >
        {#if modpackInstallation.iconUrl}
          <img
            src={modpackInstallation.iconUrl}
            alt=""
            class="size-full object-cover"
            draggable="false"
          />
        {:else}
          <Icon name="modpacks" size={18} />
        {/if}
      </span>
      <span class="min-w-0 flex-1">
        <span
          class="block text-[9px] font-bold uppercase tracking-[.14em] text-[var(--color-primary)]"
        >
          {modpackInstallation.phase === "installing"
            ? modpackInstallation.source === "file"
              ? "Importing modpack"
              : "Installing modpack"
            : modpackInstallation.phase === "complete"
              ? "Import complete"
              : "Import failed"}
        </span>
        <span class="mt-1 block truncate text-xs font-bold text-white">
          {modpackInstallation.name}
        </span>
        <span class="mt-1 block truncate text-[10px] text-[var(--color-text-muted)]">
          {modpackInstallation.phase === "installing"
            ? "Creating the profile in the background…"
            : modpackInstallation.phase === "complete"
              ? "The new profile is ready to use."
              : (modpackInstallation.error ?? "The import could not be completed.")}
        </span>
      </span>
      {#if modpackInstallation.phase === "installing"}
        <span
          class="size-5 shrink-0 animate-spin rounded-full border-2 border-white/10 border-t-[var(--color-primary)]"
          aria-label="Import in progress"
        ></span>
      {:else if modpackInstallation.phase === "complete"}
        <span class="text-[var(--color-success)]"><Icon name="check" size={17} /></span>
      {:else}
        <span class="text-[var(--color-danger)]"><Icon name="warning" size={17} /></span>
      {/if}
    </button>
    {#if modpackInstallation.phase === "installing"}
      <div class="h-1 overflow-hidden bg-white/[.05]">
        <div class="modpack-import-progress h-full w-2/5 bg-[var(--color-primary)]"></div>
      </div>
    {:else}
      <button
        type="button"
        onclick={() => (modpackInstallation = null)}
        class="absolute right-1.5 top-1.5 grid size-7 place-items-center rounded-[6px] text-[var(--color-text-muted)] hover:bg-white/[.07] hover:text-white"
        aria-label="Dismiss import status"
      >
        <Icon name="close" size={11} />
      </button>
    {/if}
  </aside>
{/if}
<ToastStack items={toastItems} ondismiss={dismissToast} />
{#if crashDetails}
  <CrashDialog
    crash={crashDetails}
    onclose={() => (crashDetails = null)}
    onrepair={repairLaunchProfile}
    onupload={uploadCrashToDashboard}
    onopenconsole={(instanceId) => {
      selectedConsoleId = instanceId;
      activePage = "logs";
      crashDetails = null;
    }}
  />
{/if}
{#if !showCustomTitlebar}<NotificationCenter
    items={notifications}
    ondismiss={dismissNotification}
    onclear={clearAllNotifications}
    onaction={openNotification}
  />{/if}
{#if fabricDialogOpen && fabricCrashDetails}
  <CrashDialog
    crash={fabricCrashDetails}
    onclose={() => (fabricDialogOpen = false)}
    onrepair={repairFabric}
    onupload={uploadCrashToDashboard}
    onopenconsole={(instanceId) => {
      selectedConsoleId = instanceId;
      activePage = "logs";
      fabricDialogOpen = false;
    }}
  />
{/if}
{#if !loading && !settings.setupComplete}
  <SetupWizard
    {settings}
    {overview}
    initialAccount={account}
    initialAccounts={accounts}
    defaultInstanceId={selectedId || "default-fabric"}
    oninstalled={refreshAfterSetup}
    onfinish={(completedSettings, completedAccount, completedAccounts) => {
      launcherAssembly = completedSettings.animationsEnabled;
      settings = completedSettings;
      account = completedAccount;
      accounts = completedAccounts;
      if (launcherAssembly) {
        window.setTimeout(() => (launcherAssembly = false), 2400);
      }
    }}
  />
{/if}
{#if accountOpen}<AccountDialog
    bind:account
    bind:accounts
    {networkRanks}
    onclose={() => (accountOpen = false)}
  />{/if}
{#if appUpdate && ["downloading", "installing", "restarting"].includes(appUpdate.phase)}
  <div class="fixed inset-0 z-[80] grid place-items-center bg-[#010612]/90 p-6">
    <section class="mc-panel w-full max-w-md overflow-hidden" aria-live="assertive">
      <div class="border-b border-cyan-300/10 bg-[#071a35] px-6 py-5">
        <div class="mb-4 flex items-center gap-4">
          <img src="/brand/ninja-launcher-mark.png" alt="" class="h-12 w-14 object-contain" />
          <div>
            <p class="text-[9px] font-bold uppercase tracking-[.15em] text-cyan-300/60">
              Automatic update
            </p>
            <h2 class="block-title mt-1 text-lg uppercase">
              Ninja Launcher {appUpdate.version}
            </h2>
          </div>
        </div>
        <p class="text-xs text-[#86a8c7]">
          {appUpdate.phase === "downloading"
            ? `Downloading update… ${updatePercent}%`
            : appUpdate.phase === "installing"
              ? "Installing the verified update…"
              : "Restarting Ninja Launcher…"}
        </p>
      </div>
      <div class="bg-[#030c1c] p-6">
        <div class="h-2 overflow-hidden bg-blue-950">
          <div
            class="h-full bg-cyan-400 transition-all duration-300"
            style={`width: ${appUpdate.phase === "downloading" ? Math.max(4, updatePercent) : 100}%`}
          ></div>
        </div>
        <p class="mt-3 text-[10px] leading-5 text-[#557896]">
          The update was signed by Ninja Launcher. Do not close the application while it is being
          installed.
        </p>
      </div>
    </section>
  </div>
{/if}

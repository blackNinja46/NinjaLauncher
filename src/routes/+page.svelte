<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import Sidebar from "$lib/components/layout/Sidebar.svelte";
  import WindowTitlebar from "$lib/components/layout/WindowTitlebar.svelte";
  import HomePage from "$lib/components/launcher/HomePage.svelte";
  import ModsPage from "$lib/components/mods/ModsPage.svelte";
  import ModpacksPage from "$lib/components/modpacks/ModpacksPage.svelte";
  import InstancesPage from "$lib/components/instances/InstancesPage.svelte";
  import SettingsPage from "$lib/components/settings/SettingsPage.svelte";
  import LogsPage from "$lib/components/logs/LogsPage.svelte";
  import AccountDialog from "$lib/components/account/AccountDialog.svelte";
  import {
    getAccounts,
    getLauncherOverview,
    getSettings,
    installAvailableUpdate,
    installModpack,
    launchGame,
    listInstances,
    listRunningInstances,
    listSupportedVersions,
    messageFromError,
    repairModConfiguration,
    saveInstance,
    stopGame,
    type AppUpdateProgress,
  } from "$lib/api/tauri";
  import type {
    Account,
    GameLogEntry,
    LauncherInstance,
    LauncherOverview,
    LauncherSettings,
    ModpackInstallation,
    ModrinthHit,
    NavigationItem,
    SupportedVersion,
  } from "$lib/types";

  let activePage = $state<NavigationItem>("home");
  let loading = $state(true);
  let startupError = $state("");
  let settings = $state<LauncherSettings>({
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
  let accountOpen = $state(false);
  let allVersions = $state<SupportedVersion[]>([]);
  let versions = $derived(
    allVersions.filter((version) => settings.showSnapshots || version.versionType === "release"),
  );
  let gameLogs = $state<GameLogEntry[]>([]);
  let runningInstanceIds = $state<string[]>([]);
  let launchingInstanceIds = $state<string[]>([]);
  let stoppingInstanceIds = $state<string[]>([]);
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
  let showWindowsTitlebar = $state(false);
  let instance = $derived(instances.find((item) => item.id === selectedId) ?? instances[0]);
  let gameRunning = $derived(Boolean(instance && runningInstanceIds.includes(instance.id)));
  let launching = $derived(Boolean(instance && launchingInstanceIds.includes(instance.id)));
  let stopping = $derived(Boolean(instance && stoppingInstanceIds.includes(instance.id)));
  let updatePercent = $derived(
    appUpdate?.totalBytes && appUpdate.downloadedBytes
      ? Math.min(100, Math.round((appUpdate.downloadedBytes / appUpdate.totalBytes) * 100))
      : 0,
  );

  async function runAutomaticUpdate() {
    try {
      const installing = await installAvailableUpdate((progress) => (appUpdate = progress));
      if (!installing) appUpdate = null;
    } catch (cause) {
      appUpdate = null;
      updateError = `Automatic update failed: ${messageFromError(cause)}`;
    }
  }

  async function refreshInstances() {
    instances = await listInstances();
    if (!instances.some((item) => item.id === selectedId)) selectedId = instances[0]?.id ?? "";
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
      fabricDialogOpen = true;
    } else if (gameProcessExited && minecraftClientStarted) {
      fabricIssueCandidate = "";
    }
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
      fabricRepairNotice = `Fabric configuration repaired. ${report.resetVersions} pinned mod version(s) reset.${disabled} Start the profile again to apply the repair.`;
      fabricIssue = "";
      fabricIssueCandidate = "";
      fabricDialogOpen = false;
    } catch (cause) {
      fabricIssue = messageFromError(cause);
    } finally {
      fabricRepairing = false;
    }
  }
  async function changeVersion(minecraftVersion: string) {
    if (!instance) return;
    const supported = versions.find((item) => item.minecraftVersion === minecraftVersion);
    if (!supported) return;
    try {
      await saveInstance({
        ...instance,
        minecraftVersion,
        loaderVersion: supported.fabricLoaderVersion,
      });
      await refreshInstances();
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

  async function play(serverAddress?: string) {
    if (!instance || launching || gameRunning) return;
    const launchingId = instance.id;
    fabricIssue = "";
    fabricIssueCandidate = "";
    fabricDialogOpen = false;
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
      launcherStatus = messageFromError(error);
    }
  }

  async function stopSelectedGame() {
    if (!instance || stopping) return;
    const stoppingId = instance.id;
    stoppingInstanceIds = [...stoppingInstanceIds, stoppingId];
    launcherStatus = `Stopping ${instance.name}…`;
    try {
      await stopGame(stoppingId);
      runningInstanceIds = runningInstanceIds.filter((id) => id !== stoppingId);
      launcherStatus = `${instance.name} stopped`;
    } catch (cause) {
      launcherStatus = messageFromError(cause);
      runningInstanceIds = await listRunningInstances();
    } finally {
      stoppingInstanceIds = stoppingInstanceIds.filter((id) => id !== stoppingId);
    }
  }

  function dismissStartScreen() {
    const startScreen = document.getElementById("app-start-screen");
    if (!startScreen) return;
    startScreen.classList.add("is-hidden");
    window.setTimeout(() => startScreen.remove(), 260);
  }

  onMount(() => {
    let unlisteners: (() => void)[] = [];
    if ("__TAURI_INTERNALS__" in window) {
      showWindowsTitlebar = navigator.userAgent.includes("Windows");
      Promise.all([
        listen<GameLogEntry>("launcher://log", (event) => {
          gameLogs = [...gameLogs.slice(-1999), event.payload];
          collectFabricIssue(event.payload.line);
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
          gameLogs = [];
          runningInstanceIds = [...new Set([...runningInstanceIds, event.payload])];
          gameProcessExited = false;
          minecraftClientStarted = false;
          launchingInstanceIds = launchingInstanceIds.filter((id) => id !== event.payload);
          launchProgress = 100;
          launcherStatus = "Minecraft is running";
          if (settings.showLogsOnLaunch) activePage = "logs";
        }),
        listen<{ instanceId: string; exitCode: number | null }>(
          "launcher://game-exited",
          (event) => {
            runningInstanceIds = runningInstanceIds.filter((id) => id !== event.payload.instanceId);
            launchingInstanceIds = launchingInstanceIds.filter(
              (id) => id !== event.payload.instanceId,
            );
            stoppingInstanceIds = stoppingInstanceIds.filter(
              (id) => id !== event.payload.instanceId,
            );
            gameProcessExited = true;
            launchProgress = null;
            launcherStatus =
              event.payload.exitCode === 0
                ? "Minecraft closed normally"
                : `Minecraft exited with code ${event.payload.exitCode ?? "unknown"}`;
            window.setTimeout(finalizeFabricFailure, 300);
          },
        ),
      ]).then((listeners) => (unlisteners = listeners));
    }
    Promise.all([
      getSettings(),
      getLauncherOverview(),
      listInstances(),
      getAccounts(),
      listSupportedVersions(),
      listRunningInstances(),
    ])
      .then(
        ([
          loadedSettings,
          loadedOverview,
          loadedInstances,
          loadedAccounts,
          loadedVersions,
          loadedRunning,
        ]) => {
          settings = loadedSettings;
          overview = loadedOverview;
          instances = loadedInstances;
          accounts = loadedAccounts.accounts;
          account =
            accounts.find((item) => item.uuid === loadedAccounts.activeUuid) ?? accounts[0] ?? null;
          allVersions = loadedVersions;
          runningInstanceIds = loadedRunning;
          if (loadedSettings.checkForUpdates) void runAutomaticUpdate();
        },
      )
      .catch((error) => (startupError = messageFromError(error)))
      .finally(() => {
        loading = false;
        requestAnimationFrame(dismissStartScreen);
      });
    return () => unlisteners.forEach((unlisten) => unlisten());
  });
</script>

<svelte:head
  ><meta name="description" content="Ninja Launcher for Minecraft Java Edition" /></svelte:head
>

<div
  class="launcher-shell flex h-screen min-h-[640px] flex-col overflow-hidden text-[#f1f7ff] {showWindowsTitlebar
    ? 'border border-cyan-300/20'
    : ''}"
>
  {#if showWindowsTitlebar}<WindowTitlebar />{/if}
  <div class="flex min-h-0 flex-1">
    <Sidebar
      active={activePage}
      {account}
      modCount={instance?.mods.length ?? 0}
      onnavigate={(page) => (activePage = page)}
      onaccount={() => (accountOpen = true)}
    />
    <main class="pixel-grid min-w-0 flex-1 overflow-y-auto">
      <div class="mx-auto w-full max-w-[1040px] px-9 py-8">
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
          {#if startupError}<div
              class="mb-5 rounded-lg border border-red-400/20 bg-red-400/10 px-4 py-3 text-xs text-red-300"
            >
              {startupError}
            </div>{/if}
          {#if updateError}<div
              class="mb-5 border border-amber-400/20 bg-amber-400/[0.08] px-4 py-3 text-xs text-amber-200"
            >
              {updateError}
            </div>{/if}
          {#if gameRunning && fabricIssueCandidate && !fabricDialogOpen}<button
              onclick={() => {
                fabricIssue = fabricIssueCandidate;
                fabricDialogOpen = true;
              }}
              class="mb-4 ml-auto flex items-center gap-2 border border-amber-300/20 bg-amber-300/[0.06] px-3 py-2 text-[10px] text-amber-100/75 hover:bg-amber-300/10"
              aria-live="polite"
            >
              <span class="size-1.5 bg-amber-300"></span>Fabric warning detected · Minecraft is
              still running
            </button>{/if}
          {#if fabricRepairNotice}<div
              class="mb-5 border border-emerald-300/25 bg-emerald-400/[0.07] px-4 py-3 text-xs text-emerald-100"
            >
              {fabricRepairNotice}
            </div>{/if}
          {#if activePage === "instances"}<InstancesPage
              {instances}
              {versions}
              {selectedId}
              installation={modpackInstallation}
              runningIds={runningInstanceIds}
              onselect={(id) => (selectedId = id)}
              onchanged={refreshInstances}
            />
          {:else if activePage === "logs"}<LogsPage
              logs={gameLogs}
              running={gameRunning}
              onclear={() => (gameLogs = [])}
            />
          {:else if activePage === "settings"}<SettingsPage bind:settings {overview} />
          {:else if activePage === "modpacks"}<ModpacksPage
              {versions}
              installation={modpackInstallation}
              oninstall={installSelectedModpack}
              onimport={finishModpackInstall}
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
              {versions}
              signedIn={account !== null}
              {launching}
              running={gameRunning}
              {stopping}
              progress={launchProgress}
              status={launcherStatus}
              onplay={() => play()}
              onconnect={play}
              onstop={stopSelectedGame}
              onopenmods={() => (activePage = "mods")}
              onversionchange={changeVersion}
            />
          {:else}<ModsPage
              {instance}
              {versions}
              onchanged={refreshInstances}
              onversionchange={changeVersion}
            />{/if}
        {/if}
      </div>
    </main>
  </div>
</div>
{#if fabricDialogOpen && fabricIssue}<div
    class="fixed inset-0 z-[75] grid place-items-center bg-[#010612]/88 p-6"
    role="presentation"
    onclick={(event) =>
      event.currentTarget === event.target && !fabricRepairing && (fabricDialogOpen = false)}
  >
    <div
      class="mc-panel w-full max-w-2xl overflow-hidden border-red-400/30"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="fabric-error-title"
    >
      <div class="flex items-start gap-4 border-b border-red-300/15 bg-red-400/[0.08] px-6 py-5">
        <div
          class="grid size-11 shrink-0 place-items-center bg-red-400/15 text-xl font-black text-red-200"
        >
          !
        </div>
        <div>
          <p class="text-[9px] font-bold uppercase tracking-[.16em] text-red-300/65">
            Fabric Loader
          </p>
          <h2 id="fabric-error-title" class="block-title mt-1 text-lg uppercase">
            Incompatible mods detected
          </h2>
          <p class="mt-2 text-xs leading-5 text-red-100/65">
            Minecraft could not start correctly. Ninja Launcher can reset incompatible pinned
            versions and disable an explicitly identified personal conflict mod if required.
          </p>
        </div>
      </div>
      <div class="p-6">
        <pre
          class="max-h-72 overflow-auto whitespace-pre-wrap break-words border border-red-300/12 bg-[#010714] p-4 font-mono text-[10px] leading-5 text-[#d7b8bf]">{fabricIssue}</pre>
        {#if gameRunning}<p
            class="mt-3 border border-amber-300/15 bg-amber-300/[0.05] px-3 py-2 text-[10px] text-amber-100/70"
          >
            Minecraft is still running. Automatic repair becomes available after the game closes.
          </p>{/if}
        <div class="mt-5 flex justify-end gap-3">
          <button
            onclick={() => (fabricDialogOpen = false)}
            disabled={fabricRepairing}
            class="mc-secondary px-4 py-2.5 text-xs font-bold disabled:opacity-50">Close</button
          >
          <button
            onclick={repairFabric}
            disabled={fabricRepairing || gameRunning || launching}
            class="border border-red-300/35 bg-red-400/15 px-4 py-2.5 text-xs font-bold uppercase tracking-wider text-red-100 hover:bg-red-400/25 disabled:opacity-50"
            >{fabricRepairing ? "Repairing…" : "Repair automatically"}</button
          >
        </div>
      </div>
    </div>
  </div>{/if}
{#if accountOpen}<AccountDialog
    bind:account
    bind:accounts
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

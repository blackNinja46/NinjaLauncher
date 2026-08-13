<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import Sidebar from "$lib/components/layout/Sidebar.svelte";
  import HomePage from "$lib/components/launcher/HomePage.svelte";
  import ModsPage from "$lib/components/mods/ModsPage.svelte";
  import InstancesPage from "$lib/components/instances/InstancesPage.svelte";
  import SettingsPage from "$lib/components/settings/SettingsPage.svelte";
  import LogsPage from "$lib/components/logs/LogsPage.svelte";
  import AccountDialog from "$lib/components/account/AccountDialog.svelte";
  import {
    getAccount,
    getLauncherOverview,
    getSettings,
    listInstances,
    listSupportedVersions,
    messageFromError,
    saveInstance,
  } from "$lib/api/tauri";
  import type {
    Account,
    GameLogEntry,
    LauncherInstance,
    LauncherOverview,
    LauncherSettings,
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
    language: "en",
    theme: "dark",
    checkForUpdates: true,
    microsoftClientId: null,
  });
  let overview = $state<LauncherOverview>({
    appDataDirectory: "NinjaLauncher",
    instanceCount: 1,
    version: "0.1.0",
  });
  let instances = $state<LauncherInstance[]>([]);
  let selectedId = $state("default-fabric");
  let account = $state<Account | null>(null);
  let accountOpen = $state(false);
  let versions = $state<SupportedVersion[]>([]);
  let gameLogs = $state<GameLogEntry[]>([]);
  let gameRunning = $state(false);
  let instance = $derived(instances.find((item) => item.id === selectedId) ?? instances[0]);

  async function refreshInstances() {
    instances = await listInstances();
    if (!instances.some((item) => item.id === selectedId)) selectedId = instances[0]?.id ?? "";
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

  onMount(() => {
    let unlisteners: (() => void)[] = [];
    if ("__TAURI_INTERNALS__" in window) {
      Promise.all([
        listen<GameLogEntry>("launcher://log", (event) => {
          gameLogs = [...gameLogs.slice(-1999), event.payload];
        }),
        listen("launcher://game-started", () => {
          gameLogs = [];
          gameRunning = true;
          if (settings.showLogsOnLaunch) activePage = "logs";
        }),
        listen("launcher://game-exited", () => (gameRunning = false)),
      ]).then((listeners) => (unlisteners = listeners));
    }
    Promise.all([
      getSettings(),
      getLauncherOverview(),
      listInstances(),
      getAccount(),
      listSupportedVersions(),
    ])
      .then(([loadedSettings, loadedOverview, loadedInstances, loadedAccount, loadedVersions]) => {
        settings = loadedSettings;
        overview = loadedOverview;
        instances = loadedInstances;
        account = loadedAccount;
        versions = loadedVersions;
      })
      .catch((error) => (startupError = messageFromError(error)))
      .finally(() => (loading = false));
    return () => unlisteners.forEach((unlisten) => unlisten());
  });
</script>

<svelte:head
  ><meta name="description" content="Ninja Launcher for Minecraft Java Edition" /></svelte:head
>

<div class="launcher-shell flex h-screen min-h-[640px] text-[#f1f7ff]">
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
        {#if activePage === "instances"}<InstancesPage
            {instances}
            {versions}
            {selectedId}
            onselect={(id) => (selectedId = id)}
            onchanged={refreshInstances}
          />
        {:else if activePage === "logs"}<LogsPage
            logs={gameLogs}
            running={gameRunning}
            onclear={() => (gameLogs = [])}
          />
        {:else if activePage === "settings"}<SettingsPage bind:settings {overview} />
        {:else if !instance}<div class="mc-panel p-10 text-center">
            <h2 class="block-title text-lg uppercase">No instance selected</h2>
            <p class="mt-2 text-sm text-[#7595b7]">
              Create an instance to start Minecraft or manage mods.
            </p>
            <button
              onclick={() => (activePage = "instances")}
              class="mc-primary mt-6 px-5 py-3 text-xs font-bold">Create instance</button
            >
          </div>
        {:else if activePage === "home"}<HomePage
            {instance}
            {versions}
            signedIn={account !== null}
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
{#if accountOpen}<AccountDialog
    bind:account
    {settings}
    onclose={() => (accountOpen = false)}
  />{/if}

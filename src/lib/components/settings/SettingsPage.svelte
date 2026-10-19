<script lang="ts">
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "$lib/components/ui/Toggle.svelte";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
  import MemoryRange from "$lib/components/ui/MemoryRange.svelte";
  import { onMount } from "svelte";
  import type { JavaRuntimeInfo, LauncherOverview, LauncherSettings } from "$lib/types";
  import {
    saveSettings,
    messageFromError,
    chooseInstallationDirectory,
    openLauncherFolder,
    openLogsFolder,
    chooseJavaExecutable,
    getSystemMemory,
    inspectJavaRuntime,
  } from "$lib/api/tauri";
  let {
    settings = $bindable(),
    overview,
    halloweenAvailable,
  }: {
    settings: LauncherSettings;
    overview: LauncherOverview;
    halloweenAvailable: boolean;
  } = $props();
  let saving = $state(false);
  let notice = $state("");
  let totalMemoryMb = $state(16384);
  let memoryMaxMb = $state(14336);
  let memoryDetection = $state("Detecting available memory…");
  let javaBusy = $state(false);
  let javaInfo = $state<JavaRuntimeInfo | null>(null);
  let javaNotice = $state("");

  onMount(() => {
    void getSystemMemory()
      .then((memory) => {
        totalMemoryMb = memory.totalMb;
        memoryMaxMb = memory.recommendedMaxMb;
        if (settings.allocatedMemoryMb > memoryMaxMb) {
          settings.allocatedMemoryMb = memoryMaxMb;
        }
        memoryDetection = `${formatMemory(totalMemoryMb)} installed · ${formatMemory(memoryMaxMb)} safely allocatable`;
      })
      .catch((error) => (memoryDetection = messageFromError(error)));
    void verifyJava(false);
  });

  function formatMemory(megabytes: number): string {
    const gigabytes = megabytes / 1024;
    return `${Number.isInteger(gigabytes) ? gigabytes : gigabytes.toFixed(1)} GB`;
  }

  async function verifyJava(showFailure = true) {
    javaBusy = true;
    javaNotice = "";
    try {
      javaInfo = await inspectJavaRuntime(settings.javaPath);
      javaNotice = `Java ${javaInfo.major} (${javaInfo.version}) · ${javaInfo.source}`;
    } catch (error) {
      javaInfo = null;
      if (showFailure || settings.javaPath) javaNotice = messageFromError(error);
    } finally {
      javaBusy = false;
    }
  }

  async function browseJava() {
    const selected = await chooseJavaExecutable(settings.javaPath);
    if (!selected) return;
    settings.javaPath = selected;
    await verifyJava();
  }

  async function useAutomaticJava() {
    settings.javaPath = null;
    await verifyJava(false);
    if (!javaInfo)
      javaNotice = "The required Java version will be installed automatically on launch.";
  }
  async function persist() {
    saving = true;
    notice = "";
    try {
      settings = await saveSettings(settings);
      notice = "Settings saved";
    } catch (e) {
      notice = messageFromError(e);
    } finally {
      saving = false;
    }
  }
  async function chooseInstallPath() {
    const selected = await chooseInstallationDirectory(settings.installationPath ?? undefined);
    if (selected) settings.installationPath = selected;
  }
  async function reopenSetup() {
    saving = true;
    notice = "";
    try {
      settings = await saveSettings({ ...settings, setupComplete: false });
    } catch (e) {
      notice = messageFromError(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="flex items-start justify-between">
  <PageHeader title="Settings" description="Configure Ninja Launcher for this device." />
  <div class="flex items-center gap-3">
    <span class="text-[11px] text-[#7297b8]">{notice}</span>
    <button
      onclick={persist}
      disabled={saving}
      class="mc-primary px-4 py-2.5 text-xs font-bold disabled:opacity-60"
      >{saving ? "Saving…" : "Save changes"}
    </button>
  </div>
</div>
<div class="space-y-5 pb-8">
  <section>
    <h2 class="mb-2 px-1 text-[10px] font-bold uppercase tracking-[.15em] text-cyan-300/55">
      Minecraft
    </h2>
    <div class="mc-panel overflow-hidden">
      <SettingRow title="Allocated memory" description={memoryDetection}>
        <MemoryRange bind:value={settings.allocatedMemoryMb} max={memoryMaxMb} />
      </SettingRow>
      <SettingRow
        title="Java runtime"
        description="Detected automatically and downloaded into Ninja Launcher when missing. Optionally enter a custom executable."
      >
        <div class="w-[min(34rem,52vw)]">
          <div class="flex gap-2">
            <input
              value={settings.javaPath ?? ""}
              oninput={(event) => (settings.javaPath = event.currentTarget.value || null)}
              placeholder="Automatic runtime"
              class="mc-field min-w-0 flex-1 px-3 py-2 text-xs text-[#d8e4f3] outline-none"
            />
            <button
              onclick={browseJava}
              disabled={javaBusy}
              class="mc-secondary px-3 text-xs font-bold">Browse</button
            >
            <button
              onclick={() => verifyJava()}
              disabled={javaBusy}
              class="mc-secondary px-3 text-xs font-bold"
              >{javaBusy ? "Checking…" : "Verify"}</button
            >
          </div>
          <div class="mt-2 flex items-center justify-between gap-3">
            <p
              class="truncate text-[10px] {javaInfo
                ? 'text-emerald-400'
                : 'text-[var(--color-text-muted)]'}"
            >
              {javaNotice || "Ninja Launcher selects the required Java version automatically."}
            </p>
            {#if settings.javaPath}<button
                onclick={useAutomaticJava}
                class="shrink-0 text-[10px] font-bold text-[var(--color-primary)] hover:underline"
                >Use automatic</button
              >{/if}
          </div>
        </div>
      </SettingRow>
      <SettingRow
        title="Close launcher on game start"
        description="Hide Ninja Launcher while Minecraft is running."
      >
        <Toggle bind:checked={settings.closeOnLaunch} label="Close launcher on game start" />
      </SettingRow>
      <SettingRow
        title="Show launcher when game exits"
        description="Return to the launcher after Minecraft closes."
      >
        <Toggle bind:checked={settings.reopenOnExit} label="Show launcher when game exits" />
      </SettingRow>
      <SettingRow
        title="Open game logs on launch"
        description="Automatically switch to the live console when Minecraft starts."
      >
        <Toggle bind:checked={settings.showLogsOnLaunch} label="Open game logs on launch" />
      </SettingRow>
      <SettingRow
        title="Show snapshots"
        description="Include the latest Minecraft preview versions in version menus."
      >
        <Toggle bind:checked={settings.showSnapshots} label="Show Minecraft snapshots" />
      </SettingRow>
    </div>
  </section>
  <section>
    <h2 class="mb-2 px-1 text-[10px] font-bold uppercase tracking-[.15em] text-cyan-300/55">
      Downloads & storage
    </h2>
    <div class="mc-panel overflow-hidden">
      <SettingRow
        title="Parallel downloads"
        description="Number of files Ninja Launcher may download at once."
      >
        <CustomSelect
          value={settings.downloadConcurrency}
          options={[
            { value: 2, label: "2 files" },
            { value: 4, label: "4 files" },
            { value: 6, label: "6 files" },
            { value: 8, label: "8 files" },
          ]}
          onchange={(value) => (settings.downloadConcurrency = Number(value))}
          ariaLabel="Parallel downloads"
          class="min-w-28 text-xs text-[#d8e4f3]"
        />
      </SettingRow>
      <SettingRow title="Launcher directory" description={overview.appDataDirectory}>
        <button
          onclick={openLauncherFolder}
          class="rounded-lg border border-white/[0.08] bg-white/[0.04] px-3 py-2 text-xs text-[#c4c7ce]"
          >Open folder</button
        >
      </SettingRow>
      <SettingRow
        title="Game data directory"
        description={settings.installationPath ?? overview.appDataDirectory}
      >
        <button
          onclick={chooseInstallPath}
          class="rounded-lg border border-white/[0.08] bg-white/[0.04] px-3 py-2 text-xs text-[#c4c7ce]"
          >Change folder</button
        >
      </SettingRow>
    </div>
  </section>
  <section>
    <h2 class="mb-2 px-1 text-[10px] font-bold uppercase tracking-[.15em] text-cyan-300/55">
      Appearance & accessibility
    </h2>
    <div class="mc-panel overflow-hidden">
      <SettingRow title="Theme" description="Use a dark, light, or system appearance.">
        <CustomSelect
          value={settings.theme}
          options={[
            { value: "dark", label: "Dark" },
            { value: "light", label: "Light" },
            { value: "system", label: "System" },
          ]}
          onchange={(value) => (settings.theme = value as LauncherSettings["theme"])}
          ariaLabel="Launcher theme"
          class="min-w-32 text-xs"
        />
      </SettingRow>
      <SettingRow title="Animations" description="Smooth page, button, and character motion.">
        <Toggle bind:checked={settings.animationsEnabled} label="Enable animations" />
      </SettingRow>
      <SettingRow title="Parallax background" description="Subtle ambient movement behind pages.">
        <Toggle bind:checked={settings.parallaxEnabled} label="Enable parallax background" />
      </SettingRow>
      {#if halloweenAvailable}
        <SettingRow
          title="Halloween theme"
          description="Switch the launcher to its seasonal orange, purple, and black appearance."
        >
          <Toggle bind:checked={settings.halloweenThemeEnabled} label="Enable Halloween theme" />
        </SettingRow>
        <SettingRow
          title="Halloween ambient sounds"
          description="Play an occasional spooky sound after the launcher has been idle."
        >
          <Toggle bind:checked={settings.halloweenSoundsEnabled} label="Enable Halloween sounds" />
        </SettingRow>
      {/if}
    </div>
  </section>
  <section>
    <h2 class="mb-2 px-1 text-[10px] font-bold uppercase tracking-[.15em] text-cyan-300/55">
      Application
    </h2>
    <div class="mc-panel overflow-hidden">
      <SettingRow
        title="Microsoft sign-in"
        description="Uses Ninja Launcher's built-in public OAuth application."
      >
        <span class=" px-3 py-2 text-xs font-bold text-emerald-200"> Built in </span>
      </SettingRow>
      <SettingRow
        title="Check for updates"
        description="Look for a new Ninja Launcher release when the app starts."
      >
        <Toggle bind:checked={settings.checkForUpdates} label="Check for updates" />
      </SettingRow>
      <SettingRow
        title="Discord Rich Presence"
        description="Show when you are using Ninja Launcher or playing Minecraft on your Discord profile."
      >
        <Toggle bind:checked={settings.discordRichPresence} label="Discord Rich Presence" />
      </SettingRow>
      <SettingRow
        title="Anonymous usage data"
        description="Share launcher starts, game starts, versions, and operating system. No account or device identifier is sent."
      >
        <Toggle
          bind:checked={settings.shareAnonymousUsageData}
          label="Share anonymous usage data"
        />
      </SettingRow>
      <SettingRow
        title="Launcher logs"
        description="Open the directory containing diagnostic logs."
      >
        <button
          onclick={openLogsFolder}
          class="rounded-lg border border-white/[0.08] bg-white/[0.04] px-3 py-2 text-xs text-[#c4c7ce]"
          >Open logs</button
        >
      </SettingRow>
      <SettingRow
        title="Setup assistant"
        description="Open the first-run setup again without deleting profiles or account data."
      >
        <button
          onclick={reopenSetup}
          disabled={saving}
          class="mc-secondary px-3 py-2 text-xs font-bold disabled:opacity-50"
          >Open setup again</button
        >
      </SettingRow>
      <SettingRow title="Version" description="Ninja Launcher desktop application">
        <span class="text-xs text-[#777c87]">v{overview.version}</span>
      </SettingRow>
    </div>
  </section>
</div>

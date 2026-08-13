<script lang="ts">
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import type { LauncherOverview, LauncherSettings } from "$lib/types";
  import {
    saveSettings,
    messageFromError,
    openLauncherFolder,
    openLogsFolder,
  } from "$lib/api/tauri";
  let {
    settings = $bindable(),
    overview,
  }: { settings: LauncherSettings; overview: LauncherOverview } = $props();
  let saving = $state(false);
  let notice = $state("");
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
      <SettingRow title="Allocated memory" description="Maximum RAM available to Minecraft.">
        <select
          bind:value={settings.allocatedMemoryMb}
          class="mc-field px-3 py-2 text-xs text-[#d8e4f3]"
        >
          <option value={2048}>2 GB</option>
          <option value={4096}>4 GB</option><option value={6144}>6 GB</option>
          <option value={8192}>8 GB</option><option value={12288}>12 GB</option>
        </select>
      </SettingRow>
      <SettingRow
        title="Java runtime"
        description="Detected automatically and downloaded into Ninja Launcher when missing. Optionally enter a custom executable."
      >
        <input
          bind:value={settings.javaPath}
          placeholder="Automatic installation"
          class="mc-field w-56 px-3 py-2 text-xs text-[#d8e4f3] outline-none"
        />
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
        <select
          bind:value={settings.downloadConcurrency}
          class="mc-field px-3 py-2 text-xs text-[#d8e4f3]"
        >
          <option value={2}>2 files</option>
          <option value={4}>4 files</option><option value={6}>6 files</option>
          <option value={8}>8 files</option>
        </select>
      </SettingRow>
      <SettingRow title="Launcher directory" description={overview.appDataDirectory}>
        <button
          onclick={openLauncherFolder}
          class="rounded-lg border border-white/[0.08] bg-white/[0.04] px-3 py-2 text-xs text-[#c4c7ce]"
          >Open folder</button
        >
      </SettingRow>
    </div>
  </section>
  <section>
    <h2 class="mb-2 px-1 text-[10px] font-bold uppercase tracking-[.15em] text-cyan-300/55">
      Application
    </h2>
    <div class="mc-panel overflow-hidden">
      <SettingRow
        title="Microsoft OAuth client ID"
        description="Public client ID from your Microsoft Entra app registration; required for Minecraft sign-in."
      >
        <input
          bind:value={settings.microsoftClientId}
          placeholder="00000000-0000-…"
          class="mc-field w-56 px-3 py-2 text-xs text-[#d8e4f3] outline-none"
        />
      </SettingRow>
      <SettingRow
        title="Check for updates"
        description="Look for a new Ninja Launcher release when the app starts."
      >
        <Toggle bind:checked={settings.checkForUpdates} label="Check for updates" />
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
      <SettingRow title="Version" description="Ninja Launcher desktop application">
        <span class="text-xs text-[#777c87]">v{overview.version}</span>
      </SettingRow>
    </div>
  </section>
</div>

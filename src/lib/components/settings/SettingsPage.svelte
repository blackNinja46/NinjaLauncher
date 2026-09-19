<script lang="ts">
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "$lib/components/ui/Toggle.svelte";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
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
        <CustomSelect
          value={settings.allocatedMemoryMb}
          options={[
            { value: 2048, label: "2 GB" },
            { value: 4096, label: "4 GB" },
            { value: 6144, label: "6 GB" },
            { value: 8192, label: "8 GB" },
            { value: 12288, label: "12 GB" },
          ]}
          onchange={(value) => (settings.allocatedMemoryMb = Number(value))}
          ariaLabel="Allocated memory"
          class="min-w-28 text-xs text-[#d8e4f3]"
        />
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
      <SettingRow title="Version" description="Ninja Launcher desktop application">
        <span class="text-xs text-[#777c87]">v{overview.version}</span>
      </SettingRow>
    </div>
  </section>
</div>

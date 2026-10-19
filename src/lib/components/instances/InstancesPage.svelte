<script lang="ts">
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
  import { onMount, tick } from "svelte";
  import {
    createInstance,
    createEditableModpackCopy,
    deleteInstance,
    duplicateInstance,
    exportModpackFile,
    inspectModpackFile,
    listFabricLoaderVersions,
    listExportableFolders,
    messageFromError,
    openInstanceFolder,
    saveInstance,
  } from "$lib/api/tauri";
  import type {
    FabricLoaderVersionOption,
    LauncherInstance,
    ModpackFileInspection,
    ModpackInstallation,
    SupportedVersion,
  } from "$lib/types";
  let {
    instances,
    versions,
    vanillaVersions,
    selectedId,
    installation,
    runningIds,
    onselect,
    onchanged,
    onimportfile,
  }: {
    instances: LauncherInstance[];
    versions: SupportedVersion[];
    vanillaVersions: SupportedVersion[];
    selectedId: string;
    installation: ModpackInstallation | null;
    runningIds: string[];
    onselect: (id: string) => void;
    onchanged: () => Promise<void>;
    onimportfile: (
      inspection: ModpackFileInspection,
      allowForeignMods: boolean,
    ) => Promise<LauncherInstance>;
  } = $props();
  let creating = $state(false);
  let createError = $state("");
  let name = $state("New Fabric Profile");
  let loader = $state<LauncherInstance["loader"]>("fabric");
  let version = $state("");
  let error = $state("");
  let pendingDelete = $state<LauncherInstance | null>(null);
  let deleting = $state(false);
  let exportingId = $state("");
  let exportNotice = $state("");
  let pendingExport = $state<LauncherInstance | null>(null);
  let exportPackName = $state("");
  let exportVersion = $state("1.0.0");
  let exportFolders = $state<string[]>([]);
  let selectedExportFolders = $state<string[]>([]);
  let loadingExportFolders = $state(false);
  let exportError = $state("");
  let loaderVersion = $state("0.19.3");
  let syncedCreateVersion = $state("");
  let editing = $state<LauncherInstance | null>(null);
  let editName = $state("");
  let editMinecraftVersion = $state("");
  let editLoader = $state<LauncherInstance["loader"]>("fabric");
  let editLoaderVersion = $state("");
  let editError = $state("");
  let operation = $state<{ title: string; message: string } | null>(null);
  let fabricVersions = $state<FabricLoaderVersionOption[]>([]);
  let loadingFabricVersions = $state(true);
  let modpackDropActive = $state(false);
  let droppedModpackInspection = $state<ModpackFileInspection | null>(null);
  let importingDroppedModpack = $derived(installation?.phase === "installing");
  $effect(() => {
    const compatible = versionOptions(loader);
    if (!compatible.some((item) => item.value === version)) {
      version = compatible[0]?.value ?? "";
    }
    if (loader === "fabric" && version && syncedCreateVersion !== version) {
      syncedCreateVersion = version;
      loaderVersion =
        versions.find((item) => item.minecraftVersion === version)?.fabricLoaderVersion ??
        loaderVersion;
    }
  });

  function versionOptions(selectedLoader: LauncherInstance["loader"]) {
    const available = selectedLoader === "vanilla" ? vanillaVersions : versions;
    return available.map((item) => ({
      value: item.minecraftVersion,
      label: `Minecraft ${item.minecraftVersion}${item.versionType === "snapshot" ? " · Snapshot" : ""}`,
    }));
  }

  function changeCreateLoader(value: LauncherInstance["loader"]) {
    loader = value;
    const compatible = versionOptions(value);
    if (!compatible.some((item) => item.value === version)) version = compatible[0]?.value ?? "";
    if (name === "New Fabric Profile" || name === "New Vanilla Profile") {
      name = value === "vanilla" ? "New Vanilla Profile" : "New Fabric Profile";
    }
  }

  onMount(() => {
    void listFabricLoaderVersions()
      .then((loaded) => (fabricVersions = loaded))
      .catch((cause) => (error = messageFromError(cause)))
      .finally(() => (loadingFabricVersions = false));
    let disposed = false;
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          modpackDropActive = true;
        } else if (event.payload.type === "leave") {
          modpackDropActive = false;
        } else if (!importingDroppedModpack) {
          void inspectDroppedModpack(event.payload.paths);
        }
      })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  async function inspectDroppedModpack(paths: string[]) {
    modpackDropActive = false;
    const packs = paths.filter((path) => path.toLocaleLowerCase().endsWith(".mrpack"));
    if (packs.length !== 1) {
      error = "Drop exactly one .mrpack file on the Profiles page.";
      return;
    }
    error = "";
    try {
      const inspected = await inspectModpackFile(packs[0]);
      if (inspected.foreignMods.length) droppedModpackInspection = inspected;
      else await importDroppedModpack(inspected, false);
    } catch (cause) {
      error = messageFromError(cause);
    }
  }

  async function importDroppedModpack(inspected: ModpackFileInspection, allowForeignMods: boolean) {
    try {
      const created = await onimportfile(inspected, allowForeignMods);
      droppedModpackInspection = null;
      onselect(created.id);
    } catch (cause) {
      error = messageFromError(cause);
    }
  }

  function fabricVersionOptions(current: string) {
    const available = [...fabricVersions];
    for (const supported of versions) {
      if (!available.some((item) => item.version === supported.fabricLoaderVersion)) {
        available.push({ version: supported.fabricLoaderVersion, stable: true });
      }
    }
    if (current && !available.some((item) => item.version === current)) {
      available.unshift({ version: current, stable: false });
    }
    return available.map((item) => ({
      value: item.version,
      label: `Fabric ${item.version}${item.stable ? " · Stable" : ""}`,
    }));
  }

  async function showOperation(title: string, message: string) {
    operation = { title, message };
    await tick();
  }

  async function create() {
    if (!name.trim()) {
      createError = "Profile name cannot be empty.";
      return;
    }
    createError = "";
    await showOperation("Creating profile", `Preparing ${name.trim()}…`);
    try {
      const value = await createInstance(
        name.trim(),
        version,
        loader,
        loader === "fabric" ? loaderVersion.trim() : "vanilla",
      );
      creating = false;
      await onchanged();
      onselect(value.id);
    } catch (e) {
      createError = messageFromError(e);
    } finally {
      operation = null;
    }
  }
  async function duplicate(instance: LauncherInstance) {
    await showOperation("Duplicating profile", `Copying ${instance.name} and all profile files…`);
    try {
      await duplicateInstance(instance.id);
      await onchanged();
    } catch (e) {
      error = messageFromError(e);
    } finally {
      operation = null;
    }
  }
  async function createEditableCopy(instance: LauncherInstance) {
    await showOperation(
      "Creating editable copy",
      `Copying ${instance.name} and converting its Modrinth mods…`,
    );
    try {
      const created = await createEditableModpackCopy(instance.id);
      editing = null;
      await onchanged();
      onselect(created.id);
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      operation = null;
    }
  }
  async function remove() {
    if (!pendingDelete) return;
    const instance = pendingDelete;
    deleting = true;
    error = "";
    await showOperation("Deleting profile", `Removing ${instance.name} and its files…`);
    try {
      await deleteInstance(instance.id);
      pendingDelete = null;
      await onchanged();
    } catch (e) {
      error = messageFromError(e);
    } finally {
      deleting = false;
      operation = null;
    }
  }

  function openEdit(instance: LauncherInstance) {
    editing = instance;
    editName = instance.name;
    editLoader = instance.loader;
    editMinecraftVersion = instance.minecraftVersion;
    editLoaderVersion = instance.loaderVersion;
    editError = "";
  }

  function changeEditMinecraftVersion(value: string) {
    editMinecraftVersion = value;
    if (editLoader === "fabric") {
      editLoaderVersion =
        versions.find((item) => item.minecraftVersion === value)?.fabricLoaderVersion ??
        editLoaderVersion;
    }
  }

  function changeEditLoader(value: LauncherInstance["loader"]) {
    editLoader = value;
    const compatible = versionOptions(value);
    if (!compatible.some((item) => item.value === editMinecraftVersion)) {
      changeEditMinecraftVersion(compatible[0]?.value ?? "");
    }
  }

  async function saveEdit() {
    if (!editing) return;
    if (!editName.trim()) {
      editError = "Profile name cannot be empty.";
      return;
    }
    if (editLoader === "fabric" && !editLoaderVersion.trim()) {
      editError = "Fabric version cannot be empty.";
      return;
    }
    const updated: LauncherInstance = {
      ...editing,
      name: editName.trim(),
      loader: editLoader,
      minecraftVersion: editMinecraftVersion,
      loaderVersion: editLoader === "fabric" ? editLoaderVersion.trim() : "vanilla",
    };
    await showOperation("Saving profile", `Updating ${updated.name}…`);
    try {
      await saveInstance(updated);
      editing = null;
      await onchanged();
    } catch (cause) {
      editError = messageFromError(cause);
    } finally {
      operation = null;
    }
  }

  async function openExport(instance: LauncherInstance) {
    pendingExport = instance;
    exportPackName = instance.name;
    exportVersion = "1.0.0";
    exportFolders = [];
    selectedExportFolders = [];
    exportError = "";
    loadingExportFolders = true;
    try {
      exportFolders = await listExportableFolders(instance.id);
      selectedExportFolders = [...exportFolders];
    } catch (cause) {
      exportError = messageFromError(cause);
    } finally {
      loadingExportFolders = false;
    }
  }

  function toggleExportFolder(folder: string) {
    selectedExportFolders = selectedExportFolders.includes(folder)
      ? selectedExportFolders.filter((item) => item !== folder)
      : [...selectedExportFolders, folder];
  }

  function folderLabel(folder: string): string {
    const labels: Record<string, string> = {
      mods: "Mods",
      config: "Configurations",
      resourcepacks: "Resource packs",
      shaderpacks: "Shader packs",
      screenshots: "Screenshots",
      defaultconfigs: "Default configs",
    };
    return labels[folder] ?? folder;
  }

  async function exportPack() {
    if (!pendingExport) return;
    if (!exportPackName.trim()) {
      exportError = "Enter a name for the modpack.";
      return;
    }
    if (!/^[A-Za-z0-9._+-]{1,64}$/.test(exportVersion)) {
      exportError = "Enter a valid version such as 1.0.0, 2.1-beta, or 2026.08.";
      return;
    }
    const instance = pendingExport;
    exportingId = instance.id;
    error = "";
    exportError = "";
    exportNotice = "";
    try {
      const report = await exportModpackFile(
        instance.id,
        exportPackName.trim(),
        exportVersion.trim(),
        selectedExportFolders,
      );
      if (!report) return;
      pendingExport = null;
      exportNotice = report.foreignMods.length
        ? `Exported ${report.fileCount} files. ${report.foreignMods.length} mod(s) not found on Modrinth were bundled as overrides.`
        : `Exported ${report.fileCount} files as a Modrinth modpack.`;
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      exportingId = "";
    }
  }
</script>

{#if modpackDropActive || importingDroppedModpack}
  <div
    class="pointer-events-none fixed bottom-4 left-[260px] right-4 top-4 z-[180] grid place-items-center rounded-[16px] border-2 border-dashed border-[var(--color-primary)]/70 bg-[#070a10]/85 backdrop-blur-xl"
    aria-live="polite"
  >
    <div class="text-center">
      <span class="mx-auto grid size-14 place-items-center text-[var(--color-primary)]">
        <Icon name="upload" size={30} />
      </span>
      <p class="mt-3 text-lg font-black text-white">
        {importingDroppedModpack ? "Importing modpack…" : "Drop a .mrpack here"}
      </p>
      <p class="mt-1 text-xs text-[var(--color-text-muted)]">
        Ninja Launcher will create a new isolated profile.
      </p>
    </div>
  </div>
{/if}

{#if droppedModpackInspection}
  <div
    class="fixed inset-y-0 left-[248px] right-0 z-[185] grid place-items-center bg-black/70 p-6 backdrop-blur-xl"
    role="presentation"
  >
    <div
      class="w-full max-w-lg overflow-hidden rounded-[14px] border border-amber-300/20 bg-[var(--color-section-background)] p-6 shadow-[0_30px_100px_rgba(0,0,0,.72)]"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="dropped-modpack-warning-title"
    >
      <span class="grid size-11 place-items-center text-amber-300">
        <Icon name="warning" size={24} />
      </span>
      <h2 id="dropped-modpack-warning-title" class="mt-3 text-lg font-black text-white">
        Unverified mods in {droppedModpackInspection.name}
      </h2>
      <p class="mt-2 text-xs leading-5 text-[var(--color-text-muted)]">
        {droppedModpackInspection.foreignMods.length} mod file(s) could not be verified through Modrinth.
        Continue only if you trust the source of this pack.
      </p>
      <div
        class="mt-4 max-h-36 overflow-y-auto rounded-[8px] border border-white/[.07] bg-black/20 p-3"
      >
        {#each droppedModpackInspection.foreignMods as mod}
          <p class="truncate py-1 font-mono text-[10px] text-amber-100/75">{mod}</p>
        {/each}
      </div>
      <div class="mt-6 flex justify-end gap-3">
        <button
          type="button"
          onclick={() => (droppedModpackInspection = null)}
          disabled={importingDroppedModpack}
          class="mc-secondary px-4 py-2.5 text-xs font-bold disabled:opacity-50">Cancel</button
        >
        <button
          type="button"
          onclick={() =>
            droppedModpackInspection && importDroppedModpack(droppedModpackInspection, true)}
          disabled={importingDroppedModpack}
          class="mc-primary px-4 py-2.5 text-xs font-bold disabled:opacity-50"
          >{importingDroppedModpack ? "Importing…" : "Trust and import"}</button
        >
      </div>
    </div>
  </div>
{/if}

<div class="flex items-start justify-between">
  <PageHeader
    title="Profiles"
    description="Each profile keeps its mods, worlds, and settings isolated."
  /><button
    onclick={() => {
      createError = "";
      loader = "fabric";
      name = "New Fabric Profile";
      version = versions[0]?.minecraftVersion ?? "";
      creating = true;
    }}
    class="mc-primary flex items-center gap-2 px-4 py-2.5 text-xs font-bold"
    ><Icon name="plus" size={15} />New profile</button
  >
</div>
{#if error}<div class="error-banner mb-4 px-4 py-3 text-xs">
    {error}
  </div>{/if}
{#if exportNotice}<div
    class="mb-4 border border-cyan-300/20 bg-cyan-300/[0.07] rounded-lg px-4 py-3 text-xs text-cyan-100"
  >
    {exportNotice}
  </div>{/if}
<div class="space-y-3">
  {#if installation}<article
      class="mc-panel relative overflow-hidden border-cyan-300/35 p-5"
      aria-live="polite"
    >
      <div class="flex items-center gap-3">
        <div
          class="grid size-14 shrink-0 place-items-center overflow-hidden rounded-lg bg-[#031237] ring-1 ring-cyan-300/30"
        >
          {#if installation.iconUrl}<img
              src={installation.iconUrl}
              alt={`${installation.name} modpack`}
              class="size-full object-cover"
            />{:else}<img
              src="/loaders/fabric.png"
              alt="Fabric mod loader"
              class="size-13 object-contain"
            />{/if}
        </div>
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <h2 class="block-title truncate text-[15px] uppercase">{installation.name}</h2>
            <span
              class="inline-flex rounded-sm items-center gap-1 bg-purple-400/10 px-2 py-0.5 text-[11px] uppercase font-bold text-purple-300"
              >MODPACK</span
            >
          </div>
          <p class="mt-1 text-xs text-[#91a9c2]">
            {installation.phase === "installing"
              ? installation.source === "file"
                ? "Importing modpack file and creating profile…"
                : "Installing Modrinth modpack and creating profile…"
              : installation.phase === "complete"
                ? "Import complete. The profile is ready."
                : (installation.error ?? "The modpack could not be installed.")}
          </p>
        </div>
        {#if installation.phase === "installing"}<div
            class="size-6 animate-spin rounded-full border-2 border-white/10 border-t-cyan-300"
            aria-label="Installation in progress"
          ></div>{:else if installation.phase === "complete"}<span
            class="text-[var(--color-success)]"
            aria-label="Import complete"><Icon name="check" size={20} /></span
          >{:else}<span class="text-lg font-bold text-red-300" aria-label="Installation failed"
            >!</span
          >{/if}
      </div>
    </article>{/if}
  {#each instances as instance}<article
      class="mc-panel relative overflow-hidden p-5 transition duration-200 {selectedId ===
      instance.id
        ? 'border-cyan-300/70 bg-white/[.055] shadow-[inset_4px_0_0_#3b9df5,0_12px_28px_rgba(0,0,0,.18)]'
        : 'hover:border-cyan-300/30 hover:bg-blue-500/[.04]'}"
    >
      {#if selectedId === instance.id}<div
          class="pointer-events-none absolute inset-y-0 left-0 w-1 bg-cyan-300 shadow-[0_0_12px_rgba(59,157,245,.34)]"
        ></div>{/if}
      <div class="flex items-center gap-2">
        <div
          class="grid size-14 place-items-center overflow-hidden rounded-lg border border-cyan-300/15 bg-[#07335d] transition {selectedId ===
          instance.id
            ? 'ring-1 ring-cyan-300/55 border-[var(--color-border-brand)] shadow-[inset_0_0_28px_var(--color-brand-soft)]'
            : ''}"
        >
          {#if instance.modpack?.iconUrl}<img
              src={instance.modpack.iconUrl}
              alt={`${instance.modpack.name} modpack`}
              class="size-full object-cover"
            />{:else if instance.loader === "vanilla"}<span class="text-[var(--color-primary)]"
              ><Icon name="vanilla" size={25} /></span
            >{:else}<img
              src={`/loaders/${instance.loader}.png`}
              alt={`${instance.loader} mod loader`}
              class="size-13 object-contain"
            />{/if}
        </div>
        <button
          onclick={() => onselect(instance.id)}
          aria-pressed={selectedId === instance.id}
          class="flex-1 text-left"
          ><div class="inline justify-center gap-2">
            <h2 class="block-title text-[15px] uppercase">{instance.name}</h2>
            {#if selectedId === instance.id}<span
                class="inline-flex items-center rounded-sm gap-1 bg-cyan-300/10 px-2 py-0.5 text-[11px] font-bold uppercase text-cyan-600"
                >Active profile</span
              >{/if}
            {#if runningIds.includes(instance.id)}<span
                class="inline-flex items-center rounded-sm gap-1 bg-emerald-300/10 px-2 py-0.5 text-[11px] font-bold uppercase tracking-wider text-emerald-400"
                ><span class="size-1.5 bg-emerald-300 text-center"></span>Running</span
              >{/if}
            {#if instance.modpack}<span
                class="inline-flex rounded-sm items-center gap-1 bg-purple-400/10 px-2 py-0.5 text-[11px] uppercase font-bold text-purple-300"
                >MODPACK</span
              >{/if}
          </div>
          <p class="mt-1 text-xs text-[#7899b8]">
            Minecraft {instance.minecraftVersion} · {instance.loader === "vanilla"
              ? "Vanilla"
              : `Fabric ${instance.loaderVersion} · ${
                  instance.modpack
                    ? `${instance.modpack.fileCount} pack files`
                    : `${instance.mods.length} mods`
                }`}
          </p></button
        >
        <div
          class="flex shrink-0 items-center gap-0.5 rounded-lg border border-white/[0.07] bg-black/15 p-1"
          aria-label={`Actions for ${instance.name}`}
        >
          <button
            onclick={(e) => {
              e.stopPropagation();
              openInstanceFolder(instance.id);
            }}
            title="Open folder"
            aria-label="Open profile folder"
            class="grid size-8 place-items-center rounded-md text-[#777c87] transition hover:bg-white/[0.07] hover:text-white"
            ><Icon name="folder" size={14} /></button
          ><button
            onclick={(e) => {
              e.stopPropagation();
              openEdit(instance);
            }}
            title="Edit profile"
            aria-label="Edit profile"
            class="grid size-8 place-items-center rounded-md text-[#9297a1] transition hover:bg-white/[0.07] hover:text-white"
            ><Icon name="edit" size={13} /></button
          >{#if instance.modpack}<button
              onclick={(e) => {
                e.stopPropagation();
                createEditableCopy(instance);
              }}
              title="Create an editable copy"
              aria-label="Customize modpack in an editable copy"
              class="grid size-8 place-items-center rounded-md text-cyan-300 transition hover:bg-cyan-300/[0.09] hover:text-cyan-100"
              ><Icon name="palette" size={14} /></button
            >
          {/if}<span class="mx-0.5 h-4 w-px bg-white/[0.08]"></span><button
            onclick={(e) => {
              e.stopPropagation();
              openExport(instance);
            }}
            disabled={Boolean(exportingId)}
            title={exportingId === instance.id ? "Exporting…" : "Export as Modrinth modpack"}
            aria-label="Export as Modrinth modpack"
            class="grid size-8 place-items-center rounded-md text-[#9297a1] transition hover:bg-white/[0.07] hover:text-white disabled:opacity-40"
            ><Icon name="download" size={13} /></button
          ><button
            onclick={(e) => {
              e.stopPropagation();
              duplicate(instance);
            }}
            title="Duplicate profile"
            aria-label="Duplicate profile"
            class="grid size-8 place-items-center rounded-md text-[#9297a1] transition hover:bg-white/[0.07] hover:text-white"
            ><Icon name="copy" size={13} /></button
          ><span class="mx-0.5 h-4 w-px bg-white/[0.08]"></span><button
            onclick={(e) => {
              e.stopPropagation();
              pendingDelete = instance;
            }}
            disabled={runningIds.includes(instance.id)}
            title={runningIds.includes(instance.id)
              ? "Stop the profile before deleting it"
              : "Delete profile"}
            aria-label="Delete profile"
            class="grid size-8 place-items-center rounded-md text-[var(--color-text-faint)] transition hover:bg-[var(--color-danger)]/10 hover:text-[var(--color-danger-text)] disabled:cursor-not-allowed disabled:opacity-35"
            ><Icon name="trash" size={13} /></button
          >
        </div>
      </div>
    </article>{/each}
</div>
{#if editing}<div
    class="fixed inset-0 z-50 grid place-items-center bg-[#010612]/85 p-6"
    role="presentation"
    onclick={(event) => event.currentTarget === event.target && !operation && (editing = null)}
  >
    <div
      class="mc-panel w-full max-w-md border-cyan-300/25 p-6"
      role="dialog"
      aria-modal="true"
      aria-labelledby="edit-profile-title"
    >
      <p class="text-[9px] font-bold uppercase tracking-[.15em] text-cyan-300/60">
        Profile settings
      </p>
      <h2 id="edit-profile-title" class="block-title mt-1 text-lg uppercase">Edit profile</h2>
      {#if editError}<div class="error-banner mt-4 px-3 py-2.5 text-xs">
          {editError}
        </div>{/if}
      <div class="mt-5 space-y-4">
        <label class="block">
          <span class="mb-1.5 block text-[10px] font-bold uppercase tracking-wider text-[#6f91ad]"
            >Profile name</span
          >
          <input
            bind:value={editName}
            maxlength="128"
            class="mc-field w-full px-3 py-2.5 text-sm outline-none"
          />
        </label>
        <div>
          <span class="mb-1.5 block text-[10px] font-bold uppercase tracking-wider text-[#6f91ad]"
            >Game type</span
          >
          <div class="grid grid-cols-2 gap-2">
            {#each [{ value: "fabric", label: "Fabric", icon: "fabric" }, { value: "vanilla", label: "Vanilla", icon: "vanilla" }] as option}
              <button
                type="button"
                onclick={() => changeEditLoader(option.value as LauncherInstance["loader"])}
                disabled={Boolean(editing.modpack)}
                class="flex items-center gap-3 border px-3 py-2.5 text-left transition {editLoader ===
                option.value
                  ? 'border-[var(--color-border-brand)] bg-[var(--color-brand-soft)] text-white'
                  : 'border-white/[0.08] text-[var(--color-text-muted)] hover:border-white/[0.18]'}"
              >
                {#if option.icon === "fabric"}<img
                    src="/loaders/fabric.png"
                    alt=""
                    class="size-7 object-contain"
                  />{:else}<span class="grid size-7 place-items-center"
                    ><Icon name="vanilla" size={20} /></span
                  >{/if}
                <span class="text-xs font-bold">{option.label}</span>
              </button>
            {/each}
          </div>
        </div>
        <label class="block">
          <span class="mb-1.5 block text-[10px] font-bold uppercase tracking-wider text-[#6f91ad]"
            >Minecraft version</span
          >
          <CustomSelect
            value={editMinecraftVersion}
            options={versionOptions(editLoader)}
            onchange={(value) => changeEditMinecraftVersion(String(value))}
            disabled={Boolean(editing.modpack)}
            ariaLabel="Minecraft version"
            class="w-full text-sm"
          />
        </label>
        {#if editLoader === "fabric"}<label class="block">
            <span class="mb-1.5 block text-[10px] font-bold uppercase tracking-wider text-[#6f91ad]"
              >Fabric loader version</span
            >
            <CustomSelect
              value={editLoaderVersion}
              options={fabricVersionOptions(editLoaderVersion)}
              onchange={(value) => (editLoaderVersion = String(value))}
              disabled={Boolean(editing.modpack) || loadingFabricVersions}
              ariaLabel="Fabric loader version"
              class="w-full font-mono text-sm"
            />
          </label>{/if}
        {#if editing.modpack}<p
            class="border border-cyan-300/15 bg-cyan-300/[0.06] rounded-lg px-3 py-2.5 text-[12px] leading-5 text-cyan-200/70"
          >
            Minecraft and Fabric versions are controlled by {editing.modpack.name}. The profile name
            can still be changed.
          </p>{/if}
      </div>
      <div class="mt-6 flex justify-end gap-3">
        {#if editing.modpack}<button
            onclick={() => createEditableCopy(editing!)}
            class="mc-secondary mr-auto flex items-center gap-2 px-4 py-2.5 text-xs font-bold"
            ><Icon name="copy" size={13} />Create editable copy</button
          >{/if}
        <button onclick={() => (editing = null)} class="mc-secondary px-4 py-2.5 text-xs font-bold"
          >Cancel</button
        >
        <button onclick={saveEdit} class="mc-primary px-4 py-2.5 text-xs font-bold"
          >Save changes</button
        >
      </div>
    </div>
  </div>{/if}
{#if pendingExport}<div
    class="fixed inset-0 z-50 grid place-items-center bg-[#010612]/85 p-6"
    role="presentation"
    onclick={(event) =>
      event.currentTarget === event.target && !exportingId && (pendingExport = null)}
  >
    <div
      class="mc-panel w-full max-w-lg border-cyan-300/25 p-6"
      role="dialog"
      aria-modal="true"
      aria-labelledby="export-modpack-title"
    >
      <p class="text-[9px] font-bold uppercase tracking-[.15em] text-cyan-300/60">
        Modrinth .mrpack
      </p>
      <h2 id="export-modpack-title" class="block-title mt-1 text-lg uppercase">
        Export {pendingExport.name}
      </h2>

      {#if exportError}<div class="error-banner mt-4 px-3 py-2.5 text-xs">
          {exportError}
        </div>{/if}

      <div class="mt-5 grid grid-cols-2 gap-3">
        <label class="block">
          <span class="mb-1.5 block text-[10px] font-bold uppercase tracking-wider text-[#6f91ad]"
            >Pack name</span
          >
          <input
            bind:value={exportPackName}
            maxlength="128"
            placeholder="My Modpack"
            class="mc-field w-full px-3 py-2.5 text-sm outline-none"
          />
        </label>
        <label class="block">
          <span class="mb-1.5 block text-[10px] font-bold uppercase tracking-wider text-[#6f91ad]"
            >Version</span
          >
          <input
            bind:value={exportVersion}
            maxlength="64"
            placeholder="1.0.0"
            class="mc-field w-full px-3 py-2.5 font-mono text-sm outline-none"
          />
        </label>
      </div>

      <div class="mt-5 flex items-center justify-between">
        <span class="text-[10px] font-bold uppercase tracking-wider text-[#6f91ad]"
          >Included folders</span
        >
        {#if exportFolders.length}<div class="flex gap-3 text-[10px]">
            <button
              onclick={() => (selectedExportFolders = [...exportFolders])}
              class="text-cyan-200 hover:underline">Select all</button
            >
            <button
              onclick={() => (selectedExportFolders = [])}
              class="text-[#7794ac] hover:text-white">Select none</button
            >
          </div>{/if}
      </div>
      <div class="mt-2 max-h-52 overflow-y-auto border border-cyan-300/12 bg-black/15 p-2">
        {#if loadingExportFolders}
          <p class="px-2 py-4 text-center text-xs text-[#6b89a2]">Loading folders…</p>
        {:else if exportFolders.length}
          <div class="grid grid-cols-2 gap-1.5">
            {#each exportFolders as folder}
              <label
                class="flex cursor-pointer items-center gap-3 border px-3 py-2.5 transition {selectedExportFolders.includes(
                  folder,
                )
                  ? 'border-cyan-300/25 bg-cyan-300/[0.08] text-cyan-100'
                  : 'border-white/[0.06] text-[#7895ad] hover:border-white/15'}"
              >
                <input
                  type="checkbox"
                  checked={selectedExportFolders.includes(folder)}
                  onchange={() => toggleExportFolder(folder)}
                  class="accent-cyan-400"
                />
                <span class="min-w-0 truncate text-xs">{folderLabel(folder)}</span>
                <span class="ml-auto truncate font-mono text-[9px] opacity-50">/{folder}</span>
              </label>
            {/each}
          </div>
        {:else}
          <p class="px-2 py-4 text-center text-xs text-[#6b89a2]">
            No exportable folders found. Root configuration files can still be exported.
          </p>
        {/if}
      </div>
      <p class="mt-2 text-[10px] leading-5 text-[#55748f]">
        Saves, logs, crash reports, and launcher metadata are always excluded. Files directly in the
        profile folder remain included.
      </p>

      <div class="mt-6 flex justify-end gap-3">
        <button
          onclick={() => (pendingExport = null)}
          disabled={Boolean(exportingId)}
          class="mc-secondary px-4 py-2.5 text-xs font-bold disabled:opacity-50">Cancel</button
        >
        <button
          onclick={exportPack}
          disabled={Boolean(exportingId) || loadingExportFolders}
          class="mc-primary flex items-center gap-2 px-4 py-2.5 text-xs font-bold disabled:opacity-50"
        >
          <Icon name="download" size={14} />{exportingId ? "Exporting…" : "Choose destination"}
        </button>
      </div>
    </div>
  </div>{/if}
{#if creating}<div class="fixed inset-0 z-40 grid place-items-center bg-[#010612]/80">
    <div class="mc-panel w-full max-w-md p-6">
      <h2 class="block-title text-lg uppercase">Create profile</h2>
      {#if createError}<div class="error-banner mt-4 px-3 py-2.5 text-xs">
          {createError}
        </div>{/if}
      <div class="mt-5 space-y-3">
        <input
          bind:value={name}
          placeholder="Profile name"
          class="mc-field w-full px-3 py-2.5 text-sm outline-none"
        />
        <div>
          <span class="mb-1.5 block text-[10px] font-bold uppercase tracking-wider text-[#6f91ad]"
            >Game type</span
          >
          <div class="grid grid-cols-2 gap-2">
            <button
              type="button"
              onclick={() => changeCreateLoader("fabric")}
              class="flex items-center gap-3 border px-3 py-2.5 text-left transition {loader ===
              'fabric'
                ? 'border-[var(--color-border-brand)] bg-[var(--color-brand-soft)] text-white'
                : 'border-white/[0.08] text-[var(--color-text-muted)]'}"
            >
              <img src="/loaders/fabric.png" alt="" class="size-7 object-contain" /><span
                class="text-xs font-bold">Fabric</span
              >
            </button>
            <button
              type="button"
              onclick={() => changeCreateLoader("vanilla")}
              class="flex items-center gap-3 border px-3 py-2.5 text-left transition {loader ===
              'vanilla'
                ? 'border-[var(--color-border-brand)] bg-[var(--color-brand-soft)] text-white'
                : 'border-white/[0.08] text-[var(--color-text-muted)]'}"
            >
              <span class="grid size-7 place-items-center"><Icon name="vanilla" size={20} /></span
              ><span class="text-xs font-bold">Vanilla</span>
            </button>
          </div>
        </div>
        <CustomSelect
          value={version}
          options={versionOptions(loader)}
          onchange={(value) => (version = String(value))}
          ariaLabel="Minecraft version"
          class="w-full text-sm"
        />
        {#if loader === "fabric"}<label class="block">
            <span class="mb-1.5 block text-[10px] font-bold uppercase tracking-wider text-[#6f91ad]"
              >Fabric loader version</span
            >
            <CustomSelect
              value={loaderVersion}
              options={fabricVersionOptions(loaderVersion)}
              onchange={(value) => (loaderVersion = String(value))}
              disabled={loadingFabricVersions}
              ariaLabel="Fabric loader version"
              class="w-full font-mono text-sm"
            />
          </label>{/if}
      </div>
      <div class="mt-5 flex justify-end gap-3">
        <button onclick={() => (creating = false)} class="mc-secondary px-4 py-2 text-xs"
          >Cancel</button
        ><button onclick={create} class="mc-primary px-4 py-2 text-xs font-bold">Create</button>
      </div>
    </div>
  </div>{/if}
{#if pendingDelete}<div
    class="fixed inset-0 z-50 grid place-items-center bg-black/70 p-6 backdrop-blur-xl"
    role="presentation"
    onclick={(event) => event.currentTarget === event.target && !deleting && (pendingDelete = null)}
  >
    <div
      class="mc-panel w-full max-w-md overflow-hidden rounded-[14px] border border-white/[.1] p-0 shadow-[0_30px_100px_rgba(0,0,0,.72)]"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="delete-profile-title"
    >
      <div class="flex items-center gap-4 border-b border-white/[.07] px-6 py-5">
        <div
          class="grid size-11 shrink-0 place-items-center rounded-[9px] border border-[var(--color-danger)]/25 bg-[var(--color-danger)]/10 text-[var(--color-danger-text)]"
        >
          <Icon name="trash" size={17} />
        </div>
        <div class="min-w-0">
          <p class="text-[9px] font-bold uppercase tracking-[.16em] text-[var(--color-danger)]">
            Permanent action
          </p>
          <h2 id="delete-profile-title" class="mt-1 truncate text-lg font-bold text-white">
            Delete {pendingDelete.name}?
          </h2>
        </div>
      </div>
      <div class="p-6">
        <p class="text-sm leading-6 text-[var(--color-text-secondary)]">
          This permanently removes the profile, including its mods, configurations, worlds, resource
          packs, and logs.
        </p>
        <div class="mt-6 flex justify-end gap-2 border-t border-white/[.07] pt-5">
          <button
            onclick={() => (pendingDelete = null)}
            disabled={deleting}
            class="mc-secondary h-11 px-5 text-xs font-bold disabled:opacity-50">Cancel</button
          ><button
            onclick={remove}
            disabled={deleting}
            class="mc-remove flex h-11 min-w-40 items-center justify-center gap-2 border-[var(--color-danger)]/30 bg-[var(--color-danger)]/10 px-5 text-xs font-bold text-[var(--color-danger-text)] hover:bg-[var(--color-danger)]/15 disabled:cursor-wait disabled:opacity-50"
          >
            <Icon name="trash" size={13} />
            {deleting ? "Deleting…" : "Delete permanently"}</button
          >
        </div>
      </div>
    </div>
  </div>{/if}
{#if operation}
  <div
    class="fixed inset-y-0 left-[248px] right-0 z-[80] grid place-items-center bg-[#010612]/92 p-6"
    role="status"
    aria-live="assertive"
    aria-busy="true"
  >
    <section class="mc-panel w-full max-w-sm overflow-hidden border-cyan-300/25 text-center">
      <div class="border-b border-cyan-300/10 bg-[#071a35] px-6 py-7">
        <div
          class="mx-auto mb-5 grid size-14 place-items-center border border-cyan-300/20 bg-cyan-300/[0.06]"
        >
          <div
            class="size-7 animate-spin rounded-full border-2 border-white/10 border-t-cyan-300"
          ></div>
        </div>
        <h2 class="block-title text-base uppercase">{operation.title}</h2>
        <p class="mt-2 text-xs leading-5 text-[#7899b8]">{operation.message}</p>
      </div>
      <div class="bg-[#030c1c] px-6 py-4">
        <div class="h-1.5 overflow-hidden bg-blue-950">
          <div class="h-full w-2/3 animate-pulse bg-cyan-400"></div>
        </div>
        <p class="mt-3 text-[9px] uppercase tracking-wider text-[#4f708c]">
          Please keep Ninja Launcher open
        </p>
      </div>
    </section>
  </div>
{/if}

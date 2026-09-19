<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
  import Pagination from "$lib/components/ui/Pagination.svelte";
  import { onMount } from "svelte";
  import {
    chooseModpackFile,
    importModpackFile,
    listModpackVersions,
    messageFromError,
    searchModpacks,
  } from "$lib/api/tauri";
  import type {
    LauncherInstance,
    ModpackFileInspection,
    ModpackInstallation,
    ModrinthHit,
    ModrinthVersion,
    SupportedVersion,
  } from "$lib/types";

  let {
    versions,
    installation,
    oninstall,
    onimport,
  }: {
    versions: SupportedVersion[];
    installation: ModpackInstallation | null;
    oninstall: (pack: ModrinthHit, versionId: string) => Promise<void>;
    onimport: (instance: LauncherInstance) => Promise<void>;
  } = $props();

  let query = $state("");
  let results = $state<ModrinthHit[]>([]);
  let versionsByProject = $state<Record<string, ModrinthVersion[]>>({});
  let selectedVersions = $state<Record<string, string>>({});
  let loadingVersions = $state<Record<string, boolean>>({});
  let loading = $state(false);
  let searched = $state(false);
  let page = $state(0);
  let total = $state(0);
  let searchRequest = 0;
  let error = $state("");
  let inspection = $state<ModpackFileInspection | null>(null);
  let importing = $state(false);
  let busyProjectId = $derived(installation?.phase === "installing" ? installation.projectId : "");

  async function loadPage(nextPage: number, searchQuery = query) {
    const request = ++searchRequest;
    loading = true;
    searched = true;
    error = "";
    versionsByProject = {};
    selectedVersions = {};
    try {
      const response = await searchModpacks(searchQuery, nextPage * 20);
      if (request !== searchRequest) return;
      results = response.hits;
      total = response.total_hits;
      page = nextPage;
    } catch (cause) {
      if (request !== searchRequest) return;
      results = [];
      total = 0;
      error = messageFromError(cause);
    } finally {
      if (request === searchRequest) loading = false;
    }
  }

  function search(event: SubmitEvent) {
    event.preventDefault();
    void loadPage(0);
  }

  onMount(() => void loadPage(0, ""));

  function minecraftVersionFor(version: ModrinthVersion): string | null {
    return (
      versions.find((supported) => version.gameVersions.includes(supported.minecraftVersion))
        ?.minecraftVersion ?? null
    );
  }

  async function loadVersions(projectId: string): Promise<ModrinthVersion[]> {
    if (versionsByProject[projectId]) return versionsByProject[projectId];
    if (loadingVersions[projectId]) return [];
    loadingVersions = { ...loadingVersions, [projectId]: true };
    try {
      const available = (await listModpackVersions(projectId)).filter((version) =>
        minecraftVersionFor(version),
      );
      versionsByProject = { ...versionsByProject, [projectId]: available };
      return available;
    } catch (cause) {
      error = messageFromError(cause);
      return [];
    } finally {
      loadingVersions = { ...loadingVersions, [projectId]: false };
    }
  }

  async function install(pack: ModrinthHit) {
    error = "";
    try {
      const available = versionsByProject[pack.projectId] ?? (await loadVersions(pack.projectId));
      const selected = selectedVersions[pack.projectId];
      const version =
        available.find((version) => version.id === selected) ??
        available.find((version) => version.versionType === "release") ??
        available[0];
      if (!version) throw new Error("No supported Fabric version is available for this modpack.");
      await oninstall(pack, version.id);
    } catch (cause) {
      error = messageFromError(cause);
    }
  }

  async function openProject(slug: string) {
    const url = `https://modrinth.com/modpack/${encodeURIComponent(slug)}`;
    if ("__TAURI_INTERNALS__" in window) await openUrl(url);
    else window.open(url, "_blank", "noopener,noreferrer");
  }

  async function chooseImport() {
    error = "";
    try {
      const selected = await chooseModpackFile();
      if (!selected) return;
      if (selected.foreignMods.length) inspection = selected;
      else await runImport(selected, false);
    } catch (cause) {
      error = messageFromError(cause);
    }
  }

  async function runImport(selected: ModpackFileInspection, allowForeignMods: boolean) {
    importing = true;
    error = "";
    try {
      const created = await importModpackFile(selected.path, allowForeignMods);
      inspection = null;
      await onimport(created);
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      importing = false;
    }
  }
</script>

<div class="flex items-start justify-between gap-5">
  <PageHeader
    title="Modpacks"
    description="Install complete Fabric packs from Modrinth as isolated profiles."
  />
  <button
    onclick={chooseImport}
    disabled={importing || installation?.phase === "installing"}
    class="mc-secondary flex shrink-0 items-center gap-2 px-4 py-2.5 text-xs font-bold disabled:opacity-50"
  >
    <Icon name="upload" size={15} />{importing ? "Importing…" : "Import .mrpack"}
  </button>
</div>

<form onsubmit={search} class="mb-6 flex gap-2.5">
  <label
    class="mc-field flex h-11 flex-1 items-center gap-3 px-4 text-[#5f89ac] focus-within:border-cyan-300/60"
  >
    <Icon name="search" size={17} />
    <input
      bind:value={query}
      placeholder="Search Modrinth modpacks…"
      class="w-full bg-transparent text-sm text-white outline-none placeholder:text-[#55718e]"
    />
  </label>
  <button disabled={loading} class="mc-primary px-6 text-xs font-bold disabled:opacity-60">
    {loading ? "Searching…" : "Search"}
  </button>
</form>

{#if error}
  <div class="error-banner mb-5 px-4 py-3 text-xs">
    {error}
  </div>
{/if}

{#if inspection}
  <div
    class="fixed inset-0 z-50 grid place-items-center bg-[#010612]/85 p-6"
    role="presentation"
    onclick={(event) => event.currentTarget === event.target && !importing && (inspection = null)}
  >
    <div
      class="mc-panel w-full max-w-lg border-amber-300/30 p-6"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="foreign-mods-title"
    >
      <div
        class="mb-4 grid size-11 place-items-center border border-amber-300/30 bg-amber-300/10 text-xl font-bold text-amber-200"
      >
        !
      </div>
      <h2 id="foreign-mods-title" class="block-title text-lg uppercase">
        Unverified mods in {inspection.name}
      </h2>
      <p class="mt-3 text-sm leading-6 text-[#9bb2c7]">
        {inspection.foreignMods.length}
        {inspection.foreignMods.length === 1 ? "mod was" : "mods were"}
        not found by checksum on Modrinth. These files may come from another platform or a private source.
        Only continue if you trust the pack author.
      </p>
      <div class="mt-4 max-h-36 overflow-y-auto border border-amber-300/15 bg-black/15 p-3">
        {#each inspection.foreignMods as mod}
          <p class="truncate py-1 font-mono text-[11px] text-amber-100/80">{mod}</p>
        {/each}
      </div>
      <p class="mt-3 text-[10px] text-[#66839d]">
        Minecraft {inspection.minecraftVersion} · Fabric {inspection.loaderVersion} · checksum comparison
        via Modrinth
      </p>
      <div class="mt-6 flex justify-end gap-3">
        <button
          onclick={() => (inspection = null)}
          disabled={importing}
          class="mc-secondary px-4 py-2.5 text-xs font-bold disabled:opacity-50">Cancel</button
        >
        <button
          onclick={() => inspection && runImport(inspection, true)}
          disabled={importing}
          class="border border-amber-300/35 bg-amber-400/15 px-4 py-2.5 text-xs font-bold uppercase tracking-wider text-amber-100 hover:bg-amber-400/25 disabled:opacity-50"
          >{importing ? "Importing…" : "Trust and import"}</button
        >
      </div>
    </div>
  </div>
{/if}

{#if loading}
  <div class="grid h-52 place-items-center">
    <div class="size-7 animate-spin rounded-full border-2 border-white/10 border-t-cyan-300"></div>
  </div>
{:else if results.length}
  <div class="mb-3 flex items-center justify-between">
    <h2 class="text-xs font-bold uppercase tracking-[.12em] text-[#7795ad]">Modrinth results</h2>
    <span class="text-[10px] uppercase tracking-wider text-[#55748f]">
      Fabric modpacks · Minecraft 1.21+
    </span>
  </div>
  <div class="grid gap-4 xl:grid-cols-2">
    {#each results as pack}
      <article class="mc-panel flex min-h-48 flex-col p-5 transition hover:border-cyan-300/35">
        <div class="flex min-w-0 gap-4">
          {#if pack.iconUrl}
            <img
              src={pack.iconUrl}
              alt=""
              class="size-16 shrink-0 object-cover rounded-lg border border-cyan-300/15 bg-[#07335d]"
            />
          {:else}
            <div
              class="grid size-16 shrink-0 place-items-center bg-[#07335d] text-xl font-black text-cyan-100"
            >
              {pack.title.slice(0, 1)}
            </div>
          {/if}
          <div class="min-w-0 flex-1">
            <button
              onclick={() => openProject(pack.slug)}
              class="block max-w-full truncate text-left text-base font-bold text-[#eff9ff] hover:text-cyan-300 hover:underline"
            >
              {pack.title}
            </button>
            <p class="mt-1.5 line-clamp-2 text-xs leading-5 text-[#7896ad]">{pack.description}</p>
            <p class="mt-2 text-[10px] text-[#55748f]">
              by {pack.author} · {Intl.NumberFormat().format(pack.downloads)} downloads
            </p>
          </div>
        </div>
        <div class="mt-auto flex items-center gap-2 pt-5">
          <CustomSelect
            value={selectedVersions[pack.projectId] ?? ""}
            options={[
              {
                value: "",
                label: loadingVersions[pack.projectId]
                  ? "Loading versions…"
                  : "Latest supported release",
              },
              ...(versionsByProject[pack.projectId] ?? []).map((version) => ({
                value: version.id,
                label: `${version.name} · Minecraft ${minecraftVersionFor(version) ?? "unknown"}${version.versionType === "release" ? "" : ` · ${version.versionType}`}`,
              })),
            ]}
            onopen={() => loadVersions(pack.projectId)}
            onchange={(value) =>
              (selectedVersions = {
                ...selectedVersions,
                [pack.projectId]: String(value),
              })}
            disabled={busyProjectId === pack.projectId}
            ariaLabel={`Version of ${pack.title}`}
            class="min-w-0 flex-1 text-[11px] text-[#c8e5f7]"
          />
          <button
            onclick={() => install(pack)}
            disabled={Boolean(busyProjectId)}
            class="mc-primary flex shrink-0 items-center gap-2 px-4 py-2.5 text-xs font-bold disabled:opacity-50"
          >
            <Icon name="download" size={15} />
            {busyProjectId === pack.projectId ? "Installing…" : "Install"}
          </button>
        </div>
      </article>
    {/each}
  </div>
  <Pagination {page} {total} {loading} onchange={(nextPage) => loadPage(nextPage)} />
{:else}
  <div class="mc-panel grid min-h-64 place-items-center border-dashed p-10 text-center">
    <div>
      <div
        class="mx-auto mb-4 grid size-14 place-items-center border border-cyan-300/15 bg-cyan-300/[0.05] text-cyan-200/60"
      >
        <Icon name="modpacks" size={25} />
      </div>
      <p class="text-sm font-semibold text-[#b2cadb]">
        {searched ? "No compatible Fabric modpacks found" : "Find a complete Modrinth modpack"}
      </p>
      <p class="mt-1.5 text-xs text-[#5f7b94]">
        {searched
          ? "Try another name."
          : "Each pack is installed into a separate profile with its own configs and files."}
      </p>
    </div>
  </div>
{/if}

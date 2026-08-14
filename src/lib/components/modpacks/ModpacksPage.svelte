<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import {
    installModpack,
    listModpackVersions,
    messageFromError,
    searchModpacks,
  } from "$lib/api/tauri";
  import type {
    LauncherInstance,
    ModrinthHit,
    ModrinthVersion,
    SupportedVersion,
  } from "$lib/types";

  let {
    minecraftVersion,
    versions,
    oninstalled,
  }: {
    minecraftVersion: string;
    versions: SupportedVersion[];
    oninstalled: (instance: LauncherInstance) => Promise<void>;
  } = $props();

  let selectedMinecraftVersion = $state("");
  let query = $state("");
  let results = $state<ModrinthHit[]>([]);
  let versionsByProject = $state<Record<string, ModrinthVersion[]>>({});
  let selectedVersions = $state<Record<string, string>>({});
  let loadingVersions = $state<Record<string, boolean>>({});
  let loading = $state(false);
  let searched = $state(false);
  let busyProjectId = $state("");
  let error = $state("");

  $effect(() => {
    if (!selectedMinecraftVersion) selectedMinecraftVersion = minecraftVersion;
  });

  async function search(event?: Event) {
    event?.preventDefault();
    loading = true;
    searched = true;
    error = "";
    versionsByProject = {};
    selectedVersions = {};
    try {
      results = (await searchModpacks(query, selectedMinecraftVersion)).hits;
    } catch (cause) {
      results = [];
      error = messageFromError(cause);
    } finally {
      loading = false;
    }
  }

  function changeMinecraftVersion(value: string) {
    if (value === selectedMinecraftVersion) return;
    selectedMinecraftVersion = value;
    results = [];
    versionsByProject = {};
    selectedVersions = {};
    searched = false;
  }

  async function loadVersions(projectId: string) {
    if (versionsByProject[projectId] || loadingVersions[projectId]) return;
    loadingVersions = { ...loadingVersions, [projectId]: true };
    try {
      const available = await listModpackVersions(projectId, selectedMinecraftVersion);
      versionsByProject = { ...versionsByProject, [projectId]: available };
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      loadingVersions = { ...loadingVersions, [projectId]: false };
    }
  }

  async function install(pack: ModrinthHit) {
    busyProjectId = pack.projectId;
    error = "";
    try {
      const created = await installModpack(
        pack.projectId,
        selectedVersions[pack.projectId] || null,
        pack.title,
        pack.iconUrl,
        selectedMinecraftVersion,
      );
      await oninstalled(created);
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      busyProjectId = "";
    }
  }

  async function openProject(slug: string) {
    const url = `https://modrinth.com/modpack/${encodeURIComponent(slug)}`;
    if ("__TAURI_INTERNALS__" in window) await openUrl(url);
    else window.open(url, "_blank", "noopener,noreferrer");
  }
</script>

<div class="flex items-start justify-between gap-5">
  <PageHeader
    title="Modpacks"
    description="Install complete Fabric packs from Modrinth as isolated instances."
  />
  <label class="block">
    <span class="mb-1.5 block text-[9px] font-bold uppercase tracking-[.12em] text-[#63809a]">
      Minecraft version
    </span>
    <select
      value={selectedMinecraftVersion}
      onchange={(event) => changeMinecraftVersion(event.currentTarget.value)}
      disabled={loading || Boolean(busyProjectId)}
      class="mc-field min-w-36 px-3 py-2.5 text-xs font-bold text-blue-50 outline-none disabled:opacity-50"
    >
      {#each versions as version}
        <option value={version.minecraftVersion}>Minecraft {version.minecraftVersion}</option>
      {/each}
    </select>
  </label>
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
  <div class="mb-5 border border-red-400/25 bg-red-400/[0.08] px-4 py-3 text-xs text-red-200">
    {error}
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
      Minecraft {selectedMinecraftVersion} · Fabric
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
              class="size-16 shrink-0 object-cover ring-1 ring-cyan-300/20"
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
          <select
            value={selectedVersions[pack.projectId] ?? ""}
            onfocus={() => loadVersions(pack.projectId)}
            onclick={() => loadVersions(pack.projectId)}
            onchange={(event) =>
              (selectedVersions = {
                ...selectedVersions,
                [pack.projectId]: event.currentTarget.value,
              })}
            disabled={busyProjectId === pack.projectId}
            aria-label={`Version of ${pack.title}`}
            class="mc-field min-w-0 flex-1 truncate px-3 py-2.5 text-[11px] text-[#c8e5f7] outline-none disabled:opacity-50"
          >
            <option value="">
              {loadingVersions[pack.projectId] ? "Loading versions…" : "Latest compatible release"}
            </option>
            {#each versionsByProject[pack.projectId] ?? [] as version}
              <option value={version.id}>
                {version.name}{version.versionType === "release" ? "" : ` · ${version.versionType}`}
              </option>
            {/each}
          </select>
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
          ? `Try another name or Minecraft version.`
          : "Each pack is installed into a separate instance with its own configs and files."}
      </p>
    </div>
  </div>
{/if}

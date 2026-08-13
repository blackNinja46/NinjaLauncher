<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import {
    addMod,
    analyzeModConflicts,
    getModDetails,
    listModVersions,
    messageFromError,
    removeMod,
    searchMods,
    setModEnabled,
    setModVersion,
  } from "$lib/api/tauri";
  import type {
    LauncherInstance,
    ModConflictReport,
    ModrinthHit,
    ModrinthProject,
    ModrinthVersion,
    SupportedVersion,
  } from "$lib/types";

  let {
    instance,
    versions,
    onchanged,
    onversionchange,
  }: {
    instance: LauncherInstance;
    versions: SupportedVersion[];
    onchanged: () => Promise<void>;
    onversionchange: (version: string) => Promise<void>;
  } = $props();
  let view = $state<"installed" | "search">("installed");
  let query = $state("");
  let results = $state<ModrinthHit[]>([]);
  let details = $state<Record<string, ModrinthProject>>({});
  let versionsByProject = $state<Record<string, ModrinthVersion[]>>({});
  let versionLoadErrors = $state<Record<string, string>>({});
  let conflictReport = $state<ModConflictReport>({
    compatible: true,
    message: null,
    projectIds: [],
  });
  let checkingConflicts = $state(false);
  let conflictCheckKey = $state("");
  let searchVersionsByProject = $state<Record<string, ModrinthVersion[]>>({});
  let searchVersionSelections = $state<Record<string, string>>({});
  let loadingSearchVersions = $state<Record<string, boolean>>({});
  let loadedDetailsKey = $state("");
  let loadedVersionsKey = $state("");
  let loading = $state(false);
  let error = $state("");
  let searched = $state(false);
  let busyProjectId = $state("");
  let changingMinecraftVersion = $state(false);
  let searchContext = $state("");
  let installed = $derived(new Set(instance.mods.map((mod) => mod.projectId)));
  let installedMods = $derived(
    [...instance.mods].sort((left, right) => left.name.localeCompare(right.name)),
  );
  let incompatibleCount = $derived(
    installedMods.filter((mod) => compatibilityFor(mod).state === "incompatible").length,
  );

  function compatibilityFor(mod: LauncherInstance["mods"][number]): {
    state: "checking" | "compatible" | "incompatible" | "unknown";
    message: string;
  } {
    if (conflictReport.projectIds.includes(mod.projectId)) {
      return { state: "incompatible", message: "Conflicts with another enabled mod" };
    }
    if (versionLoadErrors[mod.projectId]) {
      return { state: "unknown", message: "Compatibility check failed" };
    }
    const compatibleVersions = versionsByProject[mod.projectId];
    if (!compatibleVersions) return { state: "checking", message: "Checking compatibility…" };
    if (!compatibleVersions.length) {
      return {
        state: "incompatible",
        message: `No version for Minecraft ${instance.minecraftVersion}`,
      };
    }
    const selectedVersion = mod.versionOverrides?.[instance.minecraftVersion];
    if (selectedVersion && !compatibleVersions.some((version) => version.id === selectedVersion)) {
      return { state: "incompatible", message: "Selected mod version is incompatible" };
    }
    return { state: "compatible", message: `Compatible with ${instance.minecraftVersion}` };
  }

  $effect(() => {
    const key = `${instance.id}:${instance.minecraftVersion}:${instance.mods
      .map((mod) =>
        [
          mod.projectId,
          mod.enabled,
          mod.versionOverrides?.[instance.minecraftVersion] ?? "automatic",
        ].join(":"),
      )
      .sort()
      .join("|")}`;
    if (key === conflictCheckKey) return;
    conflictCheckKey = key;
    checkingConflicts = true;
    conflictReport = { compatible: true, message: null, projectIds: [] };
    analyzeModConflicts(instance.id)
      .then((report) => {
        if (conflictCheckKey === key) conflictReport = report;
      })
      .catch((cause) => {
        if (conflictCheckKey === key) error = messageFromError(cause);
      })
      .finally(() => {
        if (conflictCheckKey === key) checkingConflicts = false;
      });
  });

  $effect(() => {
    const context = `${instance.id}:${instance.minecraftVersion}:${instance.loader}`;
    if (searchContext && searchContext !== context) {
      results = [];
      searched = false;
      searchVersionsByProject = {};
      searchVersionSelections = {};
      loadingSearchVersions = {};
    }
    searchContext = context;
  });

  $effect(() => {
    const ids = instance.mods.map((mod) => mod.projectId).sort();
    const key = ids.join(",");
    if (key === loadedDetailsKey) return;
    loadedDetailsKey = key;
    if (!ids.length) {
      details = {};
      return;
    }
    getModDetails(ids)
      .then(
        (projects) =>
          (details = Object.fromEntries(projects.map((project) => [project.id, project]))),
      )
      .catch((cause) => (error = messageFromError(cause)));
  });

  $effect(() => {
    const ids = instance.mods.map((mod) => mod.projectId).sort();
    const key = `${instance.minecraftVersion}:${instance.loader}:${ids.join(",")}`;
    if (key === loadedVersionsKey) return;
    loadedVersionsKey = key;
    versionsByProject = {};
    versionLoadErrors = {};
    if (!ids.length) return;
    Promise.all(
      ids.map(async (projectId) => {
        try {
          return {
            projectId,
            versions: await listModVersions(projectId, instance.minecraftVersion, instance.loader),
            error: "",
          };
        } catch (cause) {
          return { projectId, versions: [], error: messageFromError(cause) };
        }
      }),
    ).then((entries) => {
      if (loadedVersionsKey !== key) return;
      versionsByProject = Object.fromEntries(
        entries.filter((entry) => !entry.error).map((entry) => [entry.projectId, entry.versions]),
      );
      versionLoadErrors = Object.fromEntries(
        entries.filter((entry) => entry.error).map((entry) => [entry.projectId, entry.error]),
      );
    });
  });

  async function search(event?: Event) {
    event?.preventDefault();
    loading = true;
    error = "";
    searched = true;
    searchVersionsByProject = {};
    searchVersionSelections = {};
    try {
      results = (await searchMods(query, instance.minecraftVersion, instance.loader)).hits;
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      loading = false;
    }
  }

  async function toggleSearchResult(mod: ModrinthHit) {
    error = "";
    busyProjectId = mod.projectId;
    try {
      if (installed.has(mod.projectId)) await removeMod(instance.id, mod.projectId);
      else
        await addMod(
          instance.id,
          mod.projectId,
          mod.title,
          searchVersionSelections[mod.projectId] || null,
        );
      await onchanged();
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      busyProjectId = "";
    }
  }

  async function removeInstalled(projectId: string) {
    error = "";
    busyProjectId = projectId;
    try {
      await removeMod(instance.id, projectId);
      await onchanged();
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      busyProjectId = "";
    }
  }

  async function toggleEnabled(projectId: string, enabled: boolean) {
    error = "";
    busyProjectId = projectId;
    try {
      await setModEnabled(instance.id, projectId, enabled);
      await onchanged();
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      busyProjectId = "";
    }
  }

  async function changeModVersion(projectId: string, versionId: string) {
    error = "";
    busyProjectId = projectId;
    try {
      await setModVersion(instance.id, projectId, instance.minecraftVersion, versionId || null);
      await onchanged();
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      busyProjectId = "";
    }
  }

  async function loadSearchVersions(projectId: string) {
    if (
      versionsByProject[projectId] ||
      searchVersionsByProject[projectId] ||
      loadingSearchVersions[projectId]
    )
      return;
    loadingSearchVersions = { ...loadingSearchVersions, [projectId]: true };
    try {
      const versions = await listModVersions(projectId, instance.minecraftVersion, instance.loader);
      searchVersionsByProject = { ...searchVersionsByProject, [projectId]: versions };
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      loadingSearchVersions = { ...loadingSearchVersions, [projectId]: false };
    }
  }

  async function changeSearchVersion(projectId: string, versionId: string) {
    if (installed.has(projectId)) {
      await changeModVersion(projectId, versionId);
      return;
    }
    searchVersionSelections = { ...searchVersionSelections, [projectId]: versionId };
  }

  async function changeMinecraftVersion(minecraftVersion: string) {
    if (minecraftVersion === instance.minecraftVersion) return;
    changingMinecraftVersion = true;
    error = "";
    try {
      await onversionchange(minecraftVersion);
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      changingMinecraftVersion = false;
    }
  }

  async function openProject(slugOrId: string) {
    const url = `https://modrinth.com/mod/${encodeURIComponent(slugOrId)}`;
    if ("__TAURI_INTERNALS__" in window) await openUrl(url);
    else window.open(url, "_blank", "noopener,noreferrer");
  }
</script>

<div class="flex items-start justify-between gap-5">
  <PageHeader title="Mods" description={`Manage mods for ${instance.name}.`} />
  <div class="flex items-end gap-3">
    <label class="block">
      <span class="mb-1.5 block text-[9px] font-bold uppercase tracking-[.12em] text-[#63809a]"
        >Minecraft version</span
      >
      <select
        value={instance.minecraftVersion}
        onchange={(event) => changeMinecraftVersion(event.currentTarget.value)}
        disabled={changingMinecraftVersion || Boolean(busyProjectId)}
        class="mc-field min-w-32 px-3 py-2.5 text-xs font-bold text-blue-50 outline-none disabled:cursor-wait disabled:opacity-50"
        aria-label="Minecraft version for this instance"
      >
        {#each versions as version}
          <option value={version.minecraftVersion}>Minecraft {version.minecraftVersion}</option>
        {/each}
      </select>
    </label>
    {#if view === "installed"}<button
        onclick={() => (view = "search")}
        class="mc-primary flex items-center gap-2 px-4 py-2.5 text-xs font-bold"
      >
        <Icon name="search" size={15} />Find mods</button
      >{/if}
  </div>
</div>

<nav class="mb-6 flex border-b border-cyan-200/[0.09]" aria-label="Mod pages">
  <button
    onclick={() => (view = "installed")}
    class="border-b-2 px-5 py-3 text-xs font-bold uppercase tracking-wider transition {view ===
    'installed'
      ? 'border-cyan-300 text-cyan-100'
      : 'border-transparent text-[#63809a] hover:text-[#a8c7dc]'}"
    >Installed
    <span class="ml-1.5 text-[10px] opacity-60">{instance.mods.length}</span>
  </button>
  <button
    onclick={() => (view = "search")}
    class="border-b-2 px-5 py-3 text-xs font-bold uppercase tracking-wider transition {view ===
    'search'
      ? 'border-cyan-300 text-cyan-100'
      : 'border-transparent text-[#63809a] hover:text-[#a8c7dc]'}">Browse Modrinth</button
  >
</nav>

{#if error}<div
    class="mb-4 rounded-lg border border-red-400/20 bg-red-400/10 px-4 py-3 text-xs text-red-300"
  >
    {error}
  </div>{/if}

{#if view === "installed" && (incompatibleCount > 0 || !conflictReport.compatible)}
  <div
    class="mb-4 flex items-start gap-3 border border-red-400/30 bg-red-400/[0.08] px-4 py-3 text-red-100"
  >
    <span class="grid size-6 shrink-0 place-items-center bg-red-400/15 text-sm font-black">!</span>
    <div>
      <p class="text-xs font-bold uppercase tracking-wider">
        {incompatibleCount > 0
          ? `${incompatibleCount} ${incompatibleCount === 1 ? "mod is" : "mods are"} incompatible`
          : "Conflict between enabled mods detected"}
      </p>
      <p class="mt-1 text-[11px] text-red-200/70">
        {conflictReport.message ??
          "Choose another mod version, disable the affected mod, or return to a supported Minecraft version."}
      </p>
    </div>
  </div>
{/if}

{#if view === "installed" && checkingConflicts}
  <div class="mb-4 flex items-center gap-2 text-[10px] uppercase tracking-wider text-[#6689a5]">
    <span class="size-2 animate-pulse bg-cyan-400"></span>Checking dependencies between enabled
    mods…
  </div>
{/if}

{#if view === "installed"}
  <section class="mc-panel overflow-hidden">
    <div
      class="flex items-center justify-between border-b border-cyan-200/[0.09] bg-[#071a35] px-5 py-4"
    >
      <div>
        <h2 class="block-title text-sm uppercase">Installed mods</h2>
        <p class="mt-1 text-[11px] text-[#698ba9]">
          Disabled mods remain configured but are not loaded when Minecraft starts.
        </p>
      </div>
      <span
        class="border border-cyan-300/20 bg-cyan-300/[0.07] px-2.5 py-1 text-[10px] font-bold uppercase tracking-wider text-cyan-200"
        >{instance.mods.filter((mod) => mod.enabled).length} active</span
      >
    </div>
    {#if installedMods.length}
      <div
        class="hidden grid-cols-[minmax(0,1.35fr)_minmax(210px,1fr)_130px_90px] gap-4 border-b border-white/[0.06] bg-[#041126] px-5 py-2.5 text-[9px] font-bold uppercase tracking-[.12em] text-[#597792] sm:grid"
      >
        <span>Mod</span><span>Version for Minecraft {instance.minecraftVersion}</span><span
          >Status</span
        ><span class="text-right">Action</span>
      </div>
      <div class="divide-y divide-white/[0.06]">
        {#each installedMods as mod}
          {@const project = details[mod.projectId]}
          {@const availableVersions = versionsByProject[mod.projectId]}
          {@const compatibility = compatibilityFor(mod)}
          <article
            class="grid gap-3 px-5 py-4 transition sm:grid-cols-[minmax(0,1.35fr)_minmax(210px,1fr)_130px_90px] sm:items-center sm:gap-4 {compatibility.state ===
            'incompatible'
              ? 'bg-red-500/[0.07] shadow-[inset_3px_0_0_#f87171] hover:bg-red-500/[0.1]'
              : 'hover:bg-cyan-300/[0.035]'}"
          >
            <div class="flex min-w-0 items-center gap-3">
              {#if project?.iconUrl}<img
                  src={project.iconUrl}
                  alt=""
                  class="size-11 shrink-0 object-cover ring-1 ring-cyan-300/20"
                />{:else}<div
                  class="grid size-11 shrink-0 place-items-center border border-cyan-300/15 bg-[#07335d] text-sm font-black uppercase text-cyan-100"
                >
                  {mod.name.slice(0, 1)}
                </div>{/if}
              <div class="min-w-0">
                <button
                  onclick={() => openProject(project?.slug ?? mod.projectId)}
                  class="block max-w-full truncate text-left text-sm font-semibold text-[#edf8ff] hover:text-cyan-300 hover:underline"
                  >{project?.title ?? mod.name}</button
                >
                <p class="mt-1 truncate font-mono text-[10px] text-[#52728f]">{mod.projectId}</p>
              </div>
            </div>
            <div class="min-w-0">
              <select
                value={mod.versionOverrides?.[instance.minecraftVersion] ?? ""}
                onchange={(event) => changeModVersion(mod.projectId, event.currentTarget.value)}
                disabled={!availableVersions ||
                  !availableVersions.length ||
                  busyProjectId === mod.projectId}
                aria-label={`Version of ${mod.name} for Minecraft ${instance.minecraftVersion}`}
                class="mc-field w-full truncate px-2.5 py-2 text-[11px] text-[#c8e5f7] outline-none disabled:cursor-wait disabled:opacity-60"
              >
                <option value="">Automatic · latest compatible</option>
                {#if compatibility.state === "incompatible" && mod.versionOverrides?.[instance.minecraftVersion]}
                  <option value={mod.versionOverrides[instance.minecraftVersion]}>
                    Selected version · incompatible
                  </option>
                {/if}
                {#each availableVersions ?? [] as version}
                  <option value={version.id}>
                    {version.name}{version.versionType === "release"
                      ? ""
                      : ` · ${version.versionType}`}
                  </option>
                {/each}
              </select>
              <p class="mt-1.5 text-[9px] text-[#55748f]">
                {versionLoadErrors[mod.projectId]
                  ? "Could not load compatible versions"
                  : availableVersions
                    ? `${availableVersions.length} compatible versions · ${instance.loader}`
                    : "Loading compatible versions…"}
              </p>
            </div>
            <div class="min-w-0">
              <div
                class="mb-2 flex items-center gap-1.5 text-[9px] font-bold uppercase tracking-wider {compatibility.state ===
                'incompatible'
                  ? 'text-red-300'
                  : compatibility.state === 'compatible'
                    ? 'text-emerald-300'
                    : 'text-[#70879c]'}"
              >
                <span
                  class="size-1.5 {compatibility.state === 'incompatible'
                    ? 'bg-red-400 shadow-[0_0_7px_#f87171]'
                    : compatibility.state === 'compatible'
                      ? 'bg-emerald-400'
                      : 'bg-slate-500'}"
                ></span>
                <span class="truncate" title={compatibility.message}>{compatibility.message}</span>
              </div>
              <button
                role="switch"
                aria-checked={mod.enabled}
                aria-label={`${mod.enabled ? "Disable" : "Enable"} ${mod.name}`}
                onclick={() => toggleEnabled(mod.projectId, !mod.enabled)}
                disabled={busyProjectId === mod.projectId}
                class="flex items-center gap-2.5 text-left disabled:cursor-wait disabled:opacity-50"
                ><span
                  class="relative h-5 w-9 border transition {mod.enabled
                    ? 'border-emerald-300/40 bg-emerald-400/20'
                    : 'border-white/15 bg-white/[0.04]'}"
                  ><span
                    class="absolute top-[3px] size-3 transition-all {mod.enabled
                      ? 'left-[18px] bg-emerald-300 shadow-[0_0_7px_#34d399]'
                      : 'left-[3px] bg-[#607286]'}"
                  ></span></span
                ><span
                  class="text-[10px] font-bold uppercase tracking-wider {mod.enabled
                    ? 'text-emerald-300'
                    : 'text-[#718096]'}">{mod.enabled ? "Enabled" : "Disabled"}</span
                ></button
              >
            </div>
            <button
              onclick={() => removeInstalled(mod.projectId)}
              disabled={busyProjectId === mod.projectId}
              class="justify-self-start border border-red-300/20 bg-red-400/[0.06] px-3 py-2 text-[10px] font-bold uppercase tracking-wider text-red-200 transition hover:bg-red-400/15 disabled:cursor-wait disabled:opacity-50 sm:justify-self-end"
              >{busyProjectId === mod.projectId ? "Working…" : "Remove"}</button
            >
          </article>
        {/each}
      </div>
    {:else}
      <div class="px-6 py-12 text-center">
        <div
          class="mx-auto mb-3 grid size-11 place-items-center border border-cyan-300/15 bg-cyan-300/[0.05] text-[#5e8eae]"
        >
          <Icon name="mods" size={20} />
        </div>
        <p class="text-sm font-semibold text-[#aec8da]">No mods installed</p>
        <p class="mt-1 text-xs text-[#5f7b94]">Browse Modrinth to add compatible mods.</p>
        <button
          onclick={() => (view = "search")}
          class="mc-primary mt-5 px-4 py-2.5 text-xs font-bold">Browse Modrinth</button
        >
      </div>
    {/if}
  </section>
{:else}
  <form onsubmit={search} class="mb-5 flex gap-2.5">
    <label
      class="mc-field flex h-11 flex-1 items-center gap-3 px-4 text-[#5f89ac] focus-within:border-cyan-300/60"
      ><Icon name="search" size={17} /><input
        bind:value={query}
        placeholder="Search Modrinth…"
        class="w-full bg-transparent text-sm text-white outline-none placeholder:text-[#55718e]"
      /></label
    >
    <div class="mc-field px-4 py-3 text-xs text-[#b8d8ed]">
      Minecraft {instance.minecraftVersion}
    </div>
    <div class="mc-field px-4 py-3 text-xs capitalize text-[#b8d8ed]">{instance.loader}</div>
    <button class="mc-primary px-5 text-xs font-bold">Search</button>
  </form>
  <div class="mb-3 flex items-center justify-between">
    <h2 class="text-xs font-semibold uppercase tracking-[.1em] text-[#707580]">
      {searched ? "Search results" : "Discover on Modrinth"}
    </h2>
    <span class="text-[11px] text-[#626771]">Java Edition · Fabric</span>
  </div>
  {#if loading}<div class="grid h-48 place-items-center">
      <div
        class="size-6 animate-spin rounded-full border-2 border-white/10 border-t-accent-400"
      ></div>
    </div>
  {:else if results.length}<div class="space-y-2.5">
      {#each results as mod}<article
          class="mc-panel flex items-center gap-4 p-4 transition hover:border-cyan-300/35 hover:bg-[#0a203d]"
        >
          {#if mod.iconUrl}<img
              src={mod.iconUrl}
              alt=""
              class="size-12 object-cover ring-1 ring-cyan-300/20"
            />{:else}<div
              class="grid size-12 shrink-0 place-items-center bg-[#0a3b68] text-lg font-bold text-cyan-100"
            >
              {mod.title[0]}
            </div>{/if}
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <button
                onclick={() => openProject(mod.slug)}
                class="truncate text-left text-sm font-semibold hover:text-cyan-300 hover:underline"
                >{mod.title}</button
              ><span
                class="shrink-0 rounded bg-emerald-400/10 px-1.5 py-0.5 text-[9px] font-semibold text-emerald-400"
                >COMPATIBLE</span
              >
            </div>
            <p class="mt-1 truncate text-xs text-[#777c87]">{mod.description}</p>
            <div class="mt-1.5 text-[10px] text-[#5f646e]">
              by {mod.author} · {Intl.NumberFormat().format(mod.downloads)} downloads
            </div>
          </div>
          <div class="flex w-60 shrink-0 items-center gap-2">
            <select
              value={installed.has(mod.projectId)
                ? (instance.mods.find((item) => item.projectId === mod.projectId)
                    ?.versionOverrides?.[instance.minecraftVersion] ?? "")
                : (searchVersionSelections[mod.projectId] ?? "")}
              onfocus={() => loadSearchVersions(mod.projectId)}
              onclick={() => loadSearchVersions(mod.projectId)}
              onchange={(event) => changeSearchVersion(mod.projectId, event.currentTarget.value)}
              disabled={busyProjectId === mod.projectId}
              aria-label={`Version of ${mod.title} for Minecraft ${instance.minecraftVersion}`}
              class="mc-field min-w-0 flex-1 truncate px-2.5 py-2 text-[10px] text-[#c8e5f7] outline-none disabled:opacity-50"
            >
              <option value="">
                {loadingSearchVersions[mod.projectId] ? "Loading versions…" : "Automatic version"}
              </option>
              {#each versionsByProject[mod.projectId] ?? searchVersionsByProject[mod.projectId] ?? [] as version}
                <option value={version.id}>
                  {version.name}{version.versionType === "release"
                    ? ""
                    : ` · ${version.versionType}`}
                </option>
              {/each}
            </select>
            <button
              onclick={() => toggleSearchResult(mod)}
              disabled={busyProjectId === mod.projectId}
              class="mc-secondary flex shrink-0 items-center gap-1.5 px-3 py-2 text-xs font-bold text-cyan-50 disabled:cursor-wait disabled:opacity-50"
              >{busyProjectId === mod.projectId
                ? "Working…"
                : installed.has(mod.projectId)
                  ? "Remove"
                  : "Add"}</button
            >
          </div>
        </article>{/each}
    </div>
  {:else}<div
      class="rounded-xl border border-dashed border-white/[0.09] p-12 text-center text-sm text-[#737884]"
    >
      {searched
        ? "No compatible projects found."
        : "Enter a mod name and search the live Modrinth catalog."}
    </div>{/if}
{/if}

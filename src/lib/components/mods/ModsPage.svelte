<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
  import Pagination from "$lib/components/ui/Pagination.svelte";
  import { untrack } from "svelte";
  import {
    addMod,
    analyzeModConflicts,
    getModDetails,
    getInstalledModVersions,
    listModVersions,
    messageFromError,
    removeMod,
    refreshImportedModpackMods,
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
  let page = $state(0);
  let total = $state(0);
  let searchRequest = 0;
  let busyProjectId = $state("");
  let changingMinecraftVersion = $state(false);
  let installedVersionIds = $state<Record<string, string>>({});
  let installedVersionsKey = $state("");
  let updatingProjectId = $state("");
  let versionInfoOpen = $state(false);
  let refreshedImportedPackId = $state("");
  let searchContext = $state("");
  let installed = $derived(new Set(instance.mods.map((mod) => mod.projectId)));
  let installedMods = $derived(
    [...instance.mods].sort((left, right) => left.name.localeCompare(right.name)),
  );
  let packMods = $derived(instance.modpack?.packMods ?? []);
  let incompatibleCount = $derived(
    installedMods.filter((mod) => modEnabled(mod) && compatibilityFor(mod).state === "incompatible")
      .length,
  );
  let modUpdates = $derived.by(() =>
    installedMods.flatMap((mod) => {
      if (!modEnabled(mod)) return [];
      const available = versionsByProject[mod.projectId] ?? [];
      const currentVersionId =
        mod.versionOverrides?.[instance.minecraftVersion] ?? installedVersionIds[mod.projectId];
      if (!currentVersionId) return [];
      const currentIndex = available.findIndex((version) => version.id === currentVersionId);
      if (currentIndex < 0) return [];
      const latestIndex = available.findIndex((version) => version.versionType === "release");
      const preferredIndex = latestIndex >= 0 ? latestIndex : 0;
      if (preferredIndex >= currentIndex) return [];
      const current = available[currentIndex];
      const latest = available[preferredIndex];
      return [
        {
          projectId: mod.projectId,
          name: details[mod.projectId]?.title ?? mod.name,
          currentName: current?.name ?? currentVersionId,
          latestId: latest.id,
          latestName: latest.name,
        },
      ];
    }),
  );

  $effect(() => {
    const localPackId = instance.modpack?.projectId === "local-import" ? instance.id : "";
    if (!localPackId || refreshedImportedPackId === localPackId) return;
    refreshedImportedPackId = localPackId;
    refreshImportedModpackMods(localPackId)
      .then(() => onchanged())
      .catch((cause) => (error = messageFromError(cause)));
  });

  $effect(() => {
    const key = `${instance.id}:${instance.minecraftVersion}`;
    if (installedVersionsKey === key) return;
    installedVersionsKey = key;
    installedVersionIds = {};
    getInstalledModVersions(instance.id)
      .then((versions) => {
        if (installedVersionsKey === key) installedVersionIds = versions;
      })
      .catch((cause) => {
        if (installedVersionsKey === key) error = messageFromError(cause);
      });
  });

  function compatibilityFor(mod: LauncherInstance["mods"][number]): {
    state: "checking" | "compatible" | "incompatible" | "unknown";
    message: string;
  } {
    if (!modEnabled(mod)) {
      return { state: "unknown", message: `Disabled for ${instance.minecraftVersion}` };
    }
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

  function modEnabled(mod: LauncherInstance["mods"][number]): boolean {
    return mod.enabled
      ? !mod.disabledVersions?.includes(instance.minecraftVersion)
      : Boolean(mod.enabledVersions?.includes(instance.minecraftVersion));
  }

  $effect(() => {
    const key = `${instance.id}:${instance.minecraftVersion}:${instance.mods
      .map((mod) =>
        [
          mod.projectId,
          modEnabled(mod),
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
    if (searchContext === context) return;
    searchContext = context;
    query = "";
    results = [];
    searched = false;
    page = 0;
    total = 0;
    searchVersionsByProject = {};
    searchVersionSelections = {};
    loadingSearchVersions = {};
    untrack(() => void loadPage(0, ""));
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

  async function loadPage(nextPage: number, searchQuery = query) {
    const request = ++searchRequest;
    loading = true;
    error = "";
    searched = true;
    searchVersionsByProject = {};
    searchVersionSelections = {};
    try {
      const response = await searchMods(
        searchQuery,
        instance.minecraftVersion,
        instance.loader,
        nextPage * 20,
      );
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

  async function installUpdate(projectId: string, versionId: string) {
    error = "";
    busyProjectId = projectId;
    updatingProjectId = projectId;
    try {
      await setModVersion(instance.id, projectId, instance.minecraftVersion, versionId);
      installedVersionIds = { ...installedVersionIds, [projectId]: versionId };
      await onchanged();
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      busyProjectId = "";
      updatingProjectId = "";
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
      <span
        class="mb-1.5 flex items-center gap-1.5 text-[9px] font-bold uppercase tracking-[.12em] text-[#63809a]"
        >Minecraft version
        <button
          type="button"
          onclick={(event) => {
            event.preventDefault();
            versionInfoOpen = true;
          }}
          title="How version switching works"
          aria-label="Explain Minecraft version switching"
          class="grid size-4 place-items-center rounded-full border border-cyan-300/15 text-[#6689a5] transition hover:border-cyan-300/35 hover:text-cyan-100"
          ><Icon name="info" size={10} strokeWidth={2} /></button
        ></span
      >
      <CustomSelect
        value={instance.minecraftVersion}
        options={versions.map((version) => ({
          value: version.minecraftVersion,
          label: `Minecraft ${version.minecraftVersion}${version.versionType === "snapshot" ? " · Snapshot" : ""}`,
        }))}
        onchange={(value) => changeMinecraftVersion(String(value))}
        disabled={changingMinecraftVersion || Boolean(busyProjectId) || Boolean(instance.modpack)}
        ariaLabel="Minecraft version for this profile"
        class="min-w-48 text-xs font-bold text-blue-50"
      />
    </label>
    {#if view === "installed"}<button
        onclick={() => (view = "search")}
        class="mc-primary flex items-center gap-2 px-4 py-2.5 text-xs font-bold"
      >
        <Icon name="search" size={15} />Find mods</button
      >{/if}
  </div>
</div>

{#if versionInfoOpen}<div
    class="fixed inset-0 z-[70] grid place-items-center bg-[#010612]/85 p-6"
    role="presentation"
    onclick={(event) => event.currentTarget === event.target && (versionInfoOpen = false)}
  >
    <div
      class="mc-panel w-full max-w-lg overflow-hidden border-cyan-300/25"
      role="dialog"
      aria-modal="true"
      aria-labelledby="version-switch-info-title"
    >
      <div class="flex items-start gap-4 border-b border-cyan-300/10 bg-[#071a35] px-6 py-5">
        <span
          class="grid size-10 shrink-0 place-items-center border border-cyan-300/20 bg-cyan-300/[0.06] text-cyan-200"
          ><Icon name="info" size={19} strokeWidth={1.8} /></span
        >
        <div>
          <p class="text-[9px] font-bold uppercase tracking-[.16em] text-cyan-300/60">
            Version switching
          </p>
          <h2 id="version-switch-info-title" class="block-title mt-1 text-lg uppercase">
            Mods for each Minecraft version
          </h2>
        </div>
      </div>
      <div class="space-y-3 p-6 text-xs leading-6 text-[#8daac1]">
        <p>
          Every Minecraft version remembers which personal mods are enabled or disabled and which
          exact mod version you selected.
        </p>
        <p>
          Switching versions does not delete your configuration. Incompatible mods are marked and
          can be disabled only for the current Minecraft version without affecting other versions.
        </p>
        <p>
          On the next launch, Ninja Launcher downloads the compatible files and removes managed mod
          files that are disabled for that version. Modpack-controlled versions remain locked by the
          pack.
        </p>
      </div>
      <div class="flex justify-end border-t border-cyan-300/10 bg-[#030c1c] px-6 py-4">
        <button
          onclick={() => (versionInfoOpen = false)}
          class="mc-primary px-4 py-2.5 text-xs font-bold">Got it</button
        >
      </div>
    </div>
  </div>{/if}

<nav class="mb-6 flex border-b border-cyan-200/[0.09]" aria-label="Mod pages">
  <button
    onclick={() => (view = "installed")}
    class="border-b-2 px-5 py-3 text-xs font-bold uppercase tracking-wider transition {view ===
    'installed'
      ? 'border-cyan-300 text-cyan-100'
      : 'border-transparent text-[#63809a] hover:text-[#a8c7dc]'}"
    >Installed
    <span class="ml-1.5 text-[10px] opacity-60">{instance.mods.length + packMods.length}</span>
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

{#if instance.modpack}
  <div
    class="mb-4 flex items-start gap-3 border border-violet-300/25 bg-violet-400/[0.07] px-4 py-3"
  >
    {#if instance.modpack.iconUrl}<img
        src={instance.modpack.iconUrl}
        alt=""
        class="size-9 shrink-0 object-cover"
      />{:else}<span
        class="grid size-9 shrink-0 place-items-center bg-violet-400/10 text-violet-200"
        ><Icon name="modpacks" size={18} /></span
      >{/if}
    <div>
      <p class="text-xs font-bold text-violet-100">Managed by {instance.modpack.name}</p>
      <p class="mt-1 text-[11px] leading-5 text-violet-200/60">
        The pack installed {instance.modpack.fileCount} verified files. Its Minecraft and Fabric versions
        stay locked; additional personal mods can still be added below.
      </p>
    </div>
  </div>
{/if}

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
        >{instance.mods.filter((mod) => modEnabled(mod)).length + packMods.length} active</span
      >
    </div>
    {#if installedMods.length || packMods.length}
      <div
        class="hidden grid-cols-[minmax(0,1.35fr)_minmax(210px,1fr)_130px_150px] gap-4 border-b border-white/[0.06] bg-[#041126] px-5 py-2.5 text-[9px] font-bold uppercase tracking-[.12em] text-[#597792] sm:grid"
      >
        <span>Mod</span><span>Version for Minecraft {instance.minecraftVersion}</span><span
          >Status</span
        ><span class="text-right">Action</span>
      </div>
      <div class="divide-y divide-white/[0.06]">
        {#each packMods as mod}
          <article
            class="grid gap-3 bg-violet-400/[0.035] px-5 py-4 transition hover:bg-violet-400/[0.06] sm:grid-cols-[minmax(0,1.35fr)_minmax(210px,1fr)_130px_150px] sm:items-center sm:gap-4"
          >
            <div class="flex min-w-0 items-center gap-3">
              {#if mod.iconUrl}<img
                  src={mod.iconUrl}
                  alt=""
                  class="size-11 shrink-0 rounded-lg border border-violet-300/15 bg-[#07335d] object-cover"
                />{:else}<div
                  class="grid size-11 shrink-0 place-items-center border border-violet-300/15 bg-violet-400/10 text-sm font-black uppercase text-violet-100"
                >
                  {mod.name.slice(0, 1)}
                </div>{/if}
              <div class="min-w-0">
                {#if mod.projectId}<button
                    onclick={() => mod.projectId && openProject(mod.projectId)}
                    class="block max-w-full truncate text-left text-sm font-semibold text-[#edf8ff] hover:text-violet-200 hover:underline"
                    >{mod.name}</button
                  >{:else}<p class="truncate text-sm font-semibold text-[#edf8ff]">
                    {mod.name}
                  </p>{/if}
                <p class="mt-1 truncate font-mono text-[10px] text-[#665f91]">
                  {mod.projectId ?? "External mod"}
                </p>
              </div>
            </div>
            <div class="min-w-0">
              <p class="truncate text-[11px] font-semibold text-[#c8c3ec]">{mod.versionName}</p>
              <p class="mt-1.5 truncate font-mono text-[9px] text-[#615d82]">
                {mod.versionId ?? "No Modrinth match"}
              </p>
            </div>
            <div>
              <span
                class="inline-flex items-center gap-1.5 border border-violet-300/20 bg-violet-400/10 px-2.5 py-1.5 text-[9px] font-bold uppercase tracking-wider text-violet-200"
              >
                <Icon name={mod.projectId ? "check" : "mods"} size={10} />{mod.projectId
                  ? "Modrinth match"
                  : "External mod"}
              </span>
            </div>
            <span
              class="justify-self-start text-[9px] font-bold uppercase tracking-wider text-[#625f7e] sm:justify-self-end"
              >Read only</span
            >
          </article>
        {/each}
        {#each installedMods as mod}
          {@const project = details[mod.projectId]}
          {@const availableVersions = versionsByProject[mod.projectId]}
          {@const compatibility = compatibilityFor(mod)}
          {@const enabled = modEnabled(mod)}
          {@const update = modUpdates.find((item) => item.projectId === mod.projectId)}
          <article
            class="grid gap-3 px-5 py-4 transition sm:grid-cols-[minmax(0,1.35fr)_minmax(210px,1fr)_130px_150px] sm:items-center sm:gap-4 {compatibility.state ===
            'incompatible'
              ? 'bg-red-500/[0.07] shadow-[inset_3px_0_0_#f87171] hover:bg-red-500/[0.1]'
              : 'hover:bg-cyan-300/[0.035]'}"
          >
            <div class="flex min-w-0 items-center gap-3">
              {#if project?.iconUrl}<img
                  src={project.iconUrl}
                  alt=""
                  class="size-11 shrink-0 object-cover rounded-lg border border-cyan-300/15 bg-[#07335d]"
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
              <CustomSelect
                value={mod.versionOverrides?.[instance.minecraftVersion] ?? ""}
                options={[
                  { value: "", label: "Automatic · latest compatible" },
                  ...(compatibility.state === "incompatible" &&
                  mod.versionOverrides?.[instance.minecraftVersion]
                    ? [
                        {
                          value: mod.versionOverrides[instance.minecraftVersion],
                          label: "Selected version · incompatible",
                        },
                      ]
                    : []),
                  ...(availableVersions ?? []).map((version) => ({
                    value: version.id,
                    label: `${version.name}${version.versionType === "release" ? "" : ` · ${version.versionType}`}`,
                  })),
                ]}
                onchange={(value) => changeModVersion(mod.projectId, String(value))}
                disabled={!availableVersions ||
                  !availableVersions.length ||
                  busyProjectId === mod.projectId}
                ariaLabel={`Version of ${mod.name} for Minecraft ${instance.minecraftVersion}`}
                class="w-full text-[11px] text-[#c8e5f7]"
              />
              <p class="mt-1.5 text-[9px] text-[#55748f]">
                {versionLoadErrors[mod.projectId]
                  ? "Could not load compatible versions"
                  : availableVersions
                    ? `${availableVersions.length} compatible versions · ${instance.loader}`
                    : "Loading compatible versions…"}
              </p>
            </div>
            <div class="min-w-0">
              <button
                role="switch"
                aria-checked={enabled}
                aria-label={`${enabled ? "Disable" : "Enable"} ${mod.name} for Minecraft ${instance.minecraftVersion}`}
                onclick={() => toggleEnabled(mod.projectId, !enabled)}
                disabled={busyProjectId === mod.projectId}
                class="flex items-center gap-2.5 text-left disabled:cursor-wait disabled:opacity-50"
                ><span
                  class="relative h-6 w-10 border rounded-md transition {enabled
                    ? 'border-emerald-300/40 bg-emerald-400/20'
                    : 'border-white/15 bg-white/[0.04]'}"
                  ><span
                    class="absolute top-[3px] size-4 transition-all rounded-sm {enabled
                      ? 'left-[18px] bg-emerald-300 shadow-[0_0_7px_#34d399]'
                      : 'left-[3px] bg-[#607286]'}"
                  ></span></span
                ><span
                  class="text-[10px] font-bold uppercase tracking-wider {enabled
                    ? 'text-emerald-300'
                    : 'text-[#718096]'}">{enabled ? "Enabled" : "Disabled"}</span
                ></button
              >
              <div
                class="mt-2 flex items-center gap-1.5 text-[9px] font-bold uppercase tracking-wider {compatibility.state ===
                'incompatible'
                  ? 'text-red-300'
                  : compatibility.state === 'compatible'
                    ? 'text-[#698ba9]'
                    : 'text-[#70879c]'}"
              >
                <span class="truncate" title={compatibility.message}>{compatibility.message}</span>
              </div>
            </div>
            <div class="flex justify-self-start gap-1.5 sm:justify-self-end">
              {#if update}<button
                  onclick={() => installUpdate(update.projectId, update.latestId)}
                  disabled={Boolean(busyProjectId)}
                  title={`Update ${update.currentName} → ${update.latestName} for Minecraft ${instance.minecraftVersion}`}
                  aria-label={`Update ${mod.name} to ${update.latestName}`}
                  class="grid size-7 place-items-center border border-emerald-300/15 bg-emerald-300/[0.035] text-[#78a99a] transition hover:border-emerald-300/30 hover:text-emerald-200 disabled:cursor-wait disabled:opacity-40"
                  ><span class={updatingProjectId === mod.projectId ? "animate-pulse" : ""}
                    ><Icon name="download" size={11} strokeWidth={2} /></span
                  ></button
                >{/if}
              <button
                onclick={() => removeInstalled(mod.projectId)}
                disabled={busyProjectId === mod.projectId}
                class="border border-red-300/15 bg-red-400/[0.035] px-2.5 py-2 text-[9px] font-bold uppercase tracking-wider text-red-200/70 transition hover:bg-red-400/10 hover:text-red-200 disabled:cursor-wait disabled:opacity-40"
                >{busyProjectId === mod.projectId && !updatingProjectId
                  ? "Working…"
                  : "Remove"}</button
              >
            </div>
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
              class="size-12 object-cover rounded-lg border border-cyan-300/15 bg-[#07335d]"
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
            <CustomSelect
              value={installed.has(mod.projectId)
                ? (instance.mods.find((item) => item.projectId === mod.projectId)
                    ?.versionOverrides?.[instance.minecraftVersion] ?? "")
                : (searchVersionSelections[mod.projectId] ?? "")}
              options={[
                {
                  value: "",
                  label: loadingSearchVersions[mod.projectId]
                    ? "Loading versions…"
                    : "Automatic version",
                },
                ...(
                  versionsByProject[mod.projectId] ??
                  searchVersionsByProject[mod.projectId] ??
                  []
                ).map((version) => ({
                  value: version.id,
                  label: `${version.name}${version.versionType === "release" ? "" : ` · ${version.versionType}`}`,
                })),
              ]}
              onopen={() => loadSearchVersions(mod.projectId)}
              onchange={(value) => changeSearchVersion(mod.projectId, String(value))}
              disabled={busyProjectId === mod.projectId}
              ariaLabel={`Version of ${mod.title} for Minecraft ${instance.minecraftVersion}`}
              class="min-w-0 flex-1 text-[10px] text-[#c8e5f7]"
            />
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
    <Pagination {page} {total} {loading} onchange={(nextPage) => loadPage(nextPage)} />
  {:else}<div
      class="rounded-xl border border-dashed border-white/[0.09] p-12 text-center text-sm text-[#737884]"
    >
      {searched
        ? "No compatible projects found."
        : "Enter a mod name and search the live Modrinth catalog."}
    </div>{/if}
{/if}

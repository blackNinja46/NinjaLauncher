<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
  import Pagination from "$lib/components/ui/Pagination.svelte";
  import Toggle from "$lib/components/ui/Toggle.svelte";
  import { untrack } from "svelte";
  import {
    addMod,
    analyzeModConflicts,
    getModDetails,
    getInstalledModVersions,
    installProfileContent,
    listProfileContent,
    listProfileContentVersions,
    listModVersions,
    messageFromError,
    removeMod,
    removeProfileContent,
    refreshImportedModpackMods,
    searchLauncherContent,
    setResourcePackEnabled,
    setModEnabled,
    setModVersion,
  } from "$lib/api/tauri";
  import type { ModrinthSearchIndex } from "$lib/api/tauri";
  import type {
    LauncherInstance,
    ModConflictReport,
    ModrinthHit,
    ModrinthProject,
    ModrinthVersion,
    ProfileContentFile,
    ProfileContentKind,
    SupportedVersion,
  } from "$lib/types";

  let {
    instance,
    versions,
    onchanged,
    onversionchange,
    onnotify,
  }: {
    instance: LauncherInstance;
    versions: SupportedVersion[];
    onchanged: () => Promise<void>;
    onversionchange: (version: string) => Promise<void>;
    onnotify: (notification: {
      title: string;
      message: string;
      tone: "info" | "success" | "warning" | "error";
    }) => void;
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
  let searchIndex = $state<ModrinthSearchIndex>("relevance");
  let page = $state(0);
  let total = $state(0);
  let searchRequest = 0;
  let busyProjectId = $state("");
  let changingMinecraftVersion = $state(false);
  let installedVersionIds = $state<Record<string, string>>({});
  let installedVersionsKey = $state("");
  let installedProfileContent = $state<(ProfileContentFile & { kind: ProfileContentKind })[]>([]);
  let installedContentKey = $state("");
  let installedFilter = $state<"all" | "mods" | ProfileContentKind>("all");
  let installedQuery = $state("");
  let updatingProjectId = $state("");
  let versionInfoOpen = $state(false);
  let versionInfoLayer = $state<HTMLDivElement>();
  let refreshedImportedPackId = $state("");
  let searchContext = $state("");
  let lastErrorNotification = $state("");
  let incompatibilityNotificationKey = $state("");
  let installed = $derived(new Set(instance.mods.map((mod) => mod.projectId)));
  let installedContent = $derived(
    new Map(
      installedProfileContent
        .filter((file) => file.projectId)
        .map((file) => [file.projectId as string, file]),
    ),
  );
  let installedMods = $derived(
    [...instance.mods].sort((left, right) => left.name.localeCompare(right.name)),
  );
  let packMods = $derived(instance.modpack?.packMods ?? []);
  let normalizedInstalledQuery = $derived(installedQuery.trim().toLocaleLowerCase());
  let filteredProfileContent = $derived(
    installedProfileContent.filter(
      (file) =>
        (installedFilter === "all" || installedFilter === file.kind) &&
        (!normalizedInstalledQuery ||
          `${file.name} ${file.fileName} ${file.projectId ?? ""}`
            .toLocaleLowerCase()
            .includes(normalizedInstalledQuery)),
    ),
  );
  let showInstalledMods = $derived(installedFilter === "all" || installedFilter === "mods");
  let filteredPackMods = $derived(
    packMods.filter(
      (mod) =>
        !normalizedInstalledQuery ||
        `${mod.name} ${mod.projectId ?? ""} ${mod.versionName}`
          .toLocaleLowerCase()
          .includes(normalizedInstalledQuery),
    ),
  );
  let filteredInstalledMods = $derived(
    installedMods.filter((mod) => {
      const project = details[mod.projectId];
      return (
        !normalizedInstalledQuery ||
        `${mod.name} ${mod.projectId} ${project?.title ?? ""} ${project?.slug ?? ""}`
          .toLocaleLowerCase()
          .includes(normalizedInstalledQuery)
      );
    }),
  );
  let visibleInstalledCount = $derived(
    filteredProfileContent.length +
      (showInstalledMods ? filteredInstalledMods.length + filteredPackMods.length : 0),
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
    if (error && error !== lastErrorNotification) {
      onnotify({ title: "Action failed", message: error, tone: "error" });
    }
    lastErrorNotification = error;
  });

  $effect(() => {
    const versionIds = instance.mods.map((mod) => mod.projectId).sort();
    const expectedVersionsKey = `${instance.minecraftVersion}:${instance.loader}:${versionIds.join(",")}`;
    const expectedConflictKey = compatibilityCheckKey();
    if (
      loadedVersionsKey !== expectedVersionsKey ||
      conflictCheckKey !== expectedConflictKey ||
      checkingConflicts
    )
      return;
    const affected = installedMods
      .filter((mod) => modEnabled(mod) && compatibilityFor(mod).state === "incompatible")
      .map((mod) => mod.projectId)
      .sort();
    const hasConflict = !conflictReport.compatible;
    const key = affected.length || hasConflict
      ? `${instance.id}:${instance.minecraftVersion}:${affected.join(",")}:${conflictReport.message ?? ""}`
      : "";
    if (key && key !== incompatibilityNotificationKey) {
      onnotify({
        title: affected.length === 1 ? "Incompatible mod" : "Incompatible mods",
        message:
          conflictReport.message ??
          `${affected.length} enabled ${affected.length === 1 ? "mod is" : "mods are"} not compatible with Minecraft ${instance.minecraftVersion}.`,
        tone: "warning",
      });
    }
    incompatibilityNotificationKey = key;
  });

  $effect(() => {
    const localPackId = instance.modpack?.projectId === "local-import" ? instance.id : "";
    if (!localPackId || refreshedImportedPackId === localPackId) return;
    refreshedImportedPackId = localPackId;
    refreshImportedModpackMods(localPackId)
      .then(() => onchanged())
      .catch((cause) => (error = messageFromError(cause)));
  });

  $effect(() => {
    const key = instance.id;
    if (installedContentKey === key) return;
    installedContentKey = key;
    void refreshInstalledContent(key);
  });

  async function refreshInstalledContent(key = installedContentKey) {
    try {
      const [resourcepacks, shaderpacks] = await Promise.all([
        listProfileContent(instance.id, "resourcepacks"),
        listProfileContent(instance.id, "shaderpacks"),
      ]);
      if (installedContentKey === key) {
        installedProfileContent = [
          ...resourcepacks.map((file) => ({ ...file, kind: "resourcepacks" as const })),
          ...shaderpacks.map((file) => ({ ...file, kind: "shaderpacks" as const })),
        ];
      }
    } catch (cause) {
      if (installedContentKey === key) error = messageFromError(cause);
    }
  }

  $effect(() => {
    if (view !== "search") return;
    untrack(() => void refreshInstalledContent());
  });

  function contentKindFor(project: ModrinthHit): ProfileContentKind | null {
    if (project.projectType === "resourcepack") return "resourcepacks";
    if (project.projectType === "shader") return "shaderpacks";
    return null;
  }

  function projectTypeLabel(project: ModrinthHit): string {
    if (project.projectType === "resourcepack") return "Resource pack";
    if (project.projectType === "shader") return "Shader";
    return "Mod";
  }

  function projectTypeIcon(project: ModrinthHit): string {
    if (project.projectType === "resourcepack") return "palette";
    if (project.projectType === "shader") return "shader";
    return "mods";
  }

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

  function compatibilityCheckKey(): string {
    return `${instance.id}:${instance.minecraftVersion}:${instance.mods
      .map((mod) =>
        [
          mod.projectId,
          modEnabled(mod),
          mod.versionOverrides?.[instance.minecraftVersion] ?? "automatic",
        ].join(":"),
      )
      .sort()
      .join("|")}`;
  }

  $effect(() => {
    const key = compatibilityCheckKey();
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

  async function loadPage(
    nextPage: number,
    searchQuery = query,
    index: ModrinthSearchIndex = searchIndex,
  ) {
    const request = ++searchRequest;
    loading = true;
    error = "";
    searched = true;
    searchVersionsByProject = {};
    searchVersionSelections = {};
    try {
      const response = await searchLauncherContent(
        searchQuery,
        instance.minecraftVersion,
        nextPage * 20,
        index,
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

  function changeSearchIndex(value: string | number) {
    searchIndex = value as ModrinthSearchIndex;
    void loadPage(0, query, searchIndex);
  }

  async function toggleSearchResult(project: ModrinthHit) {
    error = "";
    busyProjectId = project.projectId;
    try {
      const contentKind = contentKindFor(project);
      if (contentKind) {
        const installedFile = await installProfileContent(
          instance.id,
          contentKind,
          project.projectId,
          searchVersionSelections[project.projectId] || null,
          project.title,
          project.iconUrl,
        );
        installedProfileContent = [
          ...installedProfileContent.filter((file) => file.projectId !== project.projectId),
          { ...installedFile, kind: contentKind },
        ];
      } else if (installed.has(project.projectId)) {
        await removeMod(instance.id, project.projectId);
        await onchanged();
        onnotify({
          title: "Mod removed",
          message: `${project.title} was removed from ${instance.name}.`,
          tone: "success",
        });
      } else {
        await addMod(
          instance.id,
          project.projectId,
          project.title,
          searchVersionSelections[project.projectId] || null,
        );
        await onchanged();
        onnotify({
          title: "Mod installed",
          message: `${project.title} was added to ${instance.name}.`,
          tone: "success",
        });
      }
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      busyProjectId = "";
    }
  }

  async function removeInstalledContent(file: ProfileContentFile & { kind: ProfileContentKind }) {
    error = "";
    busyProjectId = file.projectId ?? `${file.kind}:${file.fileName}`;
    try {
      await removeProfileContent(instance.id, file.kind, file.fileName);
      installedProfileContent = installedProfileContent.filter(
        (item) => !(item.kind === file.kind && item.fileName === file.fileName),
      );
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      busyProjectId = "";
    }
  }

  function formatFileSize(bytes: number | null): string {
    if (bytes === null) return "Folder";
    if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  async function toggleResourcePack(
    file: ProfileContentFile & { kind: ProfileContentKind },
    enabled: boolean,
  ) {
    error = "";
    busyProjectId = file.projectId ?? `${file.kind}:${file.fileName}`;
    try {
      await setResourcePackEnabled(instance.id, file.fileName, enabled);
      installedProfileContent = installedProfileContent.map((item) =>
        item.kind === file.kind && item.fileName === file.fileName ? { ...item, enabled } : item,
      );
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
      onnotify({
        title: "Mod removed",
        message: `${details[projectId]?.title ?? installedMods.find((mod) => mod.projectId === projectId)?.name ?? "The mod"} was removed.`,
        tone: "success",
      });
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
      onnotify({
        title: enabled ? "Mod enabled" : "Mod disabled",
        message: `${details[projectId]?.title ?? installedMods.find((mod) => mod.projectId === projectId)?.name ?? "The mod"} is now ${enabled ? "enabled" : "disabled"} for Minecraft ${instance.minecraftVersion}.`,
        tone: "success",
      });
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
      const versionName = versionId
        ? versionsByProject[projectId]?.find((version) => version.id === versionId)?.name ?? versionId
        : "Automatic · latest compatible";
      onnotify({
        title: "Mod version changed",
        message: `${details[projectId]?.title ?? installedMods.find((mod) => mod.projectId === projectId)?.name ?? "The mod"}: ${versionName}`,
        tone: "success",
      });
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
      onnotify({
        title: "Mod updated",
        message: `${details[projectId]?.title ?? installedMods.find((mod) => mod.projectId === projectId)?.name ?? "The mod"} now uses the selected compatible version.`,
        tone: "success",
      });
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      busyProjectId = "";
      updatingProjectId = "";
    }
  }

  async function loadSearchVersions(project: ModrinthHit) {
    const projectId = project.projectId;
    if (
      versionsByProject[projectId] ||
      searchVersionsByProject[projectId] ||
      loadingSearchVersions[projectId]
    )
      return;
    loadingSearchVersions = { ...loadingSearchVersions, [projectId]: true };
    try {
      const contentKind = contentKindFor(project);
      const versions = contentKind
        ? await listProfileContentVersions(projectId, instance.minecraftVersion, contentKind)
        : await listModVersions(projectId, instance.minecraftVersion, instance.loader);
      searchVersionsByProject = { ...searchVersionsByProject, [projectId]: versions };
    } catch (cause) {
      error = messageFromError(cause);
    } finally {
      loadingSearchVersions = { ...loadingSearchVersions, [projectId]: false };
    }
  }

  async function changeSearchVersion(project: ModrinthHit, versionId: string) {
    if (project.projectType === "mod" && installed.has(project.projectId)) {
      await changeModVersion(project.projectId, versionId);
      return;
    }
    searchVersionSelections = {
      ...searchVersionSelections,
      [project.projectId]: versionId,
    };
  }

  async function changeMinecraftVersion(minecraftVersion: string) {
    if (minecraftVersion === instance.minecraftVersion) return;
    versionInfoOpen = false;
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

  async function openProject(slugOrId: string, projectType = "mod") {
    const url = `https://modrinth.com/${projectType}/${encodeURIComponent(slugOrId)}`;
    if ("__TAURI_INTERNALS__" in window) await openUrl(url);
    else window.open(url, "_blank", "noopener,noreferrer");
  }

  $effect(() => {
    if (versionInfoOpen && versionInfoLayer && !versionInfoLayer.matches(":popover-open")) {
      versionInfoLayer.showPopover();
    }
  });
</script>

<div class="flex items-start justify-between gap-5">
  <PageHeader
    title="Mods"
    description={`Manage mods, resource packs and shaders for ${instance.name}.`}
  />
  <div class="flex items-end gap-3">
    <div class="block">
      <div
        class="mb-1.5 flex items-center gap-1.5 text-[9px] font-bold uppercase tracking-[.12em] text-[#63809a]"
      >
        Minecraft version
        <button
          type="button"
          onclick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            versionInfoOpen = true;
          }}
          title="How version switching works"
          aria-label="Explain Minecraft version switching"
          class="grid size-4 place-items-center rounded-[4px] border border-white/[0.10] text-[var(--color-text-muted)] hover:border-white/[0.22] hover:text-white"
          ><Icon name="info" size={10} /></button
        >
      </div>
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
    </div>
  </div>
</div>

{#if versionInfoOpen}<div
    bind:this={versionInfoLayer}
    popover="manual"
    class="fixed inset-0 z-[70] m-0 grid h-auto w-auto max-w-none place-items-center border-0 bg-black/70 p-6 text-inherit backdrop-blur-sm"
    role="presentation"
    onclick={(event) => event.currentTarget === event.target && (versionInfoOpen = false)}
  >
    <div
      class="mc-panel w-full max-w-lg overflow-hidden border-white/[0.12]"
      role="dialog"
      aria-modal="true"
      aria-labelledby="version-switch-info-title"
    >
      <div class="flex items-start gap-4 border-b border-white/[0.08] px-6 py-5">
        <span
          class="grid size-10 shrink-0 place-items-center text-[var(--color-primary)]"
          ><Icon name="info" size={28} /></span
        >
        <div>
          <p class="text-[9px] font-bold uppercase tracking-[.16em] text-[var(--color-text-muted)]">
            Version switching
          </p>
          <h2 id="version-switch-info-title" class="block-title mt-1 text-lg uppercase">
            Mods for each Minecraft version
          </h2>
        </div>
      </div>
      <div class="space-y-3 p-6 text-xs leading-6 text-[var(--color-text-secondary)]">
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
      <div class="flex justify-end border-t border-white/[0.08] px-6 py-4">
        <button
          onclick={() => (versionInfoOpen = false)}
          class="mc-primary px-4 py-2.5 text-xs font-bold">Got it</button
        >
      </div>
    </div>
  </div>{/if}

<nav
  class="mb-6 flex w-fit gap-1 rounded-lg border border-white/[0.08] bg-black/20 p-1"
  aria-label="Mod pages"
>
  <button
    onclick={() => (view = "installed")}
    class="border px-5 py-2.5 text-xs font-bold uppercase tracking-wider {view === 'installed'
      ? 'border-white/[0.12] bg-white/[0.08] text-white shadow-[inset_0_-2px_0_var(--color-primary)]'
      : 'border-transparent bg-transparent text-[var(--color-text-muted)] hover:bg-white/[0.04] hover:text-white'}"
    >Installed
    <span class="ml-1.5 text-[10px] opacity-60"
      >{instance.mods.length + packMods.length + installedProfileContent.length}</span
    >
  </button>
  <button
    onclick={() => (view = "search")}
    class="border px-5 py-2.5 text-xs font-bold uppercase tracking-wider {view === 'search'
      ? 'border-white/[0.12] bg-white/[0.08] text-white shadow-[inset_0_-2px_0_var(--color-primary)]'
      : 'border-transparent bg-transparent text-[var(--color-text-muted)] hover:bg-white/[0.04] hover:text-white'}"
    ><img
      src="/brand/modrinth.svg"
      alt=""
      class="mr-2 inline size-3.5 grayscale brightness-200 opacity-70"
    />Browse Modrinth</button
  >
</nav>

{#if instance.modpack}
  <div
    class="mb-4 flex items-start gap-3 border border-cyan-300/20 bg-cyan-300/[0.05] px-4 py-3"
  >
    {#if instance.modpack.iconUrl}<img
        src={instance.modpack.iconUrl}
        alt=""
        class="size-9 shrink-0 object-cover"
      />{:else}<span
        class="grid size-9 shrink-0 place-items-center bg-cyan-300/10 text-cyan-200"
        ><Icon name="modpacks" size={18} /></span
      >{/if}
    <div>
      <p class="text-xs font-bold text-cyan-100">Managed by {instance.modpack.name}</p>
      <p class="mt-1 text-[11px] leading-5 text-[var(--color-text-muted)]">
        The pack installed {instance.modpack.fileCount} verified files. Its Minecraft and Fabric versions
        stay locked; additional personal mods can still be added below.
      </p>
    </div>
  </div>
{/if}

{#if view === "installed" && showInstalledMods && checkingConflicts}
  <div class="mb-4 flex items-center gap-2 text-[10px] uppercase tracking-wider text-[#6689a5]">
    <span class="size-2 animate-pulse bg-cyan-400"></span>Checking dependencies between enabled
    mods…
  </div>
{/if}

{#if view === "installed"}
  <section class="mc-panel overflow-hidden">
    <div
      class="flex flex-wrap items-center justify-between gap-3 border-b border-cyan-200/[0.09] bg-white/[0.035] px-5 py-4"
    >
      <div>
        <h2 class="block-title text-sm uppercase">Installed content</h2>
        <p class="mt-1 text-[11px] text-[#698ba9]">
          {installedFilter === "resourcepacks"
            ? "Choose which texture packs Minecraft loads with this profile."
            : "Mods, resource packs and shaders belonging to this profile."}
        </p>
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <label
          class="mc-field flex h-9 w-76 items-center gap-2 px-3 text-[var(--color-text-muted)] focus-within:border-[var(--color-border-brand)]"
        >
          <Icon name="search" size={12} />
          <input
            bind:value={installedQuery}
            type="search"
            aria-label="Search installed mods"
            placeholder="Search installed mods…"
            class="min-w-0 flex-1 bg-transparent text-xs text-white outline-none placeholder:text-[var(--color-text-faint)]"
          />
        </label>
        <nav
          class="flex gap-1 rounded-lg text-xs font-bold border border-white/[0.08] bg-black/15 p-1"
          aria-label="Content filter"
        >
          {#each [{ id: "all", label: "All" }, { id: "mods", label: "Mods" }, { id: "resourcepacks", label: "Resource packs" }, { id: "shaderpacks", label: "Shader" }] as filter}
            <button
              type="button"
              onclick={() => (installedFilter = filter.id as typeof installedFilter)}
              aria-pressed={installedFilter === filter.id}
              class="px-3 py-2 text-[6px] font-bold uppercase tracking-wider {installedFilter ===
              filter.id
                ? 'bg-cyan-300/[0.12] text-cyan-50'
                : 'text-[#6689a5] hover:bg-white/[0.05] hover:text-white'}">{filter.label}</button
            >
          {/each}
        </nav>
      </div>
    </div>
    {#if visibleInstalledCount > 0}
      <div
        class="hidden grid-cols-[minmax(0,1.35fr)_minmax(210px,1fr)_130px_150px] gap-4 border-b border-white/[0.06] bg-white/[0.02] px-5 py-2.5 text-[9px] font-bold uppercase tracking-[.12em] text-[#597792] sm:grid"
      >
        <span>Content</span><span>Version or file</span><span>Type &amp; status</span><span
          class="text-right">Action</span
        >
      </div>
      <div class="divide-y divide-white/[0.06]">
        {#each showInstalledMods ? filteredPackMods : [] as mod}
          <article
            class="grid gap-3 px-5 py-4 transition hover:bg-cyan-300/[0.035] sm:grid-cols-[minmax(0,1.35fr)_minmax(210px,1fr)_130px_150px] sm:items-center sm:gap-4"
          >
            <div class="flex min-w-0 items-center gap-3">
              {#if mod.iconUrl}<img
                  src={mod.iconUrl}
                  alt=""
                  class="size-11 shrink-0 rounded-lg border border-cyan-300/15 bg-[#07335d] object-cover"
                />{:else}<div
                  class="grid size-11 shrink-0 place-items-center border border-cyan-300/15 bg-cyan-300/10 text-sm font-black uppercase text-cyan-100"
                >
                  {mod.name.slice(0, 1)}
                </div>{/if}
              <div class="min-w-0">
                {#if mod.projectId}<button
                    onclick={() => mod.projectId && openProject(mod.projectId)}
                    class="block max-w-full truncate text-left text-sm font-semibold text-[#edf8ff] hover:text-cyan-200 hover:underline"
                    >{mod.name}</button
                  >{:else}<p class="truncate text-sm font-semibold text-[#edf8ff]">
                    {mod.name}
                  </p>{/if}
                <p class="mt-1 truncate font-mono text-[10px] text-[#52728f]">
                  {mod.projectId ?? "External mod"}
                </p>
              </div>
            </div>
            <div class="min-w-0">
              <p class="truncate text-[11px] font-semibold text-[#c8e5f7]">{mod.versionName}</p>
              <p class="mt-1.5 truncate font-mono text-[9px] text-[#55748f]">
                {mod.versionId ?? "No Modrinth match"}
              </p>
            </div>
            <div>
              <span
                class="inline-flex items-center gap-1.5 border border-cyan-300/20 bg-cyan-300/10 px-2.5 py-1.5 text-[9px] font-bold uppercase tracking-wider text-cyan-200"
              >
                <Icon name={mod.projectId ? "check" : "mods"} size={10} />{mod.projectId
                  ? "Modrinth match"
                  : "External mod"}
              </span>
            </div>
            <span
              class="justify-self-start text-[9px] font-bold uppercase tracking-wider text-[#55748f] sm:justify-self-end"
              >Read only</span
            >
          </article>
        {/each}
        {#each filteredProfileContent as file (`${file.kind}:${file.fileName}`)}
          {@const projectRoute = file.kind === "resourcepacks" ? "resourcepack" : "shader"}
          {@const contentBusy =
            busyProjectId === (file.projectId ?? `${file.kind}:${file.fileName}`)}
          <article
            class="grid gap-3 px-5 py-4 transition hover:bg-cyan-300/[0.035] sm:grid-cols-[minmax(0,1.35fr)_minmax(210px,1fr)_130px_150px] sm:items-center sm:gap-4"
          >
            <div class="flex min-w-0 items-center gap-3">
              {#if file.iconUrl}
                <img
                  src={file.iconUrl}
                  alt=""
                  class="size-11 shrink-0 rounded-[6px] border border-cyan-300/15 bg-[#07335d] object-cover"
                />
              {:else}
                <span
                  class="grid size-11 shrink-0 place-items-center border border-cyan-300/15 rounded-lg bg-cyan-300/[0.055] text-[#77bfe9]"
                >
                  <Icon name={file.kind === "resourcepacks" ? "palette" : "shader"} size={17} />
                </span>
              {/if}
              <div class="min-w-0">
                {#if file.projectId}
                  <button
                    type="button"
                    onclick={() => file.projectId && openProject(file.projectId, projectRoute)}
                    class="block max-w-full truncate text-left text-sm font-semibold text-[#edf8ff] hover:text-cyan-300 hover:underline"
                    >{file.name}</button
                  >
                {:else}
                  <h3 class="truncate text-sm font-semibold text-[#edf8ff]">{file.name}</h3>
                {/if}
                <p class="mt-1 text-[9px] font-bold uppercase tracking-wider text-[#52728f]">
                  {file.projectId ? "Installed from Modrinth" : "Local profile file"}
                </p>
              </div>
            </div>
            <div class="min-w-0">
              <p class="truncate text-[11px] font-semibold text-[#c8e5f7]">
                {file.versionId ?? file.fileName}
              </p>
              <p class="mt-1.5 truncate font-mono text-[9px] text-[#55748f]">
                {file.fileName} · {formatFileSize(file.sizeBytes)}
              </p>
            </div>
            <div class="min-w-0">
              {#if file.kind === "resourcepacks"}
                <Toggle
                  checked={file.enabled ?? false}
                  label={`${file.enabled ? "Do not load" : "Load"} ${file.name} with this profile`}
                  disabled={contentBusy}
                  showState
                  onLabel="Loaded"
                  offLabel="Not loaded"
                  onchange={(checked) => toggleResourcePack(file, checked)}
                />
                <p class="mt-1.5 text-[8px] font-bold uppercase tracking-wider text-[#55748f]">
                  With this profile
                </p>
              {:else}
                <span
                  class="inline-flex items-center gap-1.5 rounded-lg border border-cyan-300/20 bg-cyan-300/[0.07] px-2.5 py-1.5 text-[9px] font-bold uppercase tracking-wider text-cyan-200"
                >
                  <Icon name="shader" size={10} />Shader
                </span>
              {/if}
            </div>
            <div class="flex justify-self-start gap-1.5 sm:justify-self-end">
              <button
                type="button"
                onclick={() => removeInstalledContent(file)}
                disabled={Boolean(busyProjectId)}
                title={`Remove ${file.name}`}
                aria-label={`Remove ${file.name}`}
                class="mc-remove px-2.5 py-2 text-[9px] font-bold uppercase tracking-wider disabled:cursor-wait disabled:opacity-40"
                >{contentBusy ? "Working…" : "Remove"}</button
              >
            </div>
          </article>
        {/each}
        {#each showInstalledMods ? filteredInstalledMods : [] as mod}
          {@const project = details[mod.projectId]}
          {@const availableVersions = versionsByProject[mod.projectId]}
          {@const compatibility = compatibilityFor(mod)}
          {@const enabled = modEnabled(mod)}
          {@const update = modUpdates.find((item) => item.projectId === mod.projectId)}
          {@const selectedVersionId = mod.versionOverrides?.[instance.minecraftVersion] ?? ""}
          {@const selectedVersionIsCompatible = Boolean(
            selectedVersionId &&
              availableVersions?.some((version) => version.id === selectedVersionId),
          )}
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
                value={selectedVersionId}
                options={[
                  { value: "", label: "Automatic · latest compatible" },
                  ...(selectedVersionId && !selectedVersionIsCompatible
                    ? [
                        {
                          value: selectedVersionId,
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
                disabled={busyProjectId === mod.projectId ||
                  (!selectedVersionId && (!availableVersions || !availableVersions.length))}
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
              <Toggle
                checked={enabled}
                label={`${enabled ? "Disable" : "Enable"} ${mod.name} for Minecraft ${instance.minecraftVersion}`}
                disabled={busyProjectId === mod.projectId}
                showState
                onLabel="Enabled"
                offLabel="Disabled"
                onchange={(checked) => toggleEnabled(mod.projectId, checked)}
              />
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
            <div class="flex justify-self-start gap-2 sm:justify-self-end">
              {#if update}<button
                  onclick={() => installUpdate(update.projectId, update.latestId)}
                  disabled={Boolean(busyProjectId)}
                  title={`Update ${update.currentName} → ${update.latestName} for Minecraft ${instance.minecraftVersion}`}
                  aria-label={`Update ${mod.name} to ${update.latestName}`}
                  class="grid size-7 place-items-center text-[#78a99a] transition hover:text-emerald-200 disabled:cursor-wait disabled:opacity-40"
                  ><span class={updatingProjectId === mod.projectId ? "animate-pulse" : ""}
                    ><Icon name="download" size={25} /></span
                  ></button
                >{/if}
              <button
                onclick={() => removeInstalled(mod.projectId)}
                disabled={busyProjectId === mod.projectId}
                class="mc-remove px-2.5 py-2 text-[9px] font-bold uppercase tracking-wider disabled:cursor-wait disabled:opacity-40"
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
        <p class="text-sm font-semibold text-[#aec8da]">
          {normalizedInstalledQuery
            ? "No installed mods found"
            : installedFilter === "all"
              ? "No content installed"
              : "No matching content installed"}
        </p>
        <p class="mt-1 text-xs text-[#5f7b94]">
          {normalizedInstalledQuery
            ? `No installed content matches “${installedQuery.trim()}”.`
            : installedFilter === "all"
              ? "Browse Modrinth for mods, resource packs and shaders."
              : "Choose another filter or install matching content from Modrinth."}
        </p>
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
    <CustomSelect
      value={searchIndex}
      options={[
        { value: "relevance", label: "Relevance" },
        { value: "downloads", label: "Downloads" },
        { value: "follows", label: "Followers" },
        { value: "newest", label: "Date published" },
        { value: "updated", label: "Date updated" },
      ]}
      onchange={changeSearchIndex}
      ariaLabel="Sort Modrinth results"
      class="w-44 text-xs text-[#b8d8ed]"
    />
    <button class="mc-primary px-5 text-xs font-bold">Search</button>
  </form>
  <div class="mb-3 flex items-center justify-between">
    <h2 class="text-xs font-semibold uppercase tracking-[.1em] text-[#707580]">
      {searched ? "Search results" : "Discover on Modrinth"}
    </h2>
    <span class="text-[11px] text-[#626771]">Mods · Resource packs · Shaders</span>
  </div>
  {#if loading}<div class="grid h-48 place-items-center">
      <div
        class="size-6 animate-spin rounded-full border-2 border-white/10 border-t-accent-400"
      ></div>
    </div>
  {:else if results.length}<div class="space-y-2.5">
      {#each results as mod}<article
          class="mc-panel flex items-center gap-4 p-4 hover:border-cyan-300/35 hover:bg-white/[0.045]"
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
                onclick={() => openProject(mod.slug, mod.projectType)}
                class="truncate text-left text-sm font-semibold hover:text-cyan-300 hover:underline"
                >{mod.title}</button
              ><span
                class="inline-flex shrink-0 items-center gap-1.5 rounded bg-cyan-300/[0.08] px-1.5 py-0.5 text-[9px] font-semibold uppercase text-cyan-200"
                ><Icon name={projectTypeIcon(mod)} size={9} />{projectTypeLabel(mod)}</span
              >
              <span
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
              value={mod.projectType === "mod" && installed.has(mod.projectId)
                ? (instance.mods.find((item) => item.projectId === mod.projectId)
                    ?.versionOverrides?.[instance.minecraftVersion] ?? "")
                : (searchVersionSelections[mod.projectId] ?? "")}
              options={[
                {
                  value: "",
                  label: loadingSearchVersions[mod.projectId]
                    ? "Loading versions…"
                    : mod.projectType === "mod"
                      ? "Automatic version"
                      : installedContent.has(mod.projectId)
                        ? "Latest compatible update"
                        : "Latest compatible",
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
              onopen={() => loadSearchVersions(mod)}
              onchange={(value) => changeSearchVersion(mod, String(value))}
              disabled={busyProjectId === mod.projectId}
              ariaLabel={`Version of ${mod.title} for Minecraft ${instance.minecraftVersion}`}
              class="min-w-0 flex-1 text-[10px] text-[#c8e5f7]"
            />
            <button
              onclick={() => toggleSearchResult(mod)}
              disabled={busyProjectId === mod.projectId}
              class="{mod.projectType === 'mod' && installed.has(mod.projectId)
                ? 'mc-remove'
                : 'mc-secondary text-white'} flex shrink-0 items-center gap-1.5 px-3 py-2 text-xs font-bold disabled:cursor-wait disabled:opacity-50"
              >{busyProjectId === mod.projectId
                ? "Working…"
                : mod.projectType === "mod" && installed.has(mod.projectId)
                  ? "Remove"
                  : installedContent.has(mod.projectId)
                    ? "Update"
                    : "Install"}</button
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

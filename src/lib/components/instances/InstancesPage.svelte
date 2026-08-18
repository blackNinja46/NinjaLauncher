<script lang="ts">
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
  import {
    createInstance,
    deleteInstance,
    duplicateInstance,
    messageFromError,
    openInstanceFolder,
  } from "$lib/api/tauri";
  import type { LauncherInstance, ModpackInstallation, SupportedVersion } from "$lib/types";
  let {
    instances,
    versions,
    selectedId,
    installation,
    onselect,
    onchanged,
  }: {
    instances: LauncherInstance[];
    versions: SupportedVersion[];
    selectedId: string;
    installation: ModpackInstallation | null;
    onselect: (id: string) => void;
    onchanged: () => Promise<void>;
  } = $props();
  let creating = $state(false);
  let name = $state("New Fabric Instance");
  let version = $state("");
  let error = $state("");
  let pendingDelete = $state<LauncherInstance | null>(null);
  let deleting = $state(false);
  let loaderVersion = $derived(
    versions.find((item) => item.minecraftVersion === version)?.fabricLoaderVersion ?? "0.19.3",
  );
  $effect(() => {
    if (!version && versions[0]) version = versions[0].minecraftVersion;
  });
  async function create() {
    try {
      const value = await createInstance(name, version, loaderVersion);
      creating = false;
      await onchanged();
      onselect(value.id);
    } catch (e) {
      error = messageFromError(e);
    }
  }
  async function duplicate(id: string) {
    try {
      await duplicateInstance(id);
      await onchanged();
    } catch (e) {
      error = messageFromError(e);
    }
  }
  async function remove() {
    if (!pendingDelete) return;
    deleting = true;
    error = "";
    try {
      await deleteInstance(pendingDelete.id);
      pendingDelete = null;
      await onchanged();
    } catch (e) {
      error = messageFromError(e);
    } finally {
      deleting = false;
    }
  }
</script>

<div class="flex items-start justify-between">
  <PageHeader
    title="Instances"
    description="Each instance keeps its mods, worlds, and settings isolated."
  /><button
    onclick={() => (creating = true)}
    class="mc-primary flex items-center gap-2 px-4 py-2.5 text-xs font-bold"
    ><Icon name="plus" size={15} />New instance</button
  >
</div>
{#if error}<div class="mb-4 rounded-lg bg-red-400/10 px-4 py-3 text-xs text-red-300">
    {error}
  </div>{/if}
<div class="space-y-3">
  {#if installation}<article
      class="mc-panel relative overflow-hidden border-violet-300/35 bg-violet-400/[.07] p-5"
      aria-live="polite"
    >
      <div class="flex items-center gap-3">
        <div
          class="grid size-14 shrink-0 place-items-center overflow-hidden rounded-lg bg-[#031237] ring-1 ring-violet-300/30"
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
              class="border border-violet-300/25 bg-violet-400/10 px-2 py-.5 text-[9px] font-bold text-violet-200"
              >MODPACK</span
            >
          </div>
          <p class="mt-1 text-xs text-[#91a9c2]">
            {installation.phase === "installing"
              ? "Installing Modrinth modpack and creating instance…"
              : (installation.error ?? "The modpack could not be installed.")}
          </p>
        </div>
        {#if installation.phase === "installing"}<div
            class="size-6 animate-spin rounded-full border-2 border-white/10 border-t-violet-300"
            aria-label="Installation in progress"
          ></div>{:else}<span
            class="text-lg font-bold text-red-300"
            aria-label="Installation failed">!</span
          >{/if}
      </div>
    </article>{/if}
  {#each instances as instance}<article
      class="mc-panel relative overflow-hidden p-5 transition duration-200 {selectedId ===
      instance.id
        ? 'border-cyan-300/70 bg-gradient-to-r from-cyan-400/[.16] via-blue-500/[.09] to-[#051227]/95 shadow-[inset_5px_0_0_#19d8f1,inset_0_1px_0_rgba(133,235,255,.2),0_0_28px_rgba(9,159,240,.16)]'
        : 'hover:border-cyan-300/30 hover:bg-blue-500/[.04]'}"
    >
      {#if selectedId === instance.id}<div
          class="pointer-events-none absolute inset-y-0 left-0 w-1 bg-cyan-300 shadow-[0_0_14px_#19d8f1]"
        ></div>{/if}
      <div class="flex items-center gap-2">
        <div
          class="grid size-14 place-items-center overflow-hidden rounded-lg border border-cyan-300/15 bg-[#07335d] transition {selectedId ===
          instance.id
            ? 'ring-1 ring-cyan-300/55 shadow-[0_0_18px_rgba(25,216,241,.25)]'
            : ''}"
        >
          {#if instance.modpack?.iconUrl}<img
              src={instance.modpack.iconUrl}
              alt={`${instance.modpack.name} modpack`}
              class="size-full object-cover"
            />{:else}<img
              src={`/loaders/${instance.loader}.png`}
              alt={`${instance.loader} mod loader`}
              class="size-13 object-contain"
            />{/if}
        </div>
        <button
          onclick={() => onselect(instance.id)}
          aria-pressed={selectedId === instance.id}
          class="flex-1 text-left"
          ><div class="flex items-center gap-2">
            <h2 class="block-title text-[15px] uppercase">{instance.name}</h2>
            {#if selectedId === instance.id}<span
                class="inline-flex items-center gap-1 border border-cyan-300/35 bg-cyan-300/15 px-2 py-0.5 text-[9px] font-extrabold uppercase tracking-[.1em] text-cyan-100 shadow-[0_0_12px_rgba(25,216,241,.12)]"
                ><Icon name="check" size={10} strokeWidth={2.8} />Active instance</span
              >{/if}
            {#if instance.modpack}<span
                class="border border-violet-300/20 bg-violet-400/10 px-2 py-.5 text-[9px] font-bold text-violet-200"
                >MODPACK</span
              >{/if}
          </div>
          <p class="mt-1 text-xs text-[#7899b8]">
            Minecraft {instance.minecraftVersion} · Fabric {instance.loaderVersion} · {instance.modpack
              ? `${instance.modpack.fileCount} pack files`
              : `${instance.mods.length} mods`}
          </p></button
        >
        <button
          onclick={(e) => {
            e.stopPropagation();
            openInstanceFolder(instance.id);
          }}
          title="Open folder"
          class="grid size-9 place-items-center rounded-lg text-[#777c87] hover:bg-white/5"
          ><Icon name="folder" size={17} /></button
        ><button
          onclick={(e) => {
            e.stopPropagation();
            duplicate(instance.id);
          }}
          title="Duplicate"
          class="rounded-lg px-3 py-2 text-xs text-[#9297a1] hover:bg-white/5">Duplicate</button
        ><button
          onclick={(e) => {
            e.stopPropagation();
            pendingDelete = instance;
          }}
          title="Delete"
          class="rounded-lg px-3 py-2 text-xs text-red-300 hover:bg-red-400/10">Delete</button
        >
      </div>
    </article>{/each}
</div>
{#if creating}<div class="fixed inset-0 z-40 grid place-items-center bg-[#010612]/80">
    <div class="mc-panel w-full max-w-md p-6">
      <h2 class="block-title text-lg uppercase">Create instance</h2>
      <div class="mt-5 space-y-3">
        <input
          bind:value={name}
          placeholder="Instance name"
          class="mc-field w-full px-3 py-2.5 text-sm outline-none"
        /><CustomSelect
          value={version}
          options={versions.map((item) => ({
            value: item.minecraftVersion,
            label: `Minecraft ${item.minecraftVersion}${item.versionType === "snapshot" ? " · Snapshot" : ""}`,
          }))}
          onchange={(value) => (version = String(value))}
          ariaLabel="Minecraft version"
          class="w-full text-sm"
        />
        <div class="mc-field w-full px-3 py-2.5 text-sm text-[#91b9d7]">
          Fabric {loaderVersion} · stable
        </div>
      </div>
      <div class="mt-5 flex justify-end gap-3">
        <button onclick={() => (creating = false)} class="mc-secondary px-4 py-2 text-xs"
          >Cancel</button
        ><button onclick={create} class="mc-primary px-4 py-2 text-xs font-bold">Create</button>
      </div>
    </div>
  </div>{/if}
{#if pendingDelete}<div
    class="fixed inset-0 z-50 grid place-items-center bg-[#010612]/85 p-6"
    role="presentation"
    onclick={(event) => event.currentTarget === event.target && !deleting && (pendingDelete = null)}
  >
    <div
      class="mc-panel w-full max-w-md border-red-400/25 p-6"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="delete-instance-title"
    >
      <div
        class="mb-4 grid size-11 place-items-center border border-red-400/25 bg-red-400/10 text-xl text-red-300"
      >
        !
      </div>
      <h2 id="delete-instance-title" class="block-title text-lg uppercase">
        Delete {pendingDelete.name}?
      </h2>
      <p class="mt-3 text-sm leading-6 text-[#8ca7c0]">
        This permanently removes the instance, including its mods, configurations, worlds, resource
        packs, and logs.
      </p>
      <div class="mt-6 flex justify-end gap-3">
        <button
          onclick={() => (pendingDelete = null)}
          disabled={deleting}
          class="mc-secondary px-4 py-2.5 text-xs font-bold disabled:opacity-50">Cancel</button
        ><button
          onclick={remove}
          disabled={deleting}
          class="border border-red-300/30 bg-red-500/20 px-4 py-2.5 text-xs font-bold uppercase tracking-wider text-red-200 hover:bg-red-500/30 disabled:opacity-50"
          >{deleting ? "Deleting…" : "Delete permanently"}</button
        >
      </div>
    </div>
  </div>{/if}

<script lang="ts">
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import {
    createInstance,
    deleteInstance,
    duplicateInstance,
    messageFromError,
    openInstanceFolder,
  } from "$lib/api/tauri";
  import type { LauncherInstance, SupportedVersion } from "$lib/types";
  let {
    instances,
    versions,
    selectedId,
    onselect,
    onchanged,
  }: {
    instances: LauncherInstance[];
    versions: SupportedVersion[];
    selectedId: string;
    onselect: (id: string) => void;
    onchanged: () => Promise<void>;
  } = $props();
  let creating = $state(false);
  let name = $state("New Fabric Instance");
  let version = $state("26.2");
  let error = $state("");
  let pendingDelete = $state<LauncherInstance | null>(null);
  let deleting = $state(false);
  let loaderVersion = $derived(
    versions.find((item) => item.minecraftVersion === version)?.fabricLoaderVersion ?? "0.19.3",
  );
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
  {#each instances as instance}<article
      class="mc-panel p-5 {selectedId === instance.id
        ? 'border-cyan-300/45 bg-blue-500/[.08] shadow-[inset_4px_0_0_#19d8f1]'
        : ''}"
    >
      <div class="flex items-center gap-4">
        <div class="grid size-14 place-items-center overflow-hidden bg-[#031237]">
          <img
            src={instance.modpack?.iconUrl ?? "/brand/ninja-launcher-mark.png"}
            alt=""
            class="size-13 object-contain"
          />
        </div>
        <button onclick={() => onselect(instance.id)} class="flex-1 text-left"
          ><div class="flex items-center gap-2">
            <h2 class="block-title text-[15px] uppercase">{instance.name}</h2>
            {#if selectedId === instance.id}<span
                class="bg-accent-500/15 px-2 py-.5 text-[9px] font-bold text-accent-400"
                >SELECTED</span
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
        /><select
          bind:value={version}
          aria-label="Minecraft version"
          class="mc-field w-full px-3 py-2.5 text-sm outline-none"
          >{#each versions as item}<option value={item.minecraftVersion}
              >Minecraft {item.minecraftVersion}</option
            >{/each}</select
        >
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

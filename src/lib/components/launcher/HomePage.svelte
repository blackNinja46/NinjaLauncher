<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import type { LauncherInstance, SupportedVersion } from "$lib/types";
  let {
    instance,
    versions,
    signedIn,
    launching,
    progress,
    status,
    onplay,
    onopenmods,
    onversionchange,
  }: {
    instance: LauncherInstance;
    versions: SupportedVersion[];
    signedIn: boolean;
    launching: boolean;
    progress: number | null;
    status: string;
    onplay: () => Promise<void>;
    onopenmods: () => void;
    onversionchange: (version: string) => Promise<void>;
  } = $props();
</script>

<PageHeader title="Ready to build?" description="Choose your world, gear up, and jump in." />
<section class="mc-panel overflow-hidden">
  <div class="relative min-h-[320px] overflow-hidden border-b border-cyan-300/[0.12] p-8">
    <div class="voxel-pattern absolute inset-0 opacity-90"></div>
    <div class="absolute -right-20 -top-24 size-80 rounded-full bg-cyan-400/[0.08] blur-3xl"></div>
    <div class="relative max-w-[650px]">
      <img
        src="/brand/ninja-launcher-wordmark.png"
        alt="Ninja Launcher"
        class="mb-8 h-auto w-[570px] max-w-full drop-shadow-[0_10px_18px_rgba(0,5,35,.55)]"
      />
      <div
        class="mb-4 inline-flex items-center gap-2 border border-cyan-300/[0.18] bg-blue-950/55 px-2.5 py-1 text-[9px] font-bold uppercase tracking-[.16em] text-cyan-100/70"
      >
        <span class="size-2 bg-emerald-400 shadow-[0_0_8px_#34d399]"></span>Selected instance
      </div>
      <h2 class="block-title text-[28px] uppercase text-white">{instance.name}</h2>
      <div class="mt-4 flex items-center gap-2.5">
        <span class="text-[10px] font-semibold uppercase tracking-wider text-[#71849b]"
          >Version</span
        >
        <CustomSelect
          value={instance.minecraftVersion}
          options={versions.map((version) => ({
            value: version.minecraftVersion,
            label: `${version.minecraftVersion}${version.versionType === "snapshot" ? " · Snapshot" : ""}`,
          }))}
          onchange={(value) => onversionchange(String(value))}
          disabled={launching || Boolean(instance.modpack)}
          ariaLabel="Minecraft version"
          class="min-w-34 text-xs font-bold text-blue-50"
        />
        <span class="mc-field px-3 py-2 text-xs text-[#9bc9ed]"
          >Fabric {instance.loaderVersion}</span
        >
      </div>
      <div class="mt-7 flex items-center gap-3">
        <button
          onclick={onplay}
          disabled={launching}
          class="mc-primary flex min-w-[190px] items-center justify-center gap-2 px-8 py-3.5 text-sm font-extrabold disabled:opacity-70"
          ><Icon name="play" size={17} />{launching
            ? "PREPARING…"
            : signedIn
              ? "PLAY"
              : "PLAY DEMO"}</button
        >
        <button
          onclick={onopenmods}
          class="mc-secondary px-5 py-3.5 text-sm font-bold text-[#ccecff]">Manage mods</button
        >
      </div>
      {#if progress !== null}<div class="mt-5 max-w-lg" aria-live="polite">
          <div
            class="mb-2 flex items-center justify-between text-[10px] font-bold uppercase tracking-[.11em]"
          >
            <span class={launching ? "text-cyan-200" : "text-emerald-300"}>{status}</span>
            <span class="font-mono text-[#89b7d4]">{progress}%</span>
          </div>
          <div
            class="h-2 overflow-hidden border border-cyan-300/20 bg-[#020a1a] p-px"
            role="progressbar"
            aria-label="Minecraft launch progress"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={progress}
          >
            <div
              class="relative h-full overflow-hidden transition-[width,background-color] duration-500 ease-out {progress ===
              100
                ? 'bg-emerald-400'
                : 'bg-cyan-400'}"
              style={`width: ${progress}%`}
            >
              {#if launching}<span
                  class="absolute inset-y-0 right-0 w-12 animate-pulse bg-gradient-to-r from-transparent to-white/45"
                ></span>{/if}
            </div>
          </div>
        </div>{/if}
      {#if instance.modpack}<p class="mt-3 text-[11px] text-violet-200/70">
          Modrinth pack · {instance.modpack.name} · {instance.modpack.fileCount} managed files
        </p>{/if}
      {#if !signedIn}<p class="mt-3 text-[11px] text-[#8094ad]">
          Demo mode · Local test worlds only · Online multiplayer unavailable
        </p>{/if}
    </div>
  </div>
  <div class="grid grid-cols-3 divide-x divide-cyan-300/[0.1] bg-[#051126]/70">
    <div class="p-5">
      <div class="text-[10px] font-semibold uppercase tracking-wider text-[#6f7480]">Minecraft</div>
      <div class="mt-1.5 text-sm font-medium">{instance.minecraftVersion}</div>
    </div>
    <div class="p-5">
      <div class="text-[10px] font-semibold uppercase tracking-wider text-[#6f7480]">
        Mod loader
      </div>
      <div class="mt-1.5 flex items-center gap-2 text-sm font-medium">
        <span class="size-2 rounded-sm bg-[#d8a33c]"></span>Fabric
      </div>
    </div>
    <div class="p-5">
      <div class="text-[10px] font-semibold uppercase tracking-wider text-[#6f7480]">
        Enabled mods
      </div>
      <div class="mt-1.5 text-sm font-medium">
        {instance.modpack
          ? `${instance.modpack.fileCount} pack files`
          : `${instance.mods.filter((mod) => mod.enabled).length} mods`}
      </div>
    </div>
  </div>
</section>
<div class="mc-panel mt-4 flex items-center gap-2 px-4 py-3 text-xs text-[#86a8c7]">
  <span class="grid size-5 place-items-center bg-emerald-400/10 text-emerald-400"
    ><Icon name="check" size={12} strokeWidth={2.5} /></span
  >{status}<span class="ml-auto text-[#4f718f]">Last checked just now</span>
</div>

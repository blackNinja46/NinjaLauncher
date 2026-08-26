<script lang="ts">
  import { onMount } from "svelte";
  import { getServerIcon } from "$lib/api/tauri";
  import Icon from "$lib/components/ui/Icon.svelte";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import type { LauncherInstance, SupportedVersion } from "$lib/types";
  let {
    instance,
    versions,
    signedIn,
    launching,
    running,
    stopping,
    progress,
    status,
    onplay,
    onconnect,
    onstop,
    onopenmods,
    onversionchange,
  }: {
    instance: LauncherInstance;
    versions: SupportedVersion[];
    signedIn: boolean;
    launching: boolean;
    running: boolean;
    stopping: boolean;
    progress: number | null;
    status: string;
    onplay: () => Promise<void>;
    onconnect: (serverAddress: string) => Promise<void>;
    onstop: () => Promise<void>;
    onopenmods: () => void;
    onversionchange: (version: string) => Promise<void>;
  } = $props();
  const featuredServers = [
    { name: "blackNinja.Live", address: "blackninja.live" },
    { name: "NinjaSMP.net", address: "ninjasmp.net" },
  ];
  let serverIcons = $state<Record<string, string>>({});

  onMount(() => {
    for (const server of featuredServers) {
      getServerIcon(server.address)
        .then((icon) => {
          if (icon) serverIcons = { ...serverIcons, [server.address]: icon };
        })
        .catch(() => {});
    }
  });
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
        <span class="size-2 bg-emerald-400 shadow-[0_0_8px_#34d399]"></span>Selected profile
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
          disabled={launching || running || Boolean(instance.modpack)}
          ariaLabel="Minecraft version"
          class="min-w-34 text-xs font-bold text-blue-50"
        />
        <span class="mc-field px-3 py-2 text-xs text-[#9bc9ed]"
          >Fabric {instance.loaderVersion}</span
        >
      </div>
      <div class="mt-7 flex items-center gap-3">
        {#if running}<button
            onclick={onstop}
            disabled={stopping}
            class="flex min-w-[190px] items-center justify-center gap-2 border border-red-300/35 bg-red-500/20 px-8 py-3.5 text-sm font-extrabold text-red-100 hover:bg-red-500/30 disabled:opacity-60"
            ><span class="size-3 bg-current"></span>{stopping ? "STOPPING…" : "STOP"}</button
          >{:else}<button
            onclick={onplay}
            disabled={launching}
            class="mc-primary flex min-w-[190px] items-center justify-center gap-2 px-8 py-3.5 text-sm font-extrabold disabled:opacity-70"
            ><Icon name="play" size={17} />{launching
              ? "PREPARING…"
              : signedIn
                ? "PLAY"
                : "PLAY DEMO"}</button
          >{/if}
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
          : `${
              instance.mods.filter((mod) =>
                mod.enabled
                  ? !mod.disabledVersions?.includes(instance.minecraftVersion)
                  : mod.enabledVersions?.includes(instance.minecraftVersion),
              ).length
            } mods`}
      </div>
    </div>
  </div>
</section>
<section class="mt-4 border border-white/[0.055] bg-[#031027]/45 px-4 py-3">
  <div class="mb-2.5 flex items-center justify-between">
    <p class="text-[9px] font-bold uppercase tracking-[.14em] text-[#58748d]">Featured servers</p>
    {#if !signedIn}<span class="text-[9px] text-[#695f63]">Sign in for multiplayer</span>{/if}
  </div>
  <div class="grid gap-2 sm:grid-cols-2">
    {#each featuredServers as server}
      <article
        class="flex items-center gap-3 border border-white/[0.045] bg-black/10 px-3 py-2.5 transition hover:border-cyan-300/10 hover:bg-cyan-300/[0.025]"
      >
        {#if serverIcons[server.address]}<img
            src={serverIcons[server.address]}
            alt={`${server.name} server icon`}
            class="size-8 shrink-0 border border-white/10 bg-[#031237] object-cover opacity-80 [image-rendering:pixelated]"
          />{:else}<div
            class="grid size-8 shrink-0 place-items-center border border-white/[0.08] bg-white/[0.025] text-xs font-bold uppercase text-[#71879b]"
          >
            {server.name.slice(0, 1)}
          </div>{/if}
        <div class="min-w-0 flex-1">
          <h3 class="truncate text-xs font-semibold text-[#a9bfd2]">{server.name}</h3>
          <p class="mt-0.5 truncate font-mono text-[9px] text-[#4d687f]">{server.address}</p>
        </div>
        <button
          onclick={() => onconnect(server.address)}
          disabled={launching || running || !signedIn}
          title={signedIn ? `Connect to ${server.name}` : "Sign in to use multiplayer"}
          class="flex items-center gap-1.5 border border-cyan-300/10 bg-cyan-300/[0.035] px-2.5 py-1.5 text-[9px] font-bold text-[#789bb5] transition hover:border-cyan-300/25 hover:text-cyan-100 disabled:cursor-not-allowed disabled:opacity-35"
          ><Icon name="play" size={10} />Connect</button
        >
      </article>
    {/each}
  </div>
</section>
<div class="mc-panel mt-4 flex items-center gap-2 px-4 py-3 text-xs text-[#86a8c7]">
  <span class="grid size-5 place-items-center bg-emerald-400/10 text-emerald-400"
    ><Icon name="check" size={12} strokeWidth={2.5} /></span
  >{status}<span class="ml-auto text-[#4f718f]">Last checked just now</span>
</div>

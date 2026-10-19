<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";
  import type { LauncherInstance, SupportedVersion } from "$lib/types";
  import { compareMinecraftVersionsDescending, sortSupportedVersions } from "$lib/utils/version";

  let {
    value,
    versions,
    loader,
    disabled = false,
    onchange,
  }: {
    value: string;
    versions: SupportedVersion[];
    loader: LauncherInstance["loader"];
    disabled?: boolean;
    onchange: (version: string) => Promise<void>;
  } = $props();

  let open = $state(false);
  let changing = $state("");

  const artworkByFamily: Record<string, string> = {
    "26": "/backgrounds/versions/26.webp",
    "1.21": "/backgrounds/versions/1.21.webp",
    "1.20": "/backgrounds/versions/1.20.webp",
    "1.19": "/backgrounds/versions/1.19.webp",
    "1.18": "/backgrounds/versions/1.18.webp",
    "1.17": "/backgrounds/versions/1.17.webp",
    "1.16": "/backgrounds/versions/1.16.webp",
    "1.15": "/backgrounds/versions/1.15.webp",
    "1.14": "/backgrounds/versions/1.14.webp",
    "1.13": "/backgrounds/versions/1.13.webp",
    "1.12": "/backgrounds/versions/1.12.webp",
    "1.11": "/backgrounds/versions/1.11.webp",
    "1.10": "/backgrounds/versions/1.10.webp",
    "1.9": "/backgrounds/versions/1.9.webp",
    "1.8": "/backgrounds/versions/1.8.webp",
    "1.7": "/backgrounds/versions/1.7.webp",
    "1.6": "/backgrounds/versions/1.6.webp",
    "1.5": "/backgrounds/versions/1.5.webp",
    "1.4": "/backgrounds/versions/1.4.webp",
    "1.3": "/backgrounds/versions/1.3.webp",
    "1.2": "/backgrounds/versions/1.2.webp",
    "1.1": "/backgrounds/versions/1.1.webp",
    "1.0": "/backgrounds/versions/1.0.webp",
  };

  function familyOf(version: string): string {
    if (/^\d{2}\./.test(version)) return version.split(".")[0];
    const legacy = version.match(/^(1\.\d+)/);
    return legacy?.[1] ?? "Other";
  }

  let families = $derived.by(() => {
    const groups = new Map<string, SupportedVersion[]>();
    for (const version of versions) {
      const family = familyOf(version.minecraftVersion);
      groups.set(family, [...(groups.get(family) ?? []), version]);
    }
    return [...groups.entries()]
      .map(([name, entries]) => ({ name, entries: sortSupportedVersions(entries) }))
      .sort((left, right) => compareMinecraftVersionsDescending(left.name, right.name));
  });

  function artworkFor(family: string): string {
    return artworkByFamily[family] ?? "/backgrounds/versions/legacy.webp";
  }

  function show() {
    if (disabled) return;
    open = true;
  }

  function portalToLauncherContent(node: HTMLElement) {
    const content = document.querySelector<HTMLElement>(".launcher-main");
    document.body.appendChild(node);

    const position = () => {
      const bounds =
        content?.getBoundingClientRect() ?? document.documentElement.getBoundingClientRect();
      node.style.top = `${bounds.top}px`;
      node.style.left = `${bounds.left}px`;
      node.style.width = `${bounds.width}px`;
      node.style.height = `${bounds.height}px`;
    };

    position();
    const observer = content ? new ResizeObserver(position) : null;
    if (content) observer?.observe(content);
    window.addEventListener("resize", position);

    return {
      destroy() {
        observer?.disconnect();
        window.removeEventListener("resize", position);
        node.remove();
      },
    };
  }

  async function choose(version: string) {
    changing = version;
    try {
      await onchange(version);
      open = false;
    } finally {
      changing = "";
    }
  }
</script>

<button
  type="button"
  onclick={show}
  {disabled}
  class="mc-secondary flex h-10 w-full items-center justify-between gap-3 px-4 text-left disabled:cursor-not-allowed disabled:opacity-45"
  aria-haspopup="dialog"
>
  <span class="min-w-0">
    <span
      class="block text-[8px] font-bold uppercase tracking-[.13em] text-[var(--color-text-muted)]"
      >Change version</span
    >
    <span class="mt-0.5 block truncate text-[11px] font-bold text-white">Minecraft {value}</span>
  </span>
  <Icon name="chevron" size={11} />
</button>

{#if open}
  <div
    use:portalToLauncherContent
    class="version-picker-backdrop fixed z-[100] backdrop-blur-lg p-4"
    role="presentation"
  >
    <div
      class="version-dialog rounded-2xl flex size-full flex-col overflow-hidden bg-[var(--color-section-background)]"
      role="dialog"
      aria-modal="true"
      aria-labelledby="version-picker-title"
    >
      <header class="flex items-center justify-between border-b border-white/[0.08] px-6 py-5">
        <div>
          <p class="text-[9px] font-bold uppercase tracking-[.16em] text-[var(--color-primary)]">
            {loader === "fabric" ? "Fabric compatible" : "Official Vanilla"}
          </p>
          <h2 id="version-picker-title" class="mt-1 text-xl font-bold text-white">
            Choose a version
          </h2>
        </div>
        <button
          type="button"
          onclick={() => (open = false)}
          class="grid size-9 place-items-center text-[var(--color-text-muted)] transition hover:text-white"
          aria-label="Close"><Icon name="close" size={16} /></button
        >
      </header>

      <div class="overflow-y-auto p-5 sm:p-6">
        <div class="grid grid-cols-5 gap-3.5 xl:gap-4">
          {#each families as family, index}
            <article
              class="version-tile group relative aspect-[3/4] min-w-0 overflow-hidden border border-white/[0.10] transition duration-300 ease-out hover:-translate-y-1.5 hover:border-[var(--color-border-brand)] focus-within:border-[var(--color-primary)]"
              style={`--version-tile-column: ${index % 5}`}
            >
              <img
                src={artworkFor(family.name)}
                alt=""
                class="absolute inset-0 size-full object-cover transition duration-500 group-hover:scale-[1.04] group-focus-within:scale-[1.04]"
              />
              <div
                class="absolute inset-0 bg-gradient-to-t from-black/90 via-black/15 to-black/5"
              ></div>
              <div
                class="absolute inset-0 bg-black/55 opacity-0 transition-opacity duration-200 ease-out group-hover:opacity-100 group-focus-within:opacity-100"
              ></div>

              <div
                class="absolute inset-x-0 bottom-0 z-10 p-5 transition duration-200 group-hover:pointer-events-none group-hover:translate-y-2 group-hover:opacity-0 group-focus-within:pointer-events-none group-focus-within:translate-y-2 group-focus-within:opacity-0"
              >
                <span class="block text-lg font-black text-white"
                  >{loader === "fabric" ? "Fabric" : "Vanilla"} {family.name}</span
                >
                <span
                  class="mt-0.5 block text-[9px] font-semibold uppercase tracking-[.1em] text-white/60"
                  >{family.entries.length}
                  {family.entries.length === 1 ? "version" : "versions"}</span
                >
              </div>

              <div
                class="version-options pointer-events-none absolute inset-0 z-20 translate-y-3 overflow-y-auto overscroll-contain p-4 opacity-0 transition duration-200 group-hover:pointer-events-auto group-hover:translate-y-0 group-hover:opacity-100 group-focus-within:pointer-events-auto group-focus-within:translate-y-0 group-focus-within:opacity-100"
                class:grid-cols-2={family.entries.length > 8}
                class:content-center={family.entries.length <= 8}
                class:content-start={family.entries.length > 8}
              >
                {#each family.entries as version}
                  <button
                    type="button"
                    onclick={() => choose(version.minecraftVersion)}
                    disabled={Boolean(changing)}
                    class="rounded-3xl my-0.5 flex w-full items-center justify-between gap-2 border px-2.5 py-1.5 text-left text-[10px] font-bold transition disabled:opacity-50 {version.minecraftVersion ===
                    value
                      ? 'border-[var(--color-primary)] bg-[var(--color-primary)] text-[var(--color-primary-contrast)]'
                      : 'border-white/25 bg-[var(--color-surface-strong)]/90 text-white hover:border-[var(--color-primary)]'}"
                  >
                    <span class="truncate">{version.minecraftVersion}</span>
                    {#if changing === version.minecraftVersion}
                      <span
                        class="size-2.5 shrink-0 animate-spin rounded-full border border-current border-t-transparent"
                        aria-hidden="true"
                      ></span>
                    {:else if version.minecraftVersion === value}
                      <Icon name="check" size={10} />
                    {/if}
                  </button>
                {/each}
              </div>
            </article>
          {/each}
        </div>
        {#if families.length === 0}
          <div class="grid min-h-52 place-items-center text-sm text-[var(--color-text-muted)]">
            No compatible versions found.
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .version-picker-backdrop {
    animation: version-backdrop-enter 260ms ease-out both;
  }

  .version-tile {
    border-radius: 18px;
    box-shadow: 0 18px 44px rgb(0 4 15 / 0.3);
    animation: version-tile-enter 360ms calc(70ms + var(--version-tile-column, 0) * 36ms)
      cubic-bezier(0.16, 1, 0.3, 1) backwards;
  }

  .version-dialog {
    box-shadow: inset 0 1px 0 var(--glass-highlight);
    transform-origin: 50% 54%;
    animation: version-dialog-enter 420ms cubic-bezier(0.16, 1, 0.3, 1) both;
  }

  .version-options {
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--color-primary) 58%, transparent) transparent;
  }

  .version-options::-webkit-scrollbar {
    width: 5px;
  }

  .version-options::-webkit-scrollbar-thumb {
    border-radius: 2px;
    background: color-mix(in srgb, var(--color-primary) 58%, transparent);
  }

  @keyframes version-backdrop-enter {
    from {
      opacity: 0;
      -webkit-backdrop-filter: blur(0);
      backdrop-filter: blur(0);
    }
    to {
      opacity: 1;
    }
  }

  @keyframes version-dialog-enter {
    from {
      opacity: 0;
      transform: translate3d(0, 22px, 0) scale(0.965);
      filter: blur(7px);
    }
    to {
      opacity: 1;
      transform: none;
      filter: blur(0);
    }
  }

  @keyframes version-tile-enter {
    from {
      opacity: 0;
      clip-path: inset(10% 0 0 0 round 18px);
    }
    to {
      opacity: 1;
      clip-path: inset(0 0 0 0 round 18px);
    }
  }

  :global(.animations-disabled) .version-picker-backdrop,
  :global(.animations-disabled) .version-dialog,
  :global(.animations-disabled) .version-tile {
    animation: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .version-picker-backdrop,
    .version-dialog,
    .version-tile {
      animation: none;
    }
  }
</style>

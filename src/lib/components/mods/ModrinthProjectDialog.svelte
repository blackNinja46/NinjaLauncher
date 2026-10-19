<script lang="ts">
  import { browser } from "$app/environment";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import DOMPurify from "dompurify";
  import { marked } from "marked";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import type { ModrinthHit, ModrinthProject, ModrinthVersion } from "$lib/types";

  let {
    project,
    details = null,
    versions = [],
    versionsLoading = false,
    selectedVersionId = "",
    busy = false,
    readOnly = false,
    actionLabel = "Install",
    onaction,
    onversionchange,
    onloadversions,
    onclose,
  }: {
    project: ModrinthHit;
    details?: ModrinthProject | null;
    versions?: ModrinthVersion[];
    versionsLoading?: boolean;
    selectedVersionId?: string;
    busy?: boolean;
    readOnly?: boolean;
    actionLabel?: string;
    onaction: () => void | Promise<void>;
    onversionchange?: (versionId: string) => unknown;
    onloadversions?: () => unknown;
    onclose: () => void;
  } = $props();

  let tab = $state<"description" | "gallery" | "versions">("description");
  let selectedImage = $state(0);
  let gallery = $derived(details?.gallery ?? []);
  let selectedVersion = $derived(
    versions.find((version) => version.id === selectedVersionId) ?? null,
  );
  let renderedBody = $derived(renderMarkdown(details?.body || project.description));

  const tabs = $derived([
    { id: "description" as const, label: "Description", icon: "info" },
    {
      id: "gallery" as const,
      label: `Gallery${gallery.length ? ` (${gallery.length})` : ""}`,
      icon: "gallery",
    },
    {
      id: "versions" as const,
      label: `Versions${versions.length ? ` (${versions.length})` : ""}`,
      icon: "download",
    },
  ]);

  function renderMarkdown(source: string): string {
    if (!browser || !source.trim()) return "";
    const parsed = marked.parse(source, { gfm: true, breaks: false }) as string;
    return DOMPurify.sanitize(parsed, {
      USE_PROFILES: { html: true },
      ADD_ATTR: ["target", "rel"],
    });
  }

  function versionLabel(version: ModrinthVersion): string {
    const gameVersions = version.gameVersions.slice(0, 4).join(", ");
    const remainder = Math.max(0, version.gameVersions.length - 4);
    return `${version.name} · Minecraft ${gameVersions}${remainder ? ` +${remainder}` : ""}`;
  }

  function handleMarkdownClick(event: MouseEvent) {
    const target = event.target as HTMLElement;
    const anchor = target.closest<HTMLAnchorElement>("a[href]");
    if (!anchor) return;
    event.preventDefault();
    const url = anchor.href;
    if (/^https?:\/\//i.test(url)) void openUrl(url);
  }

  function externalLinks(node: HTMLElement) {
    node.addEventListener("click", handleMarkdownClick);
    return {
      destroy: () => node.removeEventListener("click", handleMarkdownClick),
    };
  }

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      },
    };
  }
</script>

<svelte:window onkeydown={(event) => event.key === "Escape" && onclose()} />

<div
  use:portal
  class="fixed inset-0 z-[160] grid place-items-center overflow-hidden rounded-[12px] bg-black/75 p-5 backdrop-blur-xl [.window-maximized_&]:rounded-none"
  role="presentation"
  onclick={(event) => event.currentTarget === event.target && onclose()}
>
  <div
    class="project-dialog grid h-[min(90vh,860px)] w-full max-w-6xl grid-cols-[minmax(0,1fr)_290px] overflow-hidden rounded-[12px] border border-[var(--color-border-default)] bg-[var(--color-navy-900)] shadow-[0_32px_100px_rgba(0,0,0,.72)]"
    role="dialog"
    aria-modal="true"
    aria-labelledby="modrinth-project-title"
  >
    <div class="grid min-h-0 min-w-0 grid-rows-[auto_auto_minmax(0,1fr)]">
      <header
        class="flex items-start gap-4 border-b border-[var(--color-border-subtle)] bg-[var(--color-navy-850)] p-6"
      >
        {#if project.iconUrl}
          <img
            src={project.iconUrl}
            alt=""
            class="size-20 shrink-0 rounded-[10px] object-cover shadow-lg"
          />
        {:else}
          <span
            class="grid size-20 shrink-0 place-items-center rounded-[10px] bg-[var(--color-navy-700)] text-2xl font-black"
          >
            {project.title[0]}
          </span>
        {/if}
        <div class="min-w-0 flex-1">
          <p class="text-[9px] font-bold uppercase tracking-[.15em] text-[var(--color-primary)]">
            Modrinth · {project.projectType}
          </p>
          <h2
            id="modrinth-project-title"
            class="mt-1 truncate text-2xl font-black text-[var(--color-text-primary)]"
          >
            {project.title}
          </h2>
          <p class="mt-1 text-xs text-[var(--color-text-muted)]">by {project.author}</p>
          <p
            class="mt-3 line-clamp-2 max-w-3xl text-xs leading-5 text-[var(--color-text-secondary)]"
          >
            {details?.description || project.description}
          </p>
        </div>
        <button
          type="button"
          onclick={onclose}
          class="grid size-9 shrink-0 place-items-center rounded-[7px] border border-[var(--color-border-default)] bg-[var(--color-navy-800)] text-[var(--color-text-muted)] transition hover:border-[var(--color-border-brand)] hover:text-[var(--color-text-primary)]"
          aria-label="Close"><Icon name="close" size={15} /></button
        >
      </header>

      <nav
        class="flex gap-2 border-b border-[var(--color-border-subtle)] bg-[var(--color-navy-850)] px-6 py-3"
        aria-label="Project details"
      >
        {#each tabs as item}
          <button
            type="button"
            onclick={() => {
              tab = item.id;
              if (item.id === "versions") void onloadversions?.();
            }}
            class="flex items-center gap-2 rounded-[7px] border px-4 py-2.5 text-xs font-bold transition {tab ===
            item.id
              ? 'border-[var(--color-border-brand)] bg-[var(--color-primary)] text-white shadow-[0_7px_18px_var(--color-brand-glow)]'
              : 'border-[var(--color-border-default)] bg-[var(--color-navy-800)] text-[var(--color-text-secondary)] hover:border-[var(--color-border-strong)] hover:bg-[var(--color-navy-700)] hover:text-[var(--color-text-primary)]'}"
          >
            <Icon name={item.icon} size={12} />{item.label}
          </button>
        {/each}
      </nav>

      <main class="project-content min-h-0 overflow-y-auto overscroll-contain p-7">
        {#if tab === "description"}
          {#if renderedBody}
            <article class="modrinth-markdown mx-auto max-w-4xl" use:externalLinks>
              {@html renderedBody}
            </article>
          {:else}
            <div class="grid min-h-64 place-items-center text-sm text-[var(--color-text-muted)]">
              {details ? "No description was provided for this project." : "Loading description…"}
            </div>
          {/if}
        {:else if tab === "gallery"}
          {#if gallery.length}
            <div class="mx-auto max-w-4xl">
              <figure
                class="overflow-hidden rounded-[10px] border border-[var(--color-border-default)] bg-[var(--color-navy-1000)]"
              >
                <div class="grid min-h-[380px] place-items-center p-3">
                  <img
                    src={gallery[selectedImage]?.url}
                    alt={gallery[selectedImage]?.title ?? "Project gallery"}
                    class="max-h-[480px] w-full rounded-[7px] object-contain"
                  />
                </div>
                {#if gallery[selectedImage]?.title || gallery[selectedImage]?.description}
                  <figcaption
                    class="border-t border-[var(--color-border-subtle)] bg-[var(--color-navy-850)] px-5 py-4"
                  >
                    {#if gallery[selectedImage]?.title}<p
                        class="text-sm font-bold text-[var(--color-text-primary)]"
                      >
                        {gallery[selectedImage]?.title}
                      </p>{/if}
                    {#if gallery[selectedImage]?.description}<p
                        class="mt-1 text-xs leading-5 text-[var(--color-text-muted)]"
                      >
                        {gallery[selectedImage]?.description}
                      </p>{/if}
                  </figcaption>
                {/if}
              </figure>
              <div class="mt-4 grid grid-cols-3 gap-3 xl:grid-cols-4">
                {#each gallery as image, index}
                  <button
                    type="button"
                    onclick={() => {
                      selectedImage = index;
                      document
                        .querySelector(".project-content")
                        ?.scrollTo({ top: 0, behavior: "smooth" });
                    }}
                    class="group overflow-hidden rounded-[8px] border bg-[var(--color-navy-850)] p-1.5 text-left transition {selectedImage ===
                    index
                      ? 'border-[var(--color-primary)] shadow-[0_0_0_2px_var(--color-brand-soft)]'
                      : 'border-[var(--color-border-default)] hover:border-[var(--color-border-strong)]'}"
                    aria-label={`Show image ${index + 1}`}
                  >
                    <img
                      src={image.url}
                      alt=""
                      class="aspect-video w-full rounded-[5px] object-cover transition duration-300 group-hover:scale-[1.025]"
                    />
                    {#if image.title}<span
                        class="mt-2 block truncate px-1 pb-1 text-[10px] font-semibold text-[var(--color-text-secondary)]"
                        >{image.title}</span
                      >{/if}
                  </button>
                {/each}
              </div>
            </div>
          {:else}
            <div class="grid min-h-64 place-items-center text-center">
              <div>
                <span
                  class="mx-auto grid size-12 place-items-center rounded-[9px] bg-[var(--color-navy-800)] text-[var(--color-text-muted)]"
                  ><Icon name="gallery" size={20} /></span
                >
                <p class="mt-3 text-sm font-semibold text-[var(--color-text-secondary)]">
                  No gallery images
                </p>
                <p class="mt-1 text-xs text-[var(--color-text-muted)]">
                  The project author has not uploaded screenshots.
                </p>
              </div>
            </div>
          {/if}
        {:else}
          <div class="mx-auto max-w-4xl">
            <div class="mb-5 flex items-end justify-between gap-4">
              <div>
                <h3 class="text-base font-bold text-[var(--color-text-primary)]">
                  Compatible versions
                </h3>
                <p class="mt-1 text-xs text-[var(--color-text-muted)]">
                  {readOnly
                    ? "This version is controlled by the installed modpack."
                    : "Choose the exact project version used by this profile."}
                </p>
              </div>
              <span class="text-[10px] text-[var(--color-text-faint)]"
                >{versions.length} available</span
              >
            </div>
            {#if versionsLoading}
              <div class="grid min-h-48 place-items-center">
                <span
                  class="size-6 animate-spin rounded-full border-2 border-white/10 border-t-[var(--color-primary)]"
                ></span>
              </div>
            {:else if versions.length}
              <div class="space-y-2">
                {#each versions as version}
                  <button
                    type="button"
                    onclick={() => !readOnly && onversionchange?.(version.id)}
                    disabled={readOnly}
                    class="flex w-full items-center gap-4 rounded-[8px] border px-4 py-3.5 text-left transition {selectedVersionId ===
                    version.id
                      ? 'border-[var(--color-border-brand)] bg-[var(--color-brand-soft)]'
                      : 'border-[var(--color-border-default)] bg-[var(--color-navy-850)] hover:border-[var(--color-border-strong)] hover:bg-[var(--color-navy-800)]'} disabled:cursor-not-allowed disabled:opacity-70"
                  >
                    <span
                      class="grid size-5 shrink-0 place-items-center rounded-full border {selectedVersionId ===
                      version.id
                        ? 'border-[var(--color-primary)] bg-[var(--color-primary)] text-white'
                        : 'border-[var(--color-border-strong)] text-transparent'}"
                      ><Icon name="check" size={9} /></span
                    >
                    <span class="min-w-0 flex-1">
                      <span
                        class="block truncate text-xs font-bold text-[var(--color-text-primary)]"
                        >{version.name}</span
                      >
                      <span class="mt-1 block truncate text-[10px] text-[var(--color-text-muted)]"
                        >Minecraft {version.gameVersions.join(" · ")}</span
                      >
                    </span>
                    <span
                      class="rounded-[5px] bg-[var(--color-navy-700)] px-2 py-1 text-[9px] font-bold uppercase text-[var(--color-text-secondary)]"
                      >{version.versionType}</span
                    >
                  </button>
                {/each}
              </div>
            {:else}
              <div
                class="rounded-[9px] border border-dashed border-[var(--color-border-default)] bg-[var(--color-navy-850)] p-10 text-center text-sm text-[var(--color-text-muted)]"
              >
                No compatible version is available for this profile.
              </div>
            {/if}
          </div>
        {/if}
      </main>
    </div>

    <aside
      class="min-h-0 overflow-y-auto border-l border-[var(--color-border-subtle)] bg-[var(--color-navy-850)] p-5"
    >
      <p class="text-[9px] font-bold uppercase tracking-[.13em] text-[var(--color-text-muted)]">
        Version to use
      </p>
      <CustomSelect
        value={selectedVersionId}
        options={[
          { value: "", label: versionsLoading ? "Loading versions…" : "Latest compatible" },
          ...versions.map((version) => ({ value: version.id, label: versionLabel(version) })),
        ]}
        onopen={() => onloadversions?.()}
        onchange={(value) => onversionchange?.(String(value))}
        disabled={readOnly || busy || versionsLoading}
        ariaLabel={`Version of ${project.title}`}
        class="mt-2 w-full text-xs"
      />
      {#if selectedVersion}
        <p class="mt-2 text-[10px] leading-4 text-[var(--color-text-muted)]">
          Selected for this installation: {selectedVersion.name}
        </p>
      {/if}
      <button
        type="button"
        onclick={onaction}
        disabled={readOnly || busy}
        class="mc-primary mt-4 flex w-full items-center justify-center gap-2 px-4 py-3 text-xs font-bold disabled:opacity-50"
        ><Icon name={readOnly ? "lock" : "download"} size={13} />{busy
          ? "Working…"
          : actionLabel}</button
      >

      <div class="mt-6 space-y-5 border-t border-[var(--color-border-subtle)] pt-5 text-xs">
        <div class="grid grid-cols-2 gap-2">
          <div class="rounded-[7px] bg-[var(--color-navy-800)] p-3">
            <p class="text-[8px] font-bold uppercase tracking-wider text-[var(--color-text-muted)]">
              Downloads
            </p>
            <p class="mt-1 font-bold text-[var(--color-text-primary)]">
              {Intl.NumberFormat().format(details?.downloads ?? project.downloads)}
            </p>
          </div>
          <div class="rounded-[7px] bg-[var(--color-navy-800)] p-3">
            <p class="text-[8px] font-bold uppercase tracking-wider text-[var(--color-text-muted)]">
              Followers
            </p>
            <p class="mt-1 font-bold text-[var(--color-text-primary)]">
              {Intl.NumberFormat().format(details?.followers ?? 0)}
            </p>
          </div>
        </div>
        <div>
          <p class="text-[9px] font-bold uppercase tracking-wider text-[var(--color-text-muted)]">
            Categories
          </p>
          <div class="mt-2 flex flex-wrap gap-1.5">
            {#each details?.categories ?? project.categories as category}<span
                class="rounded-[5px] bg-[var(--color-navy-700)] px-2 py-1 text-[9px] text-[var(--color-text-secondary)]"
                >{category}</span
              >{/each}
          </div>
        </div>
        {#if details?.updated}<div>
            <p class="text-[9px] font-bold uppercase tracking-wider text-[var(--color-text-muted)]">
              Last updated
            </p>
            <p class="mt-1 text-[var(--color-text-secondary)]">
              {new Date(details.updated).toLocaleDateString()}
            </p>
          </div>{/if}
      </div>
    </aside>
  </div>
</div>

<style>
  .project-dialog {
    animation: project-dialog-in 260ms cubic-bezier(0.16, 1, 0.3, 1) both;
  }
  .project-content {
    scrollbar-width: thin;
    scrollbar-color: var(--color-navy-600) transparent;
  }

  :global(.modrinth-markdown) {
    color: var(--color-text-secondary);
    font-size: 0.875rem;
    line-height: 1.75;
    overflow-wrap: anywhere;
  }
  :global(.modrinth-markdown h1),
  :global(.modrinth-markdown h2),
  :global(.modrinth-markdown h3),
  :global(.modrinth-markdown h4) {
    color: var(--color-text-primary);
    font-weight: 800;
    line-height: 1.3;
    margin: 1.7em 0 0.65em;
  }
  :global(.modrinth-markdown h1) {
    border-bottom: 1px solid var(--color-border-default);
    font-size: 1.65rem;
    padding-bottom: 0.45em;
  }
  :global(.modrinth-markdown h2) {
    border-bottom: 1px solid var(--color-border-subtle);
    font-size: 1.35rem;
    padding-bottom: 0.4em;
  }
  :global(.modrinth-markdown h3) {
    font-size: 1.1rem;
  }
  :global(.modrinth-markdown h4) {
    font-size: 0.95rem;
  }
  :global(.modrinth-markdown h1:first-child),
  :global(.modrinth-markdown h2:first-child),
  :global(.modrinth-markdown h3:first-child) {
    margin-top: 0;
  }
  :global(.modrinth-markdown p),
  :global(.modrinth-markdown ul),
  :global(.modrinth-markdown ol),
  :global(.modrinth-markdown pre),
  :global(.modrinth-markdown blockquote),
  :global(.modrinth-markdown table) {
    margin: 0 0 1.15em;
  }
  :global(.modrinth-markdown ul) {
    list-style: disc;
    padding-left: 1.5rem;
  }
  :global(.modrinth-markdown ol) {
    list-style: decimal;
    padding-left: 1.5rem;
  }
  :global(.modrinth-markdown li) {
    margin: 0.3rem 0;
  }
  :global(.modrinth-markdown a) {
    color: var(--color-primary);
    font-weight: 600;
    text-decoration: underline;
    text-decoration-color: color-mix(in srgb, var(--color-primary) 45%, transparent);
    text-underline-offset: 3px;
  }
  :global(.modrinth-markdown strong) {
    color: var(--color-text-primary);
    font-weight: 750;
  }
  :global(.modrinth-markdown blockquote) {
    border-left: 3px solid var(--color-primary);
    background: var(--color-surface-subtle);
    border-radius: 0 7px 7px 0;
    color: var(--color-text-muted);
    padding: 0.8rem 1rem;
  }
  :global(.modrinth-markdown code) {
    border: 1px solid var(--color-border-subtle);
    border-radius: 4px;
    background: var(--color-navy-800);
    color: var(--color-text-primary);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.82em;
    padding: 0.15rem 0.35rem;
  }
  :global(.modrinth-markdown pre) {
    overflow-x: auto;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    background: var(--color-navy-1000);
    padding: 1rem;
  }
  :global(.modrinth-markdown pre code) {
    border: 0;
    background: transparent;
    padding: 0;
  }
  :global(.modrinth-markdown img) {
    display: block;
    max-height: 540px;
    max-width: 100%;
    margin: 1.25rem auto;
    border: 1px solid var(--color-border-default);
    border-radius: 9px;
    object-fit: contain;
  }
  :global(.modrinth-markdown table) {
    display: block;
    width: 100%;
    overflow-x: auto;
    border-collapse: collapse;
  }
  :global(.modrinth-markdown th),
  :global(.modrinth-markdown td) {
    border: 1px solid var(--color-border-default);
    padding: 0.55rem 0.75rem;
    text-align: left;
  }
  :global(.modrinth-markdown th) {
    background: var(--color-navy-800);
    color: var(--color-text-primary);
  }
  :global(.modrinth-markdown hr) {
    margin: 1.75rem 0;
    border: 0;
    border-top: 1px solid var(--color-border-default);
  }

  @keyframes project-dialog-in {
    from {
      opacity: 0;
      transform: translateY(12px) scale(0.985);
    }
  }
  :global(.animations-disabled) .project-dialog {
    animation: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .project-dialog {
      animation: none;
    }
  }
</style>

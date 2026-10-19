<script lang="ts">
  import { onMount } from "svelte";
  import PlayerSkin from "$lib/components/account/PlayerSkin.svelte";
  import { appearances } from "$lib/components/account/appearance.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import { messageFromError } from "$lib/api/tauri";
  import {
    listSkins,
    saveSkin,
    deleteSkin,
    importSkin,
    readSkinFile,
    getSkinProfile,
    applySkin,
    type SavedSkin,
    type SkinProfile,
    type SkinModel,
    type ImportedSkin,
  } from "$lib/api/skins";
  import type { Account } from "$lib/types";

  let {
    account,
    onnotify,
  }: {
    account: Account | null;
    onnotify: (notification: {
      title: string;
      message: string;
      tone: "info" | "success" | "warning" | "error";
    }) => void;
  } = $props();
  let skins = $state<SavedSkin[]>([]);
  let loading = $state(true);
  let error = $state("");
  let notice = $state("");
  let profile = $state<SkinProfile | null>(null);
  let profileLoading = $state(false);
  let profileError = $state("");
  let currentTexture = $state("");
  let applying = $state<string | null>(null);
  let deleting = $state<string | null>(null);
  let deleteBusy = $state(false);
  let page = $state(0);
  const pageSize = 6;
  const visibleSkins = $derived(skins.slice(page * pageSize, (page + 1) * pageSize));
  const pageCount = $derived(Math.max(1, Math.ceil(skins.length / pageSize)));
  let dialog: HTMLDialogElement;
  let fileInput = $state<HTMLInputElement>();
  let editorOpen = $state(false);
  let editingId = $state<string | null>(null);
  let source = $state<"file" | "url" | "uuid" | "name">("file");
  let value = $state("");
  let name = $state("");
  let texture = $state("");
  let model = $state<SkinModel>("classic");
  let capeId = $state<string | null>(null);
  let importing = $state(false);
  let saving = $state(false);
  let editorError = $state("");
  let importGeneration = 0;
  let lastNotification = $state("");
  let alive = true;
  const sources = [
    { id: "file" as const, label: "Datei" },
    { id: "url" as const, label: "URL" },
    { id: "uuid" as const, label: "UUID" },
    { id: "name" as const, label: "Spielername" },
  ];
  const capes = $derived(profile?.capes ?? []);
  const selectedCape = $derived(capes.find((cape) => cape.id === capeId));
  const unavailableCape = $derived(Boolean(capeId && !selectedCape));
  const secureUrl = (url: string) =>
    url.replace(/^http:\/\/textures\.minecraft\.net\//, "https://textures.minecraft.net/");

  $effect(() => {
    const current = editorError
      ? { title: "Skin konnte nicht geändert werden", message: editorError, tone: "error" as const }
      : error
        ? { title: "Skin-Fehler", message: error, tone: "error" as const }
        : notice
          ? { title: "Skin aktualisiert", message: notice, tone: "success" as const }
          : profileError
            ? {
                title: "Minecraft-Profil nicht verfügbar",
                message: profileError,
                tone: "warning" as const,
              }
            : null;
    const key = current ? `${current.tone}:${current.title}:${current.message}` : "";
    if (current && key !== lastNotification) onnotify(current);
    lastNotification = key;
  });

  async function refreshProfile() {
    if (!account || profileLoading) return;
    const uuid = account.uuid;
    profileLoading = true;
    profileError = "";
    try {
      const result = await getSkinProfile(uuid);
      if (!alive) return;
      profile = result;
      const active = result.skins.find((skin) => skin.state === "ACTIVE");
      if (active) {
        const imported = await importSkin("url", secureUrl(active.url));
        if (!alive) return;
        currentTexture = imported.texture;
        appearances[uuid] = {
          texture: imported.texture,
          model: active.variant === "SLIM" ? "slim" : "classic",
          cape: result.capes.find((cape) => cape.state === "ACTIVE")?.url ?? null,
        };
      }
    } catch (reason) {
      if (alive) profileError = messageFromError(reason);
    } finally {
      if (alive) profileLoading = false;
    }
  }

  onMount(() => {
    void listSkins()
      .then((result) => {
        if (alive) skins = result;
      })
      .catch((reason) => {
        if (alive) error = messageFromError(reason);
      })
      .finally(() => {
        if (alive) loading = false;
      });
    void refreshProfile();
    return () => {
      alive = false;
      importGeneration++;
    };
  });

  function openEditor(skin?: SavedSkin) {
    editingId = skin?.id ?? null;
    source = "file";
    value = "";
    name = skin?.name ?? "";
    texture = skin?.texture ?? "";
    model = skin?.model ?? "classic";
    capeId = skin?.capeId ?? null;
    if (skin?.capeId && skin.capeAccountUuid !== account?.uuid) capeId = null;
    editorError =
      skin?.capeId && skin.capeAccountUuid !== account?.uuid
        ? "Das gespeicherte Cape gehört zu einem anderen Konto. Wähle bei Bedarf ein eigenes Cape."
        : "";
    editorOpen = true;
    dialog.showModal();
  }

  function closeEditor() {
    if (saving) return;
    importGeneration++;
    importing = false;
    editorOpen = false;
    dialog.close();
  }

  function changeSource(next: typeof source) {
    importGeneration++;
    importing = false;
    source = next;
    value = "";
    editorError = "";
    if (fileInput) fileInput.value = "";
  }

  async function loadImport(file?: File) {
    if (source === "file" && !file) return;
    const generation = ++importGeneration;
    importing = true;
    editorError = "";
    try {
      const imported: ImportedSkin = file
        ? await readSkinFile(file)
        : await importSkin(source as "url" | "uuid" | "name", value.trim());
      if (!alive || generation !== importGeneration) return;
      texture = imported.texture;
      model = imported.model;
      if (!name.trim()) name = imported.name;
    } catch (reason) {
      if (alive && generation === importGeneration) editorError = messageFromError(reason);
    } finally {
      if (alive && generation === importGeneration) importing = false;
    }
  }

  async function save() {
    if (!texture || !name.trim() || saving || importing || unavailableCape) return;
    saving = true;
    editorError = "";
    try {
      const saved = await saveSkin({
        id: editingId ?? crypto.randomUUID(),
        name: name.trim(),
        texture,
        model,
        capeId,
        capeAccountUuid: capeId ? (account?.uuid ?? null) : null,
      });
      if (!alive) return;
      skins = [...skins.filter((skin) => skin.id !== saved.id), saved].sort((a, b) =>
        a.name.localeCompare(b.name),
      );
      page = Math.floor(skins.findIndex((skin) => skin.id === saved.id) / pageSize);
      notice = `„${saved.name}“ wurde gespeichert.`;
      saving = false;
      closeEditor();
    } catch (reason) {
      if (alive) editorError = messageFromError(reason);
    } finally {
      if (alive) saving = false;
    }
  }

  function isActive(skin: SavedSkin) {
    const active = profile?.skins.find((item) => item.state === "ACTIVE");
    return Boolean(
      account &&
      currentTexture &&
      currentTexture === skin.texture &&
      (active?.variant === "SLIM" ? "slim" : "classic") === skin.model &&
      (profile?.capes.find((cape) => cape.state === "ACTIVE")?.id ?? null) === skin.capeId,
    );
  }

  async function wear(skin: SavedSkin) {
    if (!account || applying) return;
    const uuid = account.uuid;
    const previousCape = profile?.capes.find((cape) => cape.state === "ACTIVE");
    const nextCape = capes.find((cape) => cape.id === skin.capeId);
    applying = skin.id;
    error = "";
    notice = "";
    try {
      const result = await applySkin(skin.id, uuid);
      appearances[uuid] = {
        texture: skin.texture,
        model: skin.model,
        cape: (result.warning ? previousCape : nextCape)?.url ?? null,
      };
      if (!alive) return;
      currentTexture = skin.texture;
      const oldCape = profile?.capes.find((cape) => cape.state === "ACTIVE");
      if (profile) {
        profile = {
          ...profile,
          skins: [{ url: "", state: "ACTIVE", variant: skin.model.toUpperCase() }],
          capes: profile.capes.map((cape) => ({
            ...cape,
            state: (result.warning ? oldCape?.id : skin.capeId) === cape.id ? "ACTIVE" : "INACTIVE",
          })),
        };
      }
      if (result.warning) error = result.warning;
      else
        notice = `„${skin.name}“ ist jetzt auf ${account.username} aktiv. Bereits laufendes Minecraft gegebenenfalls neu starten.`;
    } catch (reason) {
      if (alive) error = messageFromError(reason);
    } finally {
      if (alive) applying = null;
    }
  }

  async function remove(skin: SavedSkin) {
    deleteBusy = true;
    error = "";
    try {
      await deleteSkin(skin.id);
      if (!alive) return;
      skins = skins.filter((item) => item.id !== skin.id);
      page = Math.min(page, Math.max(0, Math.ceil(skins.length / pageSize) - 1));
      deleting = null;
      notice = `„${skin.name}“ wurde aus der Sammlung entfernt. Dein Minecraft-Skin bleibt erhalten.`;
    } catch (reason) {
      if (alive) error = messageFromError(reason);
    } finally {
      if (alive) deleteBusy = false;
    }
  }
</script>

<section>
  <header class="mb-6 flex flex-wrap items-end justify-between gap-4">
    <div>
      <div class="text-xs font-extrabold tracking-[.16em] text-[var(--color-brand-sky)]">
        DEIN LOOK. DEIN MINECRAFT.
      </div>
      <h1 class="block-title mt-1 text-[28px] text-white">Skins</h1>
      <p class="mt-2 text-sm text-[var(--color-text-muted)]">
        Deine Sammlung – mit eigenen Skins und den Capes deines Kontos.
      </p>
    </div>
    <div class="flex items-center gap-2">
      {#if profileError}<button
          type="button"
          class="mc-secondary px-3 py-[9px] text-xs text-[var(--color-text-secondary)]"
          disabled={profileLoading}
          onclick={refreshProfile}>Profil neu laden</button
        >{/if}
      <span
        class="flex items-center gap-2 rounded-[5px] border border-[var(--color-border-default)] bg-[var(--color-surface-subtle)] px-3 py-[9px] text-sm"
        ><Icon name="skins" size={14} />{account?.username ?? "Lokale Sammlung"}</span
      >
    </div>
  </header>
  {#if !account}<div
      class="mb-5 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-subtle)] px-[15px] py-3 text-sm leading-relaxed text-[var(--color-text-secondary)]"
    >
      Du kannst Skins lokal sammeln und bearbeiten. Melde dich mit Microsoft an, um sie zu verwenden
      und deine Capes auszuwählen.
    </div>{/if}
  {#if loading}<div
      class="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-subtle)] px-[15px] py-3 text-sm leading-relaxed text-[var(--color-text-secondary)]"
      role="status"
    >
      Skin-Sammlung wird geladen …
    </div>
  {:else}
    <div class="grid grid-cols-[repeat(auto-fill,minmax(210px,1fr))] gap-4">
      <button
        class="relative flex min-h-[354px] min-w-0 flex-col items-center justify-center overflow-hidden rounded-[7px] border border-dashed border-[var(--color-border-default)] bg-[var(--color-section-background)] p-[18px] transition-colors hover:border-[var(--color-border-brand)] hover:bg-[var(--color-surface-hover)]"
        onclick={() => openEditor()}
        disabled={Boolean(applying)}
      >
        <div class="pointer-events-none relative h-[260px] w-full isolate" aria-hidden="true">
          <div
            class="pointer-events-none absolute inset-y-0 -inset-x-[18px] -z-10 opacity-50 grayscale brightness-[.65]"
          >
            <PlayerSkin
              account={null}
              rank={null}
              texture="https://mc-heads.net/skin/BlackNinja46"
              previewCape={null}
              showNameTag={false}
              animation="still"
              compact
              interactive={false}
              showEffects={false}
            />
          </div>
          <span
            class="absolute left-1/2 top-[41%] grid size-[42px] -translate-x-1/2 place-items-center rounded-full bg-[var(--color-text-primary)] text-[var(--color-bg-deep)] shadow-[0_0_24px_rgba(255,255,255,.04)]"
            ><Icon name="plus" size={24} /></span
          >
        </div>
        <strong class="text-base text-[var(--color-text-primary)]">Neuen Skin hinzufügen</strong
        ><span
          class="mt-[7px] flex items-center justify-center gap-1.5 text-[13px] text-[var(--color-text-secondary)]"
          ><Icon name="upload" size={12} /> Skin hochladen oder importieren</span
        >
      </button>
      {#each visibleSkins as skin (skin.id)}
        <article
          class="relative min-h-[354px] min-w-0 overflow-hidden rounded-[7px] border bg-[var(--color-section-background)] {isActive(
            skin,
          )
            ? 'border-[var(--color-border-brand)] shadow-[inset_0_0_28px_var(--color-brand-soft)]'
            : 'border-[var(--color-border-default)]'}"
        >
          <div class="absolute right-2.5 top-2.5 z-30 flex flex-col gap-[5px]">
            <button
              title="Skin bearbeiten"
              aria-label={`${skin.name} bearbeiten`}
              disabled={Boolean(applying)}
              class="border border-[var(--color-border-default)] bg-[var(--color-surface-field)] px-2 py-1 text-[var(--color-text-secondary)] hover:bg-[var(--color-surface-hover)] hover:text-white disabled:cursor-default disabled:opacity-45"
              onclick={() => openEditor(skin)}><Icon name="edit" size={13} /></button
            ><button
              title="Skin löschen"
              aria-label={`${skin.name} löschen`}
              disabled={Boolean(applying)}
              class="border border-[var(--color-border-default)] bg-[var(--color-surface-field)] px-2 py-1 text-[var(--color-text-secondary)] hover:bg-[var(--color-surface-hover)] hover:text-white disabled:cursor-default disabled:opacity-45"
              onclick={() => (deleting = skin.id)}><Icon name="trash" size={13} /></button
            >
          </div>
          {#if isActive(skin)}<span
              class="absolute left-3 top-[13px] z-30 flex items-center gap-[5px] text-[13px] text-[var(--color-primary)]"
              ><Icon name="check" size={11} /> Aktiv</span
            >{/if}
          <div class="h-[260px]">
            <PlayerSkin
              account={null}
              rank={null}
              showNameTag={false}
              texture={skin.texture}
              model={skin.model}
              previewCape={capes.find((cape) => cape.id === skin.capeId)?.url ?? null}
              animation="still"
              compact
            />
          </div>
          <div class="px-[18px] pb-4 text-center">
            <strong
              class="block overflow-hidden text-ellipsis whitespace-nowrap text-base text-[var(--color-text-primary)]"
              title={skin.name}>{skin.name}</strong
            ><span
              class="mt-[7px] flex items-center justify-center gap-1.5 text-[13px] text-[var(--color-text-secondary)]"
              ><Icon name="skins" size={12} />{skin.model === "slim"
                ? "Schlank"
                : "Klassisch"}{skin.capeId ? " · Cape" : ""}</span
            >
            <button
              class="mt-[13px] w-full rounded border border-[var(--color-border-default)] bg-[var(--color-surface-subtle)] p-[7px] text-sm text-[var(--color-text-secondary)] hover:enabled:border-[var(--color-border-brand)] hover:enabled:bg-[var(--color-brand-soft)] disabled:cursor-default disabled:opacity-45"
              disabled={!account ||
                Boolean(applying) ||
                profileLoading ||
                !profile ||
                isActive(skin)}
              onclick={() => wear(skin)}
              >{applying === skin.id
                ? "Wird angewendet …"
                : isActive(skin)
                  ? "Ausgewählt"
                  : "Verwenden"}</button
            >
          </div>
          {#if deleting === skin.id}<div
              class="absolute inset-0 z-40 flex flex-col justify-center gap-3 bg-[var(--color-surface-strong)] p-6 text-center text-sm"
            >
              <strong class="text-base text-[var(--color-text-primary)]"
                >„{skin.name}“ entfernen?</strong
              >
              <p class="text-[var(--color-text-secondary)]">Nur aus deiner lokalen Sammlung.</p>
              <button
                class="mc-primary px-4 py-2"
                disabled={deleteBusy}
                onclick={() => remove(skin)}>{deleteBusy ? "Wird entfernt …" : "Entfernen"}</button
              ><button
                class="mc-secondary px-4 py-2"
                disabled={deleteBusy}
                onclick={() => (deleting = null)}>Abbrechen</button
              >
            </div>{/if}
        </article>
      {/each}
    </div>
    {#if pageCount > 1}<div class="mt-5 flex items-center justify-end gap-4 text-sm">
        <button class="mc-secondary px-3 py-2" disabled={page === 0} onclick={() => page--}
          >Zurück</button
        ><span>{page + 1} / {pageCount}</span><button
          class="mc-secondary px-3 py-2"
          disabled={page + 1 >= pageCount}
          onclick={() => page++}>Weiter</button
        >
      </div>{/if}
  {/if}
</section>

<dialog
  bind:this={dialog}
  class="m-auto hidden h-[min(740px,calc(100dvh-32px))] max-h-[calc(100dvh-32px)] w-[min(920px,calc(100vw-32px))] flex-col overflow-hidden rounded-[10px] border-0 bg-[var(--color-surface-strong)] p-0 text-[var(--color-text-primary)] shadow-[0_35px_120px_rgba(0,0,0,.6)] open:flex [&::backdrop]:bg-[var(--color-overlay)] [&::backdrop]:backdrop-blur-[2px]"
  oncancel={(event) => {
    event.preventDefault();
    closeEditor();
  }}
>
  {#if editorOpen}
    <header
      class="flex items-center justify-between border-b border-[var(--color-border-subtle)] px-5 py-3.5"
    >
      <div>
        <div class="text-xs font-extrabold tracking-[.16em] text-[var(--color-brand-sky)]">
          SKIN-STUDIO
        </div>
        <h2 class="block-title mt-1 text-xl">
          {editingId ? "Skin bearbeiten" : "Neuen Skin hinzufügen"}
        </h2>
      </div>
      <button
        class="mc-secondary px-2 py-1"
        aria-label="Editor schließen"
        disabled={saving}
        onclick={closeEditor}><Icon name="close" size={16} /></button
      >
    </header>
    <form
      class="flex min-h-0 flex-1 flex-col"
      onsubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <div
        class="grid min-h-0 flex-1 grid-cols-[minmax(300px,.85fr)_minmax(380px,1fr)] max-[700px]:h-auto max-[700px]:grid-cols-1"
      >
        <div
          class="grid min-h-0 grid-rows-[minmax(250px,1fr)_auto] border-r border-[var(--color-border-subtle)] max-[700px]:border-r-0"
        >
          <div
            class="relative min-h-0 bg-[radial-gradient(ellipse_at_50%_55%,var(--color-surface-subtle),transparent_65%),var(--color-surface-field)] max-[700px]:h-80 max-[700px]:min-h-80"
          >
            {#if texture}<PlayerSkin
                account={null}
                rank={null}
                showNameTag={false}
                {texture}
                {model}
                previewCape={selectedCape ? secureUrl(selectedCape.url) : null}
                animation="wardrobe"
                compact
              />{:else}<div
                class="flex h-full min-h-0 flex-col items-center justify-center gap-5 text-[var(--color-text-muted)]"
              >
                <Icon name="skins" size={60} /><strong
                  class="text-[17px] text-[var(--color-text-primary)]">Dein nächster Look</strong
                >
                <p class="text-center text-sm leading-[1.8]">
                  Wähle einen Skin aus,<br />um ihn hier in 3D anzusehen.
                </p>
              </div>{/if}<span
              class="pointer-events-none absolute inset-x-2 bottom-4 text-center text-xs text-[var(--color-text-muted)]"
              >{texture ? "Ziehen zum Drehen · Cape auf der Rückseite" : "3D-Vorschau"}</span
            >
          </div>
        </div>
        <div class="flex min-h-0 flex-col gap-3 p-[18px] overflow-y-auto">
          <fieldset class="min-w-0" disabled={saving}>
            <legend
              class="mb-[9px] block w-full text-sm font-semibold text-[var(--color-text-secondary)]"
              >Skin importieren</legend
            >
            <div
              class="mb-2.5 grid grid-cols-4 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-field)] p-[3px]"
            >
              {#each sources as item}<button
                  type="button"
                  class="rounded-[3px] border px-1 py-2 text-[13px] {source === item.id
                    ? 'border-[var(--color-border-brand)] bg-[var(--color-brand-soft)] text-[var(--color-text-primary)]'
                    : 'border-transparent text-[var(--color-text-secondary)]'}"
                  aria-pressed={source === item.id}
                  onclick={() => changeSource(item.id)}>{item.label}</button
                >{/each}
            </div>
            {#if source === "file"}<label
                class="relative flex cursor-pointer flex-col items-center gap-1.5 rounded-md border border-dashed border-[var(--color-border-brand)] px-2 py-2.5 text-[var(--color-text-secondary)] focus-within:outline focus-within:outline-2 focus-within:outline-[var(--color-primary)]"
                ><Icon name="upload" size={20} /><strong>PNG-Datei auswählen</strong><span
                  class="text-xs">64 × 64 oder 64 × 32 Pixel · max. 1 MB</span
                ><input
                  class="absolute inset-0 size-full cursor-pointer opacity-0"
                  bind:this={fileInput}
                  type="file"
                  accept="image/png,.png"
                  disabled={importing}
                  onchange={(event) => {
                    const file = event.currentTarget.files?.[0];
                    if (file) void loadImport(file);
                  }}
                /></label
              >
            {:else}<div class="flex gap-[7px] text-sm">
                <input
                  class="min-w-0 flex-1 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-field)] px-[11px] py-2.5 text-sm font-normal text-[var(--color-text-primary)] focus:outline focus:outline-1 focus:outline-[var(--color-primary)]"
                  aria-label={source === "url"
                    ? "Skin-URL"
                    : source === "uuid"
                      ? "Spieler-UUID"
                      : "Minecraft-Spielername"}
                  bind:value
                  placeholder={source === "url"
                    ? "https://…/skin.png"
                    : source === "uuid"
                      ? "Spieler-UUID"
                      : "z. B. blackNinja46"}
                  disabled={importing}
                /><button
                  type="button"
                  class="mc-secondary px-3"
                  disabled={importing || !value.trim()}
                  onclick={() => loadImport()}>Laden</button
                >
              </div>{/if}
            {#if importing}<p class="mt-2 text-sm" role="status">
                Skin wird geprüft und geladen …
              </p>{/if}
          </fieldset>
          <label class="block w-full text-sm font-semibold text-[var(--color-text-secondary)]"
            >Name<input
              class="mt-2 w-full min-w-0 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-field)] px-[11px] py-2.5 text-sm font-normal text-[var(--color-text-primary)] focus:outline focus:outline-1 focus:outline-[var(--color-primary)]"
              bind:value={name}
              maxlength="48"
              required
              placeholder="Mein Lieblingsskin"
              disabled={saving}
            /></label
          >
          <fieldset class="min-w-0" disabled={saving}>
            <legend
              class="mb-[9px] block w-full text-sm font-semibold text-[var(--color-text-secondary)]"
              >Modell</legend
            >
            <div class="grid grid-cols-2 gap-2">
              <button
                type="button"
                class="flex items-center gap-2.5 rounded-md border p-[11px] text-left {model ===
                'classic'
                  ? 'border-[var(--color-border-brand)] bg-[var(--color-brand-soft)] text-[var(--color-text-primary)]'
                  : 'border-[var(--color-border-default)]'}"
                aria-pressed={model === "classic"}
                onclick={() => (model = "classic")}
                ><Icon name="skins" size={20} /><span
                  ><strong class="block text-sm">Klassisch</strong><small
                    class="mt-[3px] block text-xs text-[var(--color-text-muted)]">Steve</small
                  ></span
                ></button
              ><button
                type="button"
                class="flex items-center gap-2.5 rounded-md border p-[11px] text-left {model ===
                'slim'
                  ? 'border-[var(--color-border-brand)] bg-[var(--color-brand-soft)] text-[var(--color-text-primary)]'
                  : 'border-[var(--color-border-default)]'}"
                aria-pressed={model === "slim"}
                onclick={() => (model = "slim")}
                ><Icon name="skins" size={17} /><span
                  ><strong class="block text-sm">Schlank</strong><small
                    class="mt-[3px] block text-xs text-[var(--color-text-muted)]">Alex</small
                  ></span
                ></button
              >
            </div>
          </fieldset>

          <fieldset class="min-w-0" disabled={saving || profileLoading}>
            <legend
              class="mb-[9px] block w-full text-sm font-semibold text-[var(--color-text-secondary)]"
              >Cape <span class="float-right text-xs font-normal text-[var(--color-text-muted)]"
                >{account ? "Aus deinem Besitz" : "Anmeldung erforderlich"}</span
              ></legend
            >
            <div class="max-h-[100px]">
              {#if profileLoading}<p class="text-sm text-white/50" role="status">
                  Deine Capes werden geladen …
                </p>{/if}
              <div class="flex flex-wrap gap-[7px]">
                <button
                  type="button"
                  class="flex min-h-[87px] w-[94px] flex-col items-center justify-center gap-1.5 rounded-md border px-1 py-2 text-xs {capeId ===
                  null
                    ? 'border-[var(--color-border-brand)] bg-[var(--color-brand-soft)] text-[var(--color-text-primary)]'
                    : 'border-[var(--color-border-default)]'}"
                  aria-pressed={capeId === null}
                  onclick={() => (capeId = null)}
                  ><span class="grid h-12 place-items-center text-[var(--color-text-muted)]"
                    ><Icon name="close" size={18} /></span
                  ><span>Kein Cape</span></button
                >{#each capes as cape}<button
                    type="button"
                    class="flex min-h-[87px] w-[94px] flex-col items-center justify-center gap-1.5 rounded-[5px] border px-1 py-2 text-xs {capeId ===
                    cape.id
                      ? 'border-[var(--color-border-brand)] bg-[var(--color-brand-soft)] text-[var(--color-text-primary)]'
                      : 'border-[var(--color-border-default)]'}"
                    aria-pressed={capeId === cape.id}
                    onclick={() => (capeId = cape.id)}
                    ><span
                      class="block h-12 w-[30px] bg-[length:192px_96px] bg-[-3px_-3px] [image-rendering:pixelated]"
                      style:background-image={`url("${secureUrl(cape.url)}")`}
                    ></span><span>{cape.alias}</span></button
                  >{/each}
              </div>
              {#if unavailableCape}<p class="mt-2 text-sm text-amber-200">
                  Dieses Cape ist nicht verfügbar. Bitte ein anderes Cape oder „Kein Cape“
                  auswählen.
                </p>{:else if account && profile && !capes.length}<p
                  class="mt-2 text-sm text-white/45"
                >
                  Dieses Konto besitzt noch keine Java-Capes.
                </p>{/if}
              {#if profileError}<p class="mt-2 text-sm text-amber-200">
                  Capes konnten nicht vollständig geladen werden.
                </p>
                <button type="button" class="mt-2 text-sm underline" onclick={refreshProfile}
                  >Erneut laden</button
                >{/if}
            </div>
          </fieldset>
        </div>
      </div>
      <footer
        class="sticky bottom-0 z-40 flex items-center gap-[9px] border-t border-[var(--color-border-subtle)] bg-[var(--color-surface-strong)] px-5 py-3 max-[700px]:flex-wrap"
      >
        <span
          class="mr-auto max-w-[340px] text-xs leading-relaxed text-[var(--color-text-muted)] max-[700px]:w-full max-[700px]:max-w-none"
          >Speichere deinen Look und aktiviere ihn anschließend mit „Verwenden“.</span
        ><button
          type="button"
          class="mc-secondary px-4 py-2.5 text-sm"
          disabled={saving}
          onclick={closeEditor}>Abbrechen</button
        ><button
          type="submit"
          class="mc-primary px-5 py-2.5 text-sm"
          disabled={!texture || !name.trim() || importing || saving || unavailableCape}
          >{saving ? "Wird gespeichert …" : "Skin speichern"}</button
        >
      </footer>
    </form>
  {/if}
</dialog>

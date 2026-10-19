<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Icon from "$lib/components/ui/Icon.svelte";
  import Toggle from "$lib/components/ui/Toggle.svelte";
  import {
    beginMicrosoftLogin,
    chooseInstallationDirectory,
    completeMicrosoftLogin,
    getAccounts,
    getSetupPaths,
    importMinecraftData,
    messageFromError,
    saveSettings,
  } from "$lib/api/tauri";
  import type {
    Account,
    LauncherOverview,
    LauncherSettings,
    MinecraftImportReport,
  } from "$lib/types";

  let {
    settings,
    overview,
    initialAccount,
    initialAccounts,
    defaultInstanceId,
    oninstalled,
    onfinish,
  }: {
    settings: LauncherSettings;
    overview: LauncherOverview;
    initialAccount: Account | null;
    initialAccounts: Account[];
    defaultInstanceId: string;
    oninstalled: () => Promise<void>;
    onfinish: (settings: LauncherSettings, account: Account | null, accounts: Account[]) => void;
  } = $props();

  const steps = [
    { label: "Welcome", icon: "info" },
    { label: "Installation", icon: "folder" },
    { label: "Terms & privacy", icon: "check" },
    { label: "Java account", icon: "skins" },
    { label: "Appearance", icon: "settings" },
    { label: "Summary", icon: "check" },
  ];

  let step = $state(0);
  let draft = $state<LauncherSettings>(untrack(() => ({ ...settings })));
  let termsAccepted = $state(untrack(() => Boolean(settings.termsAcceptedAt)));
  let importExisting = $state(false);
  let importSource = $state("");
  let currentAccount = $state<Account | null>(untrack(() => initialAccount));
  let currentAccounts = $state<Account[]>(untrack(() => [...initialAccounts]));
  let loginBusy = $state(false);
  let loginCode = $state("");
  let loginMessage = $state("");
  let installing = $state(false);
  let installProgress = $state(0);
  let installStatus = $state("Preparing…");
  let installError = $state("");
  let importReport = $state<MinecraftImportReport | null>(null);
  let completedSettings = $state<LauncherSettings | null>(null);

  const canContinue = $derived(
    step === 1 ? Boolean(draft.installationPath?.trim()) : step === 2 ? termsAccepted : true,
  );

  onMount(() => {
    void getSetupPaths()
      .then((paths) => {
        if (!draft.installationPath) draft.installationPath = paths.recommendedInstallationPath;
        if (!importSource && paths.minecraftDirectory) {
          importSource = paths.minecraftDirectory;
          importExisting = true;
        }
      })
      .catch(() => {
        if (!draft.installationPath) draft.installationPath = overview.appDataDirectory;
      });
  });

  async function browseInstallationPath() {
    const selected = await chooseInstallationDirectory(draft.installationPath ?? undefined);
    if (selected) draft.installationPath = selected;
  }

  async function browseImportSource() {
    const selected = await chooseInstallationDirectory(importSource || undefined);
    if (selected) importSource = selected;
  }

  async function login() {
    loginBusy = true;
    loginCode = "";
    loginMessage = "Preparing secure Microsoft sign-in…";
    try {
      const device = await beginMicrosoftLogin();
      loginCode = device.user_code;
      loginMessage = device.message;
      await openUrl(device.verification_uri);
      currentAccount = await completeMicrosoftLogin(device);
      const state = await getAccounts();
      currentAccounts = state.accounts;
      currentAccount =
        currentAccounts.find((account) => account.uuid === state.activeUuid) ??
        currentAccounts[0] ??
        null;
      loginCode = "";
      loginMessage = "Minecraft Java Edition was connected successfully.";
    } catch (error) {
      loginMessage = messageFromError(error);
    } finally {
      loginBusy = false;
    }
  }

  function updateTermsAccepted(accepted: boolean) {
    termsAccepted = accepted;
    if (accepted) draft.shareAnonymousUsageData = true;
  }

  async function next() {
    if (!canContinue || installing) return;
    if (step === 5) {
      await install();
      return;
    }
    step += 1;
  }

  async function install() {
    step = 6;
    installing = true;
    installError = "";
    importReport = null;
    try {
      installProgress = 14;
      installStatus = "Saving settings…";
      const finished: LauncherSettings = {
        ...draft,
        setupComplete: true,
        termsAcceptedAt: draft.termsAcceptedAt ?? new Date().toISOString(),
      };
      const saved = await saveSettings(finished);
      installProgress = 46;
      installStatus = "Preparing launcher directories…";
      if (importExisting && importSource.trim()) {
        installStatus = "Copying existing Minecraft data…";
        importReport = await importMinecraftData(importSource.trim(), defaultInstanceId);
      }
      installProgress = 82;
      installStatus = "Checking profiles and components…";
      await oninstalled();
      completedSettings = saved;
      installProgress = 100;
      installStatus = "Ninja Launcher is ready.";
      installing = false;
      step = 7;
    } catch (error) {
      installing = false;
      installError = messageFromError(error);
      installStatus = "The installation could not be completed.";
    }
  }

  function finish() {
    if (completedSettings) onfinish(completedSettings, currentAccount, currentAccounts);
  }
</script>

<div
  class="setup-shell fixed inset-0 z-[120] grid place-items-center overflow-hidden bg-[var(--color-bg-canvas)] p-5 text-[var(--color-text-primary)]"
>
  <div class="setup-glow pointer-events-none absolute inset-0"></div>
  <section
    class="relative grid h-full max-h-[760px] min-h-[620px] w-full max-w-[1080px] grid-cols-[250px_minmax(0,1fr)] overflow-hidden rounded-xl border border-white/[0.11] bg-[var(--color-section-background)] shadow-[0_34px_110px_rgba(0,0,0,.55)]"
    aria-label="Ninja Launcher setup"
  >
    <aside class="flex flex-col border-r border-white/[0.08] bg-black/15 p-7">
      <div class="flex items-center gap-3">
        <img src="/brand/ninja-launcher-mark.png" alt="" class="h-11 w-12 object-contain" />
        <div>
          <div class="text-sm font-bold">Ninja Launcher</div>
          <div class="mt-0.5 text-[8px] font-bold uppercase tracking-[.17em] text-white/35">
            First-time setup
          </div>
        </div>
      </div>
      <nav class="mt-10 space-y-1" aria-label="Setup-Fortschritt">
        {#each steps as item, index}
          <div
            class="flex items-center gap-3 rounded-md px-3 py-2.5 transition {index === step
              ? 'bg-white/[.08] text-white'
              : index < step
                ? 'text-[var(--color-text-secondary)]'
                : 'text-[var(--color-text-faint)]'}"
          >
            <span
              class="grid size-7 shrink-0 place-items-center rounded-[5px] border {index < step
                ? 'border-[var(--color-primary)]/40 bg-[var(--color-primary)]/15 text-[var(--color-primary)]'
                : index === step
                  ? 'border-white/[.15] bg-white/[.08] text-white'
                  : 'border-white/[.07] bg-white/[.02]'}"
              ><Icon name={index < step ? "check" : item.icon} size={10} /></span
            >
            <span class="text-[10px] font-bold uppercase tracking-[.08em]">{item.label}</span>
          </div>
        {/each}
      </nav>
      <div class="mt-auto border-t border-white/[.07] pt-5 text-[9px] leading-4 text-white/35">
        Version {overview.version}<br />Minecraft Java Edition
      </div>
    </aside>

    <div class="flex min-w-0 flex-col">
      <div class="min-h-0 flex-1 overflow-y-auto px-12 py-10">
        {#if step === 0}
          <div class="mx-auto flex h-full max-w-xl flex-col justify-center">
            <span
              class="mb-6 grid size-14 place-items-center rounded-lg border border-white/[.1] bg-white/[.055] text-[var(--color-primary)]"
              ><Icon name="shader" size={24} /></span
            >
            <p class="setup-kicker">Welcome</p>
            <h1 class="mt-3 text-4xl font-extrabold tracking-[-.045em]">
              Your Minecraft.<br />Your launcher.
            </h1>
            <p class="mt-5 max-w-lg text-sm leading-7 text-[var(--color-text-secondary)]">
              Set up storage, your account, and the appearance in just a few steps. Afterwards, you
              can manage mods, resource packs, and shaders directly through Modrinth.
            </p>
            <div
              class="mt-8 flex gap-6 text-[10px] font-bold uppercase tracking-[.1em] text-white/45"
            >
              <span>Java Edition</span><span>Modrinth integrated</span><span>Multiple profiles</span
              >
            </div>
          </div>
        {:else if step === 1}
          <div class="mx-auto max-w-2xl">
            <p class="setup-kicker">Storage</p>
            <h1 class="setup-title">Choose an installation location</h1>
            <p class="setup-description">
              Ninja Launcher stores profiles, game versions, mods, and Java components here.
            </p>
            <label
              for="setup-installation-path"
              class="mt-8 block text-[10px] font-bold uppercase tracking-[.12em] text-white/50"
              >Installation path</label
            >
            <div class="mt-2 flex gap-2">
              <input
                id="setup-installation-path"
                bind:value={draft.installationPath}
                class="mc-field min-w-0 flex-1 px-4 py-3 text-xs text-white outline-none"
                placeholder="Choose an installation directory"
              />
              <button
                type="button"
                onclick={browseInstallationPath}
                class="mc-secondary flex items-center gap-2 px-4 text-xs font-bold"
                ><Icon name="folder" size={13} />Browse</button
              >
            </div>
            <div class="mt-7 rounded-lg border border-white/[.09] bg-white/[.025] p-5">
              <div class="flex items-start justify-between gap-6">
                <div>
                  <h2 class="text-sm font-bold">Import existing Minecraft data</h2>
                  <p class="mt-1.5 text-xs leading-5 text-[var(--color-text-muted)]">
                    Copies worlds, settings, servers, mods, resource packs, and shaders into the
                    default profile. Existing files will not be overwritten.
                  </p>
                </div>
                <Toggle bind:checked={importExisting} label="Import Minecraft data" />
              </div>
              {#if importExisting}<div class="mt-4 flex gap-2 border-t border-white/[.07] pt-4">
                  <input
                    bind:value={importSource}
                    class="mc-field min-w-0 flex-1 px-3 py-2.5 text-xs text-white outline-none"
                    placeholder=".minecraft directory"
                  /><button
                    type="button"
                    onclick={browseImportSource}
                    class="mc-secondary px-3 text-xs font-bold">Browse</button
                  >
                </div>{/if}
            </div>
          </div>
        {:else if step === 2}
          <div class="mx-auto max-w-2xl">
            <p class="setup-kicker">Terms &amp; Privacy</p>
            <h1 class="setup-title">Clear and transparent</h1>
            <p class="setup-description">
              Your Microsoft password is never processed by the launcher. Credentials are obtained
              through Microsoft's secure device sign-in and stored locally.
            </p>
            <div class="mt-8 grid grid-cols-2 gap-3">
              <article class="rounded-lg border border-white/[.09] bg-white/[.025] p-5">
                <Icon name="check" size={16} />
                <h2 class="mt-4 text-sm font-bold">Terms of use</h2>
                <p class="mt-2 text-xs leading-5 text-[var(--color-text-muted)]">
                  The launcher downloads Minecraft and Modrinth content for your personal use. The
                  terms of the respective providers also apply.
                </p>
              </article>
              <article class="rounded-lg border border-white/[.09] bg-white/[.025] p-5">
                <Icon name="info" size={16} />
                <h2 class="mt-4 text-sm font-bold">Privacy</h2>
                <p class="mt-2 text-xs leading-5 text-[var(--color-text-muted)]">
                  Account tokens and configuration remain on this device. Anonymous usage data
                  contains no account or device identifier.
                </p>
              </article>
            </div>
            <label
              class="mt-6 flex cursor-pointer items-start gap-3 rounded-lg border border-white/[.1] bg-white/[.035] p-4"
              ><input
                type="checkbox"
                checked={termsAccepted}
                onchange={(event) => updateTermsAccepted(event.currentTarget.checked)}
                class="mt-0.5 size-4 accent-[var(--color-primary)]"
              /><span class="text-xs leading-5 text-[var(--color-text-secondary)]"
                >I have read and accept the Terms of Use and Privacy Notice. Accepting also enables
                anonymous usage data, which can be disabled later in Settings.</span
              ></label
            >
          </div>
        {:else if step === 3}
          <div class="mx-auto max-w-2xl">
            <p class="setup-kicker">Minecraft Java</p>
            <h1 class="setup-title">Connect your account</h1>
            <p class="setup-description">
              Sign in with the Microsoft account that owns Minecraft Java Edition. You can skip this
              step and use the demo first.
            </p>
            {#if currentAccount}
              <div
                class="mt-8 flex items-center gap-4 rounded-lg border border-white/[.12] bg-white/[.055] p-5"
              >
                <img
                  src={"https://mc-heads.net/avatar/" + currentAccount.uuid + "/64"}
                  alt=""
                  class="size-12 rounded-md [image-rendering:pixelated]"
                />
                <div class="min-w-0 flex-1">
                  <p class="text-sm font-bold">{currentAccount.username}</p>
                  <p class="mt-1 truncate text-[9px] text-[var(--color-text-faint)]">
                    {currentAccount.uuid}
                  </p>
                </div>
                <span class="text-[10px] font-bold uppercase tracking-wider text-emerald-300"
                  >Connected</span
                >
              </div>
            {:else}
              <div class="mt-8 rounded-lg border border-white/[.09] bg-white/[.025] p-6">
                <div class="flex items-start gap-4">
                  <span
                    class="grid size-11 shrink-0 place-items-center rounded-md bg-white/[.055] text-[var(--color-primary)]"
                    ><Icon name="skins" size={18} /></span
                  >
                  <div>
                    <h2 class="text-sm font-bold">Secure Microsoft device sign-in</h2>
                    <p class="mt-1.5 text-xs leading-5 text-[var(--color-text-muted)]">
                      Microsoft opens in your browser. Ninja Launcher never sees your password.
                    </p>
                  </div>
                </div>
                {#if loginCode}<div
                    class="mt-5 rounded-md border border-white/[.1] bg-black/20 p-4 text-center"
                  >
                    <p class="text-[9px] font-bold uppercase tracking-widest text-white/40">
                      Microsoft code
                    </p>
                    <p class="mt-2 text-2xl font-black tracking-[.2em]">{loginCode}</p>
                  </div>{/if}
                {#if loginMessage}<p
                    class="mt-4 text-xs leading-5 text-[var(--color-text-secondary)]"
                  >
                    {loginMessage}
                  </p>{/if}
                <button
                  type="button"
                  onclick={login}
                  disabled={loginBusy}
                  class="mc-primary mt-5 w-full py-3 text-sm font-bold disabled:opacity-55"
                  >{loginBusy ? "Waiting for Microsoft…" : "Sign in with Microsoft"}</button
                >
              </div>
            {/if}
          </div>
        {:else if step === 4}
          <div class="mx-auto max-w-2xl">
            <p class="setup-kicker">Accessibility &amp; design</p>
            <h1 class="setup-title">Make the launcher yours</h1>
            <p class="setup-description">You can change all of these options later in Settings.</p>
            <h2 class="mt-8 text-[10px] font-bold uppercase tracking-[.12em] text-white/45">
              Appearance
            </h2>
            <div class="mt-3 grid grid-cols-3 gap-3">
              {#each [{ id: "dark", title: "Dark" }, { id: "light", title: "Light" }, { id: "system", title: "System" }] as theme}
                <button
                  type="button"
                  onclick={() => (draft.theme = theme.id as LauncherSettings["theme"])}
                  class="rounded-lg border p-4 text-left {draft.theme === theme.id
                    ? 'border-[var(--color-primary)]/45 bg-[var(--color-primary)]/[.09]'
                    : 'border-white/[.09] bg-white/[.025] hover:border-white/[.16]'}"
                  ><span class="theme-preview theme-{theme.id}">
                    <span class="preview-sidebar">
                      <i></i><i></i><i></i>
                    </span>
                    <span class="preview-main">
                      <i class="preview-title"></i>
                      <i class="preview-player"></i>
                      <i class="preview-button"></i>
                      <i class="preview-bottom"></i>
                    </span>
                  </span><span class="text-xs font-bold">{theme.title}</span></button
                >
              {/each}
            </div>
            <div class="mt-6 divide-y divide-white/[.07] rounded-lg border border-white/[.09]">
              <div class="flex items-center justify-between gap-6 p-4">
                <div>
                  <p class="text-xs font-bold">Animations</p>
                  <p class="mt-1 text-[10px] text-[var(--color-text-muted)]">
                    Smooth page transitions, buttons, and character movement.
                  </p>
                </div>
                <Toggle bind:checked={draft.animationsEnabled} label="Enable animations" />
              </div>
              <div class="flex items-center justify-between gap-6 p-4">
                <div>
                  <p class="text-xs font-bold">Parallax background</p>
                  <p class="mt-1 text-[10px] text-[var(--color-text-muted)]">
                    Subtle spatial movement in the background.
                  </p>
                </div>
                <Toggle bind:checked={draft.parallaxEnabled} label="Enable parallax" />
              </div>
            </div>
          </div>
        {:else if step === 5}
          <div class="mx-auto max-w-2xl">
            <p class="setup-kicker">Summary</p>
            <h1 class="setup-title">Ready to install</h1>
            <p class="setup-description">
              Review your choices. Selecting “Install” prepares the directories and saves your
              settings.
            </p>
            <dl class="mt-8 divide-y divide-white/[.07] rounded-lg border border-white/[.09]">
              <div class="grid grid-cols-[170px_1fr] gap-5 p-4">
                <dt class="summary-label">Storage</dt>
                <dd class="truncate text-xs">{draft.installationPath}</dd>
              </div>
              <div class="grid grid-cols-[170px_1fr] gap-5 p-4">
                <dt class="summary-label">Import</dt>
                <dd class="text-xs">
                  {importExisting ? "From " + importSource : "No existing data"}
                </dd>
              </div>
              <div class="grid grid-cols-[170px_1fr] gap-5 p-4">
                <dt class="summary-label">Account</dt>
                <dd class="text-xs">{currentAccount?.username ?? "Sign in later / Demo"}</dd>
              </div>
              <div class="grid grid-cols-[170px_1fr] gap-5 p-4">
                <dt class="summary-label">Design</dt>
                <dd class="text-xs capitalize">
                  {draft.theme} · Animations {draft.animationsEnabled ? "on" : "off"} · Parallax {draft.parallaxEnabled
                    ? "on"
                    : "off"}
                </dd>
              </div>
            </dl>
          </div>
        {:else if step === 6}
          <div
            class="mx-auto flex h-full max-w-xl flex-col items-center justify-center text-center"
          >
            <img
              src="/brand/ninja-launcher-mark.png"
              alt=""
              class="h-24 w-28 object-contain {installing ? 'animate-pulse' : ''}"
            />
            <p class="setup-kicker mt-7">Installation</p>
            <h1 class="mt-3 text-2xl font-extrabold">{installStatus}</h1>
            <div class="mt-8 h-2 w-full overflow-hidden rounded-full bg-white/[.07]">
              <div
                class="h-full rounded-full bg-[var(--color-primary)] transition-[width] duration-500"
                style={"width: " + installProgress + "%"}
              ></div>
            </div>
            <div class="mt-3 flex w-full justify-between text-[9px] text-white/35">
              <span>{installProgress}%</span><span>Please do not close the launcher</span>
            </div>
            {#if installError}<div class="error-banner mt-6 w-full p-4 text-left text-xs">
                {installError}
              </div>
              <div class="mt-4 flex gap-2">
                <button
                  type="button"
                  onclick={() => (step = 6)}
                  class="mc-secondary px-4 py-2.5 text-xs">Back</button
                ><button
                  type="button"
                  onclick={install}
                  class="mc-primary px-4 py-2.5 text-xs font-bold">Try again</button
                >
              </div>{/if}
          </div>
        {:else}
          <div
            class="welcome-finish mx-auto flex h-full max-w-xl flex-col items-center justify-center text-center"
          >
            <div class="welcome-mark relative">
              <span class="welcome-ring absolute inset-0"></span><img
                src="/brand/ninja-launcher-mark.png"
                alt="Ninja Launcher"
                class="relative z-10 h-28 w-32 object-contain"
              />
            </div>
            <p class="setup-kicker mt-8">All set</p>
            <h1 class="mt-3 text-4xl font-extrabold tracking-[-.04em]">Welcome to Ninja.</h1>
            <p class="mt-4 max-w-md text-sm leading-6 text-[var(--color-text-secondary)]">
              Your launcher is ready{currentAccount
                ? " and connected to " + currentAccount.username
                : ""}. Have fun in Minecraft!
            </p>
            {#if importReport}<p class="mt-3 text-[10px] text-white/40">
                {importReport.copiedFiles} files imported · {importReport.skippedFiles} skipped
              </p>{/if}
            <button
              type="button"
              onclick={finish}
              class="mc-primary mt-8 flex items-center gap-2 px-7 py-3.5 text-sm font-bold"
              >Open launcher <Icon name="chevron" size={11} /></button
            >
          </div>
        {/if}
      </div>
      {#if step <= 5}
        <footer class="flex items-center border-t border-white/[.08] px-12 py-5">
          <span class="text-[9px] font-bold uppercase tracking-[.14em] text-white/30"
            >Step {step + 1} of {steps.length}</span
          >
          <div class="ml-auto flex gap-2">
            {#if step > 0}<button
                type="button"
                onclick={() => (step -= 1)}
                class="mc-secondary px-5 py-2.5 text-xs font-bold">Back</button
              >{/if}<button
              type="button"
              onclick={next}
              disabled={!canContinue}
              class="mc-primary min-w-28 px-5 py-2.5 text-xs font-bold disabled:cursor-not-allowed disabled:opacity-40"
              >{step === 5 ? "Install" : "Continue"}</button
            >
          </div>
        </footer>
      {/if}
    </div>
  </section>
</div>

<style>
  .setup-shell {
    isolation: isolate;
  }
  .setup-glow {
    background:
      radial-gradient(ellipse at 72% 18%, rgb(59 157 245 / 0.13), transparent 35%),
      radial-gradient(ellipse at 25% 90%, rgb(59 157 245 / 0.06), transparent 34%);
  }
  .setup-kicker {
    font-size: 9px;
    font-weight: 800;
    letter-spacing: 0.18em;
    color: var(--color-primary);
    text-transform: uppercase;
  }
  .setup-title {
    margin-top: 0.65rem;
    font-size: 2rem;
    font-weight: 800;
    letter-spacing: -0.04em;
  }
  .setup-description {
    margin-top: 0.9rem;
    max-width: 36rem;
    color: var(--color-text-secondary);
    font-size: 0.8rem;
    line-height: 1.6rem;
  }
  .summary-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: rgb(255 255 255 / 0.4);
    text-transform: uppercase;
  }
  .theme-preview {
    position: relative;
    display: block;
    height: 78px;
    margin-bottom: 0.75rem;
    overflow: hidden;
    border: 1px solid rgb(255 255 255 / 0.12);
    border-radius: 5px;
    background: #0d0f14;
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.06);
  }
  .theme-preview.theme-light {
    border-color: rgb(24 28 36 / 0.14);
    background: #f0f2f5;
  }
  .theme-preview.theme-system {
    background: linear-gradient(90deg, #0d0f14 0 50%, #f0f2f5 50% 100%);
  }
  .theme-preview.theme-system::after {
    position: absolute;
    inset: 0 auto 0 50%;
    width: 1px;
    background: rgb(59 157 245 / 0.5);
    content: "";
  }
  .preview-sidebar {
    position: absolute;
    inset: 5px auto 5px 5px;
    width: 23%;
    border: 1px solid rgb(255 255 255 / 0.08);
    border-radius: 3px;
    background: rgb(255 255 255 / 0.035);
    padding: 7px 4px;
  }
  .theme-light .preview-sidebar {
    border-color: rgb(24 28 36 / 0.1);
    background: rgb(24 28 36 / 0.045);
  }
  .preview-sidebar i {
    display: block;
    height: 3px;
    margin-bottom: 5px;
    border-radius: 1px;
    background: rgb(255 255 255 / 0.2);
  }
  .theme-light .preview-sidebar i {
    background: rgb(24 28 36 / 0.2);
  }
  .preview-sidebar i:first-child {
    background: var(--color-primary);
  }
  .preview-main {
    position: absolute;
    inset: 7px 6px 6px 30%;
  }
  .preview-main i {
    position: absolute;
    display: block;
    border-radius: 1px;
  }
  .preview-title {
    top: 0;
    left: 30%;
    width: 40%;
    height: 4px;
    background: rgb(255 255 255 / 0.55);
  }
  .theme-light .preview-title {
    background: rgb(24 28 36 / 0.55);
  }
  .preview-player {
    top: 10px;
    left: 42%;
    width: 16%;
    height: 28px;
    background: linear-gradient(90deg, #151923 0 50%, var(--color-primary) 50%);
    image-rendering: pixelated;
  }
  .preview-button {
    top: 40px;
    left: 27%;
    width: 46%;
    height: 7px;
    background: var(--color-primary);
  }
  .preview-bottom {
    right: 0;
    bottom: 0;
    left: 0;
    height: 8px;
    border: 1px solid rgb(255 255 255 / 0.1);
    background: rgb(255 255 255 / 0.04);
  }
  .theme-light .preview-bottom {
    border-color: rgb(24 28 36 / 0.12);
    background: rgb(24 28 36 / 0.04);
  }
  .welcome-finish {
    animation: welcome-in 700ms cubic-bezier(0.16, 1, 0.3, 1) both;
  }
  .welcome-ring {
    border: 1px solid rgb(59 157 245 / 0.32);
    border-radius: 22%;
    box-shadow: 0 0 45px rgb(59 157 245 / 0.24);
    animation: welcome-ring 2.8s ease-in-out infinite;
  }
  @keyframes welcome-in {
    from {
      opacity: 0;
      transform: translateY(20px) scale(0.97);
    }
  }
  @keyframes welcome-ring {
    0%,
    100% {
      transform: scale(0.8) rotate(-3deg);
      opacity: 0.25;
    }
    50% {
      transform: scale(1.16) rotate(3deg);
      opacity: 0.8;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .welcome-finish,
    .welcome-ring {
      animation: none;
    }
  }
</style>

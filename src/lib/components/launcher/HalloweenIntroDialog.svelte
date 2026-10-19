<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";

  let {
    onclose,
    onopensettings,
  }: {
    onclose: () => void;
    onopensettings: () => void;
  } = $props();
</script>

<div
  class="fixed bottom-0 left-[248px] right-0 top-0 z-[185] grid place-items-center bg-black/55 p-6 backdrop-blur-md"
  role="presentation"
  onclick={(event) => {
    if (event.currentTarget === event.target) onclose();
  }}
  onkeydown={(event) => {
    if (event.key === "Escape") onclose();
  }}
>
  <div
    class="halloween-intro relative w-full max-w-[520px] overflow-hidden rounded-[22px] border border-orange-300/25 bg-[#110b18]/95 p-7 shadow-[0_30px_100px_rgba(0,0,0,.72),0_0_80px_rgba(131,57,207,.2)]"
    role="dialog"
    aria-modal="true"
    aria-labelledby="halloween-intro-title"
  >
    <div class="pointer-events-none absolute inset-0" aria-hidden="true">
      <span class="absolute -right-5 -top-7 text-orange-300/10"
        ><Icon name="moon" size={128} /></span
      >
      <span class="absolute bottom-14 left-7 text-purple-300/10"
        ><Icon name="ghost" size={62} /></span
      >
    </div>
    <div class="relative">
      <div
        class="mb-5 grid size-14 place-items-center rounded-2xl border border-orange-300/25 bg-orange-400/10 text-orange-300 shadow-[0_0_32px_rgba(249,115,22,.2)]"
      >
        <Icon name="ghost" size={26} />
      </div>
      <p class="text-[10px] font-bold uppercase tracking-[.2em] text-orange-300/70">
        Limited seasonal event
      </p>
      <h2 id="halloween-intro-title" class="mt-2 text-2xl font-black tracking-[-.03em] text-white">
        Halloween has arrived
      </h2>
      <p class="mt-3 max-w-md text-sm leading-6 text-white/65">
        The event is enabled by default and brings a haunted 3D scene, seasonal colors, and
        occasional ambient sounds while the launcher is idle.
      </p>
      <div class="mt-5 rounded-xl border border-white/[.08] bg-white/[.045] px-4 py-3">
        <p class="text-xs leading-5 text-white/70">
          You can turn the theme and its sounds off separately under
          <strong class="text-white">Settings → Appearance &amp; accessibility</strong>.
        </p>
      </div>
      <div class="mt-6 flex justify-end gap-3">
        <button
          type="button"
          onclick={onopensettings}
          class="rounded-xl border border-white/[.1] bg-white/[.07] px-4 py-2.5 text-xs font-bold text-white/80 transition hover:bg-white/[.12] hover:text-white"
          >Open settings</button
        >
        <button type="button" onclick={onclose} class="mc-primary px-5 py-2.5 text-xs font-bold"
          >Keep Halloween on</button
        >
      </div>
    </div>
  </div>
</div>

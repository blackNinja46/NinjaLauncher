<script lang="ts">
  import { onMount } from "svelte";

  const idleDelayMs = 45_000;
  const soundDelayMinMs = 12_000;
  const soundDelayMaxMs = 38_000;
  const sounds = [
    "/audio/halloween/dragon-studio-witch-laugh-401713.mp3",
    "/audio/halloween/charlie_raven-halloween-impact-02-93804.mp3",
    "/audio/halloween/dragon-studio-spooky-wolf-howl-410547.mp3",
    "/audio/halloween/charlie_raven-halloween-impact-03-93806.mp3",
    "/audio/halloween/dragon-studio-creepy-crow-caw-322991.mp3",
    "/audio/halloween/dragon-studio-halloween-scary-sound-410561.mp3",
    "/audio/halloween/dragon-studio-spooky-scary-sound-410563.mp3",
    "/audio/halloween/dragon-studio-spooky-halloween-intro-410549.mp3",
  ];

  onMount(() => {
    let idleTimer = 0;
    let soundTimer = 0;
    let currentAudio: HTMLAudioElement | null = null;
    let idle = false;
    let lastSound = -1;
    let lastPointerReset = 0;

    function stopCurrentSound() {
      if (!currentAudio) return;
      currentAudio.pause();
      currentAudio.currentTime = 0;
      currentAudio = null;
    }

    function clearTimers() {
      window.clearTimeout(idleTimer);
      window.clearTimeout(soundTimer);
    }

    function randomDelay() {
      return soundDelayMinMs + Math.random() * (soundDelayMaxMs - soundDelayMinMs);
    }

    function scheduleSound() {
      window.clearTimeout(soundTimer);
      if (!idle || document.visibilityState !== "visible" || !document.hasFocus()) return;
      soundTimer = window.setTimeout(playRandomSound, randomDelay());
    }

    function playRandomSound() {
      if (!idle || document.visibilityState !== "visible" || !document.hasFocus()) return;
      let nextSound = Math.floor(Math.random() * sounds.length);
      if (sounds.length > 1 && nextSound === lastSound) nextSound = (nextSound + 1) % sounds.length;
      lastSound = nextSound;

      const audio = new Audio(sounds[nextSound]);
      audio.preload = "auto";
      audio.volume = 0.24;
      currentAudio = audio;
      const finish = () => {
        if (currentAudio === audio) currentAudio = null;
        scheduleSound();
      };
      audio.addEventListener("ended", finish, { once: true });
      audio.addEventListener("error", finish, { once: true });
      void audio.play().catch(finish);
    }

    function beginIdle() {
      idle = true;
      scheduleSound();
    }

    function registerActivity(event?: Event) {
      if (event?.type === "pointermove") {
        const now = performance.now();
        if (now - lastPointerReset < 800) return;
        lastPointerReset = now;
      }
      idle = false;
      clearTimers();
      stopCurrentSound();
      if (document.visibilityState === "visible" && document.hasFocus()) {
        idleTimer = window.setTimeout(beginIdle, idleDelayMs);
      }
    }

    const activityEvents = ["pointerdown", "pointermove", "keydown", "wheel", "touchstart"];
    for (const eventName of activityEvents) {
      window.addEventListener(eventName, registerActivity, { passive: true });
    }
    window.addEventListener("focus", registerActivity);
    window.addEventListener("blur", registerActivity);
    document.addEventListener("visibilitychange", registerActivity);
    registerActivity();

    return () => {
      clearTimers();
      stopCurrentSound();
      for (const eventName of activityEvents) {
        window.removeEventListener(eventName, registerActivity);
      }
      window.removeEventListener("focus", registerActivity);
      window.removeEventListener("blur", registerActivity);
      document.removeEventListener("visibilitychange", registerActivity);
    };
  });
</script>

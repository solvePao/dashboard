<script setup lang="ts">
import { computed } from "vue";
import { Mic, Volume2, Loader2 } from "@lucide/vue";
import type { WalkieStatus } from "../../types/walkieTalkie";

const props = defineProps<{
  status: WalkieStatus;
  active: boolean;
}>();

const emit = defineEmits<{
  (e: "press"): void;
  (e: "release"): void;
}>();

function onPointerDown(event: PointerEvent): void {
  if (event.button !== 0) return;
  if (props.status === "busy" || props.status === "connecting" || props.status === "reconnecting") return;
  try {
    (event.currentTarget as HTMLButtonElement)?.setPointerCapture(event.pointerId);
  } catch {
    // pointer capture may fail in restricted test harnesses
  }
  emit("press");
}

function onPointerUp(event: PointerEvent): void {
  try {
    if ((event.currentTarget as HTMLButtonElement)?.hasPointerCapture?.(event.pointerId)) {
      (event.currentTarget as HTMLButtonElement)?.releasePointerCapture(event.pointerId);
    }
  } catch {
    // ignore
  }
  emit("release");
}

const statusText = computed(() => {
  switch (props.status) {
    case "calling":
      return "Calling group…";
    case "talking":
      return "Talking live";
    case "listening":
      return "Listening";
    case "connecting":
    case "reconnecting":
      return "Connecting…";
    case "busy":
      return "Channel busy";
    case "ready":
      return "Hold to talk";
    default:
      return "Hold to talk";
  }
});
</script>

<template>
  <div class="relative flex items-center justify-center p-1.5">
    <!-- Concentric animated ripple rings when talking or calling -->
    <transition name="fade">
      <div v-if="status === 'talking'" class="pointer-events-none absolute inset-0 flex items-center justify-center">
        <span class="talk-ring talk-ring-1 absolute h-24 w-24 rounded-full border-2 border-emerald-500/50 sm:h-28 sm:w-28" />
        <span class="talk-ring talk-ring-2 absolute h-24 w-24 rounded-full border border-emerald-400/40 sm:h-28 sm:w-28" />
        <span class="talk-glow absolute h-24 w-24 rounded-full bg-emerald-500/15 blur-md sm:h-28 sm:w-28" />
      </div>
      <div v-else-if="status === 'calling'" class="pointer-events-none absolute inset-0 flex items-center justify-center">
        <span class="talk-ring talk-ring-1 absolute h-24 w-24 rounded-full border-2 border-amber-400/60 sm:h-28 sm:w-28" />
        <span class="talk-ring talk-ring-2 absolute h-24 w-24 rounded-full border border-amber-300/40 sm:h-28 sm:w-28" />
        <span class="talk-glow absolute h-24 w-24 rounded-full bg-amber-500/20 blur-md sm:h-28 sm:w-28" />
      </div>
    </transition>

    <button
      type="button"
      class="walkie-talk-btn relative z-10 flex h-24 w-24 shrink-0 touch-none select-none flex-col items-center justify-center gap-1 rounded-full border-3 shadow-lg transition-all duration-200 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary sm:h-28 sm:w-28"
      :class="{
        'border-amber-400 bg-amber-500 text-white shadow-amber-500/40 animate-pulse scale-[1.01]': status === 'calling',
        'border-emerald-400/80 bg-emerald-600 text-white shadow-emerald-600/30 scale-[1.02]': status === 'talking',
        'border-primary/20 bg-primary/10 text-primary shadow-primary/5': status === 'listening',
        'border-primary/20 bg-primary text-primary-foreground hover:bg-primary/95 active:scale-95': status === 'ready' || status === 'activation',
        'border-muted-foreground/20 bg-muted/70 text-muted-foreground cursor-not-allowed': status === 'busy' || status === 'error',
        'border-primary/30 bg-muted text-primary animate-pulse': status === 'connecting' || status === 'reconnecting',
      }"
      :disabled="status === 'busy' || status === 'connecting' || status === 'reconnecting'"
      :aria-pressed="status === 'talking' || status === 'calling'"
      aria-label="Hold to talk"
      @pointerdown.prevent.stop="onPointerDown"
      @pointerup.prevent.stop="onPointerUp"
      @pointercancel="emit('release')"
      @lostpointercapture="emit('release')"
      @keydown.space.prevent="!$event.repeat && emit('press')"
      @keyup.space.prevent="emit('release')"
      @keydown.enter.prevent="!$event.repeat && emit('press')"
      @keyup.enter.prevent="emit('release')"
      @blur="emit('release')"
      @contextmenu.prevent
    >
      <Loader2 v-if="status === 'connecting' || status === 'reconnecting'" class="h-6 w-6 animate-spin sm:h-7 sm:w-7" />
      <Loader2 v-else-if="status === 'calling'" class="h-6 w-6 animate-spin text-white sm:h-7 sm:w-7" />
      <Volume2 v-else-if="status === 'listening'" class="h-6 w-6 animate-pulse text-primary sm:h-7 sm:w-7" />
      <Mic v-else class="h-6 w-6 transition-transform duration-150 sm:h-7 sm:w-7" :class="{ 'scale-110': status === 'talking' }" />

      <span class="text-[11px] font-semibold tracking-tight">
        {{ statusText }}
      </span>
    </button>
  </div>
</template>

<style scoped>
.walkie-talk-btn {
  -webkit-touch-callout: none;
  -webkit-user-select: none;
  user-select: none;
}

@keyframes talk-ripple {
  0% {
    transform: scale(0.95);
    opacity: 0.8;
  }
  100% {
    transform: scale(1.55);
    opacity: 0;
  }
}

.talk-ring-1 {
  animation: talk-ripple 1.8s cubic-bezier(0.2, 0.8, 0.4, 1) infinite;
}

.talk-ring-2 {
  animation: talk-ripple 1.8s cubic-bezier(0.2, 0.8, 0.4, 1) infinite;
  animation-delay: 0.6s;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>


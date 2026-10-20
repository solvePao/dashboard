<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { useOrientationLock } from "../composables/useOrientationLock";

export interface PagerItem {
  key: string;
  label?: string;
}

const props = withDefaults(
  defineProps<{
    modelValue: string;
    items: PagerItem[];
    enabled?: boolean;
    dotVariant?: "light" | "amber" | "theme";
  }>(),
  {
    enabled: true,
    dotVariant: "theme",
  }
);

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "change", value: string, direction: "up" | "down"): void;
}>();

const { isPortrait } = useOrientationLock();

const direction = ref<"up" | "down">("up");
const isAnimating = ref(false);
let animationTimer: ReturnType<typeof setTimeout> | undefined;

const currentIndex = computed(() => {
  const idx = props.items.findIndex((item) => item.key === props.modelValue);
  return idx >= 0 ? idx : 0;
});

const transitionName = computed(() => {
  return direction.value === "up" ? "v-slide-up" : "v-slide-down";
});

function goToIndex(nextIndex: number, forcedDirection?: "up" | "down"): void {
  if (props.items.length <= 1 || !props.enabled) return;
  const current = currentIndex.value;
  if (nextIndex === current) return;

  if (forcedDirection) {
    direction.value = forcedDirection;
  } else {
    direction.value = nextIndex > current ? "up" : "down";
  }

  isAnimating.value = true;
  if (animationTimer) clearTimeout(animationTimer);
  animationTimer = setTimeout(() => {
    isAnimating.value = false;
  }, 400);

  const nextKey = props.items[nextIndex].key;
  emit("update:modelValue", nextKey);
  emit("change", nextKey, direction.value);
}

function next(): void {
  if (props.items.length <= 1 || !props.enabled) return;
  const nextIdx = (currentIndex.value + 1) % props.items.length;
  goToIndex(nextIdx, "up");
}

function prev(): void {
  if (props.items.length <= 1 || !props.enabled) return;
  const prevIdx = (currentIndex.value - 1 + props.items.length) % props.items.length;
  goToIndex(prevIdx, "down");
}

// Gesture Handling
let touchStartX = 0;
let touchStartY = 0;
const SWIPE_THRESHOLD = 30; // Minimum vertical distance in px to trigger page switch
const WHEEL_THRESHOLD = 18; // Minimum wheel delta to trigger page switch

function onTouchStart(e: TouchEvent): void {
  if (e.touches.length > 0) {
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
  }
}

function onTouchEnd(e: TouchEvent): void {
  if (
    e.changedTouches.length === 0 ||
    isAnimating.value ||
    !props.enabled ||
    props.items.length <= 1
  ) {
    return;
  }

  const rawDeltaX = e.changedTouches[0].clientX - touchStartX;
  const rawDeltaY = e.changedTouches[0].clientY - touchStartY;

  // Account for CSS rotate(90deg) when portrait orientation lock is active:
  // In rotated mode, visual Y delta corresponds to -rawDeltaX and visual X to rawDeltaY.
  const visualDeltaY = isPortrait.value ? -rawDeltaX : rawDeltaY;
  const visualDeltaX = isPortrait.value ? rawDeltaY : rawDeltaX;

  // Verify swipe is primarily vertical along the panel's visual axis
  if (
    Math.abs(visualDeltaY) > Math.abs(visualDeltaX) &&
    Math.abs(visualDeltaY) >= SWIPE_THRESHOLD
  ) {
    if (visualDeltaY < 0) {
      // Swiped UP -> advance to next slide
      next();
    } else {
      // Swiped DOWN -> return to previous slide
      prev();
    }
  }
}

let wheelCooldown = false;
let wheelCooldownTimer: ReturnType<typeof setTimeout> | undefined;

function onWheel(e: WheelEvent): void {
  if (wheelCooldown || isAnimating.value || !props.enabled || props.items.length <= 1) {
    return;
  }

  if (Math.abs(e.deltaY) >= WHEEL_THRESHOLD) {
    wheelCooldown = true;
    if (wheelCooldownTimer) clearTimeout(wheelCooldownTimer);
    wheelCooldownTimer = setTimeout(() => {
      wheelCooldown = false;
    }, 420);

    if (e.deltaY > 0) {
      // Wheel down / scroll down -> next slide
      next();
    } else {
      // Wheel up / scroll up -> prev slide
      prev();
    }
  }
}

function getDotClass(idx: number): string {
  const active = idx === currentIndex.value;
  switch (props.dotVariant) {
    case "light":
      return active
        ? "w-5 bg-white shadow-xs"
        : "w-1.5 bg-white/40 hover:bg-white/70";
    case "amber":
      return active
        ? "w-5 bg-amber-400 shadow-[0_0_8px_rgba(245,158,11,0.5)]"
        : "w-1.5 bg-white/40 hover:bg-white/70";
    case "theme":
    default:
      return active
        ? "w-5 bg-primary shadow-xs"
        : "w-1.5 bg-muted-foreground/40 hover:bg-muted-foreground/70";
  }
}

onBeforeUnmount(() => {
  if (animationTimer) clearTimeout(animationTimer);
  if (wheelCooldownTimer) clearTimeout(wheelCooldownTimer);
});
</script>

<template>
  <div
    class="mo-pager-root relative flex h-full w-full flex-col overflow-hidden select-none"
    @touchstart.passive="onTouchStart"
    @touchend.passive="onTouchEnd"
    @wheel="onWheel"
  >
    <div class="relative flex-1 w-full h-full overflow-hidden">
      <Transition :name="transitionName">
        <div
          :key="modelValue"
          class="mo-pager-slide absolute inset-0 flex h-full w-full flex-col overflow-hidden will-change-transform"
        >
          <slot :name="modelValue">
            <slot />
          </slot>
        </div>
      </Transition>
    </div>

    <!-- Floating Morphing Pagination Dots -->
    <div
      v-if="items.length > 1"
      class="pointer-events-auto absolute bottom-2 inset-x-0 z-30 flex items-center justify-center gap-1.5 py-1"
    >
      <button
        v-for="(item, idx) in items"
        :key="item.key"
        type="button"
        class="h-1.5 rounded-full transition-all duration-300 cursor-pointer"
        :class="getDotClass(idx)"
        :aria-label="item.label || item.key"
        :title="item.label || item.key"
        @click="goToIndex(idx)"
      />
    </div>
  </div>
</template>

<style scoped>
/* High-performance directional vertical slide transitions */
.v-slide-up-enter-active,
.v-slide-up-leave-active,
.v-slide-down-enter-active,
.v-slide-down-leave-active {
  transition: transform 0.38s cubic-bezier(0.16, 1, 0.3, 1),
              opacity 0.38s cubic-bezier(0.16, 1, 0.3, 1);
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  backface-visibility: hidden;
  will-change: transform, opacity;
}

/* Slide UP: swiping up or moving forward */
.v-slide-up-enter-from {
  transform: translate3d(0, 100%, 0);
  opacity: 0.25;
}
.v-slide-up-leave-to {
  transform: translate3d(0, -100%, 0);
  opacity: 0.25;
}

/* Slide DOWN: swiping down or moving backward */
.v-slide-down-enter-from {
  transform: translate3d(0, -100%, 0);
  opacity: 0.25;
}
.v-slide-down-leave-to {
  transform: translate3d(0, 100%, 0);
  opacity: 0.25;
}

/* Respect user accessibility motion preference */
@media (prefers-reduced-motion: reduce) {
  .v-slide-up-enter-active,
  .v-slide-up-leave-active,
  .v-slide-down-enter-active,
  .v-slide-down-leave-active {
    transition: opacity 0.2s ease !important;
    transform: none !important;
  }
}
</style>

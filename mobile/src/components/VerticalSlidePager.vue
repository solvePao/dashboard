<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { useOrientationLock } from "../composables/useOrientationLock";

export interface PagerItem {
  key: string;
  label?: string;
  reserveNavigationSpace?: boolean;
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

const reserveNavigationSpace = computed(() => props.items.length > 1 && props.items[currentIndex.value]?.reserveNavigationSpace);

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
let touchGesture: {
  x: number;
  y: number;
  scrollArea: HTMLElement | null;
  owner: "content" | "pager" | null;
} | null = null;
const SWIPE_THRESHOLD = 30; // Minimum vertical distance in px to trigger page switch
const WHEEL_THRESHOLD = 18; // Minimum wheel delta to trigger page switch
// A pause separates wheel sequences because wheel events have no end event.
const WHEEL_SEQUENCE_GAP_MS = 180;
let contentWheelUntil = 0;

function getScrollArea(target: EventTarget | null): HTMLElement | null {
  return target instanceof Element ? target.closest<HTMLElement>("[data-pager-scroll-area]") : null;
}

function canScroll(area: HTMLElement, deltaY: number): boolean {
  // scrollTop can be fractional, while scrollHeight and clientHeight are rounded.
  const maxScrollTop = area.scrollHeight - area.clientHeight;
  return deltaY < 0 ? area.scrollTop > 1 : area.scrollTop < maxScrollTop - 1;
}

function onTouchStart(e: TouchEvent): void {
  const touch = e.touches[0];
  touchGesture = e.touches.length === 1
    ? { x: touch.clientX, y: touch.clientY, scrollArea: getScrollArea(e.target), owner: null }
    : null;
}

function clearTouchGesture(): void {
  touchGesture = null;
}

function getSwipeDelta(touch: Touch, start: { x: number; y: number }): { x: number; y: number } {
  const x = touch.clientX - start.x;
  const y = touch.clientY - start.y;

  // The portrait orientation lock rotates the dashboard 90 degrees.
  return isPortrait.value ? { x: y, y: -x } : { x, y };
}

function onTouchMove(e: TouchEvent): void {
  const gesture = touchGesture;
  if (!gesture?.scrollArea || e.touches.length !== 1) return;

  if (!gesture.owner) {
    const { x, y } = getSwipeDelta(e.touches[0], gesture);
    if (x === 0 && y === 0) return;

    gesture.owner = Math.abs(x) >= Math.abs(y) || canScroll(gesture.scrollArea, -y)
      ? "content"
      : "pager";
  }

  // Claim an edge swipe before the browser takes over the touch sequence.
  if (gesture.owner === "pager" && props.enabled && props.items.length > 1 && e.cancelable) {
    e.preventDefault();
  }
}

function onTouchEnd(e: TouchEvent): void {
  const gesture = touchGesture;
  clearTouchGesture();
  if (!gesture || gesture.owner === "content" || !e.changedTouches.length || isAnimating.value) return;

  const { x, y } = getSwipeDelta(e.changedTouches[0], gesture);
  if (Math.abs(y) <= Math.abs(x) || Math.abs(y) < SWIPE_THRESHOLD) return;
  if (!gesture.owner && gesture.scrollArea && canScroll(gesture.scrollArea, -y)) return;

  if (y < 0) next();
  else prev();
}

let wheelCooldown = false;
let wheelCooldownTimer: ReturnType<typeof setTimeout> | undefined;

function onWheel(e: WheelEvent): void {
  const now = performance.now();
  if (now < contentWheelUntil) {
    contentWheelUntil = now + WHEEL_SEQUENCE_GAP_MS;
    return;
  }

  const scrollArea = getScrollArea(e.target);
  if (scrollArea && (e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY))) return;
  if (scrollArea && e.deltaY !== 0 && canScroll(scrollArea, e.deltaY)) {
    contentWheelUntil = now + WHEEL_SEQUENCE_GAP_MS;
    return;
  }

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
    @touchmove="onTouchMove"
    @touchend.passive="onTouchEnd"
    @touchcancel.passive="clearTouchGesture"
    @wheel="onWheel"
  >
    <div class="relative min-h-0 flex-1 w-full overflow-hidden">
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

    <div
      v-if="items.length > 1"
      class="z-30 flex items-center justify-center"
      :class="reserveNavigationSpace ? 'h-11 shrink-0 border-t border-border/70' : 'absolute bottom-2 inset-x-0 gap-1.5 py-1'"
    >
      <button
        v-for="(item, idx) in items"
        :key="item.key"
        type="button"
        class="flex cursor-pointer items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
        :class="reserveNavigationSpace ? 'h-11 w-11' : undefined"
        :aria-current="idx === currentIndex ? 'true' : undefined"
        :aria-label="item.label || item.key"
        :title="item.label || item.key"
        @click="goToIndex(idx)"
      >
        <span aria-hidden="true" class="block h-1.5 rounded-full transition-all duration-300" :class="getDotClass(idx)" />
      </button>
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

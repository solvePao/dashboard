<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { CalendarDays, Sparkles } from "@lucide/vue";

const now = ref(new Date());
let timer: ReturnType<typeof setInterval> | undefined;

const is24Hour = ref(false);

// Load 24h preference from localStorage if available
try {
  const saved = localStorage.getItem("mo_clock_24h");
  if (saved !== null) {
    is24Hour.value = saved === "true";
  }
} catch {
  // Ignore localStorage issues
}

function toggle24Hour(): void {
  is24Hour.value = !is24Hour.value;
  try {
    localStorage.setItem("mo_clock_24h", String(is24Hour.value));
  } catch {
    // Ignore
  }
}

function updateTime(): void {
  now.value = new Date();
}

onMounted(() => {
  updateTime();
  timer = setInterval(updateTime, 1000);
});

onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});

const hours = computed(() => {
  const h = now.value.getHours();
  if (is24Hour.value) {
    return String(h).padStart(2, "0");
  }
  const h12 = h % 12 || 12;
  return String(h12).padStart(2, "0");
});

const minutes = computed(() => {
  return String(now.value.getMinutes()).padStart(2, "0");
});

const seconds = computed(() => {
  return String(now.value.getSeconds()).padStart(2, "0");
});

const ampm = computed(() => {
  return now.value.getHours() >= 12 ? "PM" : "AM";
});

const fullDateString = computed(() => {
  return now.value.toLocaleDateString([], {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  });
});

const dayOfYear = computed(() => {
  const start = new Date(now.value.getFullYear(), 0, 0);
  const diff = now.value.getTime() - start.getTime();
  const oneDay = 1000 * 60 * 60 * 24;
  return Math.floor(diff / oneDay);
});

const totalDaysInYear = computed(() => {
  const y = now.value.getFullYear();
  return (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0 ? 366 : 365;
});
</script>

<template>
  <div class="mo-digital-clock-root flex h-full w-full flex-col items-center justify-center text-center select-none p-2 my-auto">
    <!-- Center Time Display -->
    <div
      class="group relative inline-flex items-baseline justify-center cursor-pointer transition-transform duration-200 active:scale-95 max-w-full"
      title="Click to toggle 12h / 24h format"
      @click="toggle24Hour"
    >
      <div class="flex items-baseline font-mono font-extrabold tracking-tight text-foreground text-[clamp(1.75rem,8.5vw,4.5rem)] leading-none">
        <span>{{ hours }}</span>
        <span class="animate-pulse text-primary mx-0.5 opacity-80">:</span>
        <span>{{ minutes }}</span>
        <span class="animate-pulse text-primary mx-0.5 opacity-80">:</span>
        <span class="text-[clamp(1.2rem,5.5vw,3rem)] text-foreground/80 font-semibold leading-none">{{ seconds }}</span>
      </div>

      <!-- AM / PM Indicator in 12h Mode -->
      <span
        v-if="!is24Hour"
        class="ml-1.5 sm:ml-2.5 self-start rounded-md border border-primary/20 bg-primary/10 px-1.5 py-0.5 text-[clamp(0.6rem,1.8vw,0.7rem)] font-bold text-primary tracking-wider"
      >
        {{ ampm }}
      </span>
      <span
        v-else
        class="ml-1.5 sm:ml-2.5 self-start rounded-md border border-border/60 bg-muted/50 px-1.5 py-0.5 text-[clamp(0.55rem,1.8vw,0.65rem)] font-semibold text-muted-foreground tracking-wider"
      >
        24H
      </span>
    </div>

    <!-- Date Badge -->
    <div class="mt-3 flex items-center gap-1.5 sm:gap-2 rounded-full border border-border/70 bg-card/80 px-3 py-1 sm:px-3.5 sm:py-1.5 text-[11px] sm:text-xs font-medium text-foreground shadow-xs backdrop-blur-xs max-w-full truncate">
      <CalendarDays class="h-3 w-3 sm:h-3.5 sm:w-3.5 text-primary shrink-0" />
      <span class="truncate">{{ fullDateString }}</span>
    </div>

    <!-- Sub-info: Day of Year Progress -->
    <div class="mt-2 flex items-center gap-1.5 text-[10px] sm:text-[11px] text-muted-foreground/90">
      <Sparkles class="h-2.5 w-2.5 sm:h-3 sm:w-3 text-amber-500/80 shrink-0" />
      <span>Day {{ dayOfYear }} of {{ totalDaysInYear }}</span>
    </div>
  </div>
</template>


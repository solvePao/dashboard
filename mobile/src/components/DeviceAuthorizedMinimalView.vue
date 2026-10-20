<script setup lang="ts">
import { ref } from "vue";
import {
  CalendarDays,
  CheckCircle2,
  Clock,
  Timer,
} from "@lucide/vue";
import type { UserProfile } from "../types/auth";
import DigitalClockView from "./clock/DigitalClockView.vue";
import AnalogClockView from "./clock/AnalogClockView.vue";
import MonthCalendarView from "./clock/MonthCalendarView.vue";

defineProps<{
  user: UserProfile;
}>();

type ClockViewMode = "digital" | "analog" | "calendar";

const STORAGE_KEY_MODE = "mo_clock_view_mode";
const STORAGE_KEY_ANIM = "mo_clock_smooth_anim";

// 1. View Mode Persistence
const savedMode = (typeof localStorage !== "undefined" &&
  localStorage.getItem(STORAGE_KEY_MODE)) as ClockViewMode | null;
const activeMode = ref<ClockViewMode>(
  savedMode && ["digital", "analog", "calendar"].includes(savedMode)
    ? savedMode
    : "digital"
);

function setMode(mode: ClockViewMode): void {
  activeMode.value = mode;
  try {
    localStorage.setItem(STORAGE_KEY_MODE, mode);
  } catch {
    // Ignore
  }
}

// 2. Animation Preference Persistence
const savedAnim = typeof localStorage !== "undefined" &&
  localStorage.getItem(STORAGE_KEY_ANIM);
const smoothAnimation = ref<boolean>(savedAnim !== "false"); // Default to true

function toggleAnimation(): void {
  smoothAnimation.value = !smoothAnimation.value;
  try {
    localStorage.setItem(STORAGE_KEY_ANIM, String(smoothAnimation.value));
  } catch {
    // Ignore
  }
}
</script>

<template>
  <div class="mo-minimal-root flex h-full w-full flex-col justify-between p-2.5 sm:p-3 overflow-hidden select-none">
    <!-- Streamlined Header: Ultra-compact Connected & Welcome row + View Switcher -->
    <div class="mo-minimal-header flex items-center justify-between border-b border-border/50 pb-1.5 shrink-0 gap-2">
      <!-- Minimized Connected & Welcome Name -->
      <div class="flex items-center gap-1.5 min-w-0 pr-1">
        <span class="h-2 w-2 rounded-full bg-emerald-500 animate-pulse shrink-0"></span>
        <span class="truncate text-xs text-muted-foreground font-medium">
          Connected · <strong class="text-foreground font-semibold">{{ user.name ? `Welcome, ${user.name}` : "Welcome" }}</strong>
        </span>
      </div>

      <!-- Segmented View Mode Switcher -->
      <div class="flex items-center rounded-lg border border-border/70 bg-muted/40 p-0.5 shrink-0">
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded-md px-2 py-1 text-[11px] font-medium transition-all"
          :class="
            activeMode === 'digital'
              ? 'bg-background text-foreground shadow-xs font-semibold'
              : 'text-muted-foreground hover:text-foreground'
          "
          title="Digital Clock"
          @click="setMode('digital')"
        >
          <Clock class="h-3 w-3" />
          <span class="hidden sm:inline">Digital</span>
        </button>

        <button
          type="button"
          class="inline-flex items-center gap-1 rounded-md px-2 py-1 text-[11px] font-medium transition-all"
          :class="
            activeMode === 'analog'
              ? 'bg-background text-foreground shadow-xs font-semibold'
              : 'text-muted-foreground hover:text-foreground'
          "
          title="Analog Clock"
          @click="setMode('analog')"
        >
          <Timer class="h-3 w-3" />
          <span class="hidden sm:inline">Analog</span>
        </button>

        <button
          type="button"
          class="inline-flex items-center gap-1 rounded-md px-2 py-1 text-[11px] font-medium transition-all"
          :class="
            activeMode === 'calendar'
              ? 'bg-background text-foreground shadow-xs font-semibold'
              : 'text-muted-foreground hover:text-foreground'
          "
          title="Month Calendar"
          @click="setMode('calendar')"
        >
          <CalendarDays class="h-3 w-3" />
          <span class="hidden sm:inline">Calendar</span>
        </button>
      </div>
    </div>

    <!-- Center Display: Active View with Transition Animation -->
    <div class="mo-minimal-clock flex-1 min-h-0 w-full flex flex-col items-center justify-center py-1 overflow-hidden">
      <Transition name="clock-fade" mode="out-in">
        <DigitalClockView
          v-if="activeMode === 'digital'"
          key="digital"
        />

        <AnalogClockView
          v-else-if="activeMode === 'analog'"
          key="analog"
          :smooth-animation="smoothAnimation"
          @toggle-animation="toggleAnimation"
        />

        <MonthCalendarView
          v-else-if="activeMode === 'calendar'"
          key="calendar"
        />
      </Transition>
    </div>

    <!-- Streamlined Footer: Synchronized info only (Token Expiry removed) -->
    <div class="mo-minimal-footer border-t border-border/50 pt-1.5 flex items-center justify-between text-xs text-muted-foreground shrink-0">
      <div class="flex items-center gap-1.5">
        <CheckCircle2 class="h-3.5 w-3.5 text-emerald-500" />
        <span class="text-[11px] font-medium">Synchronized</span>
      </div>

      <div class="flex items-center gap-2 text-[11px]">
        <span v-if="activeMode === 'analog'" class="text-muted-foreground/80 font-mono">
          {{ smoothAnimation ? "Sweep: 60fps" : "Tick: 1s" }}
        </span>
        <span v-else-if="activeMode === 'digital'" class="text-muted-foreground/80">
          Live Clock
        </span>
        <span v-else class="text-muted-foreground/80">
          Interactive Calendar
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.clock-fade-enter-active,
.clock-fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.clock-fade-enter-from {
  opacity: 0;
  transform: scale(0.97);
}

.clock-fade-leave-to {
  opacity: 0;
  transform: scale(1.03);
}
</style>

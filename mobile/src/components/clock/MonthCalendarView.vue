<script setup lang="ts">
import { computed, ref } from "vue";
import {
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  RotateCcw,
} from "@lucide/vue";

const realToday = new Date();
const currentYear = ref(realToday.getFullYear());
const currentMonth = ref(realToday.getMonth()); // 0-indexed (0 = Jan)
const selectedDay = ref<number | null>(realToday.getDate());

// Slide transition direction
const slideDirection = ref<"left" | "right">("left");

const monthNames = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

const weekDays = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

const currentMonthName = computed(() => monthNames[currentMonth.value]);

const isViewingRealCurrentMonth = computed(() => {
  return (
    currentYear.value === realToday.getFullYear() &&
    currentMonth.value === realToday.getMonth()
  );
});

function goToToday(): void {
  slideDirection.value =
    currentYear.value > realToday.getFullYear() ||
    (currentYear.value === realToday.getFullYear() &&
      currentMonth.value > realToday.getMonth())
      ? "right"
      : "left";
  currentYear.value = realToday.getFullYear();
  currentMonth.value = realToday.getMonth();
  selectedDay.value = realToday.getDate();
}

function prevMonth(): void {
  slideDirection.value = "right";
  if (currentMonth.value === 0) {
    currentMonth.value = 11;
    currentYear.value -= 1;
  } else {
    currentMonth.value -= 1;
  }
}

function nextMonth(): void {
  slideDirection.value = "left";
  if (currentMonth.value === 11) {
    currentMonth.value = 0;
    currentYear.value += 1;
  } else {
    currentMonth.value += 1;
  }
}

function prevYear(): void {
  slideDirection.value = "right";
  currentYear.value -= 1;
}

function nextYear(): void {
  slideDirection.value = "left";
  currentYear.value += 1;
}

interface CalendarCell {
  day: number;
  isCurrentMonth: boolean;
  isToday: boolean;
  key: string;
}

const calendarGrid = computed<CalendarCell[]>(() => {
  const year = currentYear.value;
  const month = currentMonth.value;

  const firstDayOfWeek = new Date(year, month, 1).getDay(); // 0 = Sun
  const daysInCurrentMonth = new Date(year, month + 1, 0).getDate();
  const daysInPrevMonth = new Date(year, month, 0).getDate();

  const cells: CalendarCell[] = [];

  // Previous month trailing days
  for (let i = firstDayOfWeek - 1; i >= 0; i--) {
    const day = daysInPrevMonth - i;
    cells.push({
      day,
      isCurrentMonth: false,
      isToday: false,
      key: `prev-${day}`,
    });
  }

  // Current month days
  for (let d = 1; d <= daysInCurrentMonth; d++) {
    const isToday =
      isViewingRealCurrentMonth.value && d === realToday.getDate();
    cells.push({
      day: d,
      isCurrentMonth: true,
      isToday,
      key: `curr-${d}`,
    });
  }

  // Next month leading days to complete grid (up to 35 or 42 cells)
  const totalCells = cells.length > 35 ? 42 : 35;
  const remaining = totalCells - cells.length;
  for (let d = 1; d <= remaining; d++) {
    cells.push({
      day: d,
      isCurrentMonth: false,
      isToday: false,
      key: `next-${d}`,
    });
  }

  return cells;
});
</script>

<template>
  <div class="mo-calendar-view flex h-full w-full max-w-sm mx-auto flex-col justify-center select-none p-1 my-auto overflow-hidden">
    <!-- Month & Year Navigation Header -->
    <div class="flex items-center justify-between border-b border-border/40 pb-1 px-1 shrink-0">
      <!-- Year & Month Backward -->
      <div class="flex items-center gap-0.5">
        <button
          type="button"
          class="flex h-5 w-5 sm:h-6 sm:w-6 items-center justify-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-foreground active:scale-90"
          title="Previous Year"
          aria-label="Previous Year"
          @click="prevYear"
        >
          <ChevronsLeft class="h-3 w-3 sm:h-3.5 sm:w-3.5" />
        </button>
        <button
          type="button"
          class="flex h-5 w-5 sm:h-6 sm:w-6 items-center justify-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-foreground active:scale-90"
          title="Previous Month"
          aria-label="Previous Month"
          @click="prevMonth"
        >
          <ChevronLeft class="h-3 w-3 sm:h-3.5 sm:w-3.5" />
        </button>
      </div>

      <!-- Month & Year Title + Today Reset -->
      <div class="flex items-center gap-1 sm:gap-1.5 min-w-0">
        <span class="text-xs sm:text-sm font-bold tracking-tight text-foreground font-mono truncate">
          {{ currentMonthName }} {{ currentYear }}
        </span>

        <button
          v-if="!isViewingRealCurrentMonth"
          type="button"
          class="inline-flex items-center gap-1 rounded-full bg-primary/10 px-1.5 py-0.5 text-[9px] sm:text-[10px] font-semibold text-primary transition hover:bg-primary/20 active:scale-95 animate-in fade-in zoom-in-95 duration-200 shrink-0"
          title="Reset to Today"
          @click="goToToday"
        >
          <RotateCcw class="h-2.5 w-2.5" />
          <span class="hidden sm:inline">Today</span>
        </button>
      </div>

      <!-- Month & Year Forward -->
      <div class="flex items-center gap-0.5">
        <button
          type="button"
          class="flex h-5 w-5 sm:h-6 sm:w-6 items-center justify-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-foreground active:scale-90"
          title="Next Month"
          aria-label="Next Month"
          @click="nextMonth"
        >
          <ChevronRight class="h-3 w-3 sm:h-3.5 sm:w-3.5" />
        </button>
        <button
          type="button"
          class="flex h-5 w-5 sm:h-6 sm:w-6 items-center justify-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-foreground active:scale-90"
          title="Next Year"
          aria-label="Next Year"
          @click="nextYear"
        >
          <ChevronsRight class="h-3 w-3 sm:h-3.5 sm:w-3.5" />
        </button>
      </div>
    </div>

    <!-- Calendar Table Grid -->
    <div class="flex-1 flex flex-col justify-center min-h-0 pt-1">
      <!-- Weekday Headers -->
      <div class="grid grid-cols-7 text-center pb-0.5">
        <span
          v-for="dayName in weekDays"
          :key="dayName"
          class="text-[9px] sm:text-[11px] font-semibold text-muted-foreground/70 uppercase tracking-wider"
        >
          {{ dayName }}
        </span>
      </div>

      <!-- Days Grid with Transition -->
      <div class="grid grid-cols-7 gap-y-0.5 sm:gap-y-1 text-center flex-1 items-center">
        <div
          v-for="cell in calendarGrid"
          :key="cell.key"
          class="flex items-center justify-center py-0.5"
        >
          <button
            type="button"
            class="relative flex h-[clamp(1.3rem,4.5vw,1.75rem)] w-[clamp(1.3rem,4.5vw,1.75rem)] items-center justify-center rounded-full text-[clamp(0.65rem,2.2vw,0.75rem)] font-medium transition-all duration-150"
            :class="[
              cell.isToday
                ? 'bg-primary text-primary-foreground font-bold shadow-xs scale-105 ring-2 ring-primary/30'
                : cell.isCurrentMonth
                ? 'text-foreground hover:bg-accent hover:text-accent-foreground'
                : 'text-muted-foreground/30 pointer-events-none',
            ]"
            :disabled="!cell.isCurrentMonth"
          >
            {{ cell.day }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>


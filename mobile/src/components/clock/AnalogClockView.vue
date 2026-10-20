<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { Play, Sparkles } from "@lucide/vue";

const props = withDefaults(
  defineProps<{
    smoothAnimation?: boolean;
  }>(),
  {
    smoothAnimation: true,
  }
);

const emit = defineEmits<{
  (e: "toggle-animation"): void;
}>();

const now = ref(new Date());
let rafId: number | undefined;

// Cumulative angles to ensure monotonic clockwise rotation (no 354° -> 0° reverse spin)
const secondAngle = ref(0);
const minuteAngle = ref(0);
const hourAngle = ref(0);

let prevSecond = -1;
let secondRevs = 0;
let prevMinute = -1;
let minuteRevs = 0;
let prevHour = -1;
let hourRevs = 0;

function updateAngles(): void {
  const d = new Date();
  now.value = d;
  const ms = d.getMilliseconds();
  const s = d.getSeconds();
  const m = d.getMinutes();
  const h = d.getHours() % 12;

  // Track revolutions for monotonic clockwise motion
  if (prevSecond === 59 && s === 0) {
    secondRevs++;
  } else if (prevSecond === -1) {
    secondRevs = 0;
  }
  prevSecond = s;

  if (prevMinute === 59 && m === 0) {
    minuteRevs++;
  } else if (prevMinute === -1) {
    minuteRevs = 0;
  }
  prevMinute = m;

  if (prevHour === 11 && h === 0) {
    hourRevs++;
  } else if (prevHour === -1) {
    hourRevs = 0;
  }
  prevHour = h;

  if (props.smoothAnimation) {
    // 60fps continuous fluid sweep
    const secWithMs = s + ms / 1000;
    secondAngle.value = secondRevs * 360 + secWithMs * 6;
    minuteAngle.value = minuteRevs * 360 + (m + secWithMs / 60) * 6;
    hourAngle.value = hourRevs * 360 + (h + (m + secWithMs / 60) / 60) * 30;
  } else {
    // Mechanical Tick: Exact, frame-synchronized step with physical gear ratios
    secondAngle.value = secondRevs * 360 + s * 6;
    minuteAngle.value = minuteRevs * 360 + (m + s / 60) * 6;
    hourAngle.value = hourRevs * 360 + (h + m / 60 + s / 3600) * 30;
  }

  rafId = requestAnimationFrame(updateAngles);
}

function startClock(): void {
  stopClock();
  rafId = requestAnimationFrame(updateAngles);
}

function stopClock(): void {
  if (rafId) {
    cancelAnimationFrame(rafId);
    rafId = undefined;
  }
}

watch(
  () => props.smoothAnimation,
  () => {
    startClock();
  }
);

onMounted(() => {
  startClock();
});

onBeforeUnmount(() => {
  stopClock();
});

// Mathematically exact 12-hour positions on circle r = 68 from center (100, 100)
// angle = (hour * 30 * PI) / 180; x = 100 + 68*sin(angle); y = 100 - 68*cos(angle)
const hourNumbers = [
  { label: "12", x: 100, y: 32, isCardinal: true },
  { label: "1", x: 134, y: 41.1, isCardinal: false },
  { label: "2", x: 158.9, y: 66, isCardinal: false },
  { label: "3", x: 168, y: 100, isCardinal: true },
  { label: "4", x: 158.9, y: 134, isCardinal: false },
  { label: "5", x: 134, y: 158.9, isCardinal: false },
  { label: "6", x: 100, y: 168, isCardinal: true },
  { label: "7", x: 66, y: 158.9, isCardinal: false },
  { label: "8", x: 41.1, y: 134, isCardinal: false },
  { label: "9", x: 32, y: 100, isCardinal: true },
  { label: "10", x: 41.1, y: 66, isCardinal: false },
  { label: "11", x: 66, y: 41.1, isCardinal: false },
];

// 12 hour tick marks radiating outwards
const hourTicks = Array.from({ length: 12 }).map((_, i) => {
  const angle = (i * 30 * Math.PI) / 180;
  const r1 = 80;
  const r2 = 89;
  return {
    x1: 100 + r1 * Math.sin(angle),
    y1: 100 - r1 * Math.cos(angle),
    x2: 100 + r2 * Math.sin(angle),
    y2: 100 - r2 * Math.cos(angle),
    isMain: i % 3 === 0,
  };
});

// 60 minute ticks (subtle dots along perimeter at r = 84.5)
const minuteTicks = Array.from({ length: 60 })
  .map((_, i) => {
    if (i % 5 === 0) return null; // skip hour ticks
    const angle = (i * 6 * Math.PI) / 180;
    const r = 84.5;
    return {
      cx: 100 + r * Math.sin(angle),
      cy: 100 - r * Math.cos(angle),
    };
  })
  .filter(Boolean) as { cx: number; cy: number }[];

const dateString = computed(() => {
  return now.value.toLocaleDateString([], {
    month: "short",
    day: "numeric",
  }).toUpperCase();
});

const digitalString = computed(() => {
  return now.value.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
});
</script>

<template>
  <div class="mo-analog-clock-root flex h-full w-full flex-col items-center justify-between select-none py-1">
    <!-- SVG Clock Dial -->
    <div class="relative flex flex-1 items-center justify-center min-h-0 w-full py-1">
      <svg
        viewBox="0 0 200 200"
        class="h-full max-h-[195px] sm:max-h-[210px] aspect-square drop-shadow-sm select-none"
      >
        <!-- Bezel & Background Face -->
        <circle
          cx="100"
          cy="100"
          r="95"
          class="fill-card stroke-border stroke-[2.5]"
        />
        <circle
          cx="100"
          cy="100"
          r="91"
          class="fill-none stroke-border/40 stroke-[0.75]"
        />

        <!-- Minute Tick Dots (60 total) -->
        <circle
          v-for="(dot, idx) in minuteTicks"
          :key="`min-${idx}`"
          :cx="dot.cx"
          :cy="dot.cy"
          r="0.85"
          class="fill-muted-foreground/40"
        />

        <!-- 12 Hour Ticks -->
        <line
          v-for="(tick, idx) in hourTicks"
          :key="`tick-${idx}`"
          :x1="tick.x1"
          :y1="tick.y1"
          :x2="tick.x2"
          :y2="tick.y2"
          :class="tick.isMain ? 'stroke-foreground stroke-[2.5]' : 'stroke-muted-foreground/70 stroke-[1.5]'"
          stroke-linecap="round"
        />

        <!-- All 12 Numbers (Mathematically aligned) -->
        <text
          v-for="num in hourNumbers"
          :key="`num-${num.label}`"
          :x="num.x"
          :y="num.y"
          text-anchor="middle"
          dominant-baseline="central"
          class="fill-foreground font-mono font-bold pointer-events-none"
          :class="num.isCardinal ? 'text-[11px]' : 'text-[9px] fill-muted-foreground/80 font-semibold'"
        >
          {{ num.label }}
        </text>

        <!-- Date Complication Box (Centered at 6 o'clock above numeral 6) -->
        <g transform="translate(82, 133)">
          <rect
            x="0"
            y="0"
            width="36"
            height="15"
            rx="3"
            class="fill-muted/80 stroke-border/80 stroke-[0.75]"
          />
          <text
            x="18"
            y="7.5"
            text-anchor="middle"
            dominant-baseline="central"
            class="fill-foreground text-[8px] font-mono font-bold tracking-tight"
          >
            {{ dateString }}
          </text>
        </g>

        <!-- Hands Container (Centered exactly at 100, 100) -->

        <!-- Hour Hand: points accurately from (100, 100) towards tip -->
        <g
          :transform="`rotate(${hourAngle}, 100, 100)`"
          :class="!smoothAnimation ? 'transition-transform duration-75 ease-out' : ''"
        >
          <line
            x1="100"
            y1="107"
            x2="100"
            y2="52"
            class="stroke-foreground stroke-[3.5]"
            stroke-linecap="round"
          />
        </g>

        <!-- Minute Hand: points accurately to exact minute ticks -->
        <g
          :transform="`rotate(${minuteAngle}, 100, 100)`"
          :class="!smoothAnimation ? 'transition-transform duration-75 ease-out' : ''"
        >
          <line
            x1="100"
            y1="109"
            x2="100"
            y2="30"
            class="stroke-foreground/90 stroke-[2.5]"
            stroke-linecap="round"
          />
        </g>

        <!-- Second Hand with Counterweight Tail & Needle Tip -->
        <g
          :transform="`rotate(${secondAngle}, 100, 100)`"
          :class="!smoothAnimation ? 'transition-transform duration-75 cubic-bezier(0.2, 0, 0, 1)' : ''"
        >
          <!-- Needle Hand -->
          <line
            x1="100"
            y1="118"
            x2="100"
            y2="16"
            class="stroke-primary stroke-[1.25]"
            stroke-linecap="round"
          />
          <!-- Counterweight circle -->
          <circle cx="100" cy="112" r="2.5" class="fill-primary" />
        </g>

        <!-- Center Pivot Layered Cap -->
        <circle cx="100" cy="100" r="4.5" class="fill-foreground stroke-card stroke-[1]" />
        <circle cx="100" cy="100" r="2" class="fill-primary" />
      </svg>
    </div>

    <!-- Bottom Controls & Time Readout -->
    <div class="flex items-center justify-between w-full px-2 pt-1 border-t border-border/30 text-xs">
      <div class="flex items-center gap-1.5 font-mono font-semibold text-foreground">
        <span>{{ digitalString }}</span>
      </div>

      <!-- Animation Mode Toggle Button -->
      <button
        type="button"
        class="inline-flex items-center gap-1 rounded-md border border-border/60 bg-muted/30 px-2 py-0.5 text-[11px] font-medium text-muted-foreground transition hover:bg-accent hover:text-foreground active:scale-95"
        title="Toggle between Smooth 60fps Sweep and Mechanical 1-second Tick"
        @click="emit('toggle-animation')"
      >
        <Sparkles v-if="smoothAnimation" class="h-3 w-3 text-amber-500 animate-pulse" />
        <Play v-else class="h-3 w-3 text-primary" />
        <span>{{ smoothAnimation ? "Smooth Sweep" : "Mechanical Tick" }}</span>
      </button>
    </div>
  </div>
</template>

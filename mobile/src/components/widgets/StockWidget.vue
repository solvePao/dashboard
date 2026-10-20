<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  ArrowDown,
  ArrowLeft,
  ArrowUp,
  ChevronRight,
  RefreshCw,
  Terminal,
} from "@lucide/vue";
import {
  fetchMultipleStockQuotes,
  formatCompactNumber,
  formatQuoteChange,
  formatQuotePrice,
  getCachedQuote,
  type StockPeriod,
  type StockQuoteData,
} from "../../services/stockApi";
import type { StockSymbolItem, StockWidgetConfig } from "../../types/widget";

const props = defineProps<{
  config: StockWidgetConfig;
}>();

const PERIOD_STORAGE_KEY = "mo_stock_selected_period";
const savedPeriod = (typeof localStorage !== "undefined" && localStorage.getItem(PERIOD_STORAGE_KEY)) as StockPeriod | null;
const validPeriods: StockPeriod[] = ["1D", "5D", "1W", "1M", "3M", "6M", "YTD", "1Y"];
const selectedPeriod = ref<StockPeriod>(
  savedPeriod && validPeriods.includes(savedPeriod) ? savedPeriod : "1D"
);

watch(selectedPeriod, (newPeriod) => {
  try {
    localStorage.setItem(PERIOD_STORAGE_KEY, newPeriod);
  } catch {
    // Ignore storage issues
  }
});

const periodOptions: { value: StockPeriod; label: string }[] = [
  { value: "1D", label: "1D Change" },
  { value: "5D", label: "5D Change" },
  { value: "1W", label: "1W Change" },
  { value: "1M", label: "1M Change" },
  { value: "3M", label: "3M Change" },
  { value: "6M", label: "6M Change" },
  { value: "YTD", label: "YTD Change" },
  { value: "1Y", label: "1Y Change" },
];

function getQuoteChangePercent(quote: StockQuoteData | null | undefined, period: StockPeriod): number | null {
  if (!quote) return null;
  if (period === "1D") return quote.changePercent;
  const val = quote.perf?.[period];
  return typeof val === "number" && Number.isFinite(val) ? val : null;
}

function getQuoteNetChange(quote: StockQuoteData | null | undefined, period: StockPeriod): number | null {
  if (!quote) return null;
  if (period === "1D") return quote.change;
  const pct = getQuoteChangePercent(quote, period);
  if (pct === null) return null;
  const p0 = quote.price / (1 + pct / 100);
  return quote.price - p0;
}

const inspectedSymbol = ref<string | null>(null);
const quotes = ref<Record<string, StockQuoteData>>({});
const previousPrices = ref<Record<string, number>>({});
const flashStates = ref<Record<string, "up" | "down" | "neutral" | null>>({});
const loading = ref(true);
const error = ref<string | null>(null);

let aborter: AbortController | null = null;
let refreshTimer: ReturnType<typeof setInterval> | undefined;
const flashTimers = new Map<string, ReturnType<typeof setTimeout>>();

const REFRESH_MS = 5 * 60 * 1000; // 5 minutes

function triggerPriceFlash(sym: string, type: "up" | "down" | "neutral"): void {
  const upper = sym.toUpperCase();
  const existingTimer = flashTimers.get(upper);
  if (existingTimer) clearTimeout(existingTimer);

  flashStates.value[upper] = type;
  const timer = setTimeout(() => {
    flashStates.value[upper] = null;
    flashTimers.delete(upper);
  }, 900);
  flashTimers.set(upper, timer);
}

function getFlashClass(sym: string): string {
  const state = flashStates.value[sym.toUpperCase()];
  if (state === "up") return "mo-flash-up";
  if (state === "down") return "mo-flash-down";
  if (state === "neutral") return "mo-flash-neutral";
  return "";
}

function applyQuotesWithFlash(newQuotes: Record<string, StockQuoteData>, isInitial = false): void {
  for (const [sym, quote] of Object.entries(newQuotes)) {
    const prev = previousPrices.value[sym];
    if (prev !== undefined && !isInitial) {
      if (quote.price > prev) {
        triggerPriceFlash(sym, "up");
      } else if (quote.price < prev) {
        triggerPriceFlash(sym, "down");
      } else {
        triggerPriceFlash(sym, "neutral");
      }
    } else if (!isInitial) {
      triggerPriceFlash(sym, "neutral");
    }
    previousPrices.value[sym] = quote.price;
  }
  quotes.value = { ...quotes.value, ...newQuotes };
}

const symbolsList = computed<StockSymbolItem[]>(() => {
  if (props.config.symbols && props.config.symbols.length > 0) {
    return props.config.symbols.slice(0, 4);
  }
  return [
    {
      symbol: props.config.symbol || "AAPL",
      name: props.config.name,
    },
  ];
});

// Currently inspected symbol in full view
const currentSymbol = computed<StockSymbolItem>(() => {
  if (inspectedSymbol.value) {
    const found = symbolsList.value.find(
      (s) => s.symbol.toUpperCase() === inspectedSymbol.value?.toUpperCase()
    );
    if (found) return found;
  }
  return symbolsList.value[0];
});

const currentQuote = computed<StockQuoteData | null>(() => {
  return quotes.value[currentSymbol.value.symbol.toUpperCase()] || null;
});

const currentChangePercent = computed<number | null>(() => {
  return getQuoteChangePercent(currentQuote.value, selectedPeriod.value);
});

const currentNetChange = computed<number | null>(() => {
  return getQuoteNetChange(currentQuote.value, selectedPeriod.value);
});

const isCurrentPositive = computed(() => {
  const p = currentChangePercent.value;
  return p !== null && p >= 0;
});

const dayRangePercent = computed(() => {
  if (!currentQuote.value) return 50;
  const { price, dayHigh, dayLow } = currentQuote.value;
  if (dayHigh <= dayLow) return 50;
  const pct = ((price - dayLow) / (dayHigh - dayLow)) * 100;
  return Math.min(100, Math.max(0, pct));
});

const daySpread = computed(() => {
  if (!currentQuote.value) return 0;
  return Math.max(0, currentQuote.value.dayHigh - currentQuote.value.dayLow);
});

// At most four symbols are displayed; saved legacy watchlists remain intact.
const gridClass = computed(() =>
  symbolsList.value.length <= 2 ? "grid-cols-2 grid-rows-1" : "grid-cols-2 grid-rows-2"
);

function getQuoteFor(sym: string): StockQuoteData | null {
  return quotes.value[sym.toUpperCase()] || null;
}

function isTilePositive(sym: string): boolean {
  const q = getQuoteFor(sym);
  const p = getQuoteChangePercent(q, selectedPeriod.value);
  return p !== null && p >= 0;
}

function getTileBorderClass(sym: string): string {
  const q = getQuoteFor(sym);
  if (!q) return "";
  const p = getQuoteChangePercent(q, selectedPeriod.value);
  if (p === null) return "border-l-2 border-l-zinc-700";
  return p >= 0 ? "border-l-2 border-l-emerald-500" : "border-l-2 border-l-rose-500";
}

function getTileBadgeClass(sym: string): string {
  const q = getQuoteFor(sym);
  if (!q) return "bg-zinc-800 text-zinc-400";
  const p = getQuoteChangePercent(q, selectedPeriod.value);
  if (p === null) return "bg-zinc-800 text-zinc-400";
  return p >= 0
    ? "bg-emerald-500 text-black shadow-emerald-500/20"
    : "bg-rose-500 text-white shadow-rose-500/20";
}

function selectSymbol(sym: string): void {
  inspectedSymbol.value = sym;
}

function backToGrid(): void {
  inspectedSymbol.value = null;
}

function nextInspectedSymbol(): void {
  const list = symbolsList.value;
  if (list.length <= 1) return;
  const idx = list.findIndex(
    (s) => s.symbol.toUpperCase() === currentSymbol.value.symbol.toUpperCase()
  );
  const nextIdx = (idx + 1) % list.length;
  inspectedSymbol.value = list[nextIdx].symbol;
}

async function load(force = false): Promise<void> {
  const symbols = symbolsList.value.map((s) => s.symbol);
  if (symbols.length === 0) return;

  aborter?.abort();
  aborter = new AbortController();

  let hasAllCached = true;
  const cachedBatch: Record<string, StockQuoteData> = {};
  for (const s of symbols) {
    const c = getCachedQuote(s);
    if (c) {
      cachedBatch[s.toUpperCase()] = c;
    } else {
      hasAllCached = false;
    }
  }

  if (Object.keys(cachedBatch).length > 0) {
    applyQuotesWithFlash(cachedBatch, true);
  }

  if (hasAllCached && !force) {
    loading.value = false;
    error.value = null;
    return;
  }

  if (Object.keys(quotes.value).length === 0) {
    loading.value = true;
  }
  error.value = null;

  try {
    const fresh = await fetchMultipleStockQuotes(symbols, aborter.signal, force);
    if (Object.keys(fresh).length === 0 && Object.keys(quotes.value).length === 0) {
      throw new Error("Quotes unavailable. Please try refreshing.");
    }
    applyQuotesWithFlash(fresh, false);
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") return;
    if (Object.keys(quotes.value).length === 0) {
      error.value = err instanceof Error ? err.message : "TERMINAL ERROR: Quotes unavailable.";
    }
  } finally {
    loading.value = false;
  }
}

function restartTimer(): void {
  if (refreshTimer) clearInterval(refreshTimer);
  refreshTimer = setInterval(() => {
    if (document.visibilityState === "visible") {
      void load(true);
    }
  }, REFRESH_MS);
}

function onVisibilityChange(): void {
  if (document.visibilityState === "visible") {
    void load();
  }
}

onMounted(() => {
  void load();
  restartTimer();
  document.addEventListener("visibilitychange", onVisibilityChange);
});

watch(
  () => symbolsList.value.map((s) => s.symbol).join(","),
  () => void load()
);

onBeforeUnmount(() => {
  aborter?.abort();
  if (refreshTimer) clearInterval(refreshTimer);
  document.removeEventListener("visibilitychange", onVisibilityChange);
  for (const timer of flashTimers.values()) {
    clearTimeout(timer);
  }
  flashTimers.clear();
});
</script>

<template>
  <div
    class="mo-stock-root min-h-0 min-w-0 relative flex h-full w-full flex-col justify-between overflow-hidden p-2.5 text-zinc-100 font-mono select-none bg-[#090a0f] border border-amber-500/20 shadow-[inset_0_0_24px_rgba(245,158,11,0.02)]"
  >
    <!-- Terminal Scanline Background Texture -->
    <div
      class="pointer-events-none absolute inset-0 opacity-[0.02] bg-[radial-gradient(#fff_1px,transparent_1px)] [background-size:12px_12px]"
    />

    <!-- Initial Loading State -->
    <div v-if="loading && Object.keys(quotes).length === 0" class="mo-stock-loading flex h-full flex-col justify-between animate-pulse">
      <div class="flex items-center justify-between border-b border-zinc-800 pb-2">
        <div class="h-4 w-28 bg-zinc-800 rounded-xs" />
        <div class="h-4 w-16 bg-zinc-800 rounded-xs" />
      </div>
      <div class="my-auto py-4 space-y-3">
        <div class="h-10 w-44 bg-zinc-800 rounded-xs" />
        <div class="h-4 w-32 bg-zinc-850 rounded-xs" />
      </div>
      <div class="grid grid-cols-3 gap-1.5">
        <div v-for="i in 6" :key="i" class="h-12 bg-zinc-900 border border-zinc-800 rounded-xs" />
      </div>
    </div>

    <!-- Error State -->
    <div
      v-else-if="error && Object.keys(quotes).length === 0"
      class="mo-stock-error flex h-full flex-col items-center justify-center p-4 text-center font-mono"
    >
      <Terminal class="h-9 w-9 text-amber-500/80 mb-2 animate-pulse" />
      <p class="text-xs font-semibold text-amber-400 tracking-wider uppercase mb-1">
        SYS // CONNECTION REJECTED
      </p>
      <p class="text-[11px] text-zinc-400 max-w-[220px]">{{ error }}</p>
      <button
        type="button"
        class="mt-3 inline-flex items-center gap-1.5 border border-amber-500/40 bg-amber-500/10 px-3 py-1.5 text-[11px] font-bold text-amber-400 tracking-wider uppercase hover:bg-amber-500/20 active:scale-95 transition cursor-pointer"
        @click="load(true)"
      >
        <RefreshCw class="h-3 w-3" />
        <span>[RETRY &lt;GO&gt;]</span>
      </button>
    </div>

    <!-- MODE A & B WITH VIEW MORPH TRANSITION -->
    <Transition v-else name="stock-view-morph" mode="out-in">
      <!-- MODE A: FULL DETAILED BLOOMBERG TERMINAL VIEW (Single Symbol or Inspected Symbol) -->
      <div
        v-if="inspectedSymbol !== null || symbolsList.length === 1"
        key="detail-view"
        class="mo-stock-view flex min-h-0 min-w-0 h-full w-full flex-col gap-2 overflow-hidden"
      >
        <!-- 1. Top Terminal Bar -->
        <div class="mo-stock-header flex items-center justify-between gap-2 border-b border-zinc-800/80 pb-2 shrink-0">
          <!-- Left: Back Button (if multi-symbol) or Bloomberg Tag -->
          <div class="flex min-w-0 items-center gap-1.5">
            <button
              v-if="symbolsList.length > 1"
              type="button"
              class="inline-flex items-center gap-1 rounded-xs border border-amber-500/50 bg-amber-500/15 px-2 py-0.5 text-[10px] font-bold text-amber-400 uppercase tracking-wider hover:bg-amber-500/30 active:scale-95 transition cursor-pointer shrink-0"
              title="Return to Grid"
              @click="backToGrid"
            >
              <ArrowLeft class="h-3 w-3" />
              <span>[GRID]</span>
            </button>

            <span
              v-else
              class="inline-flex items-center gap-0.5 bg-amber-500 text-black px-1.5 py-0.5 text-[10px] font-black uppercase tracking-wider rounded-xs leading-none shadow-xs"
            >
              BBG
            </span>

            <span class="min-w-0 truncate font-bold text-xs tracking-wider text-amber-400"
              :title="currentSymbol.symbol">
              {{ currentSymbol.symbol }}
            </span>
            <span class="truncate text-[11px] text-zinc-400 max-w-[90px] uppercase">
              {{ currentQuote?.name || currentSymbol.name }}
            </span>

            <button
              v-if="symbolsList.length > 1"
              type="button"
              class="inline-flex items-center gap-0.5 rounded-xs border border-amber-500/40 bg-amber-500/10 px-1.5 py-0.5 text-[9px] font-bold text-amber-400 uppercase tracking-wider transition hover:bg-amber-500/25 active:scale-95 cursor-pointer ml-0.5 shrink-0"
              aria-label="Next symbol"
              title="Next symbol"
              @click="nextInspectedSymbol"
            >
              <ChevronRight class="h-2.5 w-2.5" />
            </button>
          </div>

          <!-- Right: Status & Actions -->
          <div class="mo-stock-actions flex flex-wrap items-center gap-2 text-[10px]">
            <div class="flex items-center gap-1.5 font-semibold">
              <span class="relative flex h-2 w-2">
                <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75 duration-1000" />
                <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
              </span>
              <span class="text-emerald-400 tracking-wider">LIVE</span>
            </div>

            <!-- Performance Period Dropdown -->
            <select
              v-model="selectedPeriod"
              class="mo-period-select rounded-xs border border-zinc-800 bg-zinc-900/90 px-1.5 py-0.5 text-[9px] font-mono font-bold text-amber-400 uppercase tracking-wider outline-none transition hover:border-amber-500/50 focus:border-amber-500 cursor-pointer"
              aria-label="Select performance period"
            >
              <option
                v-for="opt in periodOptions"
                :key="opt.value"
                :value="opt.value"
                class="bg-zinc-900 text-zinc-200"
              >
                {{ opt.label }}
              </option>
            </select>

            <span class="text-zinc-600">|</span>
            <span class="text-zinc-400 text-[10px]">{{ currentQuote?.lastUpdated || "--:--" }}</span>
            <button
              type="button"
              class="rounded-xs border border-zinc-800 bg-zinc-900/80 p-1 text-zinc-400 transition hover:border-amber-500/40 hover:text-amber-400 cursor-pointer"
              aria-label="Refresh quote"
              title="Refresh quote (F9)"
              :disabled="loading"
              @click="load(true)"
            >
              <RefreshCw class="h-3 w-3" :class="loading && 'animate-spin text-amber-400'" />
            </button>
          </div>
        </div>

        <!-- Inspected Symbol Content with Flip Transition -->
        <Transition name="stock-symbol-flip" mode="out-in">
          <div :key="currentSymbol.symbol" class="mo-stock-detail flex flex-1 flex-col gap-2 min-h-0 overflow-y-auto overscroll-contain">
            <p v-if="!currentQuote" class="py-4 text-xs text-zinc-400" role="status">
              Quote unavailable for {{ currentSymbol.symbol }}. Use refresh to try again.
            </p>
            <!-- 2. Hero: Monospace Price & Bloomberg Block Change -->
            <div
              v-if="currentQuote"
              class="mo-stock-hero my-auto flex items-center justify-between gap-3 py-1 rounded-xs p-1.5 transition-colors"
              :class="[getFlashClass(currentSymbol.symbol)]"
            >
              <div>
                <div class="flex items-baseline gap-1.5">
                  <span
                    class="mo-detail-price text-4xl font-black font-mono tracking-tight leading-none text-white drop-shadow-[0_2px_10px_rgba(255,255,255,0.1)]"
                  >
                    {{ formatQuotePrice(currentQuote.price) }}
                  </span>
                  <span class="text-xs font-bold text-amber-400 font-mono tracking-wider">
                    {{ currentQuote.currency }}
                  </span>
                </div>
                <p class="mt-1 text-[10px] uppercase tracking-wider text-zinc-400 font-medium truncate max-w-[200px]">
                  TICKER // {{ currentQuote.ticker }}
                </p>
              </div>

              <div class="text-right flex flex-col items-end">
                <div
                  class="inline-flex items-center gap-1 rounded-xs px-2.5 py-1 text-sm font-black font-mono tracking-tight shadow-md"
                  :class="
                    currentChangePercent === null
                      ? 'bg-zinc-800 text-zinc-400'
                      : isCurrentPositive
                        ? 'bg-emerald-500 text-black shadow-emerald-500/25'
                        : 'bg-rose-500 text-white shadow-rose-500/25'
                  "
                >
                  <component
                    v-if="currentChangePercent !== null"
                    :is="isCurrentPositive ? ArrowUp : ArrowDown"
                    class="h-4 w-4 stroke-[3]"
                  />
                  <span>
                    {{ selectedPeriod }}
                    <template v-if="currentChangePercent !== null">
                      {{ isCurrentPositive ? '+' : '' }}{{ currentChangePercent.toFixed(2) }}%
                    </template>
                    <template v-else>--</template>
                  </span>
                </div>
                <p
                  class="mt-1 text-xs font-mono font-bold tracking-tight"
                  :class="
                    currentChangePercent === null
                      ? 'text-zinc-500'
                      : isCurrentPositive
                        ? 'text-emerald-400'
                        : 'text-rose-400'
                  "
                >
                  {{ selectedPeriod }} NET:
                  <template v-if="currentNetChange !== null">
                    {{ formatQuoteChange(currentNetChange) }}
                  </template>
                  <template v-else>--</template>
                </p>
              </div>
            </div>

            <!-- 3. Day Range Bar -->
            <div v-if="currentQuote" class="mo-stock-range border border-zinc-800/80 bg-zinc-950/60 rounded-xs p-1.5 mb-1">
              <div class="flex items-center justify-between text-[10px] text-zinc-400 font-mono mb-1">
                <span><span class="text-zinc-400 font-bold">1D LO:</span> {{ formatQuotePrice(currentQuote.dayLow) }}</span>
                <span class="text-[9px] uppercase tracking-widest text-amber-400/90 font-bold">
                  1D SPREAD {{ formatQuotePrice(daySpread) }}
                </span>
                <span><span class="text-zinc-400 font-bold">1D HI:</span> {{ formatQuotePrice(currentQuote.dayHigh) }}</span>
              </div>
              <div class="relative h-2 w-full rounded-2xs bg-zinc-900 border border-zinc-800 overflow-hidden">
                <div
                  class="absolute top-0 bottom-0 transition-all duration-500"
                  :class="isCurrentPositive ? 'bg-emerald-500/40' : 'bg-rose-500/40'"
                  :style="{ width: `${dayRangePercent}%` }"
                />
                <div
                  class="mo-range-needle absolute top-0 bottom-0 w-1 bg-amber-400 shadow-[0_0_8px_rgba(245,158,11,1)] -translate-x-1/2 transition-all duration-500"
                  :style="{ left: `${dayRangePercent}%` }"
                />
              </div>
            </div>

            <!-- 4. Dense Bloomberg Matrix Grid -->
            <div v-if="currentQuote" class="mo-stock-grid grid grid-cols-3 gap-1.5 pb-2">
              <div class="mo-matrix-card border border-zinc-800/90 bg-zinc-950/70 p-1.5 rounded-xs flex flex-col justify-between" style="--i: 0">
                <span class="text-[9px] uppercase tracking-wider text-zinc-400 font-bold">OPEN</span>
                <p class="mt-0.5 text-xs font-bold font-mono tracking-tight text-zinc-100">
                  {{ formatQuotePrice(currentQuote.open) }}
                </p>
              </div>
              <div class="mo-matrix-card border border-zinc-800/90 bg-zinc-950/70 p-1.5 rounded-xs flex flex-col justify-between" style="--i: 1">
                <span class="text-[9px] uppercase tracking-wider text-zinc-400 font-bold">PREV CLS</span>
                <p class="mt-0.5 text-xs font-bold font-mono tracking-tight text-zinc-100">
                  {{ formatQuotePrice(currentQuote.prevClose) }}
                </p>
              </div>
              <div class="mo-matrix-card border border-zinc-800/90 bg-zinc-950/70 p-1.5 rounded-xs flex flex-col justify-between" style="--i: 2">
                <span class="text-[9px] uppercase tracking-wider text-zinc-400 font-bold">DAY HI</span>
                <p class="mt-0.5 text-xs font-bold font-mono tracking-tight text-emerald-400">
                  {{ formatQuotePrice(currentQuote.dayHigh) }}
                </p>
              </div>
              <div class="mo-matrix-card border border-zinc-800/90 bg-zinc-950/70 p-1.5 rounded-xs flex flex-col justify-between" style="--i: 3">
                <span class="text-[9px] uppercase tracking-wider text-zinc-400 font-bold">DAY LO</span>
                <p class="mt-0.5 text-xs font-bold font-mono tracking-tight text-rose-400">
                  {{ formatQuotePrice(currentQuote.dayLow) }}
                </p>
              </div>
              <div class="mo-matrix-card border border-zinc-800/90 bg-zinc-950/70 p-1.5 rounded-xs flex flex-col justify-between" style="--i: 4">
                <span class="text-[9px] uppercase tracking-wider text-zinc-400 font-bold">VOLUME</span>
                <p class="mt-0.5 text-xs font-bold font-mono tracking-tight text-zinc-100">
                  {{ formatCompactNumber(currentQuote.volume) }}
                </p>
              </div>
              <div class="mo-matrix-card border border-zinc-800/90 bg-zinc-950/70 p-1.5 rounded-xs flex flex-col justify-between" style="--i: 5">
                <span class="text-[9px] uppercase tracking-wider text-zinc-400 font-bold">MKT CAP</span>
                <p class="mt-0.5 text-xs font-bold font-mono tracking-tight text-amber-400">
                  {{ formatCompactNumber(currentQuote.marketCap) }}
                </p>
              </div>
            </div>
          </div>
        </Transition>

        <!-- 5. Terminal Command Strip -->
        <div class="mo-stock-cmd border-t border-zinc-800/80 pt-1 flex items-center justify-between text-[9px] text-zinc-400 uppercase tracking-widest font-mono shrink-0">
          <span class="flex items-center gap-1">
            <span class="text-amber-500 font-bold">&lt;GO&gt;</span>
            <span>EQ // FEED: TV_SCAN</span>
          </span>
          <span class="text-zinc-400">
            5M SYNC
          </span>
        </div>
      </div>

      <!-- MODE B: MULTI-SYMBOL GRID (2 to 4 Tickers) -->
      <div
        v-else
        key="grid-view"
        class="mo-stock-view flex min-h-0 min-w-0 h-full w-full flex-col gap-2 overflow-hidden"
      >
        <!-- Grid Header Strip -->
        <div class="mo-stock-header flex items-center justify-between border-b border-zinc-800/80 pb-1.5 mb-1.5 shrink-0">
          <div class="flex items-center gap-1.5">
            <span class="inline-flex items-center gap-0.5 bg-amber-500 text-black px-1.5 py-0.5 text-[9px] font-black uppercase tracking-wider rounded-xs leading-none shadow-xs">
              BBG
            </span>
            <span class="text-amber-400 font-bold text-xs tracking-wider">
              MATRIX // {{ symbolsList.length }} TICKERS ({{ selectedPeriod }})
            </span>
          </div>

          <div class="mo-stock-actions flex flex-wrap items-center gap-2 text-[10px]">
            <div class="flex items-center gap-1.5 font-semibold">
              <span class="relative flex h-2 w-2">
                <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75 duration-1000" />
                <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
              </span>
              <span class="text-emerald-400 tracking-wider">LIVE</span>
            </div>

            <!-- Performance Period Dropdown -->
            <select
              v-model="selectedPeriod"
              class="mo-period-select rounded-xs border border-zinc-800 bg-zinc-900/90 px-1.5 py-0.5 text-[9px] font-mono font-bold text-amber-400 uppercase tracking-wider outline-none transition hover:border-amber-500/50 focus:border-amber-500 cursor-pointer"
              aria-label="Select performance period"
            >
              <option
                v-for="opt in periodOptions"
                :key="opt.value"
                :value="opt.value"
                class="bg-zinc-900 text-zinc-200"
              >
                {{ opt.label }}
              </option>
            </select>

            <span class="text-zinc-600">|</span>
            <button
              type="button"
              class="rounded-xs border border-zinc-800 bg-zinc-900/80 p-1 text-zinc-400 hover:text-amber-400 cursor-pointer"
              aria-label="Refresh all quotes"
              title="Refresh all (F9)"
              :disabled="loading"
              @click="load(true)"
            >
              <RefreshCw class="h-2.5 w-2.5" :class="loading && 'animate-spin text-amber-400'" />
            </button>
          </div>
        </div>

        <!-- Dynamic Adaptive Grid Container with TransitionGroup -->
        <TransitionGroup
          name="stock-grid-tile"
          tag="div"
          :class="['mo-stock-tiles grid flex-1 min-h-0 min-w-0 w-full gap-1', gridClass]"
        >
          <div
            v-for="item in symbolsList"
            :key="item.symbol"
            class="mo-stock-tile group relative flex min-h-0 min-w-0 flex-col justify-between overflow-hidden rounded-xs border border-zinc-800/90 bg-zinc-950/70 p-1.5 transition-all duration-200 hover:border-amber-500/60 hover:bg-zinc-900/90 hover:shadow-[0_0_12px_rgba(245,158,11,0.12)] cursor-pointer active:scale-[0.98]"
            :class="[
              getTileBorderClass(item.symbol),
              getFlashClass(item.symbol)
            ]"
            role="button"
            tabindex="0"
            :aria-label="`View ${item.symbol} details`"
            @keydown.enter="selectSymbol(item.symbol)"
            @keydown.space.prevent="selectSymbol(item.symbol)"
            @click="selectSymbol(item.symbol)"
          >
            <div class="mo-tile-heading flex min-w-0 items-center justify-between gap-1">
              <span class="mo-tile-symbol font-bold text-amber-400" :title="item.symbol">{{ item.symbol }}</span>
            </div>
            <p class="mo-tile-name truncate text-[10px] text-zinc-400" :title="getQuoteFor(item.symbol)?.name || item.name">
              {{ getQuoteFor(item.symbol)?.name || item.name }}
            </p>
            <div class="mo-tile-price font-black text-white">
              {{ formatQuotePrice(getQuoteFor(item.symbol)?.price) }}
              <span class="mo-tile-currency text-[9px] font-semibold text-amber-400">{{ getQuoteFor(item.symbol)?.currency }}</span>
            </div>
            <div class="mo-tile-performance flex flex-wrap items-center justify-between gap-1">
              <div class="mo-tile-change inline-flex w-fit max-w-full items-center gap-1 rounded-xs px-1.5 py-0.5 font-bold" :class="getTileBadgeClass(item.symbol)">
                <component
                  v-if="getQuoteChangePercent(getQuoteFor(item.symbol), selectedPeriod) !== null"
                  :is="isTilePositive(item.symbol) ? ArrowUp : ArrowDown"
                  class="h-3 w-3 shrink-0"
                />
                <span>{{ getQuoteChangePercent(getQuoteFor(item.symbol), selectedPeriod) === null ? '--' : `${isTilePositive(item.symbol) ? '+' : ''}${getQuoteChangePercent(getQuoteFor(item.symbol), selectedPeriod)!.toFixed(2)}%` }}</span>
              </div>
              <span
                class="mo-tile-net text-[10px] font-semibold"
                :class="getQuoteChangePercent(getQuoteFor(item.symbol), selectedPeriod) === null ? 'text-zinc-400' : isTilePositive(item.symbol) ? 'text-emerald-400' : 'text-rose-400'"
                :title="`${selectedPeriod} net change`"
              >
                {{ formatQuoteChange(getQuoteNetChange(getQuoteFor(item.symbol), selectedPeriod) ?? undefined) }}
              </span>
            </div>
            <dl class="mo-tile-range grid grid-cols-2 gap-1 border-t border-zinc-800/70 pt-1 text-[9px] leading-tight">
              <div class="min-w-0">
                <dt class="text-zinc-500">DAY LOW</dt>
                <dd class="text-zinc-300">{{ formatQuotePrice(getQuoteFor(item.symbol)?.dayLow) }}</dd>
              </div>
              <div class="min-w-0 text-right">
                <dt class="text-zinc-500">DAY HIGH</dt>
                <dd class="text-zinc-300">{{ formatQuotePrice(getQuoteFor(item.symbol)?.dayHigh) }}</dd>
              </div>
            </dl>
            <div class="mo-tile-volume flex items-center justify-between gap-1 text-[9px] text-zinc-400">
              <span>VOLUME</span>
              <span>{{ getQuoteFor(item.symbol) ? formatCompactNumber(getQuoteFor(item.symbol)!.volume) : '--' }}</span>
            </div>
          </div>
        </TransitionGroup>

        <!-- Bottom Command Strip -->
        <div class="border-t border-zinc-800/80 pt-1 mt-1 flex items-center justify-between text-[9px] text-zinc-400 uppercase tracking-widest font-mono shrink-0">
          <span class="flex items-center gap-1">
            <span class="text-amber-500 font-bold">&lt;GO&gt;</span>
            <span>TAP TILE FOR FULL VIEW</span>
          </span>
          <span class="text-zinc-400">
            5M SYNC
          </span>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.mo-stock-root {
  container: stock / size;
}
.mo-stock-header {
  flex-wrap: wrap;
  gap: 0.375rem;
}
.mo-stock-header > div {
  min-width: 0;
  max-width: 100%;
}
.mo-stock-actions {
  margin-left: auto;
}
.mo-stock-tiles {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}
.mo-stock-tile {
  gap: 0.25rem;
}
.mo-tile-symbol {
  font-size: clamp(0.625rem, 3cqw, 0.875rem);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.mo-tile-price {
  font-size: clamp(0.75rem, 4.5cqw, 1.5rem);
  line-height: 1.15;
  overflow-wrap: anywhere;
}
.mo-tile-name, .mo-tile-volume {
  display: none;
}
.mo-tile-range dd, .mo-tile-net {
  overflow-wrap: anywhere;
}
.mo-tile-change {
  font-size: clamp(0.625rem, 2.8cqw, 0.875rem);
  line-height: 1.2;
}
.mo-stock-detail > * {
  flex-shrink: 0;
  min-width: 0;
}
.mo-stock-hero {
  flex-wrap: wrap;
}
.mo-detail-price {
  font-size: clamp(1.25rem, 8cqw, 3rem);
  overflow-wrap: anywhere;
}
.mo-stock-hero > div, .mo-stock-hero > div > div {
  min-width: 0;
  flex-wrap: wrap;
}
.mo-stock-range > div:first-child {
  flex-wrap: wrap;
  gap: 0.25rem;
}
.mo-stock-grid {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}
.mo-matrix-card p {
  overflow-wrap: anywhere;
}
.mo-stock-loading, .mo-stock-error {
  min-height: 0;
  overflow-y: auto;
}
.mo-stock-loading > div > div {
  max-width: 100%;
}
@container stock (max-width: 440px) {
  .mo-stock-header { font-size: 0.625rem; }
  .mo-stock-header .tracking-wider { letter-spacing: normal; }
}
@container stock (max-width: 320px) {
  .mo-stock-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .mo-stock-hero { flex-direction: column; align-items: flex-start; }
  .mo-stock-hero > div:last-child { align-items: flex-start; text-align: left; }
}
@container stock (max-height: 320px) {
  .mo-stock-view { gap: 0.25rem; }
  .mo-stock-tile { padding: 0.25rem; gap: 0.125rem; }
}

@container stock (min-height: 340px) {
  .mo-tile-name { display: block; }
}
@container stock (min-height: 360px) {
  .mo-tile-volume { display: flex; }
}
@container stock (max-height: 280px) {
  .mo-tile-range { display: none; }
}
@container stock (max-height: 240px) {
  .mo-tile-net { display: none; }
}

/* 1. View Morph: Terminal Detail <-> Matrix Grid */
.stock-view-morph-enter-active,
.stock-view-morph-leave-active {
  transition: opacity 0.22s cubic-bezier(0.16, 1, 0.3, 1),
              transform 0.22s cubic-bezier(0.16, 1, 0.3, 1);
}
.stock-view-morph-enter-from {
  opacity: 0;
  transform: scale(0.97);
}
.stock-view-morph-leave-to {
  opacity: 0;
  transform: scale(1.02);
}

/* 2. Detail Symbol Flip / Slide */
.stock-symbol-flip-enter-active,
.stock-symbol-flip-leave-active {
  transition: opacity 0.16s ease-out,
              transform 0.16s cubic-bezier(0.16, 1, 0.3, 1);
}
.stock-symbol-flip-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.stock-symbol-flip-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

/* 3. Grid Tile Reordering and Entry */
.stock-grid-tile-move {
  transition: transform 0.32s cubic-bezier(0.2, 0, 0, 1);
}
.stock-grid-tile-enter-active,
.stock-grid-tile-leave-active {
  transition: opacity 0.24s ease, transform 0.24s cubic-bezier(0.2, 0, 0, 1);
}
.stock-grid-tile-enter-from,
.stock-grid-tile-leave-to {
  opacity: 0;
  transform: scale(0.92);
}

/* 4. Live Quote Price Flash Animations */
.mo-flash-up {
  animation: flash-green 0.9s cubic-bezier(0.2, 0.8, 0.2, 1);
}
.mo-flash-down {
  animation: flash-red 0.9s cubic-bezier(0.2, 0.8, 0.2, 1);
}
.mo-flash-neutral {
  animation: flash-neutral 0.9s cubic-bezier(0.2, 0.8, 0.2, 1);
}

@keyframes flash-green {
  0% {
    background-color: rgba(16, 185, 129, 0.28);
    box-shadow: inset 0 0 10px rgba(16, 185, 129, 0.4);
  }
  100% {
    background-color: transparent;
    box-shadow: none;
  }
}

@keyframes flash-red {
  0% {
    background-color: rgba(244, 63, 94, 0.28);
    box-shadow: inset 0 0 10px rgba(244, 63, 94, 0.4);
  }
  100% {
    background-color: transparent;
    box-shadow: none;
  }
}

@keyframes flash-neutral {
  0% {
    background-color: rgba(245, 158, 11, 0.2);
    box-shadow: inset 0 0 8px rgba(245, 158, 11, 0.3);
  }
  100% {
    background-color: transparent;
    box-shadow: none;
  }
}

/* 5. Intraday Range Needle Pulse */
.mo-range-needle {
  animation: needle-pulse 2s ease-in-out infinite alternate;
}
@keyframes needle-pulse {
  0% {
    box-shadow: 0 0 6px rgba(245, 158, 11, 0.6);
  }
  100% {
    box-shadow: 0 0 14px rgba(245, 158, 11, 1), 0 0 4px #ffffff;
  }
}

/* 6. Matrix Card Stagger Cascade */
.mo-matrix-card {
  animation: matrix-card-entry 0.28s cubic-bezier(0.16, 1, 0.3, 1) backwards;
  animation-delay: calc(var(--i, 0) * 35ms);
}
@keyframes matrix-card-entry {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 7. Accessibility: Reduced Motion Override */
@media (prefers-reduced-motion: reduce) {
  .stock-view-morph-enter-active,
  .stock-view-morph-leave-active,
  .stock-symbol-flip-enter-active,
  .stock-symbol-flip-leave-active,
  .stock-grid-tile-move,
  .stock-grid-tile-enter-active,
  .stock-grid-tile-leave-active,
  .mo-matrix-card,
  .mo-range-needle,
  .mo-flash-up,
  .mo-flash-down,
  .mo-flash-neutral {
    animation: none !important;
    transition: none !important;
  }
}
</style>

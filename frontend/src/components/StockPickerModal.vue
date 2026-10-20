<script setup lang="ts">
import AppDialog from "./AppDialog.vue";
import { nextTick, onBeforeUnmount, ref, watch, useTemplateRef } from "vue";
import {
  ArrowLeftRight,
  Check,
  Coins,
  Loader2,
  Search,
  TrendingDown,
  TrendingUp,
  X,
} from "@lucide/vue";
import {
  fetchStockQuote,
  formatQuoteChange,
  formatQuotePrice,
  type StockQuoteData,
} from "../services/stockApi";
import type { StockSymbolItem } from "../types/widget";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "select", symbolItem: StockSymbolItem): void;
}>();

type ActiveTab = "stock" | "forex";
const activeTab = ref<ActiveTab>("stock");

const query = ref("");
const fromCurrency = ref("USD");
const toCurrency = ref("INR");

const verifiedQuote = ref<StockQuoteData | null>(null);
const verifying = ref(false);
const error = ref<string | null>(null);
const inputRef = useTemplateRef<HTMLInputElement>("inputRef");

interface CurrencyOption {
  code: string;
  name: string;
  symbol: string;
}

const POPULAR_CURRENCIES: CurrencyOption[] = [
  { code: "USD", name: "US Dollar", symbol: "$" },
  { code: "INR", name: "Indian Rupee", symbol: "₹" },
  { code: "EUR", name: "Euro", symbol: "€" },
  { code: "GBP", name: "British Pound", symbol: "£" },
  { code: "JPY", name: "Japanese Yen", symbol: "¥" },
  { code: "CAD", name: "Canadian Dollar", symbol: "C$" },
  { code: "AUD", name: "Australian Dollar", symbol: "A$" },
  { code: "CHF", name: "Swiss Franc", symbol: "Fr" },
  { code: "CNY", name: "Chinese Yuan", symbol: "¥" },
  { code: "AED", name: "UAE Dirham", symbol: "د.إ" },
  { code: "SGD", name: "Singapore Dollar", symbol: "S$" },
  { code: "NZD", name: "New Zealand Dollar", symbol: "NZ$" },
  { code: "HKD", name: "Hong Kong Dollar", symbol: "HK$" },
];

const POPULAR_PAIRS = [
  { from: "USD", to: "INR" },
  { from: "EUR", to: "USD" },
  { from: "GBP", to: "USD" },
  { from: "USD", to: "JPY" },
  { from: "USD", to: "CAD" },
  { from: "EUR", to: "INR" },
];

let aborter: AbortController | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | undefined;

function clearVerification(): void {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = undefined;
  }
  aborter?.abort();
  aborter = null;
}

async function verifySymbol(sym: string): Promise<void> {
  clearVerification();
  const trimmed = sym.trim().toUpperCase();
  if (!trimmed) {
    verifiedQuote.value = null;
    error.value = null;
    verifying.value = false;
    return;
  }

  aborter = new AbortController();
  verifying.value = true;
  error.value = null;

  try {
    const quote = await fetchStockQuote(trimmed, aborter.signal, true);
    verifiedQuote.value = quote;
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") return;
    verifiedQuote.value = null;
    error.value = err instanceof Error ? err.message : "Symbol lookup failed.";
  } finally {
    verifying.value = false;
  }
}

function verifyCurrentForexPair(): void {
  if (fromCurrency.value === toCurrency.value) {
    error.value = "Please select two different currencies.";
    verifiedQuote.value = null;
    return;
  }
  const pairSym = `${fromCurrency.value}${toCurrency.value}`;
  void verifySymbol(pairSym);
}

function swapCurrencies(): void {
  const temp = fromCurrency.value;
  fromCurrency.value = toCurrency.value;
  toCurrency.value = temp;
  verifyCurrentForexPair();
}

function selectPreset(pair: { from: string; to: string }): void {
  fromCurrency.value = pair.from;
  toCurrency.value = pair.to;
  verifyCurrentForexPair();
}

watch(activeTab, (tab) => {
  clearVerification();
  verifiedQuote.value = null;
  error.value = null;
  if (tab === "stock") {
    query.value = "";
    void nextTick(() => inputRef.value?.focus());
  } else {
    verifyCurrentForexPair();
  }
});

watch(query, (newVal) => {
  if (activeTab.value !== "stock") return;
  verifiedQuote.value = null;
  error.value = null;
  if (debounceTimer) clearTimeout(debounceTimer);
  const trimmed = newVal.trim();
  if (trimmed.length >= 1) {
    debounceTimer = setTimeout(() => {
      void verifySymbol(trimmed);
    }, 450);
  }
});

watch([fromCurrency, toCurrency], () => {
  if (activeTab.value === "forex") {
    verifyCurrentForexPair();
  }
});

function handleAdd(): void {
  if (!verifiedQuote.value) return;
  emit("select", {
    symbol: verifiedQuote.value.symbol,
    name: verifiedQuote.value.name,
    currency: verifiedQuote.value.currency,
  });
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      activeTab.value = "stock";
      query.value = "";
      verifiedQuote.value = null;
      error.value = null;
      verifying.value = false;
      void nextTick(() => inputRef.value?.focus());
    } else {
      clearVerification();
    }
  }
);

onBeforeUnmount(() => {
  clearVerification();
});
</script>

<template>
  <AppDialog :open="open" title="Add market symbol" @close="emit('close')">
    <div class="fe-stockmodal-header flex items-center justify-between border-b border-border/80 pb-4">
      <div>
        <h2 id="stock-picker-title" class="text-base font-semibold leading-tight text-foreground">
          Add Market Symbol
        </h2>
        <p class="text-xs text-muted-foreground mt-0.5">
          Add equities, crypto, or live foreign exchange pairs
        </p>
      </div>
      <button
        type="button"
        class="rounded-lg p-1.5 text-muted-foreground transition hover:bg-accent hover:text-foreground cursor-pointer"
        aria-label="Close"
        @click="emit('close')"
      >
        <X class="h-4 w-4" />
      </button>
    </div>

    <!-- Segmented Tabs: Stock/Crypto vs. Currency Pair -->
    <div class="mt-4 flex rounded-xl bg-muted/60 p-1 border border-border/60">
      <button
        type="button"
        class="flex flex-1 items-center justify-center gap-1.5 rounded-lg py-1.5 text-xs font-semibold transition cursor-pointer"
        :class="
          activeTab === 'stock'
            ? 'bg-card text-foreground shadow-xs border border-border/60'
            : 'text-muted-foreground hover:text-foreground'
        "
        @click="activeTab = 'stock'"
      >
        <Search class="h-3.5 w-3.5" />
        <span>Stocks & Crypto</span>
      </button>
      <button
        type="button"
        class="flex flex-1 items-center justify-center gap-1.5 rounded-lg py-1.5 text-xs font-semibold transition cursor-pointer"
        :class="
          activeTab === 'forex'
            ? 'bg-card text-foreground shadow-xs border border-border/60'
            : 'text-muted-foreground hover:text-foreground'
        "
        @click="activeTab = 'forex'"
      >
        <Coins class="h-3.5 w-3.5 text-amber-500" />
        <span>Currency Pair</span>
      </button>
    </div>

    <!-- TAB 1: Stock / Crypto -->
    <div v-if="activeTab === 'stock'" class="mt-4">
      <div class="relative">
        <Search class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <input
          ref="inputRef"
          v-model="query"
          type="text"
          placeholder="e.g. AAPL, NVDA, BTCUSD, RELIANCE"
          autocomplete="off"
          class="w-full rounded-lg border border-input bg-background py-2.5 pl-9 pr-9 text-sm font-mono uppercase text-foreground placeholder:font-sans placeholder:normal-case placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          @keydown.enter.prevent="query.trim() && verifySymbol(query)"
        />
        <Loader2
          v-if="verifying"
          class="absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 animate-spin text-primary"
        />
      </div>
    </div>

    <!-- TAB 2: Currency Pair (Forex) -->
    <div v-else class="mt-4 space-y-3">
      <div class="flex items-center gap-2">
        <!-- From Currency -->
        <div class="flex-1">
          <label class="block text-[11px] font-semibold text-muted-foreground uppercase tracking-wider mb-1">
            From Currency
          </label>
          <select
            v-model="fromCurrency"
            class="w-full rounded-lg border border-input bg-background px-3 py-2 text-xs sm:text-sm font-medium text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring cursor-pointer"
          >
            <option v-for="c in POPULAR_CURRENCIES" :key="c.code" :value="c.code">
              {{ c.code }} - {{ c.name }} ({{ c.symbol }})
            </option>
          </select>
        </div>

        <!-- Swap Button -->
        <div class="pt-5 shrink-0">
          <button
            type="button"
            class="rounded-lg border border-border bg-muted/40 p-2 text-muted-foreground transition hover:bg-accent hover:text-foreground active:scale-95 cursor-pointer"
            title="Swap From and To currencies"
            @click="swapCurrencies"
          >
            <ArrowLeftRight class="h-4 w-4" />
          </button>
        </div>

        <!-- To Currency -->
        <div class="flex-1">
          <label class="block text-[11px] font-semibold text-muted-foreground uppercase tracking-wider mb-1">
            To Currency
          </label>
          <select
            v-model="toCurrency"
            class="w-full rounded-lg border border-input bg-background px-3 py-2 text-xs sm:text-sm font-medium text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring cursor-pointer"
          >
            <option v-for="c in POPULAR_CURRENCIES" :key="c.code" :value="c.code">
              {{ c.code }} - {{ c.name }} ({{ c.symbol }})
            </option>
          </select>
        </div>
      </div>

      <!-- Quick Preset Chips -->
      <div>
        <span class="text-[10px] uppercase font-bold text-muted-foreground tracking-wider block mb-1.5">
          Popular Pairs
        </span>
        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="pair in POPULAR_PAIRS"
            :key="`${pair.from}-${pair.to}`"
            type="button"
            class="rounded-md border border-border/70 bg-muted/30 px-2 py-1 text-[11px] font-mono font-medium text-foreground transition hover:border-primary/50 hover:bg-muted cursor-pointer"
            :class="
              fromCurrency === pair.from && toCurrency === pair.to
                ? 'border-primary bg-primary/10 text-primary font-bold'
                : ''
            "
            @click="selectPreset(pair)"
          >
            {{ pair.from }} → {{ pair.to }}
          </button>
        </div>
      </div>
    </div>

    <!-- Error State -->
    <div
      v-if="error"
      class="mt-3 rounded-xl border border-destructive/20 bg-destructive/10 p-3 text-xs text-destructive"
      role="alert"
    >
      {{ error }}
    </div>

    <!-- Verified Quote Preview Card -->
    <div
      v-else-if="verifiedQuote"
      class="mt-4 rounded-xl border border-border bg-muted/40 p-4 transition-all animate-in fade-in-50"
    >
      <div class="flex items-center justify-between">
        <div>
          <div class="flex items-center gap-2">
            <span class="text-base font-bold text-foreground font-mono">
              {{ verifiedQuote.symbol }}
            </span>
            <span class="text-[11px] font-medium text-muted-foreground uppercase px-1.5 py-0.5 bg-muted rounded border border-border font-mono">
              {{ verifiedQuote.currency }}
            </span>
          </div>
          <p class="text-xs text-muted-foreground truncate max-w-[220px]">
            {{ verifiedQuote.name }}
          </p>
        </div>

        <div class="text-right">
          <p class="text-base font-bold font-mono text-foreground">
            {{ formatQuotePrice(verifiedQuote.price) }}
          </p>
          <div
            class="inline-flex items-center gap-0.5 text-xs font-semibold"
            :class="verifiedQuote.change >= 0 ? 'text-emerald-500' : 'text-rose-500'"
          >
            <component :is="verifiedQuote.change >= 0 ? TrendingUp : TrendingDown" class="h-3 w-3" />
            <span>1D {{ formatQuoteChange(verifiedQuote.change) }} ({{ verifiedQuote.changePercent.toFixed(2) }}%)</span>
          </div>
        </div>
      </div>

      <div class="mt-4 pt-3 border-t border-border/60 flex items-center justify-between">
        <span class="text-[10px] text-muted-foreground">
          Feed: TradingView Scanner
        </span>
        <button
          type="button"
          class="inline-flex items-center gap-1.5 rounded-lg bg-primary px-4 py-2 text-xs font-semibold text-primary-foreground shadow-xs transition hover:bg-primary/90 cursor-pointer"
          @click="handleAdd"
        >
          <Check class="h-3.5 w-3.5" />
          <span>Add to Watchlist</span>
        </button>
      </div>
    </div>

    <!-- Empty state hint -->
    <p
      v-else-if="!verifying && activeTab === 'stock' && query.trim().length === 0"
      class="mt-3 text-center text-xs text-muted-foreground"
    >
      Enter a ticker above. Supports US stocks, international equities (e.g. NSE:RELIANCE), crypto, and forex pairs.
    </p>
  </AppDialog>
</template>

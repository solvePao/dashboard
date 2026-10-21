<script setup lang="ts">
import AppDialog from "./AppDialog.vue";
import { computed, nextTick, onBeforeUnmount, ref, watch, useTemplateRef } from "vue";
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
  searchStockSymbols,
  type MarketFilter,
  type StockQuoteData,
  type StockSearchResult,
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
const activeMarket = ref<MarketFilter>("all");

const query = ref("");
const fromCurrency = ref("USD");
const toCurrency = ref("INR");

const searchResults = ref<StockSearchResult[]>([]);
const verifiedQuote = ref<StockQuoteData | null>(null);
const searching = ref(false);
const verifying = ref(false);
const error = ref<string | null>(null);
const inputRef = useTemplateRef<HTMLInputElement>("inputRef");

interface CurrencyOption {
  code: string;
  name: string;
  symbol: string;
}

interface MarketPreset {
  label: string;
  ticker: string;
  symbol: string;
  exchange: string;
  currency: string;
}

const MARKET_FILTERS: { key: MarketFilter; label: string; icon?: unknown }[] = [
  { key: "all", label: "All Markets" },
  { key: "india", label: "🇮🇳 India (NSE / BSE)" },
  { key: "nyse", label: "🏛️ NYSE (New York)" },
  { key: "us", label: "🇺🇸 NASDAQ / US" },
  { key: "crypto", label: "🪙 Crypto" },
];

const INDIA_PRESETS: MarketPreset[] = [
  { label: "NIFTY 50", ticker: "NSE:NIFTY", symbol: "NIFTY", exchange: "NSE", currency: "INR" },
  { label: "SENSEX", ticker: "BSE:SENSEX", symbol: "SENSEX", exchange: "BSE", currency: "INR" },
  { label: "Reliance", ticker: "NSE:RELIANCE", symbol: "RELIANCE", exchange: "NSE", currency: "INR" },
  { label: "TCS", ticker: "NSE:TCS", symbol: "TCS", exchange: "NSE", currency: "INR" },
  { label: "HDFC Bank", ticker: "NSE:HDFCBANK", symbol: "HDFCBANK", exchange: "NSE", currency: "INR" },
  { label: "Infosys", ticker: "NSE:INFY", symbol: "INFY", exchange: "NSE", currency: "INR" },
  { label: "ICICI Bank", ticker: "NSE:ICICIBANK", symbol: "ICICIBANK", exchange: "NSE", currency: "INR" },
  { label: "Bharti Airtel", ticker: "NSE:BHARTIARTL", symbol: "BHARTIARTL", exchange: "NSE", currency: "INR" },
  { label: "State Bank", ticker: "NSE:SBIN", symbol: "SBIN", exchange: "NSE", currency: "INR" },
  { label: "Tata Motors", ticker: "NSE:TMPV", symbol: "TMPV", exchange: "NSE", currency: "INR" },
  { label: "ITC", ticker: "NSE:ITC", symbol: "ITC", exchange: "NSE", currency: "INR" },
  { label: "L&T", ticker: "NSE:LT", symbol: "LT", exchange: "NSE", currency: "INR" },
];

const NYSE_PRESETS: MarketPreset[] = [
  { label: "NYSE Index", ticker: "NYSE:NYA", symbol: "NYA", exchange: "NYSE", currency: "USD" },
  { label: "Berkshire", ticker: "NYSE:BRK.B", symbol: "BRK.B", exchange: "NYSE", currency: "USD" },
  { label: "JPMorgan", ticker: "NYSE:JPM", symbol: "JPM", exchange: "NYSE", currency: "USD" },
  { label: "Visa", ticker: "NYSE:V", symbol: "V", exchange: "NYSE", currency: "USD" },
  { label: "Coca-Cola", ticker: "NYSE:KO", symbol: "KO", exchange: "NYSE", currency: "USD" },
  { label: "Disney", ticker: "NYSE:DIS", symbol: "DIS", exchange: "NYSE", currency: "USD" },
  { label: "IBM", ticker: "NYSE:IBM", symbol: "IBM", exchange: "NYSE", currency: "USD" },
  { label: "ExxonMobil", ticker: "NYSE:XOM", symbol: "XOM", exchange: "NYSE", currency: "USD" },
  { label: "Mastercard", ticker: "NYSE:MA", symbol: "MA", exchange: "NYSE", currency: "USD" },
  { label: "Pfizer", ticker: "NYSE:PFE", symbol: "PFE", exchange: "NYSE", currency: "USD" },
  { label: "Nike", ticker: "NYSE:NKE", symbol: "NKE", exchange: "NYSE", currency: "USD" },
  { label: "Eli Lilly", ticker: "NYSE:LLY", symbol: "LLY", exchange: "NYSE", currency: "USD" },
];

const NASDAQ_PRESETS: MarketPreset[] = [
  { label: "Apple", ticker: "NASDAQ:AAPL", symbol: "AAPL", exchange: "NASDAQ", currency: "USD" },
  { label: "Microsoft", ticker: "NASDAQ:MSFT", symbol: "MSFT", exchange: "NASDAQ", currency: "USD" },
  { label: "Nvidia", ticker: "NASDAQ:NVDA", symbol: "NVDA", exchange: "NASDAQ", currency: "USD" },
  { label: "Amazon", ticker: "NASDAQ:AMZN", symbol: "AMZN", exchange: "NASDAQ", currency: "USD" },
  { label: "Alphabet", ticker: "NASDAQ:GOOGL", symbol: "GOOGL", exchange: "NASDAQ", currency: "USD" },
  { label: "Tesla", ticker: "NASDAQ:TSLA", symbol: "TSLA", exchange: "NASDAQ", currency: "USD" },
];

const CRYPTO_PRESETS: MarketPreset[] = [
  { label: "Bitcoin", ticker: "BINANCE:BTCUSDT", symbol: "BTC", exchange: "CRYPTO", currency: "USD" },
  { label: "Ethereum", ticker: "BINANCE:ETHUSDT", symbol: "ETH", exchange: "CRYPTO", currency: "USD" },
  { label: "Solana", ticker: "BINANCE:SOLUSDT", symbol: "SOL", exchange: "CRYPTO", currency: "USD" },
];

const currentPresets = computed<MarketPreset[]>(() => {
  if (activeMarket.value === "india") return INDIA_PRESETS;
  if (activeMarket.value === "nyse") return NYSE_PRESETS;
  if (activeMarket.value === "us") return NASDAQ_PRESETS;
  if (activeMarket.value === "crypto") return CRYPTO_PRESETS;
  return [
    INDIA_PRESETS[0], // NIFTY 50
    NYSE_PRESETS[0],  // NYSE Index
    INDIA_PRESETS[2], // Reliance
    NYSE_PRESETS[4],  // Coca-Cola
    NASDAQ_PRESETS[0],// Apple
    CRYPTO_PRESETS[0],// Bitcoin
    INDIA_PRESETS[5], // Infosys
    NYSE_PRESETS[1],  // Berkshire
  ];
});

const searchPlaceholder = computed<string>(() => {
  if (activeMarket.value === "india") return "Search NSE / BSE stocks or indices (e.g. RELIANCE, TCS, NIFTY)...";
  if (activeMarket.value === "nyse") return "Search NYSE stocks (e.g. KO, DIS, IBM, BRK.B, NYA)...";
  if (activeMarket.value === "us") return "Search NASDAQ stocks (e.g. AAPL, NVDA, MSFT)...";
  if (activeMarket.value === "crypto") return "Search crypto (e.g. BTCUSD, ETHUSDT)...";
  return "Search stocks, indices, or crypto across all markets...";
});

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

async function executeSearch(text: string): Promise<void> {
  clearVerification();
  const trimmed = text.trim();
  if (!trimmed) {
    searchResults.value = [];
    verifiedQuote.value = null;
    error.value = null;
    searching.value = false;
    return;
  }

  aborter = new AbortController();
  searching.value = true;
  error.value = null;

  try {
    const results = await searchStockSymbols(trimmed, activeMarket.value, aborter.signal);
    searchResults.value = results;

    // If single exact match or explicit ticker entered, automatically verify details
    if (results.length > 0) {
      const exactMatch = results.find(
        (r) => r.ticker.toUpperCase() === trimmed.toUpperCase() || r.symbol.toUpperCase() === trimmed.toUpperCase()
      );
      if (exactMatch) {
        await verifySymbol(exactMatch.ticker);
      } else if (trimmed.includes(":") || results.length === 1) {
        await verifySymbol(results[0].ticker);
      }
    } else {
      // Fallback direct verification attempt
      await verifySymbol(trimmed);
    }
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") return;
    searchResults.value = [];
    error.value = err instanceof Error ? err.message : "Search failed. Check your query.";
  } finally {
    searching.value = false;
  }
}

function selectSearchResult(item: StockSearchResult): void {
  query.value = item.ticker;
  void verifySymbol(item.ticker);
}

function selectPresetChip(preset: MarketPreset): void {
  query.value = preset.ticker;
  void verifySymbol(preset.ticker);
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

function selectForexPreset(pair: { from: string; to: string }): void {
  fromCurrency.value = pair.from;
  toCurrency.value = pair.to;
  verifyCurrentForexPair();
}

watch(activeTab, (tab) => {
  clearVerification();
  verifiedQuote.value = null;
  searchResults.value = [];
  error.value = null;
  if (tab === "stock") {
    query.value = "";
    void nextTick(() => inputRef.value?.focus());
  } else {
    verifyCurrentForexPair();
  }
});

watch(activeMarket, () => {
  if (activeTab.value !== "stock") return;
  searchResults.value = [];
  if (query.value.trim().length >= 1) {
    void executeSearch(query.value);
  }
});

watch(query, (newVal) => {
  if (activeTab.value !== "stock") return;
  if (debounceTimer) clearTimeout(debounceTimer);
  const trimmed = newVal.trim();
  if (trimmed.length >= 1) {
    debounceTimer = setTimeout(() => {
      void executeSearch(trimmed);
    }, 350);
  } else {
    searchResults.value = [];
    verifiedQuote.value = null;
    error.value = null;
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
    ticker: verifiedQuote.value.ticker,
    exchange: verifiedQuote.value.exchange,
    name: verifiedQuote.value.name,
    currency: verifiedQuote.value.currency,
  });
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      activeTab.value = "stock";
      activeMarket.value = "all";
      query.value = "";
      searchResults.value = [];
      verifiedQuote.value = null;
      error.value = null;
      searching.value = false;
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
    <div class="fe-stockmodal-header flex items-center justify-between border-b border-border/80 pb-3 sm:pb-4">
      <div class="min-w-0 pr-2">
        <h2 id="stock-picker-title" class="text-base font-semibold leading-tight text-foreground">
          Add Market Symbol
        </h2>
        <p class="text-xs text-muted-foreground mt-0.5 truncate">
          Indian equities (NSE/BSE), NYSE, NASDAQ, crypto, or forex pairs
        </p>
      </div>
      <button
        type="button"
        class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-muted-foreground transition hover:bg-accent hover:text-foreground cursor-pointer"
        aria-label="Close"
        @click="emit('close')"
      >
        <X class="h-4 w-4" />
      </button>
    </div>

    <!-- Segmented Tabs: Stocks & Markets vs. Currency Pair -->
    <div class="mt-3 flex rounded-xl bg-muted/60 p-1 border border-border/60">
      <button
        type="button"
        class="flex flex-1 min-h-[40px] items-center justify-center gap-1.5 rounded-lg py-2 text-xs font-semibold transition cursor-pointer"
        :class="
          activeTab === 'stock'
            ? 'bg-card text-foreground shadow-xs border border-border/60'
            : 'text-muted-foreground hover:text-foreground'
        "
        @click="activeTab = 'stock'"
      >
        <Search class="h-3.5 w-3.5" />
        <span>Stocks & Markets</span>
      </button>
      <button
        type="button"
        class="flex flex-1 min-h-[40px] items-center justify-center gap-1.5 rounded-lg py-2 text-xs font-semibold transition cursor-pointer"
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

    <Transition name="fe-tab-fade" mode="out-in">
    <!-- TAB 1: Stocks & Markets -->
    <div v-if="activeTab === 'stock'" key="stock" class="mt-3 space-y-3">
      <!-- Market Category Filter Pills -->
      <div class="flex items-center gap-1.5 overflow-x-auto pb-1 text-[11px] no-scrollbar">
        <button
          v-for="m in MARKET_FILTERS"
          :key="m.key"
          type="button"
          class="shrink-0 min-h-[34px] rounded-full border px-3 py-1 font-medium transition cursor-pointer active:scale-95"
          :class="
            activeMarket === m.key
              ? 'border-primary bg-primary text-primary-foreground font-semibold shadow-xs'
              : 'border-border bg-muted/40 text-muted-foreground hover:bg-muted hover:text-foreground'
          "
          @click="activeMarket = m.key"
        >
          {{ m.label }}
        </button>
      </div>

      <!-- Search Input -->
      <div class="relative">
        <Search class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <input
          ref="inputRef"
          v-model="query"
          type="text"
          :placeholder="searchPlaceholder"
          autocomplete="off"
          class="w-full min-h-[44px] rounded-lg border border-input bg-background py-2.5 pl-9 pr-9 text-xs sm:text-sm font-mono uppercase text-foreground placeholder:font-sans placeholder:normal-case placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          @keydown.enter.prevent="query.trim() && executeSearch(query)"
        />
        <Loader2
          v-if="searching || verifying"
          class="absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 animate-spin text-primary"
        />
      </div>

      <!-- Quick Presets Tray (when query is short or empty) -->
      <div v-if="query.trim().length === 0" class="rounded-xl border border-border/70 bg-muted/20 p-2.5">
        <div class="flex items-center justify-between mb-1.5">
          <span class="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">
            Popular in {{ MARKET_FILTERS.find(f => f.key === activeMarket)?.label }}
          </span>
          <span class="text-[10px] text-muted-foreground">1-Click Preview</span>
        </div>
        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="preset in currentPresets"
            :key="preset.ticker"
            type="button"
            class="inline-flex items-center gap-1.5 rounded-md border border-border/80 bg-background/80 px-2 py-1 text-[11px] font-medium text-foreground transition hover:border-primary/50 hover:bg-muted active:scale-95 cursor-pointer"
            @click="selectPresetChip(preset)"
          >
            <span class="font-semibold">{{ preset.label }}</span>
            <span class="text-[9px] font-mono uppercase px-1 py-0.2 rounded bg-muted border border-border/60 text-muted-foreground font-bold">
              {{ preset.exchange }}
            </span>
          </button>
        </div>
      </div>

      <!-- Search Results List (Disambiguation) -->
      <div
        v-if="searchResults.length > 0 && query.trim().length > 0"
        class="space-y-1 max-h-48 overflow-y-auto overscroll-contain pr-1 rounded-lg border border-border/60 bg-muted/10 p-1 animate-in fade-in-0 duration-150"
      >
        <div class="px-2 py-1 text-[10px] font-bold uppercase tracking-wider text-muted-foreground flex justify-between">
          <span>Search Results ({{ searchResults.length }})</span>
          <span>Click to select</span>
        </div>
        <button
          v-for="item in searchResults"
          :key="item.ticker"
          type="button"
          class="w-full flex items-center justify-between rounded-md p-2 text-left transition hover:bg-muted/80 active:scale-[0.99] cursor-pointer"
          :class="
            verifiedQuote && verifiedQuote.ticker === item.ticker
              ? 'bg-primary/10 border border-primary/40'
              : 'border border-transparent'
          "
          @click="selectSearchResult(item)"
        >
          <div class="min-w-0 pr-2">
            <div class="flex items-center gap-1.5">
              <span
                v-if="item.exchange"
                class="text-[9px] font-mono uppercase px-1 py-0.2 rounded bg-primary/10 border border-primary/20 text-primary font-bold shrink-0"
              >
                {{ item.exchange }}
              </span>
              <span class="font-mono font-bold text-xs text-foreground">{{ item.symbol }}</span>
            </div>
            <p class="truncate text-[11px] text-muted-foreground mt-0.5">{{ item.name }}</p>
          </div>

          <div class="text-right shrink-0">
            <p class="font-mono text-xs font-semibold text-foreground">
              {{ formatQuotePrice(item.price) }}
              <span class="text-[10px] text-muted-foreground font-sans">{{ item.currency }}</span>
            </p>
            <p
              class="text-[10px] font-semibold"
              :class="item.change >= 0 ? 'text-emerald-500' : 'text-rose-500'"
            >
              {{ item.change >= 0 ? '+' : '' }}{{ item.changePercent.toFixed(2) }}%
            </p>
          </div>
        </button>
      </div>
    </div>

    <!-- TAB 2: Currency Pair (Forex) -->
    <div v-else key="forex" class="mt-3 space-y-3">
      <div class="flex items-center gap-2">
        <!-- From Currency -->
        <div class="flex-1 min-w-0">
          <label class="block text-[11px] font-semibold text-muted-foreground uppercase tracking-wider mb-1">
            From Currency
          </label>
          <select
            v-model="fromCurrency"
            class="w-full min-h-[44px] rounded-lg border border-input bg-background px-3 py-2 text-xs sm:text-sm font-medium text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring cursor-pointer"
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
            class="flex h-11 w-11 items-center justify-center rounded-lg border border-border bg-muted/40 p-2 text-muted-foreground transition hover:bg-accent hover:text-foreground active:scale-95 cursor-pointer"
            title="Swap From and To currencies"
            @click="swapCurrencies"
          >
            <ArrowLeftRight class="h-4 w-4" />
          </button>
        </div>

        <!-- To Currency -->
        <div class="flex-1 min-w-0">
          <label class="block text-[11px] font-semibold text-muted-foreground uppercase tracking-wider mb-1">
            To Currency
          </label>
          <select
            v-model="toCurrency"
            class="w-full min-h-[44px] rounded-lg border border-input bg-background px-3 py-2 text-xs sm:text-sm font-medium text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring cursor-pointer"
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
            class="rounded-md border border-border/70 bg-muted/30 px-2 py-1 text-[11px] font-mono font-medium text-foreground transition hover:border-primary/50 hover:bg-muted active:scale-95 cursor-pointer"
            :class="
              fromCurrency === pair.from && toCurrency === pair.to
                ? 'border-primary bg-primary/10 text-primary font-bold'
                : ''
            "
            @click="selectForexPreset(pair)"
          >
            {{ pair.from }} → {{ pair.to }}
          </button>
        </div>
      </div>
    </div>
    </Transition>

    <!-- Error State -->
    <div
      v-if="error"
      class="mt-3 rounded-xl border border-destructive/20 bg-destructive/10 p-3 text-xs text-destructive animate-in fade-in-0 duration-150"
      role="alert"
    >
      {{ error }}
    </div>

    <!-- Verified Quote Preview Card -->
    <div
      v-else-if="verifiedQuote"
      class="mt-3 rounded-xl border border-border bg-muted/40 p-3.5 transition-all animate-in fade-in-50 duration-150"
    >
      <div class="flex items-center justify-between">
        <div>
          <div class="flex items-center gap-2">
            <span
              v-if="verifiedQuote.exchange"
              class="text-[10px] font-mono uppercase px-1.5 py-0.5 rounded bg-primary/15 border border-primary/30 text-primary font-bold"
            >
              {{ verifiedQuote.exchange }}
            </span>
            <span class="text-base font-bold text-foreground font-mono">
              {{ verifiedQuote.symbol }}
            </span>
            <span class="text-[11px] font-medium text-muted-foreground uppercase px-1.5 py-0.5 bg-muted rounded border border-border font-mono">
              {{ verifiedQuote.currency }}
            </span>
          </div>
          <p class="text-xs text-muted-foreground truncate max-w-[220px] mt-0.5">
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

      <div class="mt-3 pt-3 border-t border-border/60 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2.5">
        <span class="text-[10px] text-muted-foreground font-mono">
          Feed: {{ verifiedQuote.ticker }}
        </span>
        <button
          type="button"
          class="inline-flex min-h-[44px] sm:min-h-0 items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 sm:py-1.5 sm:px-3 text-xs font-semibold text-primary-foreground shadow-xs transition hover:bg-primary/90 active:scale-[0.98] cursor-pointer"
          @click="handleAdd"
        >
          <Check class="h-4 w-4 sm:h-3.5 sm:w-3.5" />
          <span>Add to Watchlist</span>
        </button>
      </div>
    </div>

    <!-- Empty state hint -->
    <p
      v-else-if="!searching && !verifying && activeTab === 'stock' && query.trim().length === 0"
      class="mt-2 text-center text-[11px] text-muted-foreground animate-in fade-in-0 duration-150"
    >
      Select a market above, click a preset, or type any stock or index symbol to preview.
    </p>
  </AppDialog>
</template>

<style scoped>
/* Tab switch: same tokens as mobile stock-symbol-flip (opacity + 6px rise) */
.fe-tab-fade-enter-active,
.fe-tab-fade-leave-active {
  transition: opacity 0.16s ease-out,
    transform 0.16s cubic-bezier(0.16, 1, 0.3, 1);
}
.fe-tab-fade-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.fe-tab-fade-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

@media (prefers-reduced-motion: reduce) {
  .fe-tab-fade-enter-active,
  .fe-tab-fade-leave-active {
    transition: opacity 0.2s ease !important;
    transform: none !important;
  }
}
</style>

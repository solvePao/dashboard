<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from "reka-ui";
import {
  ArrowLeft,
  Ban,
  Check,
  ChevronDown,
  Loader2,
  MapPin,
  Plus,
  ShieldCheck,
  Trash2,
  TrendingUp,
  X,
} from "@lucide/vue";
import type { DeviceItem } from "../types/device";
import { asStockConfig, asWeatherConfig, type DeviceWidget, type StockPeriod, type StockSymbolItem, type WeatherCityItem } from "../types/widget";
import type { CityOption } from "../services/geocodingApi";
import { getWidgetDefinition, WIDGET_REGISTRY } from "../widgets/registry";
import { useAuth } from "../composables/useAuth";
import { useDeviceWidgets } from "../composables/useDeviceWidgets";
import CityPickerModal from "./CityPickerModal.vue";
import WalkieTalkieSettings from "./WalkieTalkieSettings.vue";
import StockPickerModal from "./StockPickerModal.vue";

const props = defineProps<{
  device: DeviceItem;
}>();

const emit = defineEmits<{
  (e: "back"): void;
  (e: "revoked", token: string): void;
}>();

const { revokeDevice } = useAuth();

const deviceToken = computed(() => props.device.token);
const widgetsApi = useDeviceWidgets(deviceToken.value);
const {
  widgets,
  widgetsLoading,
  widgetsError,
  widgetsNotice,
  saving,
  load,
  addWeatherCity,
  removeWeatherCity,
  addStockSymbol,
  removeStockSymbol,
  setStockPeriod,
  setWidgetEnabled,
  removeWidget,
  clearScreenWidgets,
  setWalkieTalkieGroup,
} = widgetsApi;

const screenWidgets = computed(() =>
  widgets.value.filter((w) => w.widget_key !== "walkie_talkie")
);

const walkieWidget = computed(() =>
  widgets.value.find((w) => w.widget_key === "walkie_talkie")
);

async function handleToggleWalkie(): Promise<void> {
  if (walkieWidget.value) {
    await setWidgetEnabled("walkie_talkie", !walkieWidget.value.enabled).catch(() => undefined);
  } else {
    await setWalkieTalkieGroup(null).catch(() => undefined);
  }
}

const revoking = ref(false);
const revokeError = ref<string | null>(null);
const showCityModal = ref(false);
const showStockModal = ref(false);

function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

function weatherCities(widget: DeviceWidget): WeatherCityItem[] {
  const config = asWeatherConfig(widget);
  return config?.cities || [];
}

function weatherSummary(widget: DeviceWidget): string {
  const cities = weatherCities(widget);
  if (cities.length === 0) return "No cities added — click + Add City";
  if (cities.length === 1) return cities[0].country ? `${cities[0].city}, ${cities[0].country}` : cities[0].city;
  return `${cities.length} cities (${cities.map((c) => c.city).join(", ")})`;
}

function stockSymbols(widget: DeviceWidget): StockSymbolItem[] {
  const config = asStockConfig(widget);
  return config?.symbols || [];
}

function stockPeriod(widget: DeviceWidget): StockPeriod {
  const config = asStockConfig(widget);
  return config?.period || "1D";
}

const STOCK_PERIOD_OPTIONS: { value: StockPeriod; label: string }[] = [
  { value: "1D", label: "1D (1 Day)" },
  { value: "5D", label: "5D (5 Days)" },
  { value: "1W", label: "1W (1 Week)" },
  { value: "1M", label: "1M (1 Month)" },
  { value: "3M", label: "3M (3 Months)" },
  { value: "6M", label: "6M (6 Months)" },
  { value: "YTD", label: "YTD (Year to Date)" },
  { value: "1Y", label: "1Y (1 Year)" },
];

function stockSummary(widget: DeviceWidget): string {
  const symbols = stockSymbols(widget);
  if (symbols.length === 0) return "No symbols added — click + Add Symbol";
  if (symbols.length === 1) return symbols[0].name ? `${symbols[0].symbol} (${symbols[0].name})` : symbols[0].symbol;
  return `${symbols.length} symbols (${symbols.map((s) => s.symbol).join(", ")})`;
}

async function handleRevoke(): Promise<void> {
  if (revoking.value) return;
  revokeError.value = null;
  if (!window.confirm("Revoke this device? It will lose access immediately.")) return;
  revoking.value = true;
  try {
    const ok = await revokeDevice(props.device.token);
    if (!ok) {
      revokeError.value = "Failed to revoke device. Please try again.";
      return;
    }
    emit("revoked", props.device.token);
    emit("back");
  } catch {
    revokeError.value = "Session expired. Please sign in again.";
  } finally {
    revoking.value = false;
  }
}

function handlePickNone(): void {
  if (screenWidgets.value.length === 0) return;
  void clearScreenWidgets().catch(() => undefined);
}

function handlePickWidget(key: string): void {
  if (key === "weather") {
    showCityModal.value = true;
  } else if (key === "stock") {
    showStockModal.value = true;
  } else if (key === "walkie_talkie") {
    if (!walkieWidget.value?.enabled) {
      void handleToggleWalkie();
    }
  }
}

async function handleCitySelect(city: CityOption): Promise<void> {
  showCityModal.value = false;
  await addWeatherCity({
    city: city.name,
    country: city.country,
    admin1: city.admin1,
    latitude: city.latitude,
    longitude: city.longitude,
  }).catch(() => undefined);
}

async function handleRemoveCity(index: number): Promise<void> {
  await removeWeatherCity(index).catch(() => undefined);
}

async function handleStockSelect(symbolItem: StockSymbolItem): Promise<void> {
  showStockModal.value = false;
  await addStockSymbol(symbolItem).catch(() => undefined);
}

async function handleRemoveStock(index: number): Promise<void> {
  await removeStockSymbol(index).catch(() => undefined);
}

function handleToggle(widget: DeviceWidget): void {
  void setWidgetEnabled(widget.widget_key, !widget.enabled).catch(() => undefined);
}

function handleRemove(widget: DeviceWidget): void {
  void removeWidget(widget.widget_key).catch(() => undefined);
}

onMounted(() => {
  void load().catch(() => undefined);
});

watch(
  () => props.device.token,
  () => {
    void load().catch(() => undefined);
  }
);
</script>

<template>
  <div class="fe-manage-page space-y-5 sm:space-y-6 animate-in fade-in-50 duration-200">
    <!-- Navigation row -->
    <div class="fe-manage-nav flex items-center justify-between">
      <button
        type="button"
        class="fe-manage-back-btn inline-flex min-h-[44px] items-center gap-2 rounded-lg border border-border bg-card px-4 py-2 text-xs font-semibold text-foreground shadow-xs transition hover:bg-accent active:scale-[0.98]"
        @click="emit('back')"
      >
        <ArrowLeft class="h-4 w-4 text-muted-foreground" />
        <span>Back to Devices</span>
      </button>
    </div>

    <!-- Device Details Card -->
    <div class="fe-manage-header-card rounded-xl border border-border bg-card p-4 text-card-foreground shadow-sm sm:rounded-2xl sm:p-6">
      <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div class="flex min-w-0 flex-1 items-start gap-3 sm:gap-3.5">
          <span
            class="flex h-10 w-10 sm:h-11 sm:w-11 shrink-0 items-center justify-center rounded-xl bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
          >
            <ShieldCheck class="h-5 w-5 sm:h-6 sm:w-6" />
          </span>
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <h1 class="break-words [overflow-wrap:anywhere] text-base sm:text-lg font-bold text-foreground">
                {{ device.device_name }}
              </h1>
              <span
                class="rounded-full bg-emerald-500/10 px-2.5 py-0.5 text-[11px] font-medium text-emerald-600 dark:text-emerald-400 border border-emerald-500/20"
              >
                Authorized
              </span>
            </div>
            <p class="mt-1 text-xs text-muted-foreground">
              Paired on {{ formatDate(device.created_at) }} · Valid until {{ formatDate(device.expires_at) }}
            </p>
          </div>
        </div>

        <button
          type="button"
          class="inline-flex min-h-[44px] w-full sm:w-auto shrink-0 items-center justify-center gap-2 rounded-lg border border-destructive/30 bg-destructive/5 px-4 py-2.5 text-xs font-semibold text-destructive transition hover:bg-destructive/15 active:scale-[0.98] disabled:opacity-60"
          :disabled="revoking"
          @click="handleRevoke"
        >
          <Loader2 v-if="revoking" class="h-4 w-4 animate-spin" />
          <Trash2 v-else class="h-4 w-4" />
          <span>Revoke Device</span>
        </button>
      </div>

      <div
        v-if="revokeError || widgetsError"
        class="mt-4 rounded-xl border border-destructive/20 bg-destructive/10 p-3 text-xs text-destructive"
        role="alert"
      >
        {{ revokeError || widgetsError }}
      </div>
    </div>

    <p v-if="widgetsNotice" role="status" class="rounded-xl border border-border bg-muted p-3 text-xs text-muted-foreground">{{ widgetsNotice }}</p>

    <!-- Walkie-talkie Feature Card (only shown when enabled) -->
    <WalkieTalkieSettings
      v-if="walkieWidget && walkieWidget.enabled"
      :key="device.token"
      :device-token="device.token"
      :group-id="typeof walkieWidget?.config?.group_id === 'string' ? walkieWidget.config.group_id : null"
      :enabled="walkieWidget ? walkieWidget.enabled : false"
      :saving="saving"
      @toggle="handleToggleWalkie"
      @assigned="setWalkieTalkieGroup"
      @changed="load"
    />

    <!-- Widgets Configuration Card -->
    <div class="fe-manage-widgets-card rounded-xl border border-border bg-card p-4 text-card-foreground shadow-sm sm:rounded-2xl sm:p-6">
      <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between border-b border-border/80 pb-4">
        <div>
          <h2 class="text-base font-semibold leading-tight text-foreground">Device Widgets</h2>
          <p class="mt-0.5 text-xs text-muted-foreground">
            Configure widgets shown on this device's screen
          </p>
        </div>

        <DropdownMenuRoot>
          <DropdownMenuTrigger as-child>
            <button
              type="button"
              class="inline-flex min-h-[42px] sm:min-h-0 items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 sm:py-1.5 sm:px-3 text-xs font-semibold text-primary-foreground shadow-xs transition hover:bg-primary/90 active:scale-[0.98]"
            >
              <Plus class="h-4 w-4 sm:h-3.5 sm:w-3.5" />
              <span>Add widget</span>
              <ChevronDown class="h-3.5 w-3.5 opacity-80" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuPortal>
            <DropdownMenuContent
              :side-offset="8"
              :collision-padding="12"
              align="end"
              class="z-50 max-h-[var(--reka-dropdown-menu-content-available-height)] max-w-[calc(100vw-1.5rem)] overflow-y-auto w-64 rounded-xl border border-border bg-popover p-1.5 text-popover-foreground shadow-xl outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
            >
              <DropdownMenuItem
                class="flex min-h-[40px] cursor-pointer items-center gap-2.5 rounded-lg px-3 py-2 text-sm outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground"
                @select="handlePickNone"
              >
                <Ban class="h-4 w-4 shrink-0 text-muted-foreground" />
                <span class="min-w-0 flex-1">
                  <span class="block font-medium">None</span>
                  <span class="block truncate text-xs text-muted-foreground">Show nothing</span>
                </span>
                <Check v-if="screenWidgets.length === 0" class="h-4 w-4 shrink-0 text-primary" />
              </DropdownMenuItem>
              <DropdownMenuItem
                v-for="entry in WIDGET_REGISTRY"
                :key="entry.key"
                class="flex min-h-[40px] cursor-pointer items-center gap-2.5 rounded-lg px-3 py-2 text-sm outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground"
                @select="handlePickWidget(entry.key)"
              >
                <component :is="entry.icon" class="h-4 w-4 shrink-0 text-primary" />
                <span class="min-w-0 flex-1">
                  <span class="block font-medium">{{ entry.label }}</span>
                  <span class="block truncate text-xs text-muted-foreground">{{ entry.description }}</span>
                </span>
                <Check
                  v-if="screenWidgets.some((w) => w.widget_key === entry.key) || (entry.key === 'walkie_talkie' && walkieWidget && walkieWidget.enabled)"
                  class="h-4 w-4 shrink-0 text-primary"
                />
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenuPortal>
        </DropdownMenuRoot>
      </div>

      <div v-if="widgetsLoading" class="py-12 text-center text-xs text-muted-foreground">
        <Loader2 class="mx-auto mb-2 h-6 w-6 animate-spin text-primary" />
        Loading widgets...
      </div>

      <div
        v-else-if="screenWidgets.length === 0"
        class="mt-6 rounded-xl border border-dashed border-border p-8 text-center text-xs text-muted-foreground"
      >
        <p class="font-medium text-foreground text-sm">No widgets active</p>
        <p class="mt-1">Click "Add widget" above to display weather or market widgets on this mobile device.</p>
      </div>

      <ul v-else class="mt-4 space-y-2.5">
        <li
          v-for="widget in screenWidgets"
          :key="widget.widget_key"
          class="flex flex-col gap-3 rounded-xl border border-border bg-card p-4 transition hover:border-border/90"
        >
          <div class="flex flex-wrap items-center gap-3.5">
            <span
              class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary"
            >
              <component :is="getWidgetDefinition(widget.widget_key)?.icon ?? Ban" class="h-5 w-5" />
            </span>
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-semibold text-foreground">
                {{ getWidgetDefinition(widget.widget_key)?.label ?? widget.widget_key }}
              </p>
              <p class="truncate text-xs text-muted-foreground">
                {{ widget.widget_key === "weather" ? weatherSummary(widget) : widget.widget_key === "stock" ? stockSummary(widget) : "" }}
                <span v-if="!widget.enabled" class="text-amber-500 dark:text-amber-400"> · Disabled</span>
              </p>
            </div>

            <div class="flex items-center gap-2 shrink-0">
              <button
                type="button"
                role="switch"
                :aria-checked="widget.enabled"
                :aria-label="`Turn ${getWidgetDefinition(widget.widget_key)?.label ?? widget.widget_key} on or off`"
                :disabled="saving"
                class="relative h-6 w-11 shrink-0 rounded-full transition-colors disabled:opacity-60"
                :class="widget.enabled ? 'bg-primary' : 'bg-muted'"
                @click="handleToggle(widget)"
              >
                <span
                  class="absolute top-0.5 h-5 w-5 rounded-full shadow transition-all"
                  :class="widget.enabled ? 'left-5.5 bg-primary-foreground' : 'left-0.5 bg-foreground'"
                />
              </button>
              <button
                type="button"
                class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-muted-foreground transition hover:bg-destructive/10 hover:text-destructive active:scale-95"
                :aria-label="`Remove ${getWidgetDefinition(widget.widget_key)?.label ?? widget.widget_key}`"
                :disabled="saving"
                @click="handleRemove(widget)"
              >
                <Trash2 class="h-4 w-4" />
              </button>
            </div>
          </div>

          <div
            v-if="widget.widget_key === 'weather'"
            class="border-t border-border/60 pt-3"
          >
            <div class="flex flex-wrap items-center justify-between gap-2 mb-2">
              <span class="text-xs font-medium text-muted-foreground">Configured Cities</span>
              <button
                type="button"
                class="inline-flex min-h-[36px] sm:min-h-0 items-center gap-1 rounded-md bg-primary/10 px-3 py-1.5 sm:px-2.5 sm:py-1 text-xs font-semibold text-primary hover:bg-primary/20 transition disabled:opacity-50 active:scale-[0.98]"
                :disabled="saving"
                @click="showCityModal = true"
              >
                <Plus class="h-3.5 w-3.5" />
                <span>Add City</span>
              </button>
            </div>

            <div v-if="weatherCities(widget).length > 0" class="flex flex-wrap gap-2">
              <span
                v-for="(cityItem, idx) in weatherCities(widget)"
                :key="`${cityItem.latitude}-${cityItem.longitude}-${idx}`"
                class="inline-flex max-w-full items-center gap-1.5 rounded-lg border border-border bg-muted/60 pl-2.5 pr-1 py-1 text-xs font-medium text-foreground shadow-2xs"
              >
                <MapPin class="h-3.5 w-3.5 text-primary shrink-0" />
                <span class="truncate max-w-[150px]">{{ cityItem.city }}{{ cityItem.country ? `, ${cityItem.country}` : '' }}</span>
                <button
                  type="button"
                  class="ml-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-destructive/15 hover:text-destructive transition active:scale-90"
                  aria-label="Remove city"
                  :disabled="saving"
                  @click="handleRemoveCity(idx)"
                >
                  <X class="h-3.5 w-3.5" />
                </button>
              </span>
            </div>
            <p v-else class="text-xs text-muted-foreground italic">
              No cities added yet. Click "+ Add City" to pick a city.
            </p>
          </div>

          <!-- Symbols Chips for Stock Widget -->
          <div
            v-if="widget.widget_key === 'stock'"
            class="border-t border-border/60 pt-3 space-y-3"
          >
            <!-- Default Performance Period Setting -->
            <div class="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border/70 bg-muted/30 p-2.5">
              <div>
                <p class="text-xs font-semibold text-foreground">Performance Period</p>
                <p class="text-[11px] text-muted-foreground">Sets the 1D / 5D timeframe on mobile</p>
              </div>
              <select
                :value="stockPeriod(widget)"
                :disabled="saving"
                class="rounded-md border border-border bg-background px-2.5 py-1 text-xs font-medium text-foreground transition hover:border-primary/50 focus:border-primary focus:outline-none cursor-pointer disabled:opacity-50"
                aria-label="Stock performance period"
                @change="setStockPeriod(($event.target as HTMLSelectElement).value as StockPeriod)"
              >
                <option v-for="opt in STOCK_PERIOD_OPTIONS" :key="opt.value" :value="opt.value">
                  {{ opt.label }}
                </option>
              </select>
            </div>

            <div class="flex flex-wrap items-center justify-between gap-2 mb-2">
              <span class="text-xs font-medium text-muted-foreground">
                Configured Watchlist ({{ stockSymbols(widget).length }}/4)
              </span>
              <button
                type="button"
                class="inline-flex min-h-[36px] sm:min-h-0 items-center gap-1 rounded-md bg-primary/10 px-3 py-1.5 sm:px-2.5 sm:py-1 text-xs font-semibold text-primary hover:bg-primary/20 transition disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98]"
                :disabled="saving || stockSymbols(widget).length >= 4"
                :title="stockSymbols(widget).length >= 4 ? 'Maximum 4 symbols reached' : 'Add Stock Symbol'"
                @click="showStockModal = true"
              >
                <Plus class="h-3.5 w-3.5" />
                <span>{{ stockSymbols(widget).length >= 4 ? 'Limit Reached (4)' : 'Add Symbol' }}</span>
              </button>
            </div>

            <p v-if="stockSymbols(widget).length > 4" class="mb-2 text-xs text-muted-foreground">
              Mobile displays the first 4 stocks. Your other stocks are still saved; remove entries below to change which stocks appear.
            </p>
            <div v-if="stockSymbols(widget).length > 0" class="flex flex-wrap gap-2">
              <span
                v-for="(symItem, idx) in stockSymbols(widget)"
                :key="`${symItem.symbol}-${idx}`"
                class="inline-flex max-w-full items-center gap-1.5 rounded-lg border border-border bg-muted/60 pl-2.5 pr-1 py-1 text-xs font-medium text-foreground shadow-2xs"
              >
                <TrendingUp class="h-3.5 w-3.5 text-primary shrink-0" />
                <span
                  v-if="symItem.exchange || (symItem.ticker && symItem.ticker.includes(':'))"
                  class="text-[9px] font-mono uppercase px-1.5 py-0.5 rounded bg-primary/10 border border-primary/20 text-primary font-bold shrink-0 leading-none"
                >
                  {{ symItem.exchange || symItem.ticker?.split(':')[0] }}
                </span>
                <span class="truncate max-w-[150px] font-mono font-semibold">{{ symItem.symbol }}</span>
                <span
                  v-if="symItem.currency"
                  class="text-[10px] font-mono uppercase px-1 py-0.2 rounded border border-border bg-background text-muted-foreground font-semibold"
                >
                  {{ symItem.currency }}
                </span>
                <span v-if="symItem.name" class="text-muted-foreground truncate max-w-[120px] text-[11px]">({{ symItem.name }})</span>
                <button
                  type="button"
                  class="ml-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-destructive/15 hover:text-destructive transition active:scale-90"
                  aria-label="Remove symbol"
                  :disabled="saving"
                  @click="handleRemoveStock(idx)"
                >
                  <X class="h-3.5 w-3.5" />
                </button>
              </span>
            </div>
            <p v-else class="text-xs text-muted-foreground italic">
              No symbols added yet. Click "+ Add Symbol" to add a stock to your watchlist.
            </p>
          </div>
        </li>
      </ul>
    </div>
  </div>

  <CityPickerModal
    :open="showCityModal"
    @close="showCityModal = false"
    @select="handleCitySelect"
  />

  <StockPickerModal
    :open="showStockModal"
    @close="showStockModal = false"
    @select="handleStockSelect"
  />
</template>


<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch, type Component } from "vue";
import {
  Activity,
  ChevronLeft,
  ChevronRight,
  Cloud,
  CloudDrizzle,
  CloudFog,
  CloudHail,
  CloudLightning,
  CloudMoon,
  CloudRain,
  CloudSnow,
  CloudSun,
  Droplets,
  MapPin,
  Moon,
  RefreshCw,
  Sparkles,
  Sun,
  Sunrise,
  Thermometer,
  ThermometerSnowflake,
  Wind,
} from "@lucide/vue";
import { fetchCurrentWeather, type CurrentWeather, type WeatherEffectType } from "../../services/weatherApi";
import type { WeatherCityItem, WeatherWidgetConfig } from "../../types/widget";
import { useOrientationLock } from "../../composables/useOrientationLock";
import { syncWeatherTheme } from "../../composables/useTheme";
import WeatherEffects from "./WeatherEffects.vue";

const props = defineProps<{
  config: WeatherWidgetConfig;
}>();

const { isPortrait } = useOrientationLock();

const currentCityIndex = ref(0);
const direction = ref<"left" | "right">("left");
const isAnimating = ref(false);
let animationTimer: ReturnType<typeof setTimeout> | undefined;

const citiesList = computed<WeatherCityItem[]>(() => {
  if (props.config.cities && props.config.cities.length > 0) {
    return props.config.cities;
  }
  return [
    {
      city: props.config.city || "Unknown",
      country: props.config.country,
      admin1: props.config.admin1,
      latitude: props.config.latitude ?? 0,
      longitude: props.config.longitude ?? 0,
    },
  ];
});

const currentCity = computed<WeatherCityItem>(() => {
  const list = citiesList.value;
  const idx = currentCityIndex.value < list.length ? currentCityIndex.value : 0;
  return list[idx];
});

function getCityKey(city: WeatherCityItem): string {
  return `${city.city}_${city.latitude.toFixed(4)}_${city.longitude.toFixed(4)}`;
}

const currentCityKey = computed(() => {
  return `${getCityKey(currentCity.value)}_${currentCityIndex.value}`;
});

const transitionName = computed(() => {
  return direction.value === "left" ? "h-slide-left" : "h-slide-right";
});

function goToCityIndex(nextIndex: number, forcedDirection?: "left" | "right"): void {
  const list = citiesList.value;
  if (list.length <= 1) return;
  const current = currentCityIndex.value;
  if (nextIndex === current) return;

  if (forcedDirection) {
    direction.value = forcedDirection;
  } else {
    direction.value = nextIndex > current ? "left" : "right";
  }

  isAnimating.value = true;
  if (animationTimer) clearTimeout(animationTimer);
  animationTimer = setTimeout(() => {
    isAnimating.value = false;
  }, 400);

  currentCityIndex.value = nextIndex;
}

function nextCity(): void {
  const list = citiesList.value;
  if (list.length <= 1) return;
  const nextIdx = (currentCityIndex.value + 1) % list.length;
  goToCityIndex(nextIdx, "left");
}

function prevCity(): void {
  const list = citiesList.value;
  if (list.length <= 1) return;
  const prevIdx = (currentCityIndex.value - 1 + list.length) % list.length;
  goToCityIndex(prevIdx, "right");
}

// Gesture Handling
let touchStartX = 0;
let touchStartY = 0;
const SWIPE_THRESHOLD = 30; // Minimum horizontal distance in px to trigger city switch
const WHEEL_THRESHOLD = 18; // Minimum wheel deltaX to trigger city switch

function onTouchStart(e: TouchEvent): void {
  if (e.touches.length > 0) {
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
  }
}

function onTouchCancel(): void {
  touchStartX = 0;
  touchStartY = 0;
}

function onTouchEnd(e: TouchEvent): void {
  if (
    e.changedTouches.length === 0 ||
    isAnimating.value ||
    citiesList.value.length <= 1
  ) {
    return;
  }

  const rawDeltaX = e.changedTouches[0].clientX - touchStartX;
  const rawDeltaY = e.changedTouches[0].clientY - touchStartY;

  // Account for CSS rotate(90deg) when portrait orientation lock is active:
  // In rotated mode, visual Y delta corresponds to -rawDeltaX and visual X to rawDeltaY.
  const visualDeltaY = isPortrait.value ? -rawDeltaX : rawDeltaY;
  const visualDeltaX = isPortrait.value ? rawDeltaY : rawDeltaX;

  // Verify swipe is primarily horizontal along the panel's visual axis
  if (
    Math.abs(visualDeltaX) > Math.abs(visualDeltaY) &&
    Math.abs(visualDeltaX) >= SWIPE_THRESHOLD
  ) {
    if (visualDeltaX < 0) {
      // Swiped LEFT -> advance to next city
      nextCity();
    } else {
      // Swiped RIGHT -> return to previous city
      prevCity();
    }
  }
}

let wheelCooldown = false;
let wheelCooldownTimer: ReturnType<typeof setTimeout> | undefined;

function onWheel(e: WheelEvent): void {
  if (
    wheelCooldown ||
    isAnimating.value ||
    citiesList.value.length <= 1
  ) {
    return;
  }

  // Only react to horizontal wheel/trackpad swipe
  if (
    Math.abs(e.deltaX) >= WHEEL_THRESHOLD &&
    Math.abs(e.deltaX) > Math.abs(e.deltaY)
  ) {
    wheelCooldown = true;
    if (wheelCooldownTimer) clearTimeout(wheelCooldownTimer);
    wheelCooldownTimer = setTimeout(() => {
      wheelCooldown = false;
    }, 400);

    if (e.deltaX > 0) {
      nextCity();
    } else {
      prevCity();
    }
  }
}

// Reactive Weather Storage per City
const weatherMap = ref<Record<string, CurrentWeather>>({});
const loadingMap = ref<Record<string, boolean>>({});
const errorMap = ref<Record<string, string | null>>({});

const aborters = new Map<string, AbortController>();
let refreshTimer: ReturnType<typeof setInterval> | undefined;

const REFRESH_MS = 30 * 60 * 1000;

interface CachedWeatherData {
  weather: CurrentWeather;
  timestamp: number;
}

function getCacheKey(lat: number, lon: number): string {
  return `mo_weather_cache_${lat.toFixed(4)}_${lon.toFixed(4)}`;
}

function loadFromCache(lat: number, lon: number): CachedWeatherData | null {
  try {
    const raw = localStorage.getItem(getCacheKey(lat, lon));
    if (!raw) return null;
    const parsed = JSON.parse(raw) as CachedWeatherData;
    if (parsed?.weather && typeof parsed.timestamp === "number") {
      return parsed;
    }
  } catch {
    // Ignore JSON or storage errors
  }
  return null;
}

function saveToCache(lat: number, lon: number, data: CurrentWeather): void {
  try {
    const payload: CachedWeatherData = {
      weather: data,
      timestamp: Date.now(),
    };
    localStorage.setItem(getCacheKey(lat, lon), JSON.stringify(payload));
  } catch {
    // Storage quota exceeded or disabled
  }
}

interface ConditionStyle {
  label: string;
  icon: Component;
  gradient: string;
  effect: WeatherEffectType;
}

function conditionFor(
  code: number,
  isDay: boolean,
  temp?: number,
  aqi?: number,
  windSpeed?: number
): ConditionStyle {
  // 1. Sandstorm / Dust (WMO 04-09, 30-35, or high particulate dust with high wind)
  if (
    (code >= 4 && code <= 9) ||
    (code >= 30 && code <= 35) ||
    (aqi !== undefined && aqi >= 250 && windSpeed !== undefined && windSpeed >= 35)
  ) {
    return {
      label: "Sandstorm",
      icon: Wind,
      gradient: "from-amber-600 via-orange-800 to-stone-950",
      effect: "sandstorm",
    };
  }

  // 2. Thunderstorm / Hailstorm
  if (code === 95) {
    return {
      label: "Thunderstorm",
      icon: CloudLightning,
      gradient: "from-slate-950 via-purple-950 to-slate-900",
      effect: "thunderstorm",
    };
  }
  if (code === 96 || code === 99) {
    return {
      label: "Hailstorm",
      icon: CloudHail,
      gradient: "from-slate-950 via-purple-950 to-slate-900",
      effect: "thunderstorm",
    };
  }

  // 3. Snow
  if ((code >= 71 && code <= 77) || code === 85 || code === 86) {
    return {
      label: code === 77 ? "Snow Grains" : code >= 85 ? "Snow Showers" : "Snow",
      icon: CloudSnow,
      gradient: isDay
        ? "from-blue-950 via-slate-900 to-slate-950"
        : "from-slate-950 via-slate-900 to-indigo-950",
      effect: "snow",
    };
  }

  // 4. Rain & Drizzle (Differentiated Day vs Night)
  if ((code >= 51 && code <= 57) || (code >= 61 && code <= 67) || (code >= 80 && code <= 82)) {
    const isDrizzle = code >= 51 && code <= 57;
    const isShower = code >= 80 && code <= 82;
    return {
      label: isDrizzle ? "Drizzle" : isShower ? "Rain Showers" : "Rain",
      icon: isDrizzle ? CloudDrizzle : CloudRain,
      gradient: isDay
        ? "from-sky-900 via-slate-900 to-slate-950"
        : "from-slate-950 via-indigo-950 to-blue-950",
      effect: isDay ? "day_rain" : "night_rain",
    };
  }

  // 5. Fog
  if (code === 45 || code === 48) {
    return {
      label: "Foggy",
      icon: CloudFog,
      gradient: isDay
        ? "from-slate-800 via-slate-900 to-zinc-950"
        : "from-slate-950 via-zinc-900 to-neutral-900",
      effect: "cloud",
    };
  }

  // 6. Cloud (Partly Cloudy / Overcast)
  if (code === 2) {
    return {
      label: "Partly Cloudy",
      icon: isDay ? CloudSun : CloudMoon,
      gradient: isDay
        ? "from-sky-900 via-slate-900 to-zinc-950"
        : "from-slate-950 via-slate-900 to-zinc-900",
      effect: "cloud",
    };
  }
  if (code === 3) {
    return {
      label: "Overcast",
      icon: Cloud,
      gradient: isDay
        ? "from-slate-800 via-zinc-900 to-neutral-950"
        : "from-zinc-950 via-slate-900 to-neutral-900",
      effect: "cloud",
    };
  }

  // 7. Clear Sky / Mainly Clear (0, 1) -> Sunlight or Chilly Night
  if (code === 0 || code === 1) {
    if (isDay) {
      return {
        label: code === 0 ? "Clear Sky" : "Mainly Clear",
        icon: Sun,
        gradient: "from-amber-600 via-sky-900 to-slate-950",
        effect: "sunlight",
      };
    }
    // Night: Chilly Night when cold (<= 10°C / 50°F) or Starry Night
    const isChilly = temp !== undefined && temp <= 10;
    return {
      label: isChilly ? "Chilly Night" : (code === 0 ? "Clear Night" : "Mainly Clear"),
      icon: isChilly ? ThermometerSnowflake : Moon,
      gradient: isChilly
        ? "from-indigo-950 via-slate-950 to-cyan-950"
        : "from-slate-950 via-indigo-950 to-slate-900",
      effect: "chilly_night",
    };
  }

  // Default fallback
  return {
    label: "Cloudy",
    icon: Cloud,
    gradient: isDay
      ? "from-sky-900 via-slate-900 to-slate-950"
      : "from-slate-950 via-slate-900 to-zinc-900",
    effect: "cloud",
  };
}

const PREVIEW_PRESETS: Record<WeatherEffectType, ConditionStyle> = {
  sunlight: {
    label: "Sunlight (Clear)",
    icon: Sun,
    gradient: "from-amber-600 via-sky-900 to-slate-950",
    effect: "sunlight",
  },
  day_rain: {
    label: "Day Time Rain",
    icon: CloudRain,
    gradient: "from-sky-900 via-slate-900 to-slate-950",
    effect: "day_rain",
  },
  night_rain: {
    label: "Night Rain",
    icon: CloudRain,
    gradient: "from-slate-950 via-indigo-950 to-blue-950",
    effect: "night_rain",
  },
  cloud: {
    label: "Cloudy & Overcast",
    icon: Cloud,
    gradient: "from-slate-800 via-zinc-900 to-neutral-950",
    effect: "cloud",
  },
  sandstorm: {
    label: "Sandstorm",
    icon: Wind,
    gradient: "from-amber-700 via-stone-900 to-black",
    effect: "sandstorm",
  },
  snow: {
    label: "Snowfall",
    icon: CloudSnow,
    gradient: "from-blue-950 via-slate-900 to-slate-950",
    effect: "snow",
  },
  chilly_night: {
    label: "Chilly Night",
    icon: ThermometerSnowflake,
    gradient: "from-indigo-950 via-slate-950 to-cyan-950",
    effect: "chilly_night",
  },
  thunderstorm: {
    label: "Thunderstorm",
    icon: CloudLightning,
    gradient: "from-slate-950 via-purple-950 to-slate-900",
    effect: "thunderstorm",
  },
};

const PREVIEW_EFFECTS: { key: WeatherEffectType | null; label: string }[] = [
  { key: null, label: "Live" },
  { key: "sunlight", label: "Sunlight" },
  { key: "day_rain", label: "Day Rain" },
  { key: "night_rain", label: "Night Rain" },
  { key: "cloud", label: "Cloud" },
  { key: "sandstorm", label: "Sandstorm" },
  { key: "snow", label: "Snow" },
  { key: "chilly_night", label: "Chilly Night" },
  { key: "thunderstorm", label: "Thunderstorm" },
];

const previewEffect = ref<WeatherEffectType | null>(null);

function cyclePreviewEffect(): void {
  const currentIdx = PREVIEW_EFFECTS.findIndex((p) => p.key === previewEffect.value);
  const nextIdx = (currentIdx + 1) % PREVIEW_EFFECTS.length;
  previewEffect.value = PREVIEW_EFFECTS[nextIdx].key;
}

function uvCategory(uv: number): string {
  if (uv < 3) return "Low";
  if (uv < 6) return "Moderate";
  if (uv < 8) return "High";
  if (uv < 11) return "Very High";
  return "Extreme";
}

const currentWeather = computed<CurrentWeather | null>(() => {
  const key = getCityKey(currentCity.value);
  return weatherMap.value[key] || null;
});

const currentLoading = computed<boolean>(() => {
  const key = getCityKey(currentCity.value);
  return loadingMap.value[key] ?? false;
});

const currentError = computed<string | null>(() => {
  const key = getCityKey(currentCity.value);
  return errorMap.value[key] ?? null;
});

const displayWeather = computed<CurrentWeather | null>(() => {
  if (currentWeather.value) return currentWeather.value;
  if (previewEffect.value) {
    return {
      temperature: 21,
      apparentTemperature: 20,
      humidity: 55,
      windSpeed: 15,
      windDirection: 180,
      windCardinal: "S",
      weatherCode: 0,
      isDay: previewEffect.value !== "night_rain" && previewEffect.value !== "chilly_night",
      highToday: 25,
      lowToday: 16,
      precipitation: previewEffect.value === "day_rain" || previewEffect.value === "night_rain" ? 3.4 : 0,
      precipitationProbability: previewEffect.value === "day_rain" || previewEffect.value === "night_rain" ? 85 : 10,
      uvIndex: previewEffect.value === "sunlight" ? 7 : 3,
      aqi: { aqi: previewEffect.value === "sandstorm" ? 280 : 38, category: previewEffect.value === "sandstorm" ? "Very Unhealthy" : "Good" },
      sunrise: "06:18",
      sunset: "18:42",
      lastUpdated: "Preview",
    };
  }
  return null;
});

const activeCondition = computed<ConditionStyle | null>(() => {
  if (previewEffect.value) {
    return PREVIEW_PRESETS[previewEffect.value];
  }
  if (!currentWeather.value) return null;
  return conditionFor(
    currentWeather.value.weatherCode,
    currentWeather.value.isDay,
    currentWeather.value.apparentTemperature,
    currentWeather.value.aqi?.aqi,
    currentWeather.value.windSpeed
  );
});

watch(
  activeCondition,
  (cond) => {
    if (!cond) return;
    const isDay = cond.effect !== "night_rain" && cond.effect !== "chilly_night";
    syncWeatherTheme(isDay, currentWeather.value?.weatherCode, cond.label);
  },
  { immediate: true }
);

const place = computed(() =>
  [currentCity.value.city, currentCity.value.country].filter(Boolean).join(", ")
);

async function loadCityWeather(city: WeatherCityItem, force = false): Promise<void> {
  if (!city || !Number.isFinite(city.latitude) || !Number.isFinite(city.longitude)) return;
  const key = getCityKey(city);

  aborters.get(key)?.abort();
  const controller = new AbortController();
  aborters.set(key, controller);

  const cached = loadFromCache(city.latitude, city.longitude);
  const now = Date.now();
  const isCacheFresh = cached && now - cached.timestamp < REFRESH_MS;

  if (cached) {
    weatherMap.value[key] = cached.weather;
    errorMap.value[key] = null;
  }

  if (isCacheFresh && !force) {
    loadingMap.value[key] = false;
    return;
  }

  if (!weatherMap.value[key]) {
    loadingMap.value[key] = true;
  }
  errorMap.value[key] = null;

  try {
    const fresh = await fetchCurrentWeather(
      city.latitude,
      city.longitude,
      controller.signal
    );
    weatherMap.value[key] = fresh;
    saveToCache(city.latitude, city.longitude, fresh);
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") return;
    if (!weatherMap.value[key]) {
      errorMap.value[key] = err instanceof Error ? err.message : "Could not load weather.";
    }
  } finally {
    loadingMap.value[key] = false;
  }
}

async function loadAllCities(force = false): Promise<void> {
  const list = citiesList.value;
  if (list.length === 0) return;

  // Hydrate all from cache first for instant rendering
  for (const city of list) {
    const cached = loadFromCache(city.latitude, city.longitude);
    if (cached) {
      weatherMap.value[getCityKey(city)] = cached.weather;
    }
  }

  // Fetch currently active city first
  const activeCity = currentCity.value;
  await loadCityWeather(activeCity, force);

  // Background pre-fetch remaining cities
  for (const city of list) {
    if (getCityKey(city) !== getCityKey(activeCity)) {
      void loadCityWeather(city, force);
    }
  }
}

function restartTimer(): void {
  if (refreshTimer) clearInterval(refreshTimer);
  refreshTimer = setInterval(() => void loadAllCities(true), REFRESH_MS);
}

onMounted(() => {
  void loadAllCities();
  restartTimer();
});

watch(
  () => [currentCity.value.latitude, currentCity.value.longitude] as const,
  () => void loadCityWeather(currentCity.value)
);

watch(
  () => props.config.cities,
  () => {
    if (currentCityIndex.value >= citiesList.value.length) {
      currentCityIndex.value = 0;
    }
    void loadAllCities();
  },
  { deep: true }
);

onBeforeUnmount(() => {
  if (animationTimer) clearTimeout(animationTimer);
  if (wheelCooldownTimer) clearTimeout(wheelCooldownTimer);
  if (refreshTimer) clearInterval(refreshTimer);
  aborters.forEach((controller) => controller.abort());
  aborters.clear();
});
</script>

<template>
  <div
    class="mo-weather-root relative flex h-full w-full flex-col justify-between overflow-hidden p-4 sm:p-5 text-white shadow-inner transition-colors duration-700 select-none"
    :class="activeCondition ? `bg-gradient-to-br ${activeCondition.gradient}` : 'bg-gradient-to-br from-slate-950 via-zinc-900 to-black'"
    @touchstart.passive="onTouchStart"
    @touchend.passive="onTouchEnd"
    @touchcancel.passive="onTouchCancel"
    @wheel.passive="onWheel"
  >
    <!-- Background Atmospheric Effects Overlay (Pure CSS animations, z-0) -->
    <WeatherEffects v-if="activeCondition" :effect="activeCondition.effect" />

    <div class="relative z-10 flex-1 w-full h-full overflow-hidden">
      <Transition :name="transitionName">
        <div
          :key="currentCityKey"
          class="mo-weather-slide absolute inset-0 flex h-full w-full flex-col justify-between overflow-hidden will-change-transform"
        >
          <!-- Loading State (if no cached weather available for this city) -->
          <div
            v-if="currentLoading && !displayWeather"
            class="mo-weather-loading flex h-full flex-col justify-between animate-pulse"
          >
            <div class="flex items-center justify-between">
              <div class="h-4 w-36 rounded-md bg-white/30" />
              <div class="h-4 w-16 rounded-md bg-white/20" />
            </div>
            <div class="my-auto py-4 flex items-center gap-4">
              <div class="h-16 w-16 rounded-2xl bg-white/30" />
              <div class="space-y-2">
                <div class="h-12 w-28 rounded-lg bg-white/30" />
                <div class="h-4 w-24 rounded-md bg-white/20" />
              </div>
            </div>
            <div class="grid grid-cols-3 gap-2">
              <div v-for="i in 6" :key="i" class="h-14 rounded-xl bg-white/20" />
            </div>
          </div>

          <!-- Error State -->
          <div
            v-else-if="currentError && !displayWeather"
            class="mo-weather-error flex h-full flex-col items-center justify-center p-4 text-center"
          >
            <CloudFog class="h-12 w-12 text-white/60 mb-2" />
            <p class="text-xs font-medium text-white/90">{{ currentError }}</p>
            <button
              type="button"
              class="mt-3 inline-flex items-center gap-1.5 rounded-lg bg-white/20 px-3 py-1.5 text-xs font-semibold text-white shadow-xs transition hover:bg-white/30"
              @click.stop="loadCityWeather(currentCity, true)"
            >
              <RefreshCw class="h-3.5 w-3.5" />
              Retry
            </button>
          </div>

          <!-- Weather View -->
          <div v-else-if="displayWeather && activeCondition" class="flex h-full w-full flex-col justify-between">
            <!-- Top Header -->
            <div class="mo-weather-header flex items-center justify-between gap-2">
              <div class="flex min-w-0 items-center gap-1.5 text-xs font-medium text-white/95">
                <MapPin class="h-3.5 w-3.5 shrink-0 text-white/80" />
                <span class="truncate font-semibold tracking-tight">{{ place }}</span>

                <!-- Multi-city interactive controls and dots -->
                <div v-if="citiesList.length > 1" class="flex items-center gap-1 ml-1 shrink-0">
                  <button
                    type="button"
                    class="inline-flex items-center justify-center rounded-md p-1 bg-white/15 hover:bg-white/25 active:scale-95 transition text-white"
                    aria-label="Previous city"
                    title="Previous city (or swipe right)"
                    @click.stop="prevCity"
                  >
                    <ChevronLeft class="h-3.5 w-3.5" />
                  </button>

                  <div class="flex items-center gap-1 px-0.5">
                    <button
                      v-for="(item, idx) in citiesList"
                      :key="`${item.city}-${idx}`"
                      type="button"
                      class="h-1.5 rounded-full transition-all duration-300 cursor-pointer"
                      :class="idx === currentCityIndex ? 'w-4 bg-white shadow-xs' : 'w-1.5 bg-white/40 hover:bg-white/70'"
                      :aria-label="item.city"
                      :title="item.city"
                      @click.stop="goToCityIndex(idx)"
                    />
                  </div>

                  <button
                    type="button"
                    class="inline-flex items-center justify-center rounded-md p-1 bg-white/15 hover:bg-white/25 active:scale-95 transition text-white"
                    aria-label="Next city"
                    title="Next city (or swipe left)"
                    @click.stop="nextCity"
                  >
                    <ChevronRight class="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>

              <div class="flex items-center gap-1.5 shrink-0">
                <!-- Weather FX Preview Cycler -->
                <button
                  type="button"
                  class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-semibold tracking-tight transition cursor-pointer"
                  :class="previewEffect ? 'bg-amber-400/30 text-amber-100 border border-amber-300/40 shadow-xs' : 'bg-white/15 text-white/80 hover:bg-white/25 hover:text-white'"
                  aria-label="Cycle weather effect preview"
                  :title="previewEffect ? `Previewing: ${activeCondition.label} (click to cycle)` : 'Click to preview weather visual effects'"
                  @click.stop="cyclePreviewEffect"
                >
                  <Sparkles class="h-3 w-3" />
                  <span>{{ previewEffect ? PREVIEW_PRESETS[previewEffect].label : 'FX' }}</span>
                </button>

                <span class="text-[10px] font-medium text-white/70">
                  {{ displayWeather.lastUpdated }}
                </span>
                <button
                  type="button"
                  class="rounded-lg p-1.5 text-white/80 transition hover:bg-white/20 hover:text-white"
                  aria-label="Refresh weather"
                  title="Refresh weather (auto-refreshes every 30m)"
                  :disabled="currentLoading"
                  @click.stop="loadCityWeather(currentCity, true)"
                >
                  <RefreshCw class="h-3.5 w-3.5" :class="currentLoading && 'animate-spin'" />
                </button>
              </div>
            </div>

            <!-- Main Hero Section -->
            <div class="mo-weather-hero my-auto flex items-center justify-between gap-4 py-1">
              <div
                class="flex items-center gap-3 sm:gap-4 cursor-pointer group"
                title="Click to cycle weather visual effects"
                @click="cyclePreviewEffect"
              >
                <component
                  :is="activeCondition.icon"
                  class="h-14 w-14 sm:h-16 sm:w-16 shrink-0 text-white drop-shadow-md group-hover:scale-105 transition-transform"
                />
                <div>
                  <div class="flex items-baseline gap-1.5">
                    <span class="text-5xl sm:text-6xl font-black tracking-tight leading-none drop-shadow-sm">
                      {{ Math.round(displayWeather.temperature) }}°
                    </span>
                  </div>
                  <p class="mt-1 text-xs font-semibold text-white/95 drop-shadow-xs flex items-center gap-1">
                    <span>{{ activeCondition.label }}</span>
                    <span v-if="previewEffect" class="text-[10px] opacity-75 font-normal">(preview)</span>
                  </p>
                </div>
              </div>

              <div class="text-right text-xs space-y-1">
                <div class="inline-flex items-center gap-1 rounded-full bg-white/15 px-2.5 py-0.5 text-[11px] font-medium backdrop-blur-xs">
                  <Thermometer class="h-3 w-3 text-white/80" />
                  <span>Feels {{ Math.round(displayWeather.apparentTemperature) }}°</span>
                </div>
                <p class="text-[11px] font-medium text-white/85">
                  H: {{ Math.round(displayWeather.highToday) }}° · L: {{ Math.round(displayWeather.lowToday) }}°
                </p>
              </div>
            </div>

            <!-- Rich Metrics Grid (3 columns x 2 rows) -->
            <div class="mo-weather-grid grid grid-cols-3 gap-2 pb-3">
              <!-- Wind -->
              <div class="rounded-xl border border-white/15 bg-white/10 dark:bg-black/20 p-2 sm:p-2.5 backdrop-blur-xs flex flex-col justify-between">
                <div class="flex items-center justify-between text-white/75">
                  <span class="text-[10px] font-medium uppercase tracking-wider">Wind</span>
                  <Wind class="h-3.5 w-3.5" />
                </div>
                <div class="mt-1">
                  <p class="text-xs sm:text-sm font-bold leading-tight">{{ Math.round(displayWeather.windSpeed) }} km/h</p>
                  <p class="text-[10px] text-white/70">{{ displayWeather.windCardinal }}</p>
                </div>
              </div>

              <!-- Humidity -->
              <div class="rounded-xl border border-white/15 bg-white/10 dark:bg-black/20 p-2 sm:p-2.5 backdrop-blur-xs flex flex-col justify-between">
                <div class="flex items-center justify-between text-white/75">
                  <span class="text-[10px] font-medium uppercase tracking-wider">Humidity</span>
                  <Droplets class="h-3.5 w-3.5" />
                </div>
                <div class="mt-1">
                  <p class="text-xs sm:text-sm font-bold leading-tight">{{ Math.round(displayWeather.humidity) }}%</p>
                  <p class="text-[10px] text-white/70">Relative</p>
                </div>
              </div>

              <!-- Rain / Precip -->
              <div class="rounded-xl border border-white/15 bg-white/10 dark:bg-black/20 p-2 sm:p-2.5 backdrop-blur-xs flex flex-col justify-between">
                <div class="flex items-center justify-between text-white/75">
                  <span class="text-[10px] font-medium uppercase tracking-wider">Rain</span>
                  <CloudRain class="h-3.5 w-3.5" />
                </div>
                <div class="mt-1">
                  <p class="text-xs sm:text-sm font-bold leading-tight">{{ displayWeather.precipitationProbability }}%</p>
                  <p class="text-[10px] text-white/70">{{ displayWeather.precipitation }} mm</p>
                </div>
              </div>

              <!-- UV Index -->
              <div class="rounded-xl border border-white/15 bg-white/10 dark:bg-black/20 p-2 sm:p-2.5 backdrop-blur-xs flex flex-col justify-between">
                <div class="flex items-center justify-between text-white/75">
                  <span class="text-[10px] font-medium uppercase tracking-wider">UV Index</span>
                  <Sun class="h-3.5 w-3.5" />
                </div>
                <div class="mt-1">
                  <p class="text-xs sm:text-sm font-bold leading-tight">{{ Math.round(displayWeather.uvIndex) }}</p>
                  <p class="text-[10px] text-white/70">{{ uvCategory(displayWeather.uvIndex) }}</p>
                </div>
              </div>

              <!-- AQI (Air Quality Index) -->
              <div class="rounded-xl border border-white/15 bg-white/10 dark:bg-black/20 p-2 sm:p-2.5 backdrop-blur-xs flex flex-col justify-between">
                <div class="flex items-center justify-between text-white/75">
                  <span class="text-[10px] font-medium uppercase tracking-wider">Air Quality</span>
                  <Activity class="h-3.5 w-3.5" />
                </div>
                <div class="mt-1">
                  <p class="text-xs sm:text-sm font-bold leading-tight">{{ displayWeather.aqi?.aqi ?? "--" }}</p>
                  <p class="text-[10px] text-white/70 truncate">{{ displayWeather.aqi?.category ?? "AQI" }}</p>
                </div>
              </div>

              <!-- Sunrise / Sunset -->
              <div class="rounded-xl border border-white/15 bg-white/10 dark:bg-black/20 p-2 sm:p-2.5 backdrop-blur-xs flex flex-col justify-between">
                <div class="flex items-center justify-between text-white/75">
                  <span class="text-[10px] font-medium uppercase tracking-wider">Sun</span>
                  <Sunrise class="h-3.5 w-3.5" />
                </div>
                <div class="mt-1">
                  <p class="text-xs sm:text-sm font-bold leading-tight">{{ displayWeather.sunrise }}</p>
                  <p class="text-[10px] text-white/70">Set {{ displayWeather.sunset }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
/* High-performance directional horizontal slide transitions */
.h-slide-left-enter-active,
.h-slide-left-leave-active,
.h-slide-right-enter-active,
.h-slide-right-leave-active {
  transition: transform 0.36s cubic-bezier(0.16, 1, 0.3, 1),
              opacity 0.36s cubic-bezier(0.16, 1, 0.3, 1);
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  backface-visibility: hidden;
  will-change: transform, opacity;
}

/* Slide LEFT: next city (swiping left / moving forward) */
.h-slide-left-enter-from {
  transform: translate3d(100%, 0, 0);
  opacity: 0.25;
}
.h-slide-left-leave-to {
  transform: translate3d(-100%, 0, 0);
  opacity: 0.25;
}

/* Slide RIGHT: previous city (swiping right / moving backward) */
.h-slide-right-enter-from {
  transform: translate3d(-100%, 0, 0);
  opacity: 0.25;
}
.h-slide-right-leave-to {
  transform: translate3d(100%, 0, 0);
  opacity: 0.25;
}

/* Respect user accessibility motion preference */
@media (prefers-reduced-motion: reduce) {
  .h-slide-left-enter-active,
  .h-slide-left-leave-active,
  .h-slide-right-enter-active,
  .h-slide-right-leave-active {
    transition: opacity 0.2s ease !important;
    transform: none !important;
  }
}
</style>

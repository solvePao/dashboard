import { computed, ref } from "vue";
import { logger } from "../lib/logger";

export type ResolvedTheme = "dark";

// Always locked to pure black / dark theme
const currentTheme = ref<ResolvedTheme>("dark");
let initialized = false;

function apply(): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.classList.add("dark");
  root.classList.remove("light");
  root.style.colorScheme = "dark";
  root.dataset.theme = "dark";
  root.dataset.themeMode = "black";
}

function init(): void {
  if (initialized) return;
  initialized = true;

  // Clear any legacy stored light theme to prevent accidental white flashes
  try {
    localStorage.removeItem("dashboard.theme");
    localStorage.removeItem("dashboard.weather_theme");
  } catch {
    // Storage quota or disabled
  }

  apply();
  logger.debug("theme", "initialized pure black theme");
}

/**
 * Weather widget synchronization helper.
 * Theme remains permanently black/dark; atmospheric FX inside the widget adapt to weather.
 */
export function syncWeatherTheme(_isDay?: boolean, _weatherCode?: number, _label?: string): void {
  apply();
}

if (typeof window !== "undefined") {
  init();
}

export function useTheme() {
  const resolvedTheme = computed<ResolvedTheme>(() => currentTheme.value);
  const isDark = computed(() => true);

  return {
    resolvedTheme,
    isDark,
    weatherLabel: computed(() => "Black Theme"),
    themeSource: computed(() => "permanent_black" as const),
    syncWeatherTheme,
  };
}

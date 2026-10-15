import { computed, ref } from "vue";
import { logger } from "../lib/logger";

export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

const STORAGE_KEY = "dashboard.theme";
const VALID: ReadonlySet<string> = new Set(["light", "dark", "system"]);

const preference = ref<ThemePreference>("system");
const systemDark = ref(false);

let initialized = false;
let mediaQuery: MediaQueryList | null = null;

function isValid(value: string | null): value is ThemePreference {
  return value !== null && VALID.has(value);
}

function readStored(): ThemePreference {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return isValid(raw) ? raw : "system";
  } catch {
    logger.warn("theme", "theme storage unavailable; using system");
    return "system";
  }
}

function persist(value: ThemePreference): void {
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
    logger.warn("theme", "could not persist theme preference; session-only");
  }
}

function apply(resolved: ResolvedTheme): void {
  const root = document.documentElement;
  root.classList.toggle("dark", resolved === "dark");
  root.style.colorScheme = resolved;
  root.dataset.theme = resolved;
  root.dataset.themePreference = preference.value;
}

function onSystemChange(event: MediaQueryListEvent): void {
  systemDark.value = event.matches;
  logger.debug("theme", "system theme changed", event.matches ? "dark" : "light");
  if (preference.value === "system") {
    apply(event.matches ? "dark" : "light");
  }
}

function onStorage(event: StorageEvent): void {
  if (event.key !== STORAGE_KEY) return;
  const next = isValid(event.newValue) ? event.newValue : "system";
  preference.value = next;
  apply(resolveTheme(next, systemDark.value));
  logger.debug("theme", "theme preference synced from another tab", next);
}

export function resolveTheme(pref: ThemePreference, prefersDark: boolean): ResolvedTheme {
  if (pref === "system") return prefersDark ? "dark" : "light";
  return pref;
}

function init(): void {
  if (initialized) return;
  initialized = true;
  preference.value = readStored();
  mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  systemDark.value = mediaQuery.matches;
  const resolved = resolveTheme(preference.value, systemDark.value);
  apply(resolved);
  logger.debug("theme", "initialized theme", { preference: preference.value, resolved });
  mediaQuery.addEventListener("change", onSystemChange);
  window.addEventListener("storage", onStorage);
}

function setPreference(next: ThemePreference): void {
  preference.value = next;
  persist(next);
  apply(resolveTheme(next, systemDark.value));
  logger.debug("theme", "theme preference changed", next);
}

function toggle(): void {
  // Simple light <-> dark flip. From system, flip opposite of resolved.
  const resolved = resolveTheme(preference.value, systemDark.value);
  setPreference(resolved === "dark" ? "light" : "dark");
}

if (typeof window !== "undefined") {
  init();
}

export function useTheme() {
  const resolvedTheme = computed<ResolvedTheme>(() =>
    resolveTheme(preference.value, systemDark.value),
  );
  const isDark = computed(() => resolvedTheme.value === "dark");

  return {
    preference,
    resolvedTheme,
    isDark,
    setPreference,
    toggle,
  };
}

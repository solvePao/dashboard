<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch, useTemplateRef } from "vue";
import { Loader2, MapPin, Search, X } from "@lucide/vue";
import { searchCities, type CityOption } from "../services/geocodingApi";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "select", city: CityOption): void;
}>();

const query = ref("");
const results = ref<CityOption[]>([]);
const searching = ref(false);
const searchError = ref<string | null>(null);
const inputRef = useTemplateRef<HTMLInputElement>("inputRef");

let debounceTimer: ReturnType<typeof setTimeout> | undefined;
let aborter: AbortController | null = null;
let requestSeq = 0;

function clearSearch(): void {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = undefined;
  }
  aborter?.abort();
  aborter = null;
}

async function runSearch(text: string, seq: number): Promise<void> {
  aborter?.abort();
  aborter = new AbortController();
  searching.value = true;
  searchError.value = null;
  try {
    const found = await searchCities(text, aborter.signal);
    // Drop stale responses when the user kept typing.
    if (seq === requestSeq) {
      results.value = found;
    }
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") return;
    if (seq === requestSeq) {
      results.value = [];
      searchError.value = err instanceof Error ? err.message : "City search failed.";
    }
  } finally {
    if (seq === requestSeq) {
      searching.value = false;
    }
  }
}

watch(query, (text) => {
  clearSearch();
  results.value = [];
  searchError.value = null;
  if (text.trim().length < 2) {
    searching.value = false;
    return;
  }
  const seq = ++requestSeq;
  debounceTimer = setTimeout(() => void runSearch(text, seq), 350);
});

function handleSelect(city: CityOption): void {
  emit("select", city);
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") emit("close");
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      query.value = "";
      results.value = [];
      searchError.value = null;
      searching.value = false;
      window.addEventListener("keydown", handleKeydown);
      void nextTick(() => inputRef.value?.focus());
    } else {
      window.removeEventListener("keydown", handleKeydown);
      clearSearch();
    }
  }
);

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleKeydown);
  clearSearch();
});
</script>

<template>
  <div
    v-if="open"
    class="fe-citymodal-overlay fixed inset-0 z-[60] flex items-center justify-center p-4 bg-background/80 backdrop-blur-sm animate-in fade-in-0 duration-200"
    role="dialog"
    aria-modal="true"
    aria-labelledby="city-picker-title"
  >
    <div
      class="fe-citymodal-dialog relative w-full max-w-md rounded-2xl border border-border bg-card p-6 text-card-foreground shadow-2xl outline-none animate-in zoom-in-95 duration-200"
    >
      <div class="fe-citymodal-header flex items-center justify-between border-b border-border/80 pb-4">
        <div class="fe-citymodal-header-text">
          <h2 id="city-picker-title" class="fe-citymodal-title text-base font-semibold leading-tight text-foreground">
            Choose a city
          </h2>
          <p class="fe-citymodal-subtitle text-xs text-muted-foreground mt-0.5">
            Search for the city to show weather for
          </p>
        </div>
        <button
          type="button"
          class="fe-citymodal-close-btn rounded-lg p-1.5 text-muted-foreground transition hover:bg-accent hover:text-foreground"
          aria-label="Close"
          @click="emit('close')"
        >
          <X class="fe-citymodal-close-icon h-4 w-4" />
        </button>
      </div>

      <div class="fe-citymodal-search relative mt-4">
        <Search class="fe-citymodal-search-icon pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <input
          ref="inputRef"
          v-model="query"
          type="text"
          placeholder="Type at least 2 letters, e.g. Berlin"
          autocomplete="off"
          class="fe-citymodal-input w-full rounded-lg border border-input bg-background py-2 pl-9 pr-9 text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
        <Loader2
          v-if="searching"
          class="fe-citymodal-searching-icon absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 animate-spin text-primary"
        />
      </div>

      <div
        v-if="searchError"
        class="fe-citymodal-error mt-3 rounded-xl border border-destructive/20 bg-destructive/10 p-3 text-xs text-destructive"
        role="alert"
      >
        {{ searchError }}
      </div>

      <ul v-else-if="results.length > 0" class="fe-citymodal-results mt-3 max-h-64 space-y-1 overflow-y-auto">
        <li v-for="city in results" :key="`${city.latitude},${city.longitude}`">
          <button
            type="button"
            class="fe-citymodal-result-btn flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-left transition hover:bg-accent"
            @click="handleSelect(city)"
          >
            <MapPin class="fe-citymodal-result-icon h-4 w-4 shrink-0 text-primary" />
            <span class="fe-citymodal-result-text min-w-0">
              <span class="block truncate text-sm font-medium text-foreground">{{ city.name }}</span>
              <span v-if="city.country" class="block truncate text-xs text-muted-foreground">
                {{ [city.admin1, city.country].filter(Boolean).join(", ") }}
              </span>
            </span>
          </button>
        </li>
      </ul>

      <p
        v-else-if="query.trim().length >= 2 && !searching"
        class="fe-citymodal-empty mt-3 rounded-xl bg-muted p-3 text-center text-xs text-muted-foreground"
      >
        No cities found. Try another spelling.
      </p>
    </div>
  </div>
</template>

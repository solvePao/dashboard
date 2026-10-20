<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Loader2, RefreshCw, Smartphone } from "@lucide/vue";
import { useDeviceAuth } from "../composables/useDeviceAuth";
import { useWidgets } from "../composables/useWidgets";
import { asWeatherConfig } from "../types/widget";
import DevicePairingSteps from "./DevicePairingSteps.vue";
import WeatherWidget from "./widgets/WeatherWidget.vue";
import VerticalSlidePager, { type PagerItem } from "./VerticalSlidePager.vue";
import DeviceAuthorizedMinimalView from "./DeviceAuthorizedMinimalView.vue";

const {
  authState,
  statusMessage,
  isAuthenticated,
  checkAuth,
  user,
} = useDeviceAuth();

const { widgets, refreshWidgets, startWidgetSync } = useWidgets();

const activeTab = ref<"weather" | "clock">("weather");

const leftPanelItems: PagerItem[] = [
  { key: "weather", label: "View Weather" },
  { key: "clock", label: "View Clock" },
];

const weatherWidget = computed(() => {
  return widgets.value.find(
    (w) => w.widget_key === "weather" && w.enabled && asWeatherConfig(w)
  );
});

const weatherConfig = computed(() => {
  return weatherWidget.value ? asWeatherConfig(weatherWidget.value) : null;
});

watch(
  () => isAuthenticated.value,
  (authed) => {
    if (authed) {
      void refreshWidgets();
    }
  },
  { immediate: true }
);

onMounted(() => {
  startWidgetSync();
  if (isAuthenticated.value) {
    void refreshWidgets();
  }
});
</script>

<template>
  <div class="mo-leftpanel-root relative flex h-full w-full flex-col justify-between overflow-hidden">
    <!-- Authenticated State -->
    <template v-if="isAuthenticated">
      <!-- Case 1: Weather is configured -> Smooth vertical slide pager -->
      <VerticalSlidePager
        v-if="weatherConfig"
        v-model="activeTab"
        :items="leftPanelItems"
        :dot-variant="activeTab === 'weather' ? 'light' : 'theme'"
      >
        <template #weather>
          <WeatherWidget :config="weatherConfig" />
        </template>

        <template #clock>
          <div class="relative flex h-full w-full flex-col justify-between overflow-hidden">
            <DeviceAuthorizedMinimalView v-if="user" :user="user" />
          </div>
        </template>

      </VerticalSlidePager>

      <!-- Case 2: No weather configured -> Show Clock directly -->
      <div v-else-if="user" class="relative flex h-full w-full flex-col justify-between overflow-hidden">
        <DeviceAuthorizedMinimalView :user="user" />
      </div>

      <!-- Case 3: Authenticated but waiting for profile -->
      <div v-else class="relative flex h-full w-full flex-col justify-between p-4 sm:p-5 overflow-y-auto">
        <div class="mo-instr-header flex items-center justify-between border-b border-border/60 pb-3">
          <div class="flex items-center gap-2">
            <span class="flex h-7 w-7 items-center justify-center rounded-lg bg-primary/10 text-primary">
              <Smartphone class="h-4 w-4" />
            </span>
            <span class="text-sm font-semibold tracking-tight text-foreground">
              Device Linked
            </span>
          </div>
        </div>

        <div class="my-auto py-6 text-center flex flex-col items-center justify-center">
          <div class="flex h-12 w-12 items-center justify-center rounded-2xl bg-primary/10 text-primary mb-3">
            <Smartphone class="h-6 w-6" />
          </div>
          <h3 class="text-sm font-semibold text-foreground">Loading profile…</h3>
        </div>
      </div>
    </template>

    <!-- Unauthenticated / Pairing State -->
    <template v-else>
      <div class="flex h-full w-full flex-col justify-between p-4 sm:p-5">
        <!-- Header -->
        <div class="mo-instr-header flex items-center justify-between border-b border-border/60 pb-3">
          <div class="flex items-center gap-2">
            <span class="flex h-7 w-7 items-center justify-center rounded-lg bg-primary/10 text-primary">
              <Smartphone class="h-4 w-4" />
            </span>
            <span class="text-sm font-semibold tracking-tight text-foreground">
              Device Pairing
            </span>
          </div>
        </div>

        <!-- Body: Pairing steps -->
        <div class="my-auto py-3">
          <DevicePairingSteps
            :auth-state="authState"
            :status-message="statusMessage"
          />
        </div>

        <!-- Footer: Check auth status button -->
        <div class="border-t border-border/60 pt-3 flex items-center justify-between">
          <span class="text-[11px] text-muted-foreground">
            {{ authState === "checking" ? "Checking…" : "Waiting for authorization…" }}
          </span>
          <button
            type="button"
            class="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-border bg-card text-foreground shadow-xs transition hover:bg-accent disabled:opacity-50"
            :disabled="authState === 'checking' || authState === 'loading'"
            aria-label="Check for Auth"
            title="Check for Auth"
            @click="checkAuth"
          >
            <Loader2 v-if="authState === 'checking' || authState === 'loading'" class="h-4 w-4 animate-spin text-primary" />
            <RefreshCw v-else class="h-4 w-4" />
          </button>
        </div>
      </div>
    </template>
  </div>
</template>

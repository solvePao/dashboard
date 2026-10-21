<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import {
  AlertCircle,
  Clock,
  Loader2,
  Maximize,
  Minimize,
  Radio,
  RefreshCw,
  Smartphone,
} from "@lucide/vue";
import { useDeviceAuth } from "../composables/useDeviceAuth";
import { useOrientationLock } from "../composables/useOrientationLock";
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
  createSession,
  user,
} = useDeviceAuth();

const { isFullscreen, fullscreenSupported, toggleFullscreen } = useOrientationLock();

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
      <div class="mo-pairing-instr-root flex h-full w-full flex-col justify-between p-4 sm:p-5 overflow-hidden">
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

          <!-- Header Right: Quick Fullscreen & Reactive Status Pill -->
          <div class="flex items-center gap-2">
            <button
              v-if="fullscreenSupported"
              type="button"
              class="inline-flex h-7 w-7 items-center justify-center rounded-lg border border-border/70 bg-card text-muted-foreground transition hover:bg-accent hover:text-foreground active:scale-95"
              :title="isFullscreen ? 'Exit Fullscreen' : 'Enter Fullscreen'"
              :aria-label="isFullscreen ? 'Exit Fullscreen' : 'Enter Fullscreen'"
              @click="toggleFullscreen"
            >
              <Minimize v-if="isFullscreen" class="h-3.5 w-3.5" />
              <Maximize v-else class="h-3.5 w-3.5" />
            </button>

            <!-- Status Pill -->
            <div
              v-if="authState === 'pending'"
              class="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2.5 py-0.5 text-[11px] font-medium text-emerald-400"
            >
              <span class="relative flex h-1.5 w-1.5">
                <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                <span class="relative inline-flex rounded-full h-1.5 w-1.5 bg-emerald-500"></span>
              </span>
              <span>Ready to Pair</span>
            </div>

            <div
              v-else-if="authState === 'checking'"
              class="inline-flex items-center gap-1.5 rounded-full border border-primary/20 bg-primary/10 px-2.5 py-0.5 text-[11px] font-medium text-primary"
            >
              <Loader2 class="h-3 w-3 animate-spin text-primary" />
              <span>Checking…</span>
            </div>

            <div
              v-else-if="authState === 'loading'"
              class="inline-flex items-center gap-1.5 rounded-full border border-primary/20 bg-primary/10 px-2.5 py-0.5 text-[11px] font-medium text-primary"
            >
              <Loader2 class="h-3 w-3 animate-spin text-primary" />
              <span>Connecting…</span>
            </div>

            <div
              v-else-if="authState === 'expired'"
              class="inline-flex items-center gap-1 rounded-full border border-destructive/20 bg-destructive/10 px-2.5 py-0.5 text-[11px] font-medium text-destructive"
            >
              <Clock class="h-3 w-3" />
              <span>Expired</span>
            </div>

            <div
              v-else-if="authState === 'error'"
              class="inline-flex items-center gap-1 rounded-full border border-destructive/20 bg-destructive/10 px-2.5 py-0.5 text-[11px] font-medium text-destructive"
            >
              <AlertCircle class="h-3 w-3" />
              <span>Offline</span>
            </div>
          </div>
        </div>

        <!-- Body: Pairing steps -->
        <div class="my-auto flex flex-1 flex-col justify-center min-h-0 overflow-y-auto overscroll-contain py-1 sm:py-2">
          <DevicePairingSteps
            :auth-state="authState"
            :status-message="statusMessage"
            @refresh="createSession"
            @check="checkAuth"
          />
        </div>

        <!-- Footer: Real-time status & Check Auth button -->
        <div class="border-t border-border/60 pt-3 flex items-center justify-between text-[11px] text-muted-foreground">
          <!-- Left: Real-time Connection State -->
          <div class="flex items-center gap-1.5 truncate pr-2">
            <template v-if="authState === 'checking'">
              <Loader2 class="h-3 w-3 animate-spin text-primary shrink-0" />
              <span class="truncate">Verifying authorization with server…</span>
            </template>
            <template v-else-if="authState === 'loading'">
              <Loader2 class="h-3 w-3 animate-spin text-primary shrink-0" />
              <span class="truncate">Preparing pairing session…</span>
            </template>
            <template v-else-if="authState === 'expired'">
              <Clock class="h-3 w-3 text-destructive shrink-0" />
              <span class="truncate text-destructive font-medium">Pairing code expired</span>
            </template>
            <template v-else-if="authState === 'error'">
              <AlertCircle class="h-3 w-3 text-destructive shrink-0" />
              <span class="truncate text-destructive font-medium">Connection failed</span>
            </template>
            <template v-else>
              <Radio class="h-3 w-3 text-emerald-400 shrink-0 animate-pulse" />
              <span class="truncate">Listening for web authorization…</span>
            </template>
          </div>

          <!-- Right: Check Status Button -->
          <button
            type="button"
            class="mo-pairing-check-btn inline-flex items-center gap-1.5 shrink-0 rounded-md px-2.5 py-1 text-[11px] font-medium text-muted-foreground transition hover:bg-accent hover:text-foreground active:scale-95 disabled:opacity-50"
            :disabled="authState === 'checking' || authState === 'loading'"
            aria-label="Check authorization status"
            title="Check authorization status"
            @click="checkAuth"
          >
            <Loader2 v-if="authState === 'checking' || authState === 'loading'" class="h-3 w-3 animate-spin text-primary" />
            <RefreshCw v-else class="h-3 w-3" />
            <span>Check Status</span>
          </button>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from "vue";
import { Settings } from "@lucide/vue";
import DashboardWidgetPanel from "./components/DashboardWidgetPanel.vue";
import HorizontalSplitLayout from "./components/HorizontalSplitLayout.vue";
import DeviceLeftPanel from "./components/DeviceLeftPanel.vue";
import DevicePairingView from "./components/DevicePairingView.vue";
import DeviceSettingsModal from "./components/DeviceSettingsModal.vue";
import CallerWaitingModal from "./components/widgets/CallerWaitingModal.vue";
import { useDeviceAuth } from "./composables/useDeviceAuth";
import { useOrientationLock } from "./composables/useOrientationLock";
import { useWalkieTalkie } from "./composables/useWalkieTalkie";
import { useWidgetLayout } from "./composables/useWidgetLayout";
import { useWidgets } from "./composables/useWidgets";
import { WIDGET_SIDES, type WidgetKey, type WidgetSide } from "./types/layout";
import { resolveDashboardWidgets } from "./widgets/registry";

const { authState, createSession, expiresAt, isAuthenticated, qrSvg, shortCode, user } = useDeviceAuth();
const { isFullscreen, fullscreenSupported, fullscreenPending, fullscreenError, toggleFullscreen } = useOrientationLock();
const { widgets, refreshWidgets, resetWidgets, startWidgetSync } = useWidgets();
const showSettingsModal = ref(false);

const userId = computed(() => isAuthenticated.value ? user.value?.id : undefined);
const availableWidgets = computed(() => resolveDashboardWidgets(widgets.value, userId.value === undefined ? null : user.value));
const { layout, panels, layoutSaveFailed, moveWidget: saveWidgetMove, resetLayout: resetSavedLayout } = useWidgetLayout(userId, availableWidgets);
const selected = reactive<Record<WidgetSide, WidgetKey | null>>({ left: null, right: null });
const instantCallerSide = ref<WidgetSide>();

const callerSide = computed(() => WIDGET_SIDES.find((side) => panels.value[side].some((widget) => widget.key === "walkie_talkie")));
const callerGroup = computed(() => {
  const widget = widgets.value.find((item) => item.widget_key === "walkie_talkie" && item.enabled);
  return typeof widget?.config.group_id === "string" ? widget.config.group_id : null;
});
const caller = useWalkieTalkie(callerGroup, computed(() => callerSide.value !== undefined));

watch(userId, (id) => {
  selected.left = null;
  selected.right = null;
  showSettingsModal.value = false;
  resetWidgets();
  if (id !== undefined) void refreshWidgets();
}, { immediate: true });

watch(panels, (next) => {
  for (const side of WIDGET_SIDES) {
    if (!next[side].some((widget) => widget.key === selected[side])) selected[side] = null;
  }
});

watch(caller.incomingSequence, () => {
  const side = callerSide.value;
  if (!side || selected[side] === "walkie_talkie") return;
  instantCallerSide.value = side;
  selected[side] = "walkie_talkie";
  void nextTick(() => { instantCallerSide.value = undefined; });
});

onMounted(() => { startWidgetSync(); });

function moveWidget(key: WidgetKey, side: WidgetSide, index: number): void {
  const source = saveWidgetMove(key, side, index);
  if (source) {
    if (source !== side) selected[source] = null;
    selected[side] = key;
  }
}

function resetLayout(): void {
  resetSavedLayout();
  selected.left = null;
  selected.right = null;
}
</script>

<template>
  <div class="mo-app-root orientation-locked-container bg-background text-foreground relative">
    <HorizontalSplitLayout>
      <template #left>
        <DashboardWidgetPanel v-if="isAuthenticated" :key="layout.left.join(',')" v-model="selected.left" :widgets="panels.left" :skip-transition="instantCallerSide === 'left'" />
        <DeviceLeftPanel v-else />
      </template>

      <template #right>
        <DashboardWidgetPanel v-if="isAuthenticated" :key="layout.right.join(',')" v-model="selected.right" :widgets="panels.right" :skip-transition="instantCallerSide === 'right'" />
        <DevicePairingView
          v-else
          :auth-state="authState"
          :qr-svg="qrSvg"
          :short-code="shortCode"
          :expires-at="expiresAt"
          @refresh="createSession"
        />
      </template>
    </HorizontalSplitLayout>

    <CallerWaitingModal v-if="callerSide" :target="`.mo-split-${callerSide}`" />

    <!-- Floating Settings Button (Bottom Left) -->
    <button
      v-if="isAuthenticated"
      type="button"
      class="mo-floating-settings-btn fixed bottom-3.5 left-3.5 z-40 flex h-9 w-9 items-center justify-center rounded-full border border-border bg-card/90 text-muted-foreground shadow-lg backdrop-blur-xs transition hover:bg-accent hover:text-foreground active:scale-95"
      aria-label="Open settings and profile"
      title="Settings & Profile"
      @click="showSettingsModal = true"
    >
      <Settings class="h-4.5 w-4.5" />
    </button>

    <!-- Settings & Profile Modal -->
    <DeviceSettingsModal
      :open="showSettingsModal"
      :is-fullscreen="isFullscreen"
      :fullscreen-supported="fullscreenSupported"
      :fullscreen-pending="fullscreenPending"
      :fullscreen-error="fullscreenError"
      :widget-panels="panels"
      :layout-save-failed="layoutSaveFailed"
      @move-widget="moveWidget"
      @reset-layout="resetLayout"
      @toggle-fullscreen="toggleFullscreen"
      @close="showSettingsModal = false"
    />
  </div>
</template>

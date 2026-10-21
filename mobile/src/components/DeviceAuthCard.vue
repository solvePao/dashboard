<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useDeviceAuth } from "../composables/useDeviceAuth";
import { useWidgets } from "../composables/useWidgets";
import { asStockConfig } from "../types/widget";
import DevicePairingView from "./DevicePairingView.vue";
import WalkieTalkieWidget from "./widgets/WalkieTalkieWidget.vue";
import StockWidget from "./widgets/StockWidget.vue";
import VerticalSlidePager, { type PagerItem } from "./VerticalSlidePager.vue";

const {
  shortCode,
  qrSvg,
  expiresAt,
  user,
  authState,
  isAuthenticated,
  createSession,
} = useDeviceAuth();

const { widgets } = useWidgets();

const walkieWidget = computed(() =>
  widgets.value.find((w) => w.widget_key === "walkie_talkie" && w.enabled)
);

const stockWidget = computed(() => {
  return widgets.value.find(
    (w) => w.widget_key === "stock" && w.enabled && asStockConfig(w)
  );
});

const stockConfig = computed(() => {
  return stockWidget.value ? asStockConfig(stockWidget.value) : null;
});

const rightPanelItems = computed<PagerItem[]>(() => {
  const items: PagerItem[] = [];
  if (walkieWidget.value) {
    items.push({ key: "walkie", label: "Walkie-talkie" });
  }
  if (stockConfig.value) {
    items.push({ key: "stock", label: "Stocks" });
  }
  return items;
});

const selectedPanel = ref("walkie");
watch(
  rightPanelItems,
  (items) => {
    if (items.length > 0 && !items.some((i) => i.key === selectedPanel.value)) {
      selectedPanel.value = items[0].key;
    }
  },
  { immediate: true }
);

const isStockInspected = ref(false);
function handleStockViewMode(mode: "grid" | "detail"): void {
  isStockInspected.value = mode === "detail";
}
</script>

<template>
  <div class="mo-authcard-root relative flex h-full w-full flex-col justify-between overflow-hidden">
    <!-- Authenticated State -->
    <template v-if="isAuthenticated && user">
      <!-- Case 1: Both Walkie-talkie & Stocks configured -> Vertical Slide Pager -->
      <VerticalSlidePager
        v-if="walkieWidget && stockConfig"
        v-model="selectedPanel"
        :items="rightPanelItems"
        :enabled="!isStockInspected"
        :dot-variant="selectedPanel === 'stock' ? 'amber' : 'theme'"
      >
        <template #walkie>
          <div class="relative flex h-full min-h-0 w-full flex-col justify-between overflow-hidden">
            <WalkieTalkieWidget :config="walkieWidget.config" />
          </div>
        </template>

        <template #stock>
          <div class="relative flex h-full w-full flex-col justify-between overflow-hidden">
            <StockWidget :config="stockConfig" @view-mode-change="handleStockViewMode" />
          </div>
        </template>
      </VerticalSlidePager>

      <!-- Case 2: Only Walkie-talkie configured -->
      <div
        v-else-if="walkieWidget"
        class="relative flex h-full min-h-0 w-full flex-col justify-between overflow-hidden"
      >
        <WalkieTalkieWidget :config="walkieWidget.config" />
      </div>

      <!-- Case 3: Only Stocks configured -->
      <div
        v-else-if="stockConfig"
        class="relative flex h-full w-full flex-col justify-between overflow-hidden"
      >
        <StockWidget :config="stockConfig" @view-mode-change="handleStockViewMode" />
      </div>

      <!-- Case 4: Neither configured -->
      <div
        v-else
        class="relative flex h-full w-full flex-col items-center justify-center p-4 text-center"
      >
        <span class="text-xs text-muted-foreground">No stock or walkie-talkie configured.</span>
      </div>
    </template>

    <!-- Unauthenticated / Pairing State -->
    <DevicePairingView
      v-else
      :auth-state="authState"
      :qr-svg="qrSvg"
      :short-code="shortCode"
      :expires-at="expiresAt"
      @refresh="createSession"
    />
  </div>
</template>

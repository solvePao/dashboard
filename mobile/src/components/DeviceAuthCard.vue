<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useDeviceAuth } from "../composables/useDeviceAuth";
import { useWidgets } from "../composables/useWidgets";
import { asStockConfig } from "../types/widget";
import DevicePairingView from "./DevicePairingView.vue";
import WalkieTalkieWidget from "./widgets/WalkieTalkieWidget.vue";
import StockWidget from "./widgets/StockWidget.vue";

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

const walkieWidget = computed(() => widgets.value.find(w => w.widget_key === "walkie_talkie" && w.enabled));
const showStocks = computed(() => !walkieWidget.value || selectedPanel.value === "stock");
const selectedPanel = ref("walkie");
watch(() => walkieWidget.value, () => { selectedPanel.value = "walkie"; });

const stockWidget = computed(() => {
  return widgets.value.find(
    (w) => w.widget_key === "stock" && w.enabled && asStockConfig(w)
  );
});

const stockConfig = computed(() => {
  return stockWidget.value ? asStockConfig(stockWidget.value) : null;
});
</script>

<template>
  <div class="mo-authcard-root relative flex h-full w-full flex-col justify-between overflow-hidden">
    <!-- Authenticated State -->
    <template v-if="isAuthenticated && user">
      <div v-if="walkieWidget && stockConfig" class="flex shrink-0 gap-1 border-b border-border bg-card p-1.5" role="group" aria-label="Device widgets">
        <button class="flex-1 rounded-lg px-3 py-1.5 text-xs font-semibold" :class="selectedPanel === 'walkie' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground'" @click="selectedPanel = 'walkie'">Walkie-talkie</button>
        <button class="flex-1 rounded-lg px-3 py-1.5 text-xs font-semibold" :class="selectedPanel === 'stock' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground'" @click="selectedPanel = 'stock'">Stocks</button>
      </div>
      <div v-if="walkieWidget" v-show="!showStocks" class="min-h-0 flex-1">
        <WalkieTalkieWidget :config="walkieWidget.config" />
      </div>
      <div v-if="stockConfig" v-show="showStocks" class="min-h-0 flex-1"><StockWidget :config="stockConfig" /></div>

      <!-- No stock configured: placeholder state -->
      <div v-if="!stockConfig && !walkieWidget" class="relative flex h-full w-full flex-col items-center justify-center p-4 text-center">
        <span class="text-xs text-muted-foreground">No stock widget configured.</span>
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

<script setup lang="ts">
import { computed } from "vue";
import { useDeviceAuth } from "../composables/useDeviceAuth";
import { useWidgets } from "../composables/useWidgets";
import { asStockConfig } from "../types/widget";
import DevicePairingView from "./DevicePairingView.vue";
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
      <!-- Stock widget if configured -->
      <StockWidget v-if="stockConfig" :config="stockConfig" />

      <!-- No stock configured: placeholder state -->
      <div v-else class="relative flex h-full w-full flex-col items-center justify-center p-4 text-center">
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

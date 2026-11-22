<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { WidgetKey } from "../types/layout";
import type { DashboardWidget } from "../widgets/registry";
import VerticalSlidePager from "./VerticalSlidePager.vue";

const props = defineProps<{ widgets: DashboardWidget[] }>();
const selected = defineModel<WidgetKey | null>({ required: true });

const activeWidget = computed(() =>
  props.widgets.find((widget) => widget.key === selected.value) ?? props.widgets[0],
);
const activeKey = computed({
  get: () => activeWidget.value?.key ?? "",
  set: (key: string) => {
    const widget = props.widgets.find((item) => item.key === key);
    if (widget) selected.value = widget.key;
  },
});
const isStockInspected = ref(false);

watch(activeKey, () => { isStockInspected.value = false; });

function handleStockViewMode(mode: "grid" | "detail"): void {
  isStockInspected.value = activeKey.value === "stock" && mode === "detail";
}
</script>

<template>
  <VerticalSlidePager
    v-if="activeWidget"
    v-model="activeKey"
    :items="widgets"
    :enabled="!isStockInspected"
    :dot-variant="activeWidget.dotVariant ?? 'theme'"
  >
    <component :is="activeWidget.component" v-bind="activeWidget.props" @view-mode-change="handleStockViewMode" />
  </VerticalSlidePager>
  <div v-else class="flex h-full items-center justify-center p-4 text-center text-xs text-muted-foreground">
    No widgets in this panel.
  </div>
</template>

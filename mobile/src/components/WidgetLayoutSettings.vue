<script setup lang="ts">
import { nextTick, ref } from "vue";
import { WIDGET_SIDES, type WidgetKey, type WidgetSide } from "../types/layout";
import type { DashboardWidgetPanels } from "../widgets/registry";

const props = defineProps<{ panels: DashboardWidgetPanels; saveFailed: boolean }>();
const emit = defineEmits<{
  move: [key: WidgetKey, side: WidgetSide, index: number];
  reset: [];
}>();
const section = ref<HTMLElement | null>(null);
const sideLabel: Record<WidgetSide, string> = { left: "Left panel", right: "Right panel" };

async function handleMove(key: WidgetKey, event: Event): Promise<void> {
  if (!(event.target instanceof HTMLSelectElement)) return;
  const side = event.target.value;
  if (side !== "left" && side !== "right") return;

  emit("move", key, side, props.panels[side].length);
  await nextTick();
  section.value?.querySelector<HTMLSelectElement>(`#widget-move-${key}`)?.focus();
}
</script>

<template>
  <section ref="section" aria-labelledby="widget-layout-heading" class="space-y-3 rounded-xl border border-border/70 bg-muted/20 p-3.5">
    <div class="flex items-center justify-between">
      <h3 id="widget-layout-heading" class="text-sm font-semibold text-foreground">Widget Layout</h3>
      <button
        type="button"
        class="text-xs text-muted-foreground transition hover:text-foreground active:scale-95 underline underline-offset-2"
        @click="emit('reset')"
      >
        Reset layout
      </button>
    </div>

    <div v-for="side in WIDGET_SIDES" :key="side" class="space-y-1.5">
      <h4 :id="`widget-layout-${side}`" class="text-xs font-medium text-muted-foreground">{{ sideLabel[side] }}</h4>
      <ol v-if="panels[side].length" :aria-labelledby="`widget-layout-${side}`" class="divide-y divide-border/70 rounded-lg border border-border/70 bg-background/60">
        <li v-for="widget in panels[side]" :key="widget.key" class="flex min-w-0 items-center justify-between gap-2 px-2.5 py-0.5">
          <span class="min-w-0 text-xs font-medium text-foreground">{{ widget.label }}</span>
          <select
            :id="`widget-move-${widget.key}`"
            :aria-label="`Move ${widget.label}`"
            class="min-h-11 w-28 shrink-0 rounded-md border border-border bg-background px-2 text-xs font-medium text-foreground focus-visible:outline-2 focus-visible:outline-ring"
            :value="side"
            @change="handleMove(widget.key, $event)"
          >
            <option v-for="target in WIDGET_SIDES" :key="target" :value="target">{{ sideLabel[target] }}</option>
          </select>
        </li>
      </ol>
      <p v-else class="text-xs text-muted-foreground">No widgets</p>
    </div>

    <p v-if="saveFailed" role="status" class="text-xs text-muted-foreground">
      This browser couldn’t save the layout. Your changes will last for this session.
    </p>
  </section>
</template>

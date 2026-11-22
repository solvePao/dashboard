import { computed, ref, watch, type Ref } from "vue";
import { WIDGET_SIDES, type WidgetKey, type WidgetLayout, type WidgetSide } from "../types/layout";
import type { DashboardWidget, DashboardWidgetPanels } from "../widgets/registry";

export const DEFAULT_LAYOUT: WidgetLayout = {
  left: ["weather", "clock"],
  right: ["walkie_talkie", "stock"],
};

export function normalizeLayout(saved: unknown): WidgetLayout {
  const layout: WidgetLayout = { left: [], right: [] };
  const remaining = new Set([...DEFAULT_LAYOUT.left, ...DEFAULT_LAYOUT.right]);
  if (saved && typeof saved === "object") {
    const stored = saved as Partial<Record<WidgetSide, unknown>>;
    for (const side of WIDGET_SIDES) {
      const keys = stored[side];
      if (!Array.isArray(keys)) continue;
      for (const key of keys) {
        if (remaining.delete(key as WidgetKey)) layout[side].push(key as WidgetKey);
      }
    }
  }
  for (const side of WIDGET_SIDES) {
    layout[side].push(...DEFAULT_LAYOUT[side].filter((key) => remaining.has(key)));
  }
  return layout;
}

export function useWidgetLayout(
  userId: Readonly<Ref<number | undefined>>,
  available: Readonly<Ref<ReadonlyMap<WidgetKey, DashboardWidget>>>,
) {
  const layout = ref<WidgetLayout>(normalizeLayout(null));
  const layoutSaveFailed = ref(false);
  const storageKey = computed(() => userId.value === undefined ? null : `dashboard.mobile.widget_layout.${userId.value}`);

  watch(storageKey, (key) => {
    layoutSaveFailed.value = false;
    try {
      const raw = key && typeof localStorage !== "undefined" ? localStorage.getItem(key) : null;
      layout.value = normalizeLayout(raw ? JSON.parse(raw) : null);
    } catch {
      layout.value = normalizeLayout(null);
    }
  }, { immediate: true });

  function visibleWidgets(side: WidgetSide): DashboardWidget[] {
    return layout.value[side].flatMap((key) => {
      const widget = available.value.get(key);
      return widget ? [widget] : [];
    });
  }

  const panels = computed<DashboardWidgetPanels>(() => ({
    left: visibleWidgets("left"),
    right: visibleWidgets("right"),
  }));

  function moveWidget(key: WidgetKey, side: WidgetSide, index: number): WidgetSide | null {
    const source = WIDGET_SIDES.find((candidate) => panels.value[candidate].some((widget) => widget.key === key));
    if (!source || !storageKey.value) return null;

    const others = panels.value[side].filter((widget) => widget.key !== key);
    if (!Number.isInteger(index) || index < 0 || index > others.length) return null;
    if (source === side && panels.value[side][index]?.key === key) return null;

    const next = { ...layout.value, [source]: layout.value[source].filter((widgetKey) => widgetKey !== key) };
    const destination = [...next[side]];
    // Anchor to a visible neighbor so hidden widgets keep their saved order.
    const before = others[index];
    destination.splice(before ? destination.indexOf(before.key) : destination.length, 0, key);
    next[side] = destination;
    layout.value = next;

    try {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem(storageKey.value, JSON.stringify(next));
      }
      layoutSaveFailed.value = false;
    } catch {
      layoutSaveFailed.value = true;
    }
    return source;
  }

  function resetLayout(): void {
    layout.value = normalizeLayout(null);
    if (storageKey.value && typeof localStorage !== "undefined") {
      try {
        localStorage.removeItem(storageKey.value);
        layoutSaveFailed.value = false;
      } catch {
        layoutSaveFailed.value = true;
      }
    }
  }

  return { layout, panels, layoutSaveFailed, moveWidget, resetLayout };
}

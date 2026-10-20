import type { Component } from "vue";
import WeatherWidget from "../components/widgets/WeatherWidget.vue";
import WalkieTalkieWidget from "../components/widgets/WalkieTalkieWidget.vue";
import StockWidget from "../components/widgets/StockWidget.vue";

/**
 * Mobile Widget Registry: maps a widget key to its render Strategy.
 * Unknown keys render nothing, so the backend and web can ship a new
 * key first and the mobile follows by adding one entry here plus its
 * component — the host never changes.
 */
export interface MobileWidgetEntry {
  key: string;
  component: Component;
}

export const MOBILE_WIDGET_REGISTRY: readonly MobileWidgetEntry[] = [
  { key: "weather", component: WeatherWidget },
  { key: "stock", component: StockWidget },
  { key: "walkie_talkie", component: WalkieTalkieWidget },
];

export function getMobileWidget(key: string): Component | null {
  return MOBILE_WIDGET_REGISTRY.find((entry) => entry.key === key)?.component ?? null;
}

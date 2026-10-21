import type { Component } from "vue";
import { CloudSun, Radio, TrendingUp } from "@lucide/vue";

/**
 * Widget Registry: the single list driving the Manage-sheet dropdown
 * today. To add a future widget (stocks, gallery, ...), append one
 * entry here plus its config UI and mobile renderer — the Manage
 * shell, list item, and save flow stay untouched.
 */
export interface WidgetDefinition {
  key: string;
  label: string;
  description: string;
  icon: Component;
}

export const WIDGET_REGISTRY: readonly WidgetDefinition[] = [
  {
    key: "weather",
    label: "Weather",
    description: "Current conditions for a chosen city",
    icon: CloudSun,
  },
  {
    key: "stock",
    label: "Stocks & Markets",
    description: "Live quotes, day change & market metrics",
    icon: TrendingUp,
  },
  {
    key: "walkie_talkie",
    label: "Walkie-talkie",
    description: "Push-to-talk voice groups for connected devices",
    icon: Radio,
  },
];

export function getWidgetDefinition(key: string): WidgetDefinition | undefined {
  return WIDGET_REGISTRY.find((entry) => entry.key === key);
}

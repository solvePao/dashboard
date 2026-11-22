import type { Component } from "vue";
import DeviceAuthorizedMinimalView from "../components/DeviceAuthorizedMinimalView.vue";
import type { PagerItem } from "../components/VerticalSlidePager.vue";
import WeatherWidget from "../components/widgets/WeatherWidget.vue";
import WalkieTalkieWidget from "../components/widgets/WalkieTalkieWidget.vue";
import StockWidget from "../components/widgets/StockWidget.vue";
import type { UserProfile } from "../types/auth";
import type { WidgetKey, WidgetSide } from "../types/layout";
import { asStockConfig, asWeatherConfig, type DeviceWidget } from "../types/widget";

export interface DashboardWidget extends PagerItem {
  key: WidgetKey;
  label: string;
  component: Component;
  dotVariant?: "light" | "amber";
  props: Record<string, unknown>;
}

export type DashboardWidgetPanels = Record<WidgetSide, DashboardWidget[]>;

interface MobileWidgetEntry extends Omit<DashboardWidget, "props"> {
  resolveProps: (widget: DeviceWidget | undefined, user: UserProfile) => DashboardWidget["props"] | null;
}

const MOBILE_WIDGET_REGISTRY: readonly MobileWidgetEntry[] = [
  {
    key: "weather",
    label: "Weather",
    component: WeatherWidget,
    dotVariant: "light",
    resolveProps: (widget) => {
      const config = asWeatherConfig(widget);
      return config ? { config } : null;
    },
  },
  {
    key: "clock",
    label: "Clock",
    component: DeviceAuthorizedMinimalView,
    resolveProps: (_, user) => ({ user }),
  },
  {
    key: "walkie_talkie",
    label: "Caller",
    component: WalkieTalkieWidget,
    reserveNavigationSpace: true,
    resolveProps: (widget) => widget ? { config: widget.config } : null,
  },
  {
    key: "stock",
    label: "Stocks",
    component: StockWidget,
    dotVariant: "amber",
    resolveProps: (widget) => {
      const config = asStockConfig(widget);
      return config ? { config } : null;
    },
  },
];

export function resolveDashboardWidgets(widgets: readonly DeviceWidget[], user: UserProfile | null): Map<WidgetKey, DashboardWidget> {
  const available = new Map<WidgetKey, DashboardWidget>();
  if (!user) return available;

  for (const { resolveProps, ...entry } of MOBILE_WIDGET_REGISTRY) {
    const widget = widgets.find((item) => item.widget_key === entry.key && item.enabled);
    const props = resolveProps(widget, user);
    if (props) available.set(entry.key, { ...entry, props });
  }
  return available;
}

export type SplitRatio = "50-50" | "30-70" | "70-30";

export type WidgetSide = "left" | "right";
export type WidgetKey = "weather" | "clock" | "walkie_talkie" | "stock";
export type WidgetLayout = Record<WidgetSide, WidgetKey[]>;

export const WIDGET_SIDES: readonly WidgetSide[] = ["left", "right"];


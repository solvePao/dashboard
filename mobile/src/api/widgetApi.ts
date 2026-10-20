import { apiClient } from "./client";
import type { DeviceWidget, WidgetsResponse } from "../types/widget";

/** Reads this device's own widget set with the device token as bearer. */
export async function fetchOwnWidgetsApi(token: string): Promise<DeviceWidget[]> {
  const data = await apiClient<WidgetsResponse>("/api/device-widgets", {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  if (!data || !Array.isArray(data.widgets)) return [];
  return data.widgets;
}

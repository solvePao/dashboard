import { apiClient, authHeader } from "./client";
import type { DeviceWidget, WidgetsResponse } from "../types/widget";

function widgetsPath(deviceToken: string): string {
  return `/api/devices/${encodeURIComponent(deviceToken)}/widgets`;
}

/**
 * Repository hiding the widget REST endpoints. PUT always sends the
 * complete desired set (empty clears all); the backend replaces state.
 */
export async function fetchWidgetsApi(
  token: string,
  deviceToken: string
): Promise<DeviceWidget[]> {
  const data = await apiClient<WidgetsResponse>(widgetsPath(deviceToken), {
    headers: authHeader(token),
  });
  return Array.isArray(data.widgets) ? data.widgets : [];
}

export async function saveWidgetsApi(
  token: string,
  deviceToken: string,
  widgets: DeviceWidget[]
): Promise<WidgetsResponse & { cleanup_pending?: boolean }> {
  const data = await apiClient<WidgetsResponse>(widgetsPath(deviceToken), {
    method: "PUT",
    headers: authHeader(token),
    body: JSON.stringify({ widgets }),
  });
  return data;
}

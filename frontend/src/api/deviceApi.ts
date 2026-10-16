import { apiClient, authHeader, ApiError } from "./client";
import type { DeviceItem, DevicesResponse, AuthorizeDeviceResponse } from "../types/device";

export async function fetchDevicesApi(token: string): Promise<DeviceItem[]> {
  const data = await apiClient<DevicesResponse>("/api/devices", {
    headers: authHeader(token),
  });
  return Array.isArray(data.devices) ? data.devices : [];
}

export async function authorizeDeviceApi(
  token: string,
  code: string,
  deviceName: string = "Mobile Device"
): Promise<AuthorizeDeviceResponse> {
  return apiClient<AuthorizeDeviceResponse>("/api/auth/device/authorize", {
    method: "POST",
    headers: authHeader(token),
    body: JSON.stringify({
      code: code.trim(),
      device_name: deviceName,
    }),
  });
}

export async function revokeDeviceApi(
  token: string,
  deviceToken: string
): Promise<boolean> {
  try {
    await apiClient(`/api/devices/${encodeURIComponent(deviceToken)}`, {
      method: "DELETE",
      headers: authHeader(token),
    });
    return true;
  } catch (err) {
    if (err instanceof ApiError) {
      // Already gone: the desired end-state holds, report success so the UI
      // does not show a failure for an already-revoked device.
      if (err.status === 404) return true;
      // Auth/permission problems must surface (re-login, forbidden view).
      // Server/network failures must surface too, never silent `false`.
      throw err;
    }
    throw err;
  }
}

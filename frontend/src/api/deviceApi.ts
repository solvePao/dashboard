import { apiClient } from "./client";
import type { DeviceItem, DevicesResponse, AuthorizeDeviceResponse } from "../types/device";

export async function fetchDevicesApi(credential: string): Promise<DeviceItem[]> {
  const data = await apiClient<DevicesResponse>("/api/devices", {
    headers: {
      Authorization: `Bearer ${credential}`,
    },
  });
  return data.devices || [];
}

export async function authorizeDeviceApi(
  credential: string,
  code: string,
  deviceName: string = "Mobile Device"
): Promise<AuthorizeDeviceResponse> {
  return apiClient<AuthorizeDeviceResponse>("/api/auth/device/authorize", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${credential}`,
    },
    body: JSON.stringify({
      code: code.trim(),
      device_name: deviceName,
    }),
  });
}

export async function revokeDeviceApi(
  credential: string,
  token: string
): Promise<boolean> {
  try {
    await apiClient(`/api/devices/${encodeURIComponent(token)}`, {
      method: "DELETE",
      headers: {
        Authorization: `Bearer ${credential}`,
      },
    });
    return true;
  } catch {
    return false;
  }
}


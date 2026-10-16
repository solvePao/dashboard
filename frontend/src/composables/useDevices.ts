import { ref } from "vue";
import { logger } from "../lib/logger";
import { ApiError } from "../api/client";
import { fetchDevicesApi, authorizeDeviceApi, revokeDeviceApi } from "../api/deviceApi";
import type { DeviceItem, AuthorizeDeviceResult } from "../types/device";

const devices = ref<DeviceItem[]>([]);
const devicesLoading = ref(false);
const devicesError = ref<string | null>(null);

export function useDevices() {
  async function fetchDevices(token: string | null): Promise<void> {
    if (!token) return;
    devicesLoading.value = true;
    devicesError.value = null;
    try {
      devices.value = await fetchDevicesApi(token);
    } catch (err) {
      logger.error("devices", "failed to fetch devices", err);
      devicesError.value = err instanceof Error ? err.message : "Failed to load devices.";
      if (err instanceof ApiError && err.status === 401) throw err;
    } finally {
      devicesLoading.value = false;
    }
  }

  async function authorizeDevice(
    token: string | null,
    codeOrSessionId: string,
    deviceName: string = "Mobile Device"
  ): Promise<AuthorizeDeviceResult> {
    if (!token) {
      return { success: false, message: "Please sign in to authorize devices." };
    }
    try {
      const res = await authorizeDeviceApi(token, codeOrSessionId, deviceName);
      if (res.error) {
        return { success: false, message: res.error };
      }
      await fetchDevices(token);
      return {
        success: true,
        message: res.message || "Device authorized successfully!",
      };
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) throw err;
      const msg = err instanceof Error ? err.message : "Network error";
      return { success: false, message: msg };
    }
  }

  async function revokeDevice(
    token: string | null,
    deviceToken: string
  ): Promise<boolean> {
    if (!token) return false;
    try {
      const success = await revokeDeviceApi(token, deviceToken);
      if (success) {
        devices.value = devices.value.filter((d) => d.token !== deviceToken);
        return true;
      }
      return false;
    } catch (err) {
      if (err instanceof ApiError && (err.status === 401 || err.status === 403)) throw err;
      if (err instanceof ApiError) {
        devicesError.value = err.message || "Failed to revoke device.";
      }
      logger.error("devices", "failed to revoke device", err);
      return false;
    }
  }

  function clearDevices(): void {
    devices.value = [];
    devicesError.value = null;
  }

  return {
    devices,
    devicesLoading,
    devicesError,
    fetchDevices,
    authorizeDevice,
    revokeDevice,
    clearDevices,
  };
}


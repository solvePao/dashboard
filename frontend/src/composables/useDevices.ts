import { ref } from "vue";
import { logger } from "../lib/logger";
import { fetchDevicesApi, authorizeDeviceApi, revokeDeviceApi } from "../api/deviceApi";
import type { DeviceItem, AuthorizeDeviceResult } from "../types/device";

const devices = ref<DeviceItem[]>([]);
const devicesLoading = ref(false);

export function useDevices() {
  async function fetchDevices(credential: string | null): Promise<void> {
    if (!credential) return;
    devicesLoading.value = true;
    try {
      devices.value = await fetchDevicesApi(credential);
    } catch (err) {
      logger.error("devices", "failed to fetch devices", err);
    } finally {
      devicesLoading.value = false;
    }
  }

  async function authorizeDevice(
    credential: string | null,
    codeOrSessionId: string,
    deviceName: string = "Mobile Device"
  ): Promise<AuthorizeDeviceResult> {
    if (!credential) {
      return { success: false, message: "Please sign in to authorize devices." };
    }
    try {
      const res = await authorizeDeviceApi(credential, codeOrSessionId, deviceName);
      if (res.error) {
        return { success: false, message: res.error };
      }
      await fetchDevices(credential);
      return {
        success: true,
        message: res.message || "Device authorized successfully!",
      };
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Network error";
      return { success: false, message: msg };
    }
  }

  async function revokeDevice(
    credential: string | null,
    token: string
  ): Promise<boolean> {
    if (!credential) return false;
    try {
      const success = await revokeDeviceApi(credential, token);
      if (success) {
        devices.value = devices.value.filter((d) => d.token !== token);
        return true;
      }
      return false;
    } catch (err) {
      logger.error("devices", "failed to revoke device", err);
      return false;
    }
  }

  function clearDevices(): void {
    devices.value = [];
  }

  return {
    devices,
    devicesLoading,
    fetchDevices,
    authorizeDevice,
    revokeDevice,
    clearDevices,
  };
}


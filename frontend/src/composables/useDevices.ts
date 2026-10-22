import { ref } from "vue";
import { logger } from "../lib/logger";
import { ApiError } from "../api/client";
import {
  fetchDevicesApi,
  authorizeDeviceApi,
  reauthorizeDeviceApi,
  revokeDeviceApi,
  devicesHubUrl,
} from "../api/deviceApi";
import { isStoredSessionExpired } from "../services/sessionStorage";
import { liveSocket } from "../services/liveSocket";
import type { DeviceItem, AuthorizeDeviceResult } from "../types/device";

const devices = ref<DeviceItem[]>([]);
const devicesLoading = ref(false);
const devicesError = ref<string | null>(null);

interface HubEvent {
  kind: string;
  token?: string;
  revision: number;
}

/// Observer hook for hub events (e.g. `widgets_updated`): Manage sheets
/// subscribe to refresh one device without owning the socket.
export type HubListener = (event: HubEvent) => void;

const hubListeners = new Set<HubListener>();

export function subscribeHubEvents(listener: HubListener): () => void {
  hubListeners.add(listener);
  return () => {
    hubListeners.delete(listener);
  };
}

function notifyHubListeners(event: HubEvent): void {
  for (const listener of hubListeners) {
    try {
      listener(event);
    } catch (err) {
      logger.warn("devices", "hub listener failed", err);
    }
  }
}

interface HubSnapshot {
  kind: "snapshot";
  revision: number;
  last_event?: HubEvent | null;
}

let hubUnsubscribe: (() => void) | undefined;
let lastHubRevision = 0;
let lastFetchToken: string | null = null;

export function useDevices() {
  function applyHubEvent(event: HubEvent): void {
    if (event.revision <= lastHubRevision) return;
    lastHubRevision = event.revision;
    if (
      (event.kind === "revoked" || event.kind === "disconnected") &&
      event.token
    ) {
      devices.value = devices.value.filter((d) => d.token !== event.token);
    } else if (event.kind === "authorized") {
      // A new row needs its full details; re-fetch the list once.
      void fetchDevices(lastFetchToken).catch(() => undefined);
    }
    // `widgets_updated` needs no list change, but subscribed sheets do.
    notifyHubListeners(event);
  }

  function handleHubSnapshot(snapshot: HubSnapshot): void {
    if (lastHubRevision === 0) {
      // First connect: the in-flight list load is the baseline.
      lastHubRevision = snapshot.revision;
      return;
    }
    if (snapshot.revision === lastHubRevision) return;
    const missed = snapshot.last_event;
    if (
      missed &&
      missed.revision === snapshot.revision &&
      missed.revision === lastHubRevision + 1
    ) {
      applyHubEvent(missed);
    } else {
      // More than one change was missed; one re-fetch resynchronizes.
      lastHubRevision = snapshot.revision;
      void fetchDevices(lastFetchToken).catch(() => undefined);
    }
  }

  function connectHub(): void {
    if (isStoredSessionExpired()) return;
    liveSocket.connect(devicesHubUrl());
    if (hubUnsubscribe) return;
    hubUnsubscribe = liveSocket.subscribe("hub", (envelope) => {
      const body = envelope as unknown as HubEvent | HubSnapshot;
      if (body.kind === "snapshot") {
        handleHubSnapshot(body as HubSnapshot);
      } else if (typeof body.kind === "string") {
        applyHubEvent(body as HubEvent);
      }
    });
  }

  function closeHub(): void {
    hubUnsubscribe?.();
    hubUnsubscribe = undefined;
    liveSocket.disconnect();
    lastHubRevision = 0;
    lastFetchToken = null;
  }

  async function fetchDevices(token: string | null): Promise<void> {
    if (!token) return;
    lastFetchToken = token;
    connectHub();
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

  async function reauthorizeDevice(
    token: string | null,
    deviceToken: string,
    codeOrSessionId: string
  ): Promise<AuthorizeDeviceResult> {
    if (!token) {
      return { success: false, message: "Please sign in to re-pair devices." };
    }
    try {
      const res = await reauthorizeDeviceApi(token, deviceToken, codeOrSessionId);
      if (res.error) {
        return { success: false, message: res.error };
      }
      await fetchDevices(token);
      return {
        success: true,
        message: res.message || "Device re-paired successfully!",
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
    closeHub();
    devices.value = [];
    devicesError.value = null;
  }

  return {
    devices,
    devicesLoading,
    devicesError,
    fetchDevices,
    authorizeDevice,
    reauthorizeDevice,
    revokeDevice,
    clearDevices,
    closeHub,
    subscribeHubEvents,
  };
}


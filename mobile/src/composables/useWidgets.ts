import { onScopeDispose, ref } from "vue";
import { logger } from "../lib/logger";
import { ApiError } from "../api/client";
import { fetchOwnWidgetsApi } from "../api/widgetApi";
import { getStoredDeviceToken } from "../services/deviceStorage";
import { subscribeDeviceSocket } from "./useDeviceAuth";
import type { DeviceWidget } from "../types/widget";

const widgets = ref<DeviceWidget[]>([]);
const widgetsLoading = ref(false);
const widgetsError = ref<string | null>(null);

/**
 * Reactive store for this device's widget set (the Strategy host
 * reads from here). Refreshes on explicit call, on the
 * `widgets_updated` socket push, and when the tab becomes visible
 * again (push fallback). A failed refresh keeps the last known set
 * and surfaces a retryable error — it never blanks the screen.
 */
export function useWidgets() {
  function resetWidgets(): void {
    widgets.value = [];
    widgetsLoading.value = false;
    widgetsError.value = null;
  }

  async function refreshWidgets(): Promise<void> {
    const token = getStoredDeviceToken();
    if (!token) {
      resetWidgets();
      return;
    }
    widgetsLoading.value = true;
    try {
      const next = await fetchOwnWidgetsApi(token);
      if (getStoredDeviceToken() !== token) return;
      widgets.value = next;
      widgetsError.value = null;
    } catch (err) {
      if (getStoredDeviceToken() !== token) return;
      if (err instanceof ApiError && (err.status === 401 || err.status === 403)) {
        // Auth state owns token invalidation; keep serving last state.
        logger.warn("widgets", "widget fetch rejected; keeping last state");
      } else {
        widgetsError.value = err instanceof Error ? err.message : "Could not load widgets.";
        logger.warn("widgets", "widget refresh failed; keeping last state");
      }
    } finally {
      if (getStoredDeviceToken() === token) widgetsLoading.value = false;
    }
  }

  function startWidgetSync(): () => void {
    const stopSocket = subscribeDeviceSocket((event) => {
      if (event.status === "widgets_updated") {
        void refreshWidgets();
      }
    });
    const onVisible = (): void => {
      if (document.visibilityState === "visible") {
        void refreshWidgets();
      }
    };
    document.addEventListener("visibilitychange", onVisible);
    const stop = (): void => {
      stopSocket();
      document.removeEventListener("visibilitychange", onVisible);
    };
    onScopeDispose(stop);
    return stop;
  }

  return {
    widgets,
    widgetsLoading,
    widgetsError,
    resetWidgets,
    refreshWidgets,
    startWidgetSync,
  };
}

import { computed, onBeforeUnmount, ref, toValue, watch, type MaybeRefOrGetter } from "vue";
import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import { useWidgets } from "./useWidgets";
import type { WalkieDeviceCode } from "../types/walkieTalkie";

export function useWalkieDeviceCode(isOpen?: MaybeRefOrGetter<boolean>) {
  const api = new WalkieTalkieApi();
  const { refreshWidgets } = useWidgets();
  const deviceCode = ref<WalkieDeviceCode | null>(null);
  const busy = ref(false);
  const error = ref("");
  const message = ref("");
  const now = ref(Date.now());
  const controller = new AbortController();
  let timer: ReturnType<typeof setInterval> | undefined;
  let polling = false;
  let revision = 0;

  const activeCode = computed(() => (deviceCode.value && deviceCode.value.expires_at > now.value ? deviceCode.value : null));
  const expiry = computed(() => (activeCode.value ? new Date(activeCode.value.expires_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : ""));

  function stopPolling(): void {
    if (timer) {
      clearInterval(timer);
      timer = undefined;
    }
  }

  function startPollingIfNeeded(): void {
    if (timer || !activeCode.value || (isOpen !== undefined && !toValue(isOpen))) return;
    timer = setInterval(() => {
      now.value = Date.now();
      if (!activeCode.value || (isOpen !== undefined && !toValue(isOpen))) {
        stopPolling();
        return;
      }
      if (busy.value || polling || document.visibilityState !== "visible") return;
      polling = true;
      void update("GET")
        .catch(() => undefined)
        .finally(() => {
          polling = false;
        });
    }, 5000);
  }

  async function update(method: "GET" | "POST" | "DELETE"): Promise<void> {
    const current = ++revision;
    const result = await api.deviceCode(method, controller.signal);
    if (controller.signal.aborted || current !== revision) return;
    const previouslyActive = deviceCode.value !== null;
    deviceCode.value = result.device_code;
    message.value =
      result.status === "used"
        ? "Device added. Generate a new code to join another group."
        : result.status === "expired"
        ? "Code expired. Generate a new code."
        : "";
    if (result.status === "used" && previouslyActive) {
      await refreshWidgets();
    }
    if (activeCode.value && (isOpen === undefined || toValue(isOpen))) {
      startPollingIfNeeded();
    } else {
      stopPolling();
    }
  }

  async function act(method: "POST" | "DELETE"): Promise<void> {
    if (busy.value) return;
    busy.value = true;
    error.value = "";
    try {
      await update(method);
    } catch (cause) {
      if (!controller.signal.aborted) {
        error.value = cause instanceof Error ? cause.message : "Could not update device code.";
      }
    } finally {
      busy.value = false;
    }
  }

  async function copy(): Promise<void> {
    if (!activeCode.value) return;
    try {
      await navigator.clipboard.writeText(activeCode.value.code);
      message.value = "Device code copied.";
    } catch {
      error.value = "Could not copy. Share the displayed code manually.";
    }
  }

  if (isOpen !== undefined) {
    watch(
      () => toValue(isOpen),
      (open) => {
        if (open) {
          now.value = Date.now();
          void update("GET").catch((cause) => {
            error.value = cause instanceof Error ? cause.message : "Could not load device code.";
          });
        } else {
          stopPolling();
        }
      },
      { immediate: true }
    );
  } else {
    void update("GET").catch((cause) => {
      error.value = cause instanceof Error ? cause.message : "Could not load device code.";
    });
  }

  onBeforeUnmount(() => {
    controller.abort();
    stopPolling();
  });

  return { activeCode, expiry, busy, error, message, generate: () => act("POST"), cancel: () => act("DELETE"), copy };
}

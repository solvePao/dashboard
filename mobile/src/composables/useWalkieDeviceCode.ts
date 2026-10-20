import { computed, onMounted, onBeforeUnmount, ref } from "vue";
import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import { useWidgets } from "./useWidgets";
import type { WalkieDeviceCode } from "../types/walkieTalkie";

export function useWalkieDeviceCode() {
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
  const activeCode = computed(() => deviceCode.value && deviceCode.value.expires_at > now.value ? deviceCode.value : null);
  const expiry = computed(() => activeCode.value ? new Date(activeCode.value.expires_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "");

  async function update(method: "GET" | "POST" | "DELETE"): Promise<void> {
    const current = ++revision;
    const result = await api.deviceCode(method, controller.signal);
    if (controller.signal.aborted || current !== revision) return;
    const previouslyActive = deviceCode.value !== null;
    deviceCode.value = result.device_code;
    message.value = result.status === "used" ? "Device added. Generate a new code to join another group." : result.status === "expired" ? "Code expired. Generate a new code." : "";
    if (result.status === "used" && previouslyActive) await refreshWidgets();
  }

  async function act(method: "POST" | "DELETE"): Promise<void> {
    if (busy.value) return;
    busy.value = true; error.value = "";
    try { await update(method); }
    catch (cause) { if (!controller.signal.aborted) error.value = cause instanceof Error ? cause.message : "Could not update device code."; }
    finally { busy.value = false; }
  }

  async function copy(): Promise<void> {
    if (!activeCode.value) return;
    try { await navigator.clipboard.writeText(activeCode.value.code); message.value = "Device code copied."; }
    catch { error.value = "Could not copy. Share the displayed code manually."; }
  }

  onMounted(() => {
    void update("GET").catch(cause => { error.value = cause instanceof Error ? cause.message : "Could not load device code."; });
    let ticks = 0;
    timer = setInterval(() => {
      now.value = Date.now();
      ticks++;
      if (ticks % 5 || busy.value || polling || document.visibilityState !== "visible") return;
      polling = true;
      void update("GET").catch(() => undefined).finally(() => { polling = false; });
    }, 1000);
  });
  onBeforeUnmount(() => { controller.abort(); clearInterval(timer); });
  return { activeCode, expiry, busy, error, message, generate: () => act("POST"), cancel: () => act("DELETE"), copy };
}

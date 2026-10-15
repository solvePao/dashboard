import { ref, onMounted, onUnmounted } from "vue";
import { fetchHealthApi } from "../api/healthApi";

const DEFAULT_POLL_MS = 30_000;

export function useHealth(pollMs: number = DEFAULT_POLL_MS) {
  const status = ref<"online" | "offline">("offline");
  const version = ref<string>("unknown");

  async function checkHealth(): Promise<void> {
    try {
      const data = await fetchHealthApi();
      if (data.status === "ok") {
        status.value = "online";
        version.value = data.release || "unknown";
        return;
      }
      status.value = "offline";
    } catch {
      status.value = "offline";
    }
  }

  let intervalId: ReturnType<typeof setInterval> | undefined;

  function handleVisibility(): void {
    if (typeof document !== "undefined" && !document.hidden) {
      void checkHealth();
    }
  }

  onMounted(() => {
    void checkHealth();
    intervalId = setInterval(() => {
      if (typeof document !== "undefined" && !document.hidden) {
        void checkHealth();
      }
    }, pollMs);
    if (typeof document !== "undefined") {
      document.addEventListener("visibilitychange", handleVisibility);
    }
  });

  onUnmounted(() => {
    if (intervalId) clearInterval(intervalId);
    if (typeof document !== "undefined") {
      document.removeEventListener("visibilitychange", handleVisibility);
    }
  });

  return {
    status,
    version,
    checkHealth,
  };
}


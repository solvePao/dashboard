import { ref, onMounted, onUnmounted } from "vue";
import { fetchHealthApi } from "../api/healthApi";

const DEFAULT_POLL_MS = 2000;

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

  onMounted(() => {
    void checkHealth();
    intervalId = setInterval(checkHealth, pollMs);
  });

  onUnmounted(() => {
    if (intervalId) clearInterval(intervalId);
  });

  return {
    status,
    version,
    checkHealth,
  };
}


import { ref, onMounted, onUnmounted } from "vue";
import { logger } from "../lib/logger";

export function useOrientationLock() {
  const isPortrait = ref(false);
  const isFullscreen = ref(false);
  const lockSupported = ref(false);
  const isLocked = ref(false);

  function checkOrientation() {
    if (typeof window === "undefined") return;
    const portraitMedia = window.matchMedia("(orientation: portrait)");
    isPortrait.value = portraitMedia.matches;
    isFullscreen.value = Boolean(document.fullscreenElement);
  }

  async function attemptLockLandscape() {
    if (typeof window === "undefined" || !("screen" in window)) return;
    const orientation = screen.orientation as any;
    if (orientation && typeof orientation.lock === "function") {
      lockSupported.value = true;
      try {
        await orientation.lock("landscape");
        isLocked.value = true;
        logger.debug("orientation", "screen.orientation.lock('landscape') succeeded");
      } catch (err) {
        isLocked.value = false;
        logger.debug("orientation", "screen.orientation.lock('landscape') not permitted or unsupported without fullscreen/PWA", err);
      }
    }
  }

  async function toggleFullscreen() {
    if (typeof document === "undefined") return;
    try {
      if (!document.fullscreenElement) {
        await document.documentElement.requestFullscreen();
        isFullscreen.value = true;
        await attemptLockLandscape();
      } else {
        await document.exitFullscreen();
        isFullscreen.value = false;
      }
    } catch (err) {
      logger.warn("orientation", "Fullscreen toggle failed", err);
    }
  }

  let portraitQuery: MediaQueryList | null = null;
  const onOrientationChange = () => {
    checkOrientation();
  };

  onMounted(() => {
    checkOrientation();
    attemptLockLandscape();

    portraitQuery = window.matchMedia("(orientation: portrait)");
    portraitQuery.addEventListener("change", onOrientationChange);
    window.addEventListener("resize", onOrientationChange);
    window.addEventListener("orientationchange", onOrientationChange);
    document.addEventListener("fullscreenchange", checkOrientation);
  });

  onUnmounted(() => {
    if (portraitQuery) {
      portraitQuery.removeEventListener("change", onOrientationChange);
    }
    window.removeEventListener("resize", onOrientationChange);
    window.removeEventListener("orientationchange", onOrientationChange);
    document.removeEventListener("fullscreenchange", checkOrientation);
  });

  return {
    isPortrait,
    isFullscreen,
    lockSupported,
    isLocked,
    toggleFullscreen,
    attemptLockLandscape,
  };
}


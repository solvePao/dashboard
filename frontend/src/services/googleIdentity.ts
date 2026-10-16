import { logger } from "../lib/logger";
import type { GoogleCredentialResponse } from "../types/gsi";

const GIS_SCRIPT_URL = "https://accounts.google.com/gsi/client";
const PLACEHOLDER_CLIENT_ID = "your-client-id.apps.googleusercontent.com";
const rawGoogleClientId = import.meta.env.VITE_GOOGLE_CLIENT_ID || "";

export const googleClientId =
  rawGoogleClientId === PLACEHOLDER_CLIENT_ID ? "" : rawGoogleClientId;

let gisLoadPromise: Promise<void> | null = null;
let initializedClientId: string | null = null;

export function loadGisScript(): Promise<void> {
  if (typeof window === "undefined") {
    return Promise.resolve();
  }
  if (window.google?.accounts?.id) {
    return Promise.resolve();
  }
  if (!gisLoadPromise) {
    gisLoadPromise = new Promise((resolve, reject) => {
      const script = document.createElement("script");
      script.src = GIS_SCRIPT_URL;
      script.async = true;
      script.defer = true;
      script.onload = () => resolve();
      script.onerror = () => {
        gisLoadPromise = null;
        reject(new Error("Failed to load Google Identity Services."));
      };
      document.head.appendChild(script);
    });
  }
  return gisLoadPromise;
}

export interface RenderGoogleButtonOptions {
  dark?: boolean;
}

export async function renderGoogleButton(
  element: HTMLElement,
  callback: (response: GoogleCredentialResponse) => void,
  options?: RenderGoogleButtonOptions
): Promise<void> {
  if (!googleClientId) {
    logger.error("auth", "Google login not configured (missing VITE_GOOGLE_CLIENT_ID)");
    throw new Error("Google login is not configured (missing VITE_GOOGLE_CLIENT_ID).");
  }

  const desiredTheme = options?.dark ? "filled_black" : "outline";
  if (element.dataset.renderedTheme === desiredTheme && element.children.length > 0) {
    logger.debug("auth", "Google button already rendered with desired theme", { theme: desiredTheme });
    return;
  }

  await loadGisScript();
  const id = window.google?.accounts?.id;
  if (!id) {
    logger.error("auth", "Google Identity Services failed to initialize");
    throw new Error("Google Identity Services failed to initialize.");
  }

  if (initializedClientId !== googleClientId) {
    id.initialize({
      client_id: googleClientId,
      callback,
      use_fedcm_for_button: true,
    });
    initializedClientId = googleClientId;
  }

  element.innerHTML = "";
  element.style.colorScheme = "light";
  element.dataset.renderedTheme = desiredTheme;
  id.renderButton(element, {
    theme: desiredTheme,
    size: "large",
    text: "signin_with",
    shape: "pill",
  });
  logger.debug("auth", "rendered Google sign-in button", { dark: options?.dark ?? false });
}

export function disableGoogleAutoSelect(): void {
  try {
    window.google?.accounts?.id?.disableAutoSelect();
  } catch {
    logger.debug("auth", "GIS disableAutoSelect skipped (not loaded)");
  }
}


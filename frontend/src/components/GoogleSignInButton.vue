<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useAuth } from "../composables/useAuth";
import { useTheme } from "../composables/useTheme";
import { logger } from "../lib/logger";

const { initGoogleButton, googleClientId } = useAuth();
const { isDark } = useTheme();
const buttonEl = ref<HTMLDivElement | null>(null);
const loadError = ref<string | null>(null);

async function renderButton(): Promise<void> {
  if (!buttonEl.value) {
    logger.debug("auth", "skipped rendering Google button (no element)");
    return;
  }
  try {
    await initGoogleButton(buttonEl.value, { dark: isDark.value });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Failed to load Google sign-in.";
    logger.error("auth", "failed to render Google sign-in button", message);
    loadError.value = message;
  }
}

onMounted(renderButton);
watch(isDark, renderButton);
</script>

<template>
  <div class="flex items-center" style="color-scheme: light;">
    <div ref="buttonEl" style="color-scheme: light;"></div>
    <p v-if="!googleClientId" class="py-1 text-xs text-muted-foreground">
      Google login is not configured.
    </p>
    <p v-else-if="loadError" class="py-1 text-xs text-destructive">
      {{ loadError }}
    </p>
  </div>
</template>

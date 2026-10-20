<script setup lang="ts">
import { onBeforeUnmount, watch } from "vue";
import { Loader2, LogOut, Maximize, Minimize, Moon, Settings, ShieldCheck, X } from "@lucide/vue";
import { useDeviceAuth } from "../composables/useDeviceAuth";

const props = defineProps<{
  open: boolean;
  isFullscreen: boolean;
  fullscreenSupported: boolean;
  fullscreenPending: boolean;
  fullscreenError: string | null;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "toggle-fullscreen"): void;
}>();

const {
  user,
  isAuthenticated,
  isDisconnecting,
  isConfirmingDisconnect,
  disconnectError,
  requestDisconnect,
  cancelDisconnect,
  disconnect,
} = useDeviceAuth();

async function handleDisconnect(): Promise<void> {
  await disconnect();
  if (!isAuthenticated.value) emit("close");
}

function handleKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape" && props.open) {
    if (isConfirmingDisconnect.value) {
      cancelDisconnect();
    } else {
      emit("close");
    }
  }
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      window.addEventListener("keydown", handleKeydown);
    } else {
      window.removeEventListener("keydown", handleKeydown);
      cancelDisconnect();
    }
  },
  { immediate: true }
);

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <div
    v-if="open && isAuthenticated"
    class="mo-settings-overlay absolute inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-sm animate-in fade-in-0 duration-200"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-dialog-title"
    @touchmove.stop
    @wheel.stop
    @click.self="emit('close')"
  >
    <div
      class="mo-settings-modal flex min-h-0 max-h-full flex-col w-full max-w-sm rounded-2xl border border-border bg-card p-5 text-card-foreground shadow-2xl animate-in zoom-in-95 duration-200"
    >
      <!-- Header -->
      <div class="flex shrink-0 items-center justify-between border-b border-border/80 pb-3">
        <div class="flex items-center gap-2">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Settings class="h-4 w-4" />
          </span>
          <h2 id="settings-dialog-title" class="text-sm font-semibold text-foreground">
            Settings & Account
          </h2>
        </div>
        <button
          type="button"
          class="rounded-lg p-1.5 text-muted-foreground transition hover:bg-accent hover:text-foreground"
          aria-label="Close settings"
          @click="emit('close')"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <div class="mo-settings-content min-h-0 overflow-y-auto overscroll-contain space-y-4 pt-4">
        <div class="rounded-xl border border-border/70 bg-muted/20 p-3.5">
          <button
            type="button"
            class="inline-flex min-h-10 w-full items-center justify-center gap-2 rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium disabled:opacity-50"
            :disabled="fullscreenPending || (!fullscreenSupported && !isFullscreen)"
            @click="emit('toggle-fullscreen')"
          >
            <Loader2 v-if="fullscreenPending" class="h-4 w-4 animate-spin" />
            <component v-else :is="isFullscreen ? Minimize : Maximize" class="h-4 w-4" />
            <span>{{ fullscreenPending ? 'Changing fullscreen…' : isFullscreen ? 'Exit fullscreen' : 'Enter fullscreen' }}</span>
          </button>
          <p v-if="fullscreenError || !fullscreenSupported" class="mt-2 text-[11px] text-muted-foreground" role="status">
            {{ fullscreenError || 'Fullscreen is not available in this browser.' }}
          </p>
        </div>

        <!-- Theme Status -->
        <div class="flex flex-wrap gap-2 items-center justify-between rounded-xl border border-border/70 bg-muted/20 px-3.5 py-2.5">
          <div class="text-xs">
            <p class="font-medium text-foreground">Theme Appearance</p>
            <p class="text-muted-foreground text-[11px]">Black Theme</p>
          </div>
          <div class="inline-flex items-center gap-1.5 rounded-lg border border-border/80 bg-background/60 px-2.5 py-1 text-xs font-medium text-foreground shadow-2xs">
            <Moon class="h-3.5 w-3.5 text-primary" />
            <span>Black Mode</span>
          </div>
        </div>

        <!-- Disconnect Action -->
        <div class="rounded-xl border border-border/70 bg-muted/20 p-3.5">
          <div class="flex items-center justify-between text-xs">
            <div>
              <p class="font-medium text-foreground">Device Connection</p>
              <p class="text-muted-foreground text-[11px]">Active 6-Month Token</p>
            </div>
          </div>

          <div class="mt-3">
            <button
              v-if="!isConfirmingDisconnect"
              type="button"
              class="inline-flex w-full items-center justify-center gap-2 rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs font-medium text-destructive transition hover:bg-destructive/10"
              @click="requestDisconnect"
            >
              <LogOut class="h-3.5 w-3.5" />
              <span>Disconnect Device</span>
            </button>

            <div v-else class="space-y-2">
              <p v-if="disconnectError" class="text-[11px] font-medium text-destructive">
                {{ disconnectError }}
              </p>
              <div class="flex flex-wrap items-center gap-2">
                <button
                  type="button"
                  :disabled="isDisconnecting"
                  class="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg bg-destructive px-3 py-2 text-xs font-medium text-white transition hover:bg-destructive/90 disabled:opacity-50"
                  @click="handleDisconnect"
                >
                  <Loader2 v-if="isDisconnecting" class="h-3.5 w-3.5 animate-spin" />
                  <span>{{ isDisconnecting ? "Disconnecting..." : "Confirm Disconnect" }}</span>
                </button>
                <button
                  type="button"
                  :disabled="isDisconnecting"
                  class="inline-flex items-center justify-center rounded-lg border border-border px-3 py-2 text-xs font-medium text-foreground transition hover:bg-accent disabled:opacity-50"
                  @click="cancelDisconnect"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- User Profile (Below) -->
        <div v-if="user" class="border-t border-border/80 pt-3">
          <p class="text-[11px] font-medium uppercase tracking-wider text-muted-foreground mb-2">
            Authorized User
          </p>
          <div class="flex items-center gap-3 rounded-xl border border-border/70 bg-muted/20 p-3">
            <div class="relative shrink-0">
              <img
                v-if="user.picture_url"
                :src="user.picture_url"
                alt=""
                referrerpolicy="no-referrer"
                class="h-10 w-10 rounded-full border-2 border-primary/20 object-cover"
              />
              <div
                v-else
                class="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10 text-sm font-bold text-primary"
              >
                {{ (user.name || user.email || "?").charAt(0).toUpperCase() }}
              </div>
              <span
                class="absolute -bottom-0.5 -right-0.5 flex h-4 w-4 items-center justify-center rounded-full bg-emerald-500 text-white shadow-xs"
                title="Authorized"
              >
                <ShieldCheck class="h-2.5 w-2.5" />
              </span>
            </div>

            <div class="min-w-0 flex-1">
              <p class="truncate text-xs font-semibold text-foreground">
                {{ user.name || "Authorized User" }}
              </p>
              <p class="truncate text-[11px] text-muted-foreground">
                {{ user.email }}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mo-settings-overlay {
  padding: max(0.75rem, env(safe-area-inset-top)) max(0.75rem, env(safe-area-inset-right)) max(0.75rem, env(safe-area-inset-bottom)) max(0.75rem, env(safe-area-inset-left));
}
.mo-settings-content {
  touch-action: pan-y;
  scrollbar-gutter: stable;
}
</style>


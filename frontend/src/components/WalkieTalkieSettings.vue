<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import {
  MoreHorizontal,
  Plus,
  Radio,
  RefreshCw,
  Trash2,
  Edit2,
  UserPlus,
  Users,
  X,
} from "@lucide/vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "reka-ui";
import {
  walkieTalkieApi,
  type WalkieTalkieGroup,
  type WalkieTalkieDevicePreview,
  type WalkieTalkieMember,
} from "../api/walkieTalkieApi";

const props = defineProps<{ deviceToken: string; groupId: string | null; enabled: boolean; saving?: boolean }>();
const emit = defineEmits<{
  (event: "assigned", groupId: string | null): void;
  (event: "changed"): void;
  (event: "toggle"): void;
}>();

type Panel = "create" | "add" | "manage" | "rename" | "delete";

const groups = ref<WalkieTalkieGroup[]>([]);
const userId = ref<number | null>(null);
const deviceId = ref("");
const selectedId = ref("");
const panel = ref<Panel | null>(null);
const newName = ref("");
const renameName = ref("");
const codeDigits = ref<string[]>(["", "", "", "", "", ""]);
const digitInputs = ref<HTMLInputElement[]>([]);
const preview = ref<WalkieTalkieDevicePreview | null>(null);
const previewCode = ref("");
const confirmingMemberId = ref<string | null>(null);
const busy = ref(false);
const loading = ref(true);
const failedPhotos = ref<Record<string, boolean>>({});

const code = computed({
  get: () => codeDigits.value.join(""),
  set: (val: string) => {
    const cleaned = (val || "").replace(/\D/g, "").slice(0, 6);
    for (let i = 0; i < 6; i++) {
      codeDigits.value[i] = cleaned[i] || "";
    }
  },
});

function photoFailed(url: string | null): void {
  if (url) failedPhotos.value[url] = true;
}

const error = ref("");
const notice = ref("");
let rosterTimer: ReturnType<typeof setInterval> | undefined;
let refreshing = false;

const selected = computed(() => groups.value.find(group => group.id === selectedId.value));
const owner = computed(() => selected.value?.owner_id === userId.value);
const title = computed(() => {
  switch (panel.value) {
    case "create":
      return "New group";
    case "add":
      return "Add device";
    case "manage":
      return "Manage devices";
    case "rename":
      return "Rename group";
    case "delete":
      return "Delete group";
    default:
      return "Manage devices";
  }
});

async function refresh(): Promise<void> {
  const token = props.deviceToken;
  const result = await walkieTalkieApi.list(token);
  if (token !== props.deviceToken) return;
  groups.value = result.groups;
  userId.value = result.user_id;
  deviceId.value = result.device_id;
  if (selectedId.value && !groups.value.some(group => group.id === selectedId.value)) {
    closePanel();
  }
}

async function run(action: () => Promise<void>): Promise<void> {
  if (busy.value || props.saving) return;
  busy.value = true;
  error.value = "";
  notice.value = "";
  try {
    await action();
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : "Unable to update Caller groups.";
    await refresh().catch(() => undefined);
    emit("changed");
  } finally {
    busy.value = false;
    loading.value = false;
  }
}

async function mutate(action: () => Promise<{ cleanup_pending?: boolean }>, message: string): Promise<void> {
  const result = await action();
  await refresh();
  emit("changed");
  notice.value = result.cleanup_pending
    ? `${message} Existing audio connections are still being closed; the server is retrying.`
    : message;
}

function openCreatePanel(): void {
  selectedId.value = "";
  panel.value = "create";
  newName.value = "";
  error.value = "";
  notice.value = "";
}

function openPanel(group: WalkieTalkieGroup, next: Panel): void {
  selectedId.value = group.id;
  panel.value = next;
  renameName.value = group.name;
  codeDigits.value = ["", "", "", "", "", ""];
  preview.value = null;
  previewCode.value = "";
  confirmingMemberId.value = null;
  error.value = "";
  notice.value = "";

  if (next === "add") {
    void nextTick(() => {
      digitInputs.value[0]?.focus();
    });
  }
}

function closePanel(): void {
  panel.value = null;
  selectedId.value = "";
  preview.value = null;
  codeDigits.value = ["", "", "", "", "", ""];
  previewCode.value = "";
  confirmingMemberId.value = null;
  newName.value = "";
  renameName.value = "";
}

function createGroup(): void {
  const name = newName.value.trim();
  if (!name) return;
  void run(async () => {
    await mutate(() => walkieTalkieApi.create(name), "Group created. Open its settings to add devices.");
    newName.value = "";
    closePanel();
  });
}

function activate(groupId: string): void {
  void run(() =>
    mutate(
      () => walkieTalkieApi.activate(groupId, props.deviceToken),
      "Active group updated for this device."
    )
  );
}

function previewDevice(): void {
  const group = selected.value;
  if (!group || !/^\d{6}$/.test(code.value)) return;
  const enteredCode = code.value;
  void run(async () => {
    const result = await walkieTalkieApi.previewDevice(group.id, enteredCode);
    if (selectedId.value !== group.id || panel.value !== "add" || code.value !== enteredCode) return;
    preview.value = result;
    previewCode.value = enteredCode;
  });
}

function addDevice(): void {
  const group = selected.value;
  const target = preview.value;
  if (!group || !target || target.group_id !== group.id || previewCode.value !== code.value) return;
  void run(async () => {
    await mutate(
      () => walkieTalkieApi.addDevice(group.id, previewCode.value, target.device.device_id),
      "Device added to the group."
    );
    preview.value = null;
    codeDigits.value = ["", "", "", "", "", ""];
    previewCode.value = "";
    panel.value = "manage";
  });
}

function renameGroup(): void {
  const group = selected.value;
  const name = renameName.value.trim();
  if (!group || !name) return;
  void run(async () => {
    await mutate(() => walkieTalkieApi.rename(group.id, name), "Group renamed.");
    closePanel();
  });
}

function removeGroup(): void {
  const group = selected.value;
  if (!group) return;
  void run(async () => {
    await mutate(() => walkieTalkieApi.delete(group.id), "Group deleted and all devices removed from it.");
    closePanel();
  });
}

function requestRemoveMember(member: WalkieTalkieMember): void {
  confirmingMemberId.value = member.device_id;
}

function cancelRemoveMember(): void {
  confirmingMemberId.value = null;
}

function confirmRemoveMember(member: WalkieTalkieMember): void {
  const group = selected.value;
  if (!group) return;
  const leaving = member.device_id === deviceId.value;
  confirmingMemberId.value = null;
  void run(() =>
    mutate(
      () => walkieTalkieApi.removeMember(group.id, member.device_id, props.deviceToken),
      leaving ? "This device left the group." : "Device removed from the group."
    )
  );
}

function handleDigitInput(index: number, event: Event): void {
  const target = event.target as HTMLInputElement;
  const val = target.value.replace(/\D/g, "");
  if (val.length > 0) {
    codeDigits.value[index] = val.slice(-1);
    if (index < 5) {
      digitInputs.value[index + 1]?.focus();
    }
  } else {
    codeDigits.value[index] = "";
  }
}

function handleDigitKeydown(index: number, event: KeyboardEvent): void {
  if (event.key === "Backspace") {
    if (!codeDigits.value[index] && index > 0) {
      codeDigits.value[index - 1] = "";
      digitInputs.value[index - 1]?.focus();
    } else {
      codeDigits.value[index] = "";
    }
  } else if (event.key === "ArrowLeft" && index > 0) {
    digitInputs.value[index - 1]?.focus();
  } else if (event.key === "ArrowRight" && index < 5) {
    digitInputs.value[index + 1]?.focus();
  }
}

function handleDigitPaste(event: ClipboardEvent): void {
  event.preventDefault();
  const text = event.clipboardData?.getData("text") || "";
  const cleaned = text.replace(/\D/g, "").slice(0, 6);
  if (!cleaned) return;
  for (let i = 0; i < 6; i++) {
    codeDigits.value[i] = cleaned[i] || "";
  }
  const nextFocusIndex = Math.min(cleaned.length, 5);
  digitInputs.value[nextFocusIndex]?.focus();
  if (cleaned.length === 6) {
    previewDevice();
  }
}

function memberStatus(member: WalkieTalkieMember): string {
  if (member.audio_active) return "Audio active";
  return member.online ? "Online · audio paused" : "Offline";
}

watch(code, () => {
  preview.value = null;
  previewCode.value = "";
});

watch(
  () => props.deviceToken,
  () => {
    groups.value = [];
    closePanel();
    void refresh().catch(cause => {
      error.value = cause instanceof Error ? cause.message : "Unable to load groups.";
    });
  }
);

onMounted(() => {
  void run(refresh);
  rosterTimer = setInterval(() => {
    if (busy.value || props.saving || refreshing || document.visibilityState !== "visible") return;
    refreshing = true;
    void refresh()
      .catch(() => undefined)
      .finally(() => {
        refreshing = false;
      });
  }, 10_000);
});

onUnmounted(() => {
  clearInterval(rosterTimer);
});
</script>

<template>
  <section class="relative space-y-4 rounded-xl border border-border bg-card p-3.5 sm:p-5 overflow-hidden" aria-label="Caller groups">
    <!-- Header -->
    <div class="border-b border-border/70 pb-4">
      <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
        <div>
          <div class="flex items-center justify-between sm:justify-start gap-2">
            <div class="flex items-center gap-2">
              <div class="flex h-9 w-9 items-center justify-center rounded-full bg-primary/10 text-primary">
                <Radio class="h-4 w-4" />
              </div>
              <h4 class="text-sm font-semibold tracking-tight text-foreground">Caller</h4>
            </div>

            <!-- Mobile-only switch placed in top row -->
            <div class="flex items-center gap-2 sm:hidden">
              <button
                type="button"
                role="switch"
                :aria-checked="enabled"
                aria-label="Turn Caller on or off for this mobile device"
                :disabled="busy || saving"
                class="relative h-6 w-11 shrink-0 rounded-full transition-colors disabled:opacity-60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
                :class="enabled ? 'bg-primary' : 'bg-muted'"
                @click="emit('toggle')"
              >
                <span
                  class="absolute top-0.5 h-5 w-5 rounded-full shadow transition-all"
                  :class="enabled ? 'left-5.5 bg-primary-foreground' : 'left-0.5 bg-foreground'"
                />
              </button>
              <span class="text-xs font-medium" :class="enabled ? 'text-primary' : 'text-muted-foreground'">
                {{ enabled ? "Enabled" : "Disabled" }}
              </span>
            </div>
          </div>

          <p class="mt-1 text-xs text-muted-foreground leading-relaxed">
            Create groups and manage connected devices (up to 5 per group).
          </p>
        </div>

        <!-- Desktop Controls (hidden on mobile, visible on sm:) -->
        <div class="hidden shrink-0 sm:flex items-center gap-2.5">
          <div class="flex items-center gap-2">
            <button
              type="button"
              role="switch"
              :aria-checked="enabled"
              aria-label="Turn Caller on or off for this mobile device"
              :disabled="busy || saving"
              class="relative h-6 w-11 shrink-0 rounded-full transition-colors disabled:opacity-60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
              :class="enabled ? 'bg-primary' : 'bg-muted'"
              @click="emit('toggle')"
            >
              <span
                class="absolute top-0.5 h-5 w-5 rounded-full shadow transition-all"
                :class="enabled ? 'left-5.5 bg-primary-foreground' : 'left-0.5 bg-foreground'"
              />
            </button>
            <span class="text-xs font-medium" :class="enabled ? 'text-primary' : 'text-muted-foreground'">
              {{ enabled ? "Enabled" : "Disabled" }}
            </span>
          </div>

          <button
            type="button"
            :disabled="busy || saving"
            class="inline-flex items-center gap-1.5 rounded-full border border-border bg-background px-3 py-1.5 text-xs font-medium text-foreground hover:bg-muted transition active:scale-95 disabled:opacity-50"
            title="Refresh groups"
            @click="run(refresh)"
          >
            <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading || busy }" />
            <span>Refresh</span>
          </button>

          <button
            type="button"
            :disabled="busy || saving"
            class="inline-flex items-center gap-1.5 rounded-full bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 transition active:scale-95 disabled:opacity-50"
            @click="openCreatePanel"
          >
            <Plus class="h-3.5 w-3.5" />
            <span>New group</span>
          </button>
        </div>

        <!-- Mobile-only action bar (visible on mobile, hidden on sm:) -->
        <div class="flex sm:hidden items-center gap-2 pt-1">
          <button
            type="button"
            :disabled="busy || saving"
            class="inline-flex flex-1 min-h-[44px] items-center justify-center gap-1.5 rounded-full border border-border bg-background px-3 py-2 text-xs font-medium text-foreground hover:bg-muted active:scale-[0.98] transition disabled:opacity-50"
            title="Refresh groups"
            @click="run(refresh)"
          >
            <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading || busy }" />
            <span>Refresh</span>
          </button>

          <button
            type="button"
            :disabled="busy || saving"
            class="inline-flex flex-1 min-h-[44px] items-center justify-center gap-1.5 rounded-full bg-primary px-3.5 py-2 text-xs font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 active:scale-[0.98] transition disabled:opacity-50"
            @click="openCreatePanel"
          >
            <Plus class="h-3.5 w-3.5" />
            <span>New group</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Disabled notice -->
    <div v-if="!enabled" class="rounded-lg bg-muted/60 p-3 text-xs text-muted-foreground">
      Caller is currently disabled on this mobile device. Its group memberships remain intact.
    </div>

    <!-- Alerts -->
    <p v-if="error" role="alert" class="rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">
      {{ error }}
    </p>
    <p v-if="notice" role="status" class="rounded-lg border border-border bg-muted/50 p-3 text-xs text-muted-foreground">
      {{ notice }}
    </p>

    <fieldset :disabled="busy || saving" class="min-w-0 space-y-4 disabled:opacity-60">
      <!-- Group List -->
      <div class="space-y-2">
        <div v-if="loading" role="status" class="py-8 text-center text-xs text-muted-foreground">
          <RefreshCw class="mx-auto h-5 w-5 animate-spin text-muted-foreground/50 mb-2" />
          Loading groups…
        </div>

        <div v-else-if="!groups.length" class="rounded-xl border border-dashed border-border p-8 text-center">
          <Radio class="mx-auto h-8 w-8 text-muted-foreground/40 mb-2" />
          <p class="text-xs font-medium text-foreground">No groups yet</p>
          <p class="mt-1 text-[11px] text-muted-foreground leading-relaxed max-w-sm mx-auto">
            Create your first group to add mobile devices and start communicating.
          </p>
          <button
            type="button"
            class="mt-3 inline-flex min-h-[44px] sm:min-h-0 items-center justify-center gap-1.5 rounded-full bg-primary px-4 py-2 sm:px-3 sm:py-1.5 text-xs font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 transition active:scale-95"
            @click="openCreatePanel"
          >
            <Plus class="h-3.5 w-3.5" />
            <span>Create group</span>
          </button>
        </div>

        <div v-else class="rounded-xl border border-border overflow-hidden bg-background">
          <ul class="divide-y divide-border">
            <li
              v-for="group in groups"
              :key="group.id"
              :class="[
                'p-3.5 transition-colors sm:p-3 sm:px-4',
                selectedId === group.id ? 'bg-primary/5' : 'hover:bg-muted/30',
              ]"
            >
              <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                <!-- Group info -->
                <div class="flex items-start sm:items-center gap-3 min-w-0">
                  <div
                    class="flex h-10 w-10 sm:h-9 sm:w-9 shrink-0 items-center justify-center rounded-full transition-colors"
                    :class="group.active ? 'bg-emerald-500/15 text-emerald-500' : 'bg-muted text-muted-foreground'"
                  >
                    <Radio class="h-4 w-4" />
                  </div>

                  <div class="min-w-0 flex-1">
                    <div class="flex flex-wrap items-center gap-1.5">
                      <h6 class="truncate text-sm font-semibold text-foreground max-w-[200px] sm:max-w-none">{{ group.name }}</h6>
                      <span
                        v-if="group.active"
                        class="inline-flex items-center rounded-full bg-emerald-500/10 px-2 py-0.5 text-[10px] font-semibold text-emerald-500"
                      >
                        Active
                      </span>
                      <span
                        v-if="group.owner_id === userId"
                        class="inline-flex items-center rounded-full bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground"
                      >
                        Owner
                      </span>
                    </div>
                    <p class="mt-0.5 text-xs text-muted-foreground flex flex-wrap items-center gap-1.5">
                      <span>{{ group.members.length }}/5 devices</span>
                      <span v-if="group.members.some(m => m.audio_active)" class="inline-flex items-center gap-1 text-emerald-500 font-medium">
                        <span class="inline-block h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse" />
                        Audio active
                      </span>
                    </p>
                  </div>
                </div>

                <!-- Action buttons: Full touch bar on mobile, inline on desktop -->
                <div class="flex items-center gap-2 pt-2 border-t border-border/50 sm:border-t-0 sm:pt-0 sm:shrink-0">
                  <button
                    v-if="!group.active && group.joined"
                    type="button"
                    :disabled="busy || saving"
                    class="flex-1 sm:flex-none min-h-[44px] sm:min-h-0 inline-flex items-center justify-center rounded-full border border-border bg-background px-3 py-2 sm:px-2.5 sm:py-1 text-xs font-medium text-foreground hover:bg-muted active:scale-[0.98] transition disabled:opacity-50"
                    @click="activate(group.id)"
                  >
                    Make active
                  </button>

                  <button
                    type="button"
                    :disabled="busy || saving"
                    class="flex-1 sm:flex-none min-h-[44px] sm:min-h-0 inline-flex items-center justify-center rounded-full border border-border bg-background px-3 py-2 sm:px-2.5 sm:py-1 text-xs font-medium text-foreground hover:bg-muted active:scale-[0.98] transition disabled:opacity-50"
                    @click="openPanel(group, 'manage')"
                  >
                    Manage
                  </button>

                  <!-- Dropdown for more options -->
                  <DropdownMenuRoot>
                    <DropdownMenuTrigger as-child>
                      <button
                        type="button"
                        :disabled="busy || saving"
                        :aria-label="`Options for ${group.name}`"
                        class="flex h-11 w-11 sm:h-7 sm:w-7 shrink-0 items-center justify-center rounded-full border border-border bg-background text-muted-foreground hover:bg-muted hover:text-foreground active:scale-95 transition disabled:opacity-50"
                      >
                        <MoreHorizontal class="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                      </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuPortal>
                      <DropdownMenuContent
                        align="end"
                        :side-offset="6"
                        :collision-padding="16"
                        class="z-50 min-w-48 rounded-xl border border-border bg-popover p-1.5 text-popover-foreground shadow-xl outline-none"
                      >
                        <DropdownMenuItem
                          class="flex min-h-[42px] sm:min-h-0 items-center gap-2.5 cursor-pointer rounded-lg px-3 py-2 text-xs outline-none hover:bg-accent focus:bg-accent transition-colors"
                          @select="openPanel(group, 'manage')"
                        >
                          <Users class="h-4 w-4 sm:h-3.5 sm:w-3.5 text-muted-foreground" />
                          <span class="font-medium">Manage devices</span>
                        </DropdownMenuItem>

                        <DropdownMenuItem
                          v-if="group.owner_id === userId"
                          :disabled="group.members.length >= 5"
                          class="flex min-h-[42px] sm:min-h-0 items-center gap-2.5 cursor-pointer rounded-lg px-3 py-2 text-xs outline-none hover:bg-accent focus:bg-accent transition-colors data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
                          @select="openPanel(group, 'add')"
                        >
                          <UserPlus class="h-4 w-4 sm:h-3.5 sm:w-3.5 text-muted-foreground" />
                          <span class="font-medium">Add device</span>
                        </DropdownMenuItem>

                        <template v-if="group.owner_id === userId">
                          <DropdownMenuItem
                            class="flex min-h-[42px] sm:min-h-0 items-center gap-2.5 cursor-pointer rounded-lg px-3 py-2 text-xs outline-none hover:bg-accent focus:bg-accent transition-colors"
                            @select="openPanel(group, 'rename')"
                          >
                            <Edit2 class="h-4 w-4 sm:h-3.5 sm:w-3.5 text-muted-foreground" />
                            <span class="font-medium">Rename group</span>
                          </DropdownMenuItem>

                          <DropdownMenuSeparator class="my-1 h-px bg-border/60" />

                          <DropdownMenuItem
                            class="flex min-h-[42px] sm:min-h-0 items-center gap-2.5 cursor-pointer rounded-lg px-3 py-2 text-xs text-destructive outline-none hover:bg-destructive/10 focus:bg-destructive/10 transition-colors"
                            @select="openPanel(group, 'delete')"
                          >
                            <Trash2 class="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                            <span class="font-medium">Delete group</span>
                          </DropdownMenuItem>
                        </template>
                      </DropdownMenuContent>
                    </DropdownMenuPortal>
                  </DropdownMenuRoot>
                </div>
              </div>
            </li>
          </ul>
        </div>
      </div>
    </fieldset>

    <!-- Slide-in Drawer Backdrop -->
    <transition name="drawer-fade">
      <div
        v-if="panel && (panel === 'create' || selected)"
        class="fixed inset-0 z-40 bg-black/60 backdrop-blur-xs sm:absolute sm:z-20 sm:bg-black/50 sm:backdrop-blur-[2px]"
        @click="closePanel"
      />
    </transition>

    <!-- Slide-in Drawer Detail Panel -->
    <transition name="drawer-slide">
      <div
        v-if="panel && (panel === 'create' || selected)"
        class="fixed inset-x-0 bottom-0 z-50 flex w-full max-h-[90dvh] flex-col rounded-t-2xl border-t border-border bg-card text-card-foreground shadow-2xl sm:fixed-none sm:absolute sm:inset-y-0 sm:right-0 sm:bottom-auto sm:z-30 sm:max-w-sm sm:rounded-none sm:border-l sm:border-t-0 sm:max-h-none"
      >
        <!-- Sticky Drawer Header -->
        <div class="sticky top-0 z-10 shrink-0 bg-card/95 backdrop-blur-sm border-b border-border/70 px-4 pt-3 pb-3 sm:px-5">
          <div class="mx-auto mb-2 h-1 w-10 shrink-0 rounded-full bg-muted-foreground/30 sm:hidden" aria-hidden="true" />
          <div class="flex items-center justify-between">
            <div class="min-w-0 pr-2">
              <h5 class="text-sm font-semibold truncate text-foreground">{{ title }}</h5>
              <p v-if="selected" class="text-xs text-muted-foreground truncate">{{ selected.name }} · {{ selected.members.length }}/5 devices</p>
              <p v-else-if="panel === 'create'" class="text-xs text-muted-foreground truncate">Create a new calling group</p>
            </div>
            <button
              type="button"
              class="flex h-9 w-9 sm:h-8 sm:w-8 items-center justify-center rounded-full text-muted-foreground hover:bg-muted hover:text-foreground active:scale-95 transition-colors"
              aria-label="Close panel"
              @click="closePanel"
            >
              <X class="h-4 w-4" />
            </button>
          </div>
        </div>

        <!-- Scrollable Drawer Body with momentum scrolling -->
        <div class="flex-1 min-h-0 overflow-y-auto overscroll-contain px-4 py-4 sm:px-5 pb-[max(1.5rem,env(safe-area-inset-bottom))]">
          <!-- PANEL 1: Create Group -->
          <template v-if="panel === 'create'">
            <form class="space-y-4" @submit.prevent="createGroup">
              <div>
                <p class="text-xs text-muted-foreground leading-relaxed">
                  Name your group to start live audio calls across paired devices. Each recipient chooses when to pick up.
                </p>
                <label class="mt-3 block text-xs font-semibold text-foreground">
                  Group name
                  <input
                    v-model="newName"
                    maxlength="60"
                    required
                    autofocus
                    placeholder="e.g. Operations, Field Team..."
                    class="mt-1.5 w-full min-h-[44px] rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/40 focus:border-primary transition"
                  />
                </label>
              </div>

              <div class="flex flex-col sm:flex-row gap-2 pt-2">
                <button
                  type="submit"
                  :disabled="!newName.trim() || busy || saving"
                  class="w-full sm:flex-1 min-h-[44px] inline-flex items-center justify-center rounded-full bg-primary px-4 py-2 text-xs font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 active:scale-[0.98] transition disabled:opacity-50"
                >
                  Create group
                </button>
                <button
                  type="button"
                  class="w-full sm:flex-none min-h-[44px] inline-flex items-center justify-center rounded-full border border-border bg-background px-4 py-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground active:scale-[0.98] transition"
                  @click="closePanel"
                >
                  Cancel
                </button>
              </div>
            </form>
          </template>

          <!-- PANEL 2: Add Device -->
          <template v-else-if="panel === 'add' && selected && owner">
            <div class="space-y-4">
              <form v-if="!preview" class="space-y-4" @submit.prevent="previewDevice">
                <p class="text-xs text-muted-foreground leading-relaxed">
                  Open Caller on the phone you wish to add, tap <strong class="text-foreground">Get device code</strong>, and enter the 6 digits below:
                </p>

                <!-- 6-Box Segmented Code Input -->
                <div>
                  <div class="flex items-center justify-center gap-1.5 sm:gap-2">
                    <input
                      v-for="(digit, index) in codeDigits"
                      :key="index"
                      :ref="el => { if (el) digitInputs[index] = el as HTMLInputElement; }"
                      :value="digit"
                      type="text"
                      inputmode="numeric"
                      pattern="[0-9]*"
                      maxlength="1"
                      autocomplete="one-time-code"
                      class="h-12 w-10 sm:h-12 sm:w-11 rounded-lg border border-input bg-background text-center text-xl font-bold font-mono text-foreground focus:outline-none focus:ring-2 focus:ring-primary/40 focus:border-primary transition shadow-xs"
                      @input="handleDigitInput(index, $event)"
                      @keydown="handleDigitKeydown(index, $event)"
                      @paste="handleDigitPaste($event)"
                      @focus="($event.target as HTMLInputElement).select()"
                    />
                  </div>
                  <p class="mt-2 text-center text-[11px] text-muted-foreground">
                    Auto-advances as you type · Paste supported
                  </p>
                </div>

                <button
                  type="submit"
                  :disabled="!/^\d{6}$/.test(code) || selected.members.length >= 5 || busy || saving"
                  class="w-full min-h-[44px] inline-flex items-center justify-center rounded-lg bg-primary py-2.5 text-xs font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 active:scale-[0.98] transition disabled:opacity-50"
                >
                  Preview device
                </button>
              </form>

              <!-- Device Preview Screen -->
              <div v-else class="space-y-4">
                <p class="text-xs text-muted-foreground">Verify the device before confirming admission:</p>
                <div class="flex items-center gap-3.5 rounded-xl border border-border bg-background p-3.5 shadow-xs">
                  <img
                    v-if="preview.device.picture_url && !failedPhotos[preview.device.picture_url]"
                    :src="preview.device.picture_url"
                    alt=""
                    referrerpolicy="no-referrer"
                    class="h-12 w-12 rounded-full object-cover ring-2 ring-border"
                    @error="photoFailed(preview.device.picture_url)"
                  />
                  <span
                    v-else
                    class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary text-base font-bold"
                  >
                    {{ (preview.device.first_name || preview.device.name).charAt(0) }}
                  </span>
                  <div class="min-w-0 flex-1">
                    <p class="truncate text-sm font-semibold text-foreground">{{ preview.device.first_name || preview.device.name }}</p>
                    <p class="truncate text-xs text-muted-foreground">{{ preview.device.device_name }}</p>
                  </div>
                </div>

                <div class="flex flex-col sm:flex-row gap-2">
                  <button
                    type="button"
                    :disabled="selected.members.length >= 5 || busy || saving"
                    class="flex-1 min-h-[44px] inline-flex items-center justify-center rounded-lg bg-primary py-2 text-xs font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 active:scale-[0.98] transition disabled:opacity-50"
                    @click="addDevice"
                  >
                    Confirm admission
                  </button>
                  <button
                    type="button"
                    class="min-h-[44px] inline-flex items-center justify-center rounded-full border border-border bg-background px-4 py-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground active:scale-[0.98] transition"
                    @click="preview = null"
                  >
                    Back
                  </button>
                </div>
              </div>

              <p v-if="selected.members.length >= 5" class="rounded-lg bg-destructive/10 p-3 text-xs text-destructive">
                This group has reached the 5-device maximum limit. Remove a device first.
              </p>
            </div>
          </template>

          <!-- PANEL 3: Manage Devices -->
          <template v-else-if="panel === 'manage' && selected">
            <div class="space-y-3.5">
              <div class="flex items-center justify-between">
                <span class="text-xs font-medium text-muted-foreground">{{ selected.members.length }}/5 slots filled</span>
                <button
                  v-if="owner && selected.members.length < 5"
                  type="button"
                  class="min-h-[36px] inline-flex items-center gap-1.5 text-xs font-semibold text-primary hover:underline"
                  @click="openPanel(selected, 'add')"
                >
                  <Plus class="h-3.5 w-3.5" />
                  <span>Add device</span>
                </button>
              </div>

              <ul class="divide-y divide-border rounded-xl border border-border bg-background overflow-hidden">
                <li
                  v-for="member in selected.members"
                  :key="member.device_id"
                  class="p-3 sm:px-3.5 transition-colors"
                >
                  <!-- Member row content -->
                  <div class="flex items-center gap-3">
                    <img
                      v-if="member.picture_url && !failedPhotos[member.picture_url]"
                      :src="member.picture_url"
                      alt=""
                      referrerpolicy="no-referrer"
                      class="h-10 w-10 shrink-0 rounded-full object-cover ring-1 ring-border"
                      @error="photoFailed(member.picture_url)"
                    />
                    <span
                      v-else
                      class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-bold text-foreground"
                    >
                      {{ (member.first_name || member.name).charAt(0) }}
                    </span>

                    <div class="min-w-0 flex-1">
                      <p class="truncate text-xs font-semibold text-foreground">
                        {{ member.first_name || member.name }}
                        <span v-if="member.device_id === deviceId" class="text-[11px] font-normal text-muted-foreground"> (this phone)</span>
                      </p>
                      <p class="truncate text-[11px] text-muted-foreground">
                        {{ member.device_name }} · {{ memberStatus(member) }}
                      </p>
                    </div>

                    <!-- Action Trigger (when not confirming) -->
                    <button
                      v-if="(owner || member.device_id === deviceId) && confirmingMemberId !== member.device_id"
                      type="button"
                      class="min-h-[38px] px-2.5 inline-flex items-center justify-center rounded-full text-xs font-medium text-destructive hover:bg-destructive/10 active:scale-95 transition"
                      @click="requestRemoveMember(member)"
                    >
                      {{ member.device_id === deviceId ? 'Leave' : 'Remove' }}
                    </button>
                  </div>

                  <!-- Inline Confirmation Prompt (when confirming) -->
                  <div
                    v-if="confirmingMemberId === member.device_id"
                    class="mt-2.5 flex items-center justify-between gap-2 rounded-lg bg-destructive/10 p-2 text-xs"
                  >
                    <span class="text-destructive font-medium">
                      {{ member.device_id === deviceId ? 'Leave this group?' : 'Remove device?' }}
                    </span>
                    <div class="flex items-center gap-1.5 shrink-0">
                      <button
                        type="button"
                        class="min-h-[32px] rounded-full bg-destructive px-2.5 py-1 text-xs font-semibold text-white hover:bg-destructive/90 active:scale-95 transition"
                        @click="confirmRemoveMember(member)"
                      >
                        Yes
                      </button>
                      <button
                        type="button"
                        class="min-h-[32px] rounded-full border border-border bg-background px-2.5 py-1 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground active:scale-95 transition"
                        @click="cancelRemoveMember"
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                </li>

                <li v-if="!selected.members.length" class="p-6 text-center text-xs text-muted-foreground">
                  No devices joined yet.
                </li>
              </ul>
            </div>
          </template>

          <!-- PANEL 4: Rename Group -->
          <template v-else-if="panel === 'rename' && selected && owner">
            <form class="space-y-4" @submit.prevent="renameGroup">
              <label class="block text-xs font-semibold text-foreground">
                Group name
                <input
                  v-model="renameName"
                  required
                  maxlength="60"
                  class="mt-1.5 w-full min-h-[44px] rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/40 focus:border-primary transition"
                />
              </label>

              <div class="flex flex-col sm:flex-row gap-2 pt-1">
                <button
                  type="submit"
                  :disabled="!renameName.trim() || renameName.trim() === selected.name || busy || saving"
                  class="w-full sm:flex-1 min-h-[44px] inline-flex items-center justify-center rounded-full bg-primary px-4 py-2 text-xs font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 active:scale-[0.98] transition disabled:opacity-50"
                >
                  Save name
                </button>
                <button
                  type="button"
                  class="w-full sm:flex-none min-h-[44px] inline-flex items-center justify-center rounded-full border border-border bg-background px-4 py-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground active:scale-[0.98] transition"
                  @click="closePanel"
                >
                  Cancel
                </button>
              </div>
            </form>
          </template>

          <!-- PANEL 5: Delete Group -->
          <template v-else-if="panel === 'delete' && selected && owner">
            <div class="space-y-4">
              <p class="text-xs leading-relaxed text-muted-foreground">
                Are you sure you want to delete <strong class="text-foreground">“{{ selected.name }}”</strong>? This removes all {{ selected.members.length }} member devices and closes any ongoing audio sessions.
              </p>
              <div class="flex flex-col sm:flex-row gap-2 pt-1">
                <button
                  type="button"
                  :disabled="busy || saving"
                  class="w-full sm:flex-1 min-h-[44px] inline-flex items-center justify-center rounded-full bg-destructive py-2 text-xs font-semibold text-white hover:bg-destructive/90 active:scale-[0.98] transition shadow-sm disabled:opacity-50"
                  @click="removeGroup"
                >
                  Confirm delete
                </button>
                <button
                  type="button"
                  class="w-full sm:flex-none min-h-[44px] inline-flex items-center justify-center rounded-full border border-border bg-background px-4 py-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground active:scale-[0.98] transition"
                  @click="closePanel"
                >
                  Cancel
                </button>
              </div>
            </div>
          </template>
        </div>
      </div>
    </transition>
  </section>
</template>

<style scoped>
.drawer-fade-enter-active,
.drawer-fade-leave-active {
  transition: opacity 0.22s ease;
}
.drawer-fade-enter-from,
.drawer-fade-leave-to {
  opacity: 0;
}

.drawer-slide-enter-active {
  transition: transform 0.26s cubic-bezier(0.16, 1, 0.3, 1);
}
.drawer-slide-leave-active {
  transition: transform 0.2s ease-in;
}
@media (max-width: 639px) {
  .drawer-slide-enter-from,
  .drawer-slide-leave-to {
    transform: translateY(100%);
  }
}
@media (min-width: 640px) {
  .drawer-slide-enter-from,
  .drawer-slide-leave-to {
    transform: translateX(100%);
  }
}
</style>

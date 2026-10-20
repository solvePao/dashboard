<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
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
}>();

type Panel = "add" | "manage" | "rename" | "delete";

const groups = ref<WalkieTalkieGroup[]>([]);
const userId = ref<number | null>(null);
const deviceId = ref("");
const selectedId = ref("");
const panel = ref<Panel | null>(null);
const newName = ref("");
const renameName = ref("");
const code = ref("");
const preview = ref<WalkieTalkieDevicePreview | null>(null);
const previewCode = ref("");
const busy = ref(false);
const loading = ref(true);
const isCreatingGroup = ref(false);
const failedPhotos = ref<Record<string, boolean>>({});

function photoFailed(url: string | null): void {
  if (url) failedPhotos.value[url] = true;
}

const error = ref("");
const notice = ref("");
let rosterTimer: ReturnType<typeof setInterval> | undefined;
let refreshing = false;

const selected = computed(() => groups.value.find(group => group.id === selectedId.value));
const owner = computed(() => selected.value?.owner_id === userId.value);
const activeGroup = computed(() => groups.value.find(group => group.active));
const title = computed(() => ({
  add: "Add device",
  manage: "Manage devices",
  rename: "Rename group",
  delete: "Delete group",
})[panel.value || "manage"]);

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
    error.value = cause instanceof Error ? cause.message : "Unable to update walkie-talkie groups.";
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

function openPanel(group: WalkieTalkieGroup, next: Panel): void {
  selectedId.value = group.id;
  panel.value = next;
  renameName.value = group.name;
  code.value = "";
  preview.value = null;
  previewCode.value = "";
  error.value = "";
  notice.value = "";
}

function closePanel(): void {
  panel.value = null;
  selectedId.value = "";
  preview.value = null;
  code.value = "";
  previewCode.value = "";
}

function createGroup(): void {
  const name = newName.value.trim();
  if (!name) return;
  void run(async () => {
    await mutate(() => walkieTalkieApi.create(name), "Group created. Open its settings to add devices.");
    newName.value = "";
    isCreatingGroup.value = false;
  });
}

function activate(groupId: string | null): void {
  void run(() =>
    mutate(
      () => walkieTalkieApi.activate(groupId, props.deviceToken),
      groupId
        ? "Active group updated for this device."
        : "Active group cleared. Device memberships are saved."
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
    code.value = "";
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

function removeMember(member: WalkieTalkieMember): void {
  const group = selected.value;
  if (!group) return;
  const leaving = member.device_id === deviceId.value;
  if (
    !window.confirm(
      leaving
        ? `Leave “${group.name}” on this device?`
        : `Remove “${member.device_name || member.name}” from this group?`
    )
  )
    return;
  void run(() =>
    mutate(
      () => walkieTalkieApi.removeMember(group.id, member.device_id, props.deviceToken),
      leaving ? "This device left the group." : "Device removed from the group."
    )
  );
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
  <section class="relative space-y-4 rounded-xl border border-border bg-card p-4 sm:p-5 overflow-hidden" aria-label="Walkie-talkie groups">
    <!-- Header -->
    <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between border-b border-border/70 pb-4">
      <div>
        <div class="flex items-center gap-2">
          <div class="flex h-7 w-7 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Radio class="h-4 w-4" />
          </div>
          <h4 class="text-sm font-semibold tracking-tight text-foreground">Walkie-talkie</h4>
        </div>
        <p class="mt-1 text-xs text-muted-foreground leading-relaxed">
          Create groups and manage connected devices (up to 5 per group).
        </p>
      </div>

      <div class="flex items-center gap-2">
        <button
          type="button"
          :disabled="busy || saving"
          class="inline-flex items-center gap-1.5 rounded-lg border border-border bg-background px-3 py-1.5 text-xs font-medium text-foreground hover:bg-muted transition disabled:opacity-50"
          title="Refresh groups"
          @click="run(refresh)"
        >
          <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading || busy }" />
          <span>Refresh</span>
        </button>

        <button
          v-if="!isCreatingGroup"
          type="button"
          :disabled="busy || saving"
          class="inline-flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 transition active:scale-95 disabled:opacity-50"
          @click="isCreatingGroup = true"
        >
          <Plus class="h-3.5 w-3.5" />
          <span>New group</span>
        </button>
      </div>
    </div>

    <!-- Disabled notice -->
    <div v-if="!enabled" class="rounded-lg bg-muted/60 p-3 text-xs text-muted-foreground">
      The walkie-talkie widget is currently disabled on this device. Its group memberships remain intact.
    </div>

    <!-- Alerts -->
    <p v-if="error" role="alert" class="rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">
      {{ error }}
    </p>
    <p v-if="notice" role="status" class="rounded-lg border border-border bg-muted/50 p-3 text-xs text-muted-foreground">
      {{ notice }}
    </p>

    <fieldset :disabled="busy || saving" class="min-w-0 space-y-4 disabled:opacity-60">
      <!-- Inline New Group Form -->
      <transition name="expand">
        <form
          v-if="isCreatingGroup"
          class="rounded-xl border border-primary/30 bg-muted/20 p-3.5 space-y-3"
          @submit.prevent="createGroup"
        >
          <div class="flex items-center justify-between">
            <h5 class="text-xs font-semibold text-foreground">Create a new group</h5>
            <button
              type="button"
              class="rounded-md p-1 text-muted-foreground hover:text-foreground"
              @click="isCreatingGroup = false; newName = ''"
            >
              <X class="h-3.5 w-3.5" />
            </button>
          </div>
          <div class="flex gap-2">
            <input
              v-model="newName"
              maxlength="60"
              required
              autofocus
              placeholder="e.g. Operations, Field Team..."
              class="flex-1 rounded-lg border border-input bg-background px-3 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-primary"
            />
            <button
              type="submit"
              :disabled="!newName.trim()"
              class="rounded-lg bg-primary px-3.5 py-1.5 text-xs font-semibold text-primary-foreground disabled:opacity-50 hover:bg-primary/90 transition shadow-sm"
            >
              Create
            </button>
            <button
              type="button"
              class="rounded-lg border border-border bg-background px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition"
              @click="isCreatingGroup = false; newName = ''"
            >
              Cancel
            </button>
          </div>
        </form>
      </transition>

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
            class="mt-3 inline-flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 transition active:scale-95"
            @click="isCreatingGroup = true"
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
                'flex items-center justify-between gap-3 p-3 sm:px-4 transition-colors',
                selectedId === group.id ? 'bg-primary/5' : 'hover:bg-muted/30',
              ]"
            >
              <!-- Group info -->
              <div class="flex items-center gap-3 min-w-0">
                <div
                  class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl transition-colors"
                  :class="group.active ? 'bg-emerald-500/15 text-emerald-500' : 'bg-muted text-muted-foreground'"
                >
                  <Radio class="h-4 w-4" />
                </div>

                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <h6 class="truncate text-sm font-semibold text-foreground">{{ group.name }}</h6>
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
                  <p class="mt-0.5 text-xs text-muted-foreground">
                    {{ group.members.length }}/5 devices
                    <span v-if="group.members.some(m => m.audio_active)" class="text-emerald-500 font-medium"> · Audio active</span>
                  </p>
                </div>
              </div>

              <!-- Action buttons -->
              <div class="flex items-center gap-1.5 shrink-0">
                <button
                  v-if="!group.active && group.joined"
                  type="button"
                  :disabled="busy || saving"
                  class="rounded-lg border border-border bg-background px-2.5 py-1 text-xs font-medium text-foreground hover:bg-muted transition disabled:opacity-50"
                  @click="activate(group.id)"
                >
                  Make active
                </button>

                <button
                  type="button"
                  :disabled="busy || saving"
                  class="rounded-lg border border-border bg-background px-2.5 py-1 text-xs font-medium text-foreground hover:bg-muted transition disabled:opacity-50"
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
                      class="flex h-7 w-7 items-center justify-center rounded-lg border border-border bg-background text-muted-foreground hover:bg-muted hover:text-foreground transition disabled:opacity-50"
                    >
                      <MoreHorizontal class="h-3.5 w-3.5" />
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuPortal>
                    <DropdownMenuContent
                      align="end"
                      :side-offset="6"
                      :collision-padding="16"
                      class="z-50 min-w-44 rounded-xl border border-border bg-popover p-1 text-popover-foreground shadow-xl outline-none"
                    >
                      <DropdownMenuItem
                        class="flex items-center gap-2 cursor-pointer rounded-lg px-3 py-2 text-xs outline-none hover:bg-accent focus:bg-accent"
                        @select="openPanel(group, 'manage')"
                      >
                        <Users class="h-3.5 w-3.5 text-muted-foreground" />
                        <span>Manage devices</span>
                      </DropdownMenuItem>

                      <DropdownMenuItem
                        v-if="group.owner_id === userId"
                        :disabled="group.members.length >= 5"
                        class="flex items-center gap-2 cursor-pointer rounded-lg px-3 py-2 text-xs outline-none hover:bg-accent focus:bg-accent data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
                        @select="openPanel(group, 'add')"
                      >
                        <UserPlus class="h-3.5 w-3.5 text-muted-foreground" />
                        <span>Add device</span>
                      </DropdownMenuItem>

                      <template v-if="group.owner_id === userId">
                        <DropdownMenuItem
                          class="flex items-center gap-2 cursor-pointer rounded-lg px-3 py-2 text-xs outline-none hover:bg-accent focus:bg-accent"
                          @select="openPanel(group, 'rename')"
                        >
                          <Edit2 class="h-3.5 w-3.5 text-muted-foreground" />
                          <span>Rename group</span>
                        </DropdownMenuItem>

                        <DropdownMenuSeparator class="my-1 h-px bg-border/60" />

                        <DropdownMenuItem
                          class="flex items-center gap-2 cursor-pointer rounded-lg px-3 py-2 text-xs text-destructive outline-none hover:bg-destructive/10 focus:bg-destructive/10"
                          @select="openPanel(group, 'delete')"
                        >
                          <Trash2 class="h-3.5 w-3.5" />
                          <span>Delete group</span>
                        </DropdownMenuItem>
                      </template>
                    </DropdownMenuContent>
                  </DropdownMenuPortal>
                </DropdownMenuRoot>
              </div>
            </li>
          </ul>
        </div>

        <div v-if="activeGroup" class="pt-1">
          <button
            type="button"
            class="text-xs text-muted-foreground underline underline-offset-4 hover:text-foreground transition"
            @click="activate(null)"
          >
            Clear active group for this device
          </button>
        </div>
      </div>
    </fieldset>

    <!-- Slide-in Drawer Backdrop -->
    <transition name="drawer-fade">
      <div
        v-if="selected && panel"
        class="absolute inset-0 z-20 bg-black/50 backdrop-blur-[2px]"
        @click="closePanel"
      />
    </transition>

    <!-- Slide-in Drawer Detail Panel -->
    <transition name="drawer-slide">
      <div
        v-if="selected && panel"
        class="absolute inset-y-0 right-0 z-30 flex w-full max-w-sm flex-col border-l border-border bg-card shadow-2xl p-5 overflow-y-auto text-card-foreground"
      >
        <!-- Drawer Header -->
        <div class="flex items-center justify-between border-b border-border/70 pb-3">
          <div class="min-w-0 pr-2">
            <h5 class="text-sm font-semibold truncate text-foreground">{{ title }}</h5>
            <p class="text-xs text-muted-foreground truncate">{{ selected.name }} · {{ selected.members.length }}/5 devices</p>
          </div>
          <button
            type="button"
            class="rounded-lg p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
            aria-label="Close panel"
            @click="closePanel"
          >
            <X class="h-4 w-4" />
          </button>
        </div>

        <!-- Panel Body: Add Device -->
        <template v-if="panel === 'add' && owner">
          <div class="py-3 space-y-4">
            <form v-if="!preview" class="space-y-3" @submit.prevent="previewDevice">
              <p class="text-xs text-muted-foreground leading-relaxed">
                Generate a device code on the phone you want to add, then enter the 6 digits below.
              </p>
              <label class="block text-xs font-medium text-foreground">
                6-digit code
                <input
                  v-model="code"
                  inputmode="numeric"
                  pattern="[0-9]{6}"
                  maxlength="6"
                  minlength="6"
                  required
                  autocomplete="off"
                  placeholder="000000"
                  class="mt-1.5 w-full rounded-lg border border-input bg-background px-3 py-2.5 font-mono text-center text-xl tracking-[0.25em] focus:outline-none focus:ring-1 focus:ring-primary"
                />
              </label>

              <button
                type="submit"
                :disabled="!/^\d{6}$/.test(code) || selected.members.length >= 5"
                class="w-full rounded-lg bg-primary py-2 text-xs font-semibold text-primary-foreground disabled:opacity-50 hover:bg-primary/90 transition shadow-sm"
              >
                Preview device
              </button>
            </form>

            <div v-else class="space-y-3">
              <p class="text-xs text-muted-foreground">Verify the device before confirming admission:</p>
              <div class="flex items-center gap-3 rounded-xl border border-border bg-background p-3">
                <img
                  v-if="preview.device.picture_url && !failedPhotos[preview.device.picture_url]"
                  :src="preview.device.picture_url"
                  alt=""
                  referrerpolicy="no-referrer"
                  class="h-10 w-10 rounded-full object-cover"
                  @error="photoFailed(preview.device.picture_url)"
                />
                <span
                  v-else
                  class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-muted text-sm font-semibold text-foreground"
                >
                  {{ (preview.device.first_name || preview.device.name).charAt(0) }}
                </span>
                <div class="min-w-0">
                  <p class="truncate text-sm font-medium text-foreground">{{ preview.device.first_name || preview.device.name }}</p>
                  <p class="truncate text-xs text-muted-foreground">{{ preview.device.device_name }}</p>
                </div>
              </div>

              <div class="flex gap-2">
                <button
                  type="button"
                  :disabled="selected.members.length >= 5"
                  class="flex-1 rounded-lg bg-primary py-2 text-xs font-semibold text-primary-foreground disabled:opacity-50 hover:bg-primary/90 transition shadow-sm"
                  @click="addDevice"
                >
                  Confirm admission
                </button>
                <button
                  type="button"
                  class="rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition"
                  @click="preview = null"
                >
                  Back
                </button>
              </div>
            </div>

            <p v-if="selected.members.length >= 5" class="rounded-lg bg-destructive/10 p-2.5 text-xs text-destructive">
              This group has reached the 5-device maximum limit. Remove a device first.
            </p>
          </div>
        </template>

        <!-- Panel Body: Manage Devices -->
        <template v-else-if="panel === 'manage'">
          <div class="py-3 space-y-3">
            <div class="flex items-center justify-between">
              <span class="text-xs text-muted-foreground">{{ selected.members.length }}/5 slots filled</span>
              <button
                v-if="owner && selected.members.length < 5"
                type="button"
                class="inline-flex items-center gap-1 text-xs font-semibold text-primary hover:underline"
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
                class="flex items-center gap-3 p-3"
              >
                <img
                  v-if="member.picture_url && !failedPhotos[member.picture_url]"
                  :src="member.picture_url"
                  alt=""
                  referrerpolicy="no-referrer"
                  class="h-9 w-9 shrink-0 rounded-full object-cover"
                  @error="photoFailed(member.picture_url)"
                />
                <span
                  v-else
                  class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-semibold text-foreground"
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

                <button
                  v-if="owner || member.device_id === deviceId"
                  type="button"
                  class="shrink-0 text-xs text-destructive hover:underline"
                  @click="removeMember(member)"
                >
                  {{ member.device_id === deviceId ? 'Leave' : 'Remove' }}
                </button>
              </li>

              <li v-if="!selected.members.length" class="p-6 text-center text-xs text-muted-foreground">
                No devices joined yet.
              </li>
            </ul>
          </div>
        </template>

        <!-- Panel Body: Rename Group -->
        <template v-else-if="panel === 'rename' && owner">
          <form class="py-3 space-y-3" @submit.prevent="renameGroup">
            <label class="block text-xs font-medium text-foreground">
              Group name
              <input
                v-model="renameName"
                required
                maxlength="60"
                class="mt-1.5 w-full rounded-lg border border-input bg-background px-3 py-2 text-xs focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </label>
            <div class="flex gap-2">
              <button
                type="submit"
                :disabled="!renameName.trim() || renameName.trim() === selected.name"
                class="rounded-lg bg-primary px-4 py-2 text-xs font-semibold text-primary-foreground disabled:opacity-50 hover:bg-primary/90 transition shadow-sm"
              >
                Save name
              </button>
              <button
                type="button"
                class="rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition"
                @click="closePanel"
              >
                Cancel
              </button>
            </div>
          </form>
        </template>

        <!-- Panel Body: Delete Group -->
        <template v-else-if="panel === 'delete' && owner">
          <div class="py-3 space-y-3">
            <p class="text-xs leading-relaxed text-muted-foreground">
              Are you sure you want to delete <strong class="text-foreground">“{{ selected.name }}”</strong>? This removes all {{ selected.members.length }} member devices and closes any ongoing audio sessions.
            </p>
            <div class="flex gap-2 pt-1">
              <button
                type="button"
                class="flex-1 rounded-lg bg-destructive py-2 text-xs font-semibold text-white hover:bg-destructive/90 transition shadow-sm"
                @click="removeGroup"
              >
                Confirm delete
              </button>
              <button
                type="button"
                class="rounded-lg border border-border bg-background px-4 py-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition"
                @click="closePanel"
              >
                Cancel
              </button>
            </div>
          </div>
        </template>
      </div>
    </transition>
  </section>
</template>

<style scoped>
.expand-enter-active,
.expand-leave-active {
  transition: all 0.2s ease-out;
}
.expand-enter-from,
.expand-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

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
.drawer-slide-enter-from,
.drawer-slide-leave-to {
  transform: translateX(100%);
}
</style>

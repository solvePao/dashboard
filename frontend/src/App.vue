<script setup lang="ts">
import { ref, watch } from "vue";
import AppFooter from "./components/AppFooter.vue";
import TopNavbar from "./components/TopNavbar.vue";
import ConnectedDevicesList from "./components/ConnectedDevicesList.vue";
import DeviceManageView from "./components/DeviceManageView.vue";
import AddDeviceModal from "./components/AddDeviceModal.vue";
import RepairDeviceModal from "./components/RepairDeviceModal.vue";
import UserProfileCard from "./components/UserProfileCard.vue";
import WelcomePage from "./components/WelcomePage.vue";
import { useAuth } from "./composables/useAuth";
import { useDevices } from "./composables/useDevices";
import type { DeviceItem } from "./types/device";

const { user, isLoggedIn } = useAuth();
const { devices } = useDevices();

const showAddDeviceModal = ref(false);
const managedDevice = ref<DeviceItem | null>(null);
const repairingDevice = ref<DeviceItem | null>(null);

function handleManage(device: DeviceItem): void {
  managedDevice.value = device;
}

function handleRepair(device: DeviceItem): void {
  repairingDevice.value = device;
}

function handleManageBack(): void {
  managedDevice.value = null;
}

function handleDeviceRevoked(): void {
  managedDevice.value = null;
}

watch(isLoggedIn, (loggedIn) => {
  if (!loggedIn) {
    managedDevice.value = null;
    repairingDevice.value = null;
    showAddDeviceModal.value = false;
  }
});

// Exit manage view if the device was revoked or deleted from list
watch(devices, (list) => {
  if (
    managedDevice.value &&
    !list.some((d) => d.token === managedDevice.value?.token)
  ) {
    managedDevice.value = null;
  }
});
</script>

<template>
  <TopNavbar />
  <main
    class="fe-app-main mx-auto w-full max-w-[var(--container-max)] space-y-5 px-3.5 pt-4 pb-28 sm:space-y-6 sm:px-6 sm:pt-6 sm:pb-10 pb-safe overflow-x-hidden min-h-[calc(100dvh-var(--topbar-h))]"
  >
    <!-- When managing a device, replace everything with the manage view -->
    <template v-if="isLoggedIn && user && managedDevice">
      <DeviceManageView
        :device="managedDevice"
        @back="handleManageBack"
        @revoked="handleDeviceRevoked"
      />
    </template>

    <template v-else-if="isLoggedIn && user">
      <UserProfileCard :user="user" />
      <ConnectedDevicesList
        @add-device="showAddDeviceModal = true"
        @manage="handleManage"
        @repair="handleRepair"
      />
    </template>

    <WelcomePage v-else />
  </main>

  <!-- Footer is hidden when managing a device -->
  <AppFooter v-if="!managedDevice" />

  <AddDeviceModal :open="showAddDeviceModal" @close="showAddDeviceModal = false" />
  <RepairDeviceModal
    :device="repairingDevice"
    :open="!!repairingDevice"
    @close="repairingDevice = null"
  />
</template>

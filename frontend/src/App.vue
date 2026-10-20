<script setup lang="ts">
import { ref, watch } from "vue";
import AppFooter from "./components/AppFooter.vue";
import TopNavbar from "./components/TopNavbar.vue";
import ConnectedDevicesList from "./components/ConnectedDevicesList.vue";
import DeviceManageView from "./components/DeviceManageView.vue";
import AddDeviceModal from "./components/AddDeviceModal.vue";
import UserProfileCard from "./components/UserProfileCard.vue";
import WelcomePage from "./components/WelcomePage.vue";
import { useAuth } from "./composables/useAuth";
import { useDevices } from "./composables/useDevices";
import type { DeviceItem } from "./types/device";

const { user, isLoggedIn } = useAuth();
const { devices } = useDevices();

const showAddDeviceModal = ref(false);
const managedDevice = ref<DeviceItem | null>(null);

function handleManage(device: DeviceItem): void {
  managedDevice.value = device;
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
    class="fe-app-main mx-auto w-full max-w-[var(--container-max)] space-y-6 px-4 pt-6 pb-10 sm:px-6"
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
      />
    </template>

    <WelcomePage v-else />
  </main>

  <!-- Footer is hidden when managing a device -->
  <AppFooter v-if="!managedDevice" />

  <AddDeviceModal :open="showAddDeviceModal" @close="showAddDeviceModal = false" />
</template>

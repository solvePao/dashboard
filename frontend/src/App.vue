<script setup lang="ts">
import { ref } from "vue";
import BottomNavbar from "./components/BottomNavbar.vue";
import TopNavbar from "./components/TopNavbar.vue";
import ConnectedDevicesList from "./components/ConnectedDevicesList.vue";
import AddDeviceModal from "./components/AddDeviceModal.vue";
import UserProfileCard from "./components/UserProfileCard.vue";
import SignInPrompt from "./components/SignInPrompt.vue";
import { useAuth } from "./composables/useAuth";

const { user, isLoggedIn } = useAuth();
const showAddDeviceModal = ref(false);
</script>

<template>
  <TopNavbar />
  <main
    class="mx-auto w-full max-w-[var(--container-max)] space-y-6 px-4 pt-6 pb-[calc(var(--statusbar-h)+1.5rem)] sm:px-6"
  >
    <template v-if="isLoggedIn && user">
      <UserProfileCard :user="user" />
      <ConnectedDevicesList @add-device="showAddDeviceModal = true" />
    </template>

    <SignInPrompt v-else />
  </main>
  <BottomNavbar />

  <AddDeviceModal :open="showAddDeviceModal" @close="showAddDeviceModal = false" />
</template>

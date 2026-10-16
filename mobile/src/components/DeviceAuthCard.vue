<script setup lang="ts">
import { useDeviceAuth } from "../composables/useDeviceAuth";
import DeviceAuthorizedView from "./DeviceAuthorizedView.vue";
import DevicePairingView from "./DevicePairingView.vue";

const {
  shortCode,
  qrSvg,
  expiresAt,
  user,
  authState,
  isAuthenticated,
  createSession,
  disconnect,
} = useDeviceAuth();
</script>

<template>
  <div class="flex h-full w-full flex-col justify-between">
    <DeviceAuthorizedView
      v-if="isAuthenticated && user"
      :user="user"
      @disconnect="disconnect"
    />

    <DevicePairingView
      v-else
      :auth-state="authState"
      :qr-svg="qrSvg"
      :short-code="shortCode"
      :expires-at="expiresAt"
      @refresh="createSession"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "reka-ui";
import { ChevronDown, LayoutDashboard, LogOut, Menu, Smartphone, X } from "@lucide/vue";
import { useAuth } from "../composables/useAuth";
import GoogleSignInButton from "./GoogleSignInButton.vue";
import ThemeToggle from "./ThemeToggle.vue";
import AddDeviceModal from "./AddDeviceModal.vue";

const { user, isLoggedIn, authError, signOut } = useAuth();
const mobileOpen = ref(false);
const showAddDevice = ref(false);

function closeMobile(): void {
  mobileOpen.value = false;
}

function handleSignOut(): void {
  closeMobile();
  signOut();
}

const navLinks = [{ label: "Overview", href: "#", active: true }] as const;
</script>

<template>
  <header class="fe-topnav-header sticky top-0 z-50 border-b border-border bg-background/80 backdrop-blur-md">
    <div
      class="fe-topnav-container mx-auto flex h-[var(--topbar-h)] w-full max-w-[var(--container-max)] items-center justify-between gap-3 px-4 sm:px-6"
    >
      <!-- Brand + desktop nav -->
      <div class="fe-topnav-brand-wrap flex min-w-0 items-center gap-6">
        <a href="#" class="fe-topnav-brand-link flex shrink-0 items-center gap-2.5" aria-label="Dashboard home">
          <span
            class="fe-topnav-brand-icon flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-primary-foreground"
          >
            <LayoutDashboard class="fe-topnav-brand-glyph h-4 w-4" aria-hidden="true" />
          </span>
          <span class="fe-topnav-brand-title text-[15px] font-semibold tracking-tight text-foreground">Dashboard</span>
        </a>

        <nav class="fe-topnav-desktop-nav hidden items-center gap-1 md:flex" aria-label="Primary">
          <a
            v-for="link in navLinks"
            :key="link.label"
            :href="link.href"
            :aria-current="link.active ? 'page' : undefined"
            :class="['fe-topnav-desktop-link', 'nav-link', link.active && 'nav-link-active']"
          >
            {{ link.label }}
          </a>
        </nav>
      </div>

      <!-- Right: auth + mobile toggle -->
      <div class="fe-topnav-actions flex shrink-0 items-center gap-2">
        <ThemeToggle />
        <GoogleSignInButton v-if="!isLoggedIn" />

        <template v-else>
          <button
            type="button"
            class="fe-topnav-add-btn hidden sm:inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-2.5 py-1.5 text-xs font-medium text-foreground shadow-xs transition hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            @click="showAddDevice = true"
          >
            <Smartphone class="fe-topnav-add-btn-icon h-3.5 w-3.5 text-primary" />
            <span class="fe-topnav-add-btn-label">Add Device</span>
          </button>

          <DropdownMenuRoot>
            <DropdownMenuTrigger as-child>
              <button
                type="button"
                aria-label="Profile menu"
                class="fe-topnav-profile-trigger flex items-center gap-1.5 rounded-full py-0.5 pl-0.5 pr-1.5 transition hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1"
              >
                <span
                  class="fe-topnav-avatar-wrap flex h-8 w-8 items-center justify-center overflow-hidden rounded-full border border-border bg-muted"
                >
                  <img
                    v-if="user?.picture_url"
                    :src="user.picture_url"
                    alt=""
                    referrerpolicy="no-referrer"
                    class="fe-topnav-avatar-img h-full w-full object-cover"
                  />
                  <span v-else class="fe-topnav-avatar-fallback text-xs font-semibold text-muted-foreground">
                    {{ (user?.name || user?.email || "?").charAt(0).toUpperCase() }}
                  </span>
                </span>
                <ChevronDown class="fe-topnav-profile-chevron h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />
              </button>
            </DropdownMenuTrigger>

            <DropdownMenuPortal>
              <DropdownMenuContent
                :side-offset="8"
                align="end"
                class="fe-topnav-profile-menu z-50 w-64 rounded-xl border border-border bg-popover p-1.5 text-popover-foreground shadow-md outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
              >
                <DropdownMenuLabel class="fe-topnav-profile-menu-label px-2.5 py-2">
                  <p class="fe-topnav-profile-name truncate text-sm font-medium text-foreground">
                    {{ user?.name || "Signed in" }}
                  </p>
                  <p class="fe-topnav-profile-email truncate text-xs font-normal text-muted-foreground">{{ user?.email }}</p>
                </DropdownMenuLabel>
                <DropdownMenuSeparator class="fe-topnav-profile-separator my-1 h-px bg-border" />
                <DropdownMenuItem
                  class="fe-topnav-profile-add-item flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-sm text-foreground outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground"
                  @select="showAddDevice = true"
                >
                  <Smartphone class="fe-topnav-profile-add-icon h-4 w-4 text-muted-foreground" aria-hidden="true" />
                  Add Device
                </DropdownMenuItem>
                <DropdownMenuItem
                  class="fe-topnav-profile-signout-item flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-sm text-foreground outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground"
                  @select="handleSignOut"
                >
                  <LogOut class="fe-topnav-profile-signout-icon h-4 w-4 text-muted-foreground" aria-hidden="true" />
                  Sign out
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenuPortal>
          </DropdownMenuRoot>
        </template>

        <button
          type="button"
          class="fe-topnav-mobile-toggle inline-flex h-9 w-9 items-center justify-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring md:hidden"
          :aria-expanded="mobileOpen"
          aria-controls="mobile-nav"
          aria-label="Toggle navigation menu"
          @click="mobileOpen = !mobileOpen"
        >
          <Menu v-if="!mobileOpen" class="fe-topnav-mobile-menu-icon h-5 w-5" aria-hidden="true" />
          <X v-else class="fe-topnav-mobile-close-icon h-5 w-5" aria-hidden="true" />
        </button>
      </div>
    </div>

    <!-- Mobile nav -->
    <nav
      v-if="mobileOpen"
      id="mobile-nav"
      class="fe-topnav-mobile-nav border-t border-border bg-background px-4 py-3 md:hidden"
      aria-label="Mobile"
    >
      <div class="fe-topnav-mobile-nav-list flex flex-col gap-1">
        <a
          v-for="link in navLinks"
          :key="link.label"
          :href="link.href"
          :aria-current="link.active ? 'page' : undefined"
          :class="['fe-topnav-mobile-nav-link', 'nav-link w-full justify-start', link.active && 'nav-link-active']"
          @click="closeMobile"
        >
          {{ link.label }}
        </a>
      </div>
    </nav>

    <p
      v-if="authError && !isLoggedIn"
      class="fe-topnav-auth-error border-t border-destructive/20 bg-destructive/5 px-4 py-1.5 text-center text-xs text-destructive"
      role="alert"
    >
      {{ authError }}
    </p>
  </header>

  <AddDeviceModal :open="showAddDevice" @close="showAddDevice = false" />
</template>

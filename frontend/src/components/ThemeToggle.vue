<script setup lang="ts">
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from "reka-ui";
import { Check, Monitor, Moon, Sun } from "@lucide/vue";
import { useTheme, type ThemePreference } from "../composables/useTheme";

const { preference, resolvedTheme, setPreference } = useTheme();

const options: { value: ThemePreference; label: string; icon: typeof Sun }[] = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System", icon: Monitor },
];

function labelFor(pref: ThemePreference): string {
  return options.find((o) => o.value === pref)?.label ?? "System";
}
</script>

<template>
  <DropdownMenuRoot>
    <DropdownMenuTrigger as-child>
      <button
        type="button"
        class="fe-theme-trigger inline-flex h-9 w-9 items-center justify-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1"
        :aria-label="`Theme: ${labelFor(preference)} (${resolvedTheme}). Change theme`"
        :title="`Theme: ${labelFor(preference)}`"
      >
        <Sun v-if="resolvedTheme === 'light'" class="fe-theme-sun-icon h-[18px] w-[18px]" aria-hidden="true" />
        <Moon v-else class="fe-theme-moon-icon h-[18px] w-[18px]" aria-hidden="true" />
      </button>
    </DropdownMenuTrigger>

    <DropdownMenuPortal>
      <DropdownMenuContent
        :side-offset="8"
        :collision-padding="16"
        align="end"
        class="fe-theme-menu z-50 w-44 rounded-xl border border-border bg-popover p-1.5 text-popover-foreground shadow-md outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
      >
        <DropdownMenuItem
          v-for="option in options"
          :key="option.value"
          class="fe-theme-menu-item flex min-h-[40px] cursor-pointer items-center gap-2.5 rounded-lg px-3 py-2 text-sm text-foreground outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground"
          @select="setPreference(option.value)"
        >
          <component :is="option.icon" class="fe-theme-menu-item-icon h-4 w-4 text-muted-foreground" aria-hidden="true" />
          <span class="fe-theme-menu-item-label flex-1">{{ option.label }}</span>
          <Check
            v-if="preference === option.value"
            class="fe-theme-menu-check h-4 w-4 text-foreground"
            aria-hidden="true"
          />
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>

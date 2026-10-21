<script setup lang="ts">
import { ArrowDown, ArrowUpRight, Check, CloudSun, LayoutDashboard, Link2, Smartphone, TrendingUp } from "@lucide/vue";
import GoogleSignInButton from "./GoogleSignInButton.vue";

const features = [
  { icon: Link2, label: "Connect your devices", description: "Pair your mobile device with a code or QR scan. Manage your connected devices in one place." },
  { icon: CloudSun, label: "Make room for the weather", description: "Choose your cities and see current conditions on your mobile dashboard." },
  { icon: TrendingUp, label: "Follow your markets", description: "Build a watchlist with stocks, crypto, and currency pairs. Choose what appears on each device." },
];
const steps = [
  { label: "Sign in", description: "Use your Google account to open your dashboard." },
  { label: "Pair a device", description: "Enter its pairing code or scan its QR code." },
  { label: "Make it yours", description: "Add widgets and choose the cities and markets you follow." },
];
</script>

<template>
  <div class="fe-welcome space-y-10 pb-6 sm:space-y-16 animate-in fade-in-50 duration-200">
    <section class="grid items-center gap-8 py-3 sm:py-12 lg:grid-cols-[1.1fr_1fr] lg:gap-16" aria-labelledby="welcome-title">
      <div class="min-w-0">
        <p class="mb-4 inline-flex items-center gap-2 rounded-full border border-border bg-card px-3 py-1 text-[11px] font-medium text-muted-foreground sm:text-xs">
          <span class="h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-500" aria-hidden="true"></span>
          <span class="truncate">A personal space for your everyday information</span>
        </p>
        <h1 id="welcome-title" class="max-w-xl text-[clamp(2rem,8.5vw,3.75rem)] font-semibold leading-[1.08] tracking-[-0.04em] text-foreground sm:text-6xl lg:text-7xl">
          Your devices.<br /><span class="text-muted-foreground">Your dashboard.</span>
        </h1>
        <p class="mt-4 max-w-md text-sm leading-relaxed text-muted-foreground sm:mt-6 sm:text-lg">
          Connect your mobile devices and choose the information they display, from local weather to market updates.
        </p>
        <div class="mt-6 flex flex-col items-stretch gap-3 sm:flex-row sm:items-center sm:gap-5">
          <div class="flex items-center justify-center sm:justify-start">
            <GoogleSignInButton />
          </div>
          <a
            href="#how-it-works"
            class="inline-flex min-h-[44px] items-center justify-center gap-2 rounded-lg border border-border/80 bg-background/60 px-4 py-2 text-sm font-medium text-foreground transition hover:bg-muted sm:border-0 sm:bg-transparent sm:px-0 sm:py-2 hover:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            How it works <ArrowDown class="h-4 w-4" aria-hidden="true" />
          </a>
        </div>
        <p class="mt-3 text-[11px] text-muted-foreground sm:mt-4 sm:text-xs">One account. A dashboard for every connected device.</p>
      </div>

      <figure class="min-w-0 rounded-2xl border border-border bg-muted/30 p-3 sm:rounded-3xl sm:p-6" aria-label="Sample dashboard with weather and market widgets">
        <div class="rounded-xl border border-border bg-background p-3.5 shadow-sm sm:rounded-2xl sm:p-5">
          <div class="flex items-center justify-between gap-3 border-b border-border pb-3 sm:pb-4">
            <div class="flex min-w-0 items-center gap-2 text-xs font-semibold sm:text-sm"><LayoutDashboard class="h-4 w-4 shrink-0" aria-hidden="true" /> Your daily overview</div>
            <span class="shrink-0 rounded-md bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground sm:py-1">Sample</span>
          </div>
          <div class="mt-4 flex items-center gap-2 text-xs text-muted-foreground sm:mt-5"><Smartphone class="h-4 w-4" aria-hidden="true" /> My mobile dashboard <Check class="ml-auto h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400" aria-hidden="true" /></div>
          <div class="mt-3.5 rounded-xl border border-border bg-card p-4 sm:mt-4 sm:p-5">
            <div class="flex items-center justify-between gap-3"><span class="text-xs font-medium text-muted-foreground">London</span><CloudSun class="h-6 w-6 text-amber-600 dark:text-amber-300 sm:h-7 sm:w-7" aria-hidden="true" /></div>
            <p class="mt-2 text-4xl font-medium tracking-tighter sm:text-5xl">18<span class="align-top text-xl text-muted-foreground sm:text-2xl">°</span></p>
            <p class="mt-2 text-[11px] text-muted-foreground sm:text-xs">Partly cloudy · A little sunshine for your day</p>
          </div>
          <div class="mt-3 rounded-xl border border-border bg-card p-3.5 sm:p-4">
            <div class="flex items-center gap-2 text-xs font-medium text-muted-foreground"><TrendingUp class="h-4 w-4" aria-hidden="true" /> Your watchlist</div>
            <div class="mt-3.5 flex flex-wrap items-center justify-between gap-2.5 sm:mt-4 sm:gap-3"><div><p class="text-xs font-semibold sm:text-sm">AAPL <span class="ml-1 text-[11px] font-normal text-muted-foreground sm:text-xs">Apple</span></p><p class="mt-0.5 text-lg font-medium tracking-tight sm:mt-1 sm:text-xl">$189.98</p></div><span class="inline-flex items-center gap-1 rounded-md bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-700 dark:text-emerald-400 sm:py-1 sm:text-xs"><ArrowUpRight class="h-3 w-3" aria-hidden="true" /> 1.24%</span></div>
          </div>
        </div>
        <figcaption class="mt-3 text-center text-[10px] text-muted-foreground sm:mt-4 sm:text-[11px]">An illustration of your mobile dashboard. All values are sample data.</figcaption>
      </figure>
    </section>

    <section aria-labelledby="features-title">
      <p class="text-[11px] font-medium uppercase tracking-widest text-muted-foreground sm:text-xs">A few things, well connected</p>
      <h2 id="features-title" class="mt-2 text-xl font-semibold tracking-tight sm:mt-3 sm:text-3xl">The information you want. Where you want it.</h2>
      <div class="mt-5 grid gap-3.5 sm:mt-7 sm:gap-4 md:grid-cols-3">
        <article v-for="feature in features" :key="feature.label" class="rounded-xl border border-border bg-card p-4 transition-colors sm:rounded-2xl sm:p-6">
          <span class="inline-flex h-9 w-9 items-center justify-center rounded-lg border border-border bg-muted/50 sm:h-10 sm:w-10 sm:rounded-xl"><component :is="feature.icon" class="h-4.5 w-4.5 sm:h-5 sm:w-5" aria-hidden="true" /></span>
          <h3 class="mt-3.5 text-sm font-semibold tracking-tight sm:mt-5 sm:text-base">{{ feature.label }}</h3>
          <p class="mt-1.5 text-xs leading-relaxed text-muted-foreground sm:mt-2 sm:text-sm">{{ feature.description }}</p>
        </article>
      </div>
    </section>

    <section id="how-it-works" class="scroll-mt-20 rounded-xl border border-border bg-muted/20 p-4 sm:scroll-mt-24 sm:rounded-2xl sm:p-8" aria-labelledby="steps-title">
      <h2 id="steps-title" class="text-xl font-semibold tracking-tight sm:text-2xl">From sign-in to your screen.</h2>
      <p class="mt-1.5 text-xs text-muted-foreground sm:mt-2 sm:text-sm">Three simple steps to a more personal dashboard.</p>
      <ol class="mt-6 grid gap-5 sm:mt-8 sm:gap-7 md:grid-cols-3">
        <li v-for="(step, index) in steps" :key="step.label" class="flex items-start gap-3.5 sm:gap-4">
          <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full border border-border bg-background text-[11px] font-medium text-muted-foreground sm:h-8 sm:w-8 sm:text-xs">{{ String(index + 1).padStart(2, '0') }}</span>
          <div><h3 class="text-xs font-semibold sm:text-sm">{{ step.label }}</h3><p class="mt-1 text-xs leading-relaxed text-muted-foreground sm:mt-1.5 sm:text-sm">{{ step.description }}</p></div>
        </li>
      </ol>
    </section>
  </div>
</template>

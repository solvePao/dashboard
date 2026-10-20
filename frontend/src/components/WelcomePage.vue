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
  <div class="fe-welcome space-y-16 pb-6 sm:space-y-20">
    <section class="grid items-center gap-12 py-8 sm:py-14 lg:grid-cols-[1.1fr_1fr] lg:gap-16" aria-labelledby="welcome-title">
      <div class="min-w-0">
        <p class="mb-6 inline-flex items-center gap-2 rounded-full border border-border bg-card px-3 py-1.5 text-xs font-medium text-muted-foreground">
          <span class="h-1.5 w-1.5 rounded-full bg-emerald-500" aria-hidden="true"></span>
          A personal space for your everyday information
        </p>
        <h1 id="welcome-title" class="max-w-xl text-[clamp(2.5rem,12vw,3rem)] font-semibold leading-[1.08] tracking-[-0.055em] text-foreground sm:text-6xl lg:text-7xl">
          Your devices.<br /><span class="text-muted-foreground">Your dashboard.</span>
        </h1>
        <p class="mt-6 max-w-md text-base leading-relaxed text-muted-foreground sm:text-lg">
          Connect your mobile devices and choose the information they display, from local weather to market updates.
        </p>
        <div class="mt-8 flex flex-wrap items-center gap-5">
          <GoogleSignInButton />
          <a href="#how-it-works" class="inline-flex items-center gap-2 rounded-md py-2 text-sm font-medium text-foreground hover:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
            How it works <ArrowDown class="h-4 w-4" aria-hidden="true" />
          </a>
        </div>
        <p class="mt-4 text-xs text-muted-foreground">One account. A dashboard for every connected device.</p>
      </div>

      <figure class="min-w-0 rounded-3xl border border-border bg-muted/30 p-4 sm:p-6" aria-label="Sample dashboard with weather and market widgets">
        <div class="rounded-2xl border border-border bg-background p-4 shadow-sm sm:p-5">
          <div class="flex items-center justify-between gap-3 border-b border-border pb-4">
            <div class="flex min-w-0 items-center gap-2 text-sm font-semibold"><LayoutDashboard class="h-4 w-4 shrink-0" aria-hidden="true" /> Your daily overview</div>
            <span class="shrink-0 rounded-md bg-muted px-2 py-1 text-[10px] font-medium text-muted-foreground">Sample</span>
          </div>
          <div class="mt-5 flex items-center gap-2 text-xs text-muted-foreground"><Smartphone class="h-4 w-4" aria-hidden="true" /> My mobile dashboard <Check class="ml-auto h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400" aria-hidden="true" /></div>
          <div class="mt-4 rounded-xl border border-border bg-card p-5">
            <div class="flex items-center justify-between gap-3"><span class="text-xs font-medium text-muted-foreground">London</span><CloudSun class="h-7 w-7 text-amber-600 dark:text-amber-300" aria-hidden="true" /></div>
            <p class="mt-2 text-5xl font-medium tracking-tighter">18<span class="align-top text-2xl text-muted-foreground">°</span></p>
            <p class="mt-2 text-xs text-muted-foreground">Partly cloudy · A little sunshine for your day</p>
          </div>
          <div class="mt-3 rounded-xl border border-border bg-card p-4">
            <div class="flex items-center gap-2 text-xs font-medium text-muted-foreground"><TrendingUp class="h-4 w-4" aria-hidden="true" /> Your watchlist</div>
            <div class="mt-4 flex flex-wrap items-center justify-between gap-3"><div><p class="text-sm font-semibold">AAPL <span class="ml-1 text-xs font-normal text-muted-foreground">Apple</span></p><p class="mt-1 text-xl font-medium tracking-tight">$189.98</p></div><span class="inline-flex items-center gap-1 rounded-md bg-emerald-500/10 px-2 py-1 text-xs font-medium text-emerald-700 dark:text-emerald-400"><ArrowUpRight class="h-3 w-3" aria-hidden="true" /> 1.24%</span></div>
          </div>
        </div>
        <figcaption class="mt-4 text-center text-[11px] text-muted-foreground">An illustration of your mobile dashboard. All values are sample data.</figcaption>
      </figure>
    </section>

    <section aria-labelledby="features-title">
      <p class="text-xs font-medium uppercase tracking-widest text-muted-foreground">A few things, well connected</p>
      <h2 id="features-title" class="mt-3 text-2xl font-semibold tracking-tight sm:text-3xl">The information you want. Where you want it.</h2>
      <div class="mt-7 grid gap-4 md:grid-cols-3">
        <article v-for="feature in features" :key="feature.label" class="rounded-2xl border border-border bg-card p-6">
          <span class="inline-flex h-10 w-10 items-center justify-center rounded-xl border border-border bg-muted/50"><component :is="feature.icon" class="h-5 w-5" aria-hidden="true" /></span>
          <h3 class="mt-5 text-base font-semibold tracking-tight">{{ feature.label }}</h3>
          <p class="mt-2 text-sm leading-relaxed text-muted-foreground">{{ feature.description }}</p>
        </article>
      </div>
    </section>

    <section id="how-it-works" class="scroll-mt-24 rounded-2xl border border-border bg-muted/20 p-6 sm:p-8" aria-labelledby="steps-title">
      <h2 id="steps-title" class="text-2xl font-semibold tracking-tight">From sign-in to your screen.</h2>
      <p class="mt-2 text-sm text-muted-foreground">Three simple steps to a more personal dashboard.</p>
      <ol class="mt-8 grid gap-7 md:grid-cols-3">
        <li v-for="(step, index) in steps" :key="step.label" class="flex items-start gap-4">
          <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full border border-border bg-background text-xs font-medium text-muted-foreground">{{ String(index + 1).padStart(2, '0') }}</span>
          <div><h3 class="text-sm font-semibold">{{ step.label }}</h3><p class="mt-1.5 text-sm leading-relaxed text-muted-foreground">{{ step.description }}</p></div>
        </li>
      </ol>
    </section>
  </div>
</template>

<script setup lang="ts">
import type { WeatherEffectType } from "../../services/weatherApi";

defineProps<{
  effect: WeatherEffectType;
}>();

// Deterministic pseudo-random seed arrays for particles so there's zero hydration or render drift
const DUST_MOTES = [
  { left: "15%", bottom: "10%", size: "3px", delay: "0s", duration: "7s" },
  { left: "28%", bottom: "25%", size: "4px", delay: "1.2s", duration: "8.5s" },
  { left: "45%", bottom: "15%", size: "3px", delay: "2.4s", duration: "6.5s" },
  { left: "62%", bottom: "35%", size: "5px", delay: "0.8s", duration: "9s" },
  { left: "78%", bottom: "20%", size: "3.5px", delay: "3.1s", duration: "7.8s" },
  { left: "88%", bottom: "40%", size: "4px", delay: "1.9s", duration: "8.2s" },
];

const DAY_RAINDROPS = [
  { left: "4%", top: "-30px", duration: "0.72s", delay: "0.0s", height: "26px", opacity: 0.7 },
  { left: "10%", top: "-30px", duration: "0.85s", delay: "0.35s", height: "22px", opacity: 0.5 },
  { left: "16%", top: "-30px", duration: "0.68s", delay: "0.15s", height: "30px", opacity: 0.8 },
  { left: "23%", top: "-30px", duration: "0.78s", delay: "0.5s", height: "24px", opacity: 0.6 },
  { left: "31%", top: "-30px", duration: "0.71s", delay: "0.2s", height: "28px", opacity: 0.75 },
  { left: "38%", top: "-30px", duration: "0.82s", delay: "0.45s", height: "20px", opacity: 0.55 },
  { left: "46%", top: "-30px", duration: "0.65s", delay: "0.1s", height: "32px", opacity: 0.85 },
  { left: "53%", top: "-30px", duration: "0.75s", delay: "0.6s", height: "25px", opacity: 0.65 },
  { left: "61%", top: "-30px", duration: "0.69s", delay: "0.25s", height: "29px", opacity: 0.8 },
  { left: "69%", top: "-30px", duration: "0.84s", delay: "0.4s", height: "22px", opacity: 0.5 },
  { left: "77%", top: "-30px", duration: "0.73s", delay: "0.05s", height: "27px", opacity: 0.7 },
  { left: "84%", top: "-30px", duration: "0.80s", delay: "0.55s", height: "23px", opacity: 0.6 },
  { left: "91%", top: "-30px", duration: "0.67s", delay: "0.3s", height: "30px", opacity: 0.8 },
  { left: "97%", top: "-30px", duration: "0.76s", delay: "0.18s", height: "25px", opacity: 0.65 },
];

const NIGHT_RAINDROPS = [
  { left: "6%", top: "-30px", duration: "0.75s", delay: "0.1s", height: "28px", opacity: 0.65 },
  { left: "14%", top: "-30px", duration: "0.88s", delay: "0.4s", height: "22px", opacity: 0.45 },
  { left: "21%", top: "-30px", duration: "0.70s", delay: "0.2s", height: "32px", opacity: 0.75 },
  { left: "29%", top: "-30px", duration: "0.82s", delay: "0.55s", height: "24px", opacity: 0.5 },
  { left: "37%", top: "-30px", duration: "0.73s", delay: "0.05s", height: "29px", opacity: 0.7 },
  { left: "45%", top: "-30px", duration: "0.85s", delay: "0.35s", height: "21px", opacity: 0.4 },
  { left: "54%", top: "-30px", duration: "0.68s", delay: "0.15s", height: "33px", opacity: 0.8 },
  { left: "63%", top: "-30px", duration: "0.79s", delay: "0.6s", height: "26px", opacity: 0.6 },
  { left: "72%", top: "-30px", duration: "0.71s", delay: "0.25s", height: "30px", opacity: 0.75 },
  { left: "81%", top: "-30px", duration: "0.86s", delay: "0.45s", height: "23px", opacity: 0.45 },
  { left: "89%", top: "-30px", duration: "0.74s", delay: "0.12s", height: "28px", opacity: 0.7 },
  { left: "96%", top: "-30px", duration: "0.81s", delay: "0.5s", height: "24px", opacity: 0.55 },
];

const SAND_PARTICLES = [
  { top: "12%", duration: "0.9s", delay: "0.0s", size: "3px", opacity: 0.8 },
  { top: "19%", duration: "1.1s", delay: "0.4s", size: "2.5px", opacity: 0.6 },
  { top: "28%", duration: "0.8s", delay: "0.2s", size: "4px", opacity: 0.9 },
  { top: "37%", duration: "1.2s", delay: "0.7s", size: "2px", opacity: 0.5 },
  { top: "45%", duration: "0.85s", delay: "0.15s", size: "3.5px", opacity: 0.85 },
  { top: "54%", duration: "1.05s", delay: "0.5s", size: "2.5px", opacity: 0.65 },
  { top: "63%", duration: "0.78s", delay: "0.3s", size: "4.5px", opacity: 0.9 },
  { top: "72%", duration: "1.15s", delay: "0.65s", size: "3px", opacity: 0.7 },
  { top: "81%", duration: "0.82s", delay: "0.1s", size: "4px", opacity: 0.85 },
  { top: "89%", duration: "1.0s", delay: "0.45s", size: "2.5px", opacity: 0.6 },
  { top: "95%", duration: "0.88s", delay: "0.25s", size: "3px", opacity: 0.75 },
];

const SNOWFLAKES = [
  { left: "5%", size: "4px", duration: "4.2s", delay: "0s", swayDuration: "2.5s", opacity: 0.85 },
  { left: "13%", size: "7px", duration: "3.5s", delay: "1.2s", swayDuration: "3.0s", opacity: 0.95 },
  { left: "21%", size: "3px", duration: "5.5s", delay: "0.5s", swayDuration: "2.2s", opacity: 0.7 },
  { left: "29%", size: "5px", duration: "4.0s", delay: "2.1s", swayDuration: "2.8s", opacity: 0.9 },
  { left: "38%", size: "8px", duration: "3.2s", delay: "0.8s", swayDuration: "3.3s", opacity: 0.95 },
  { left: "47%", size: "3.5px", duration: "5.0s", delay: "1.7s", swayDuration: "2.4s", opacity: 0.75 },
  { left: "56%", size: "6px", duration: "3.8s", delay: "0.3s", swayDuration: "2.9s", opacity: 0.9 },
  { left: "64%", size: "4px", duration: "4.6s", delay: "2.5s", swayDuration: "2.6s", opacity: 0.8 },
  { left: "73%", size: "7.5px", duration: "3.4s", delay: "1.0s", swayDuration: "3.1s", opacity: 0.95 },
  { left: "82%", size: "3px", duration: "5.8s", delay: "0.2s", swayDuration: "2.1s", opacity: 0.65 },
  { left: "90%", size: "5.5px", duration: "4.1s", delay: "1.9s", swayDuration: "2.7s", opacity: 0.85 },
  { left: "96%", size: "4px", duration: "4.8s", delay: "0.7s", swayDuration: "2.5s", opacity: 0.8 },
];

const CHILLY_STARS = [
  { left: "10%", top: "12%", size: "2px", delay: "0.2s", duration: "2.2s", opacity: 0.9 },
  { left: "24%", top: "8%", size: "3px", delay: "0.9s", duration: "3.1s", opacity: 1.0, isSparkle: true },
  { left: "38%", top: "16%", size: "1.5px", delay: "1.5s", duration: "2.5s", opacity: 0.75 },
  { left: "52%", top: "9%", size: "2px", delay: "0.4s", duration: "2.8s", opacity: 0.85 },
  { left: "68%", top: "14%", size: "3px", delay: "1.8s", duration: "3.4s", opacity: 1.0, isSparkle: true },
  { left: "82%", top: "10%", size: "1.5px", delay: "0.7s", duration: "2.3s", opacity: 0.8 },
  { left: "92%", top: "18%", size: "2.5px", delay: "1.2s", duration: "2.9s", opacity: 0.9 },
  { left: "16%", top: "28%", size: "1.5px", delay: "2.0s", duration: "2.4s", opacity: 0.7 },
  { left: "32%", top: "32%", size: "2.5px", delay: "0.6s", duration: "3.0s", opacity: 0.95 },
  { left: "46%", top: "26%", size: "1.5px", delay: "1.4s", duration: "2.6s", opacity: 0.75 },
  { left: "62%", top: "34%", size: "2px", delay: "0.1s", duration: "2.7s", opacity: 0.85 },
  { left: "76%", top: "29%", size: "3px", delay: "1.1s", duration: "3.2s", opacity: 1.0, isSparkle: true },
  { left: "88%", top: "35%", size: "1.5px", delay: "2.3s", duration: "2.2s", opacity: 0.7 },
  { left: "8%", top: "45%", size: "2px", delay: "0.8s", duration: "2.8s", opacity: 0.8 },
  { left: "22%", top: "48%", size: "1.5px", delay: "1.7s", duration: "2.5s", opacity: 0.7 },
  { left: "70%", top: "46%", size: "2px", delay: "1.0s", duration: "3.0s", opacity: 0.85 },
  { left: "85%", top: "50%", size: "2.5px", delay: "0.3s", duration: "2.6s", opacity: 0.9 },
];
</script>

<template>
  <div class="mo-weather-effects pointer-events-none absolute inset-0 overflow-hidden z-0 select-none">
    <!-- ========================================== -->
    <!-- 1. SUNLIGHT EFFECT                         -->
    <!-- ========================================== -->
    <template v-if="effect === 'sunlight'">
      <!-- Pulsing Solar Flare / Corona -->
      <div class="sun-corona" />
      <div class="sun-corona-inner" />

      <!-- Sweeping Light Beams -->
      <div class="sun-beam sun-beam-1" />
      <div class="sun-beam sun-beam-2" />
      <div class="sun-beam sun-beam-3" />

      <!-- Floating Solar Dust Motes -->
      <div
        v-for="(mote, i) in DUST_MOTES"
        :key="`mote-${i}`"
        class="sun-mote"
        :style="{
          left: mote.left,
          bottom: mote.bottom,
          width: mote.size,
          height: mote.size,
          animationDelay: mote.delay,
          animationDuration: mote.duration,
        }"
      />

      <!-- Ambient Solar Warmth Highlight -->
      <div class="sun-warmth-glow" />
    </template>

    <!-- ========================================== -->
    <!-- 2. DAY TIME RAIN EFFECT                    -->
    <!-- ========================================== -->
    <template v-else-if="effect === 'day_rain'">
      <!-- Falling Raindrops Layer -->
      <div class="rain-canvas">
        <div
          v-for="(drop, i) in DAY_RAINDROPS"
          :key="`day-rain-${i}`"
          class="rain-streak"
          :style="{
            left: drop.left,
            top: drop.top,
            height: drop.height,
            opacity: drop.opacity,
            animationDuration: drop.duration,
            animationDelay: drop.delay,
          }"
        />
      </div>

      <!-- Water Splash & Mist on the Ground -->
      <div class="rain-mist-overlay" />
    </template>

    <!-- ========================================== -->
    <!-- 3. NIGHT RAIN EFFECT                       -->
    <!-- ========================================== -->
    <template v-else-if="effect === 'night_rain'">
      <!-- Translucent Night Raindrops -->
      <div class="rain-canvas">
        <div
          v-for="(drop, i) in NIGHT_RAINDROPS"
          :key="`night-rain-${i}`"
          class="night-rain-streak"
          :style="{
            left: drop.left,
            top: drop.top,
            height: drop.height,
            opacity: drop.opacity,
            animationDuration: drop.duration,
            animationDelay: drop.delay,
          }"
        />
      </div>

      <!-- Distant Soft Sheet Lightning Flash (every ~12s) -->
      <div class="night-distant-flash" />

      <!-- Nocturnal Wet Ground Reflection Glow -->
      <div class="night-rain-mist" />
    </template>

    <!-- ========================================== -->
    <!-- 4. CLOUD EFFECT                            -->
    <!-- ========================================== -->
    <template v-else-if="effect === 'cloud'">
      <!-- Drifting Cloud Mass 1 (High, Slower) -->
      <div class="cloud-mass cloud-mass-1">
        <div class="cloud-puff puff-a" />
        <div class="cloud-puff puff-b" />
        <div class="cloud-puff puff-c" />
      </div>

      <!-- Drifting Cloud Mass 2 (Mid, Medium Speed) -->
      <div class="cloud-mass cloud-mass-2">
        <div class="cloud-puff puff-d" />
        <div class="cloud-puff puff-e" />
      </div>

      <!-- Drifting Cloud Mass 3 (Foreground Soft) -->
      <div class="cloud-mass cloud-mass-3">
        <div class="cloud-puff puff-f" />
        <div class="cloud-puff puff-g" />
      </div>

      <!-- Low Horizon Atmospheric Mist -->
      <div class="cloud-horizon-mist" />
    </template>

    <!-- ========================================== -->
    <!-- 5. SANDSTORM EFFECT                        -->
    <!-- ========================================== -->
    <template v-else-if="effect === 'sandstorm'">
      <!-- Warm Terracotta Dust Vignette -->
      <div class="sand-vignette" />

      <!-- Horizontal Sand Wind Streaks -->
      <div class="sand-streak sand-streak-1" />
      <div class="sand-streak sand-streak-2" />
      <div class="sand-streak sand-streak-3" />

      <!-- High-Speed Flying Dust & Sand Motes -->
      <div
        v-for="(sand, i) in SAND_PARTICLES"
        :key="`sand-${i}`"
        class="sand-particle"
        :style="{
          top: sand.top,
          width: sand.size,
          height: sand.size,
          opacity: sand.opacity,
          animationDuration: sand.duration,
          animationDelay: sand.delay,
        }"
      />

      <!-- Swirling Desert Gust Wave -->
      <div class="sand-gust-wave" />
    </template>

    <!-- ========================================== -->
    <!-- 6. SNOW EFFECT                             -->
    <!-- ========================================== -->
    <template v-else-if="effect === 'snow'">
      <!-- Fluttering Snowflakes -->
      <div
        v-for="(flake, i) in SNOWFLAKES"
        :key="`snow-${i}`"
        class="snowflake-container"
        :style="{
          left: flake.left,
          animationDuration: flake.duration,
          animationDelay: flake.delay,
        }"
      >
        <div
          class="snowflake-dot"
          :style="{
            width: flake.size,
            height: flake.size,
            opacity: flake.opacity,
            animationDuration: flake.swayDuration,
          }"
        />
      </div>

      <!-- Frosty Base Mist -->
      <div class="snow-frost-base" />
    </template>

    <!-- ========================================== -->
    <!-- 7. CHILLY NIGHT EFFECT                     -->
    <!-- ========================================== -->
    <template v-else-if="effect === 'chilly_night'">
      <!-- Twinkling Starfield -->
      <div
        v-for="(star, i) in CHILLY_STARS"
        :key="`star-${i}`"
        class="chilly-star"
        :class="{ 'chilly-sparkle': star.isSparkle }"
        :style="{
          left: star.left,
          top: star.top,
          width: star.size,
          height: star.size,
          opacity: star.opacity,
          animationDuration: star.duration,
          animationDelay: star.delay,
        }"
      />

      <!-- Frost Edge Vignette (Crystalline Border Glow) -->
      <div class="chilly-frost-border" />

      <!-- Ethereal Cold Aurora Ribbon -->
      <div class="chilly-aurora" />
    </template>

    <!-- ========================================== -->
    <!-- 8. THUNDERSTORM EFFECT (Bonus/Existing)    -->
    <!-- ========================================== -->
    <template v-else-if="effect === 'thunderstorm'">
      <div class="thunder-lightning" />
      <div class="rain-canvas">
        <div
          v-for="(drop, i) in DAY_RAINDROPS"
          :key="`thunder-rain-${i}`"
          class="night-rain-streak"
          :style="{
            left: drop.left,
            top: drop.top,
            height: drop.height,
            opacity: drop.opacity,
            animationDuration: drop.duration,
            animationDelay: drop.delay,
          }"
        />
      </div>
    </template>
  </div>
</template>

<style scoped>
/* ======================================================== */
/* 1. SUNLIGHT KEYFRAMES & STYLES                          */
/* ======================================================== */
.sun-corona {
  position: absolute;
  top: -60px;
  right: -50px;
  width: 240px;
  height: 240px;
  border-radius: 9999px;
  background: radial-gradient(circle, rgba(254, 240, 138, 0.42) 0%, rgba(245, 158, 11, 0.22) 42%, transparent 72%);
  animation: sun-pulse 6s ease-in-out infinite alternate;
  will-change: transform, opacity;
}

.sun-corona-inner {
  position: absolute;
  top: -30px;
  right: -20px;
  width: 140px;
  height: 140px;
  border-radius: 9999px;
  background: radial-gradient(circle, rgba(255, 255, 255, 0.5) 0%, rgba(253, 224, 71, 0.3) 50%, transparent 80%);
  animation: sun-pulse-fast 4s ease-in-out infinite alternate;
}

.sun-beam {
  position: absolute;
  top: -40%;
  right: -20%;
  width: 160%;
  height: 160%;
  pointer-events: none;
  transform-origin: top right;
}

.sun-beam-1 {
  background: linear-gradient(135deg, transparent 38%, rgba(254, 240, 138, 0.12) 48%, rgba(255, 255, 255, 0.22) 50%, rgba(254, 240, 138, 0.12) 52%, transparent 62%);
  animation: beam-shift-1 10s ease-in-out infinite alternate;
}

.sun-beam-2 {
  background: linear-gradient(125deg, transparent 42%, rgba(253, 224, 71, 0.1) 49%, rgba(255, 255, 255, 0.18) 51%, rgba(253, 224, 71, 0.1) 53%, transparent 60%);
  animation: beam-shift-2 13s ease-in-out infinite alternate;
}

.sun-beam-3 {
  background: linear-gradient(145deg, transparent 40%, rgba(251, 191, 36, 0.08) 50%, transparent 60%);
  animation: beam-shift-3 8s ease-in-out infinite alternate;
}

.sun-mote {
  position: absolute;
  border-radius: 9999px;
  background-color: #fef08a;
  box-shadow: 0 0 8px #fde047;
  animation: mote-float infinite ease-in-out;
}

.sun-warmth-glow {
  position: absolute;
  inset: 0;
  background: radial-gradient(circle at 80% 20%, rgba(251, 191, 36, 0.18) 0%, transparent 60%);
}

@keyframes sun-pulse {
  0% { transform: scale(0.95); opacity: 0.8; }
  100% { transform: scale(1.08); opacity: 1; }
}

@keyframes sun-pulse-fast {
  0% { transform: scale(0.92); opacity: 0.7; }
  100% { transform: scale(1.05); opacity: 0.95; }
}

@keyframes beam-shift-1 {
  0% { transform: rotate(-10deg) scale(0.95); opacity: 0.7; }
  100% { transform: rotate(-16deg) scale(1.05); opacity: 1; }
}

@keyframes beam-shift-2 {
  0% { transform: rotate(-14deg) scale(1.02); opacity: 0.8; }
  100% { transform: rotate(-8deg) scale(0.96); opacity: 0.6; }
}

@keyframes beam-shift-3 {
  0% { transform: rotate(-18deg); opacity: 0.5; }
  100% { transform: rotate(-12deg); opacity: 0.85; }
}

@keyframes mote-float {
  0% { transform: translate3d(0, 0, 0); opacity: 0; }
  25% { opacity: 0.8; }
  75% { opacity: 0.7; }
  100% { transform: translate3d(24px, -80px, 0); opacity: 0; }
}

/* ======================================================== */
/* 2. DAY & NIGHT RAIN KEYFRAMES & STYLES                   */
/* ======================================================== */
.rain-canvas {
  position: absolute;
  inset: 0;
  overflow: hidden;
  transform: rotate(-12deg) scale(1.15);
}

.rain-streak {
  position: absolute;
  width: 1.5px;
  border-radius: 2px;
  background: linear-gradient(to bottom, transparent, rgba(186, 230, 253, 0.5) 30%, rgba(255, 255, 255, 0.85) 100%);
  animation: rain-fall linear infinite;
  will-change: transform;
}

.night-rain-streak {
  position: absolute;
  width: 1.5px;
  border-radius: 2px;
  background: linear-gradient(to bottom, transparent, rgba(165, 180, 252, 0.35) 30%, rgba(224, 231, 255, 0.75) 100%);
  animation: rain-fall linear infinite;
  will-change: transform;
}

.rain-mist-overlay {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 50px;
  background: linear-gradient(to top, rgba(186, 230, 253, 0.18), transparent);
}

.night-rain-mist {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 60px;
  background: linear-gradient(to top, rgba(30, 27, 75, 0.4), transparent);
}

.night-distant-flash {
  position: absolute;
  inset: 0;
  background: radial-gradient(circle at 65% 30%, rgba(199, 210, 254, 0.35) 0%, transparent 75%);
  opacity: 0;
  animation: night-flash 12s ease-in-out infinite;
}

@keyframes rain-fall {
  0% { transform: translate3d(0, -60px, 0); }
  100% { transform: translate3d(0, 480px, 0); }
}

@keyframes night-flash {
  0%, 88%, 93%, 100% { opacity: 0; }
  89% { opacity: 0.9; }
  89.6% { opacity: 0.1; }
  90.4% { opacity: 0.65; }
  91.2% { opacity: 0.15; }
}

/* ======================================================== */
/* 3. CLOUDS KEYFRAMES & STYLES                            */
/* ======================================================== */
.cloud-mass {
  position: absolute;
  display: flex;
  opacity: 0.45;
  filter: blur(12px);
  will-change: transform;
}

.cloud-mass-1 {
  top: 15%;
  animation: cloud-drift-1 28s linear infinite;
}

.cloud-mass-2 {
  top: 35%;
  animation: cloud-drift-2 22s linear infinite;
  opacity: 0.35;
}

.cloud-mass-3 {
  top: 55%;
  animation: cloud-drift-3 18s linear infinite;
  opacity: 0.25;
}

.cloud-puff {
  border-radius: 9999px;
  background-color: #ffffff;
}

.puff-a { width: 140px; height: 70px; }
.puff-b { width: 100px; height: 60px; margin-left: -35px; margin-top: -15px; }
.puff-c { width: 80px; height: 50px; margin-left: -20px; }
.puff-d { width: 180px; height: 80px; }
.puff-e { width: 120px; height: 70px; margin-left: -45px; }
.puff-f { width: 130px; height: 65px; }
.puff-g { width: 90px; height: 55px; margin-left: -30px; }

.cloud-horizon-mist {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 80px;
  background: linear-gradient(to top, rgba(255, 255, 255, 0.14), transparent);
}

@keyframes cloud-drift-1 {
  0% { transform: translate3d(-120%, 0, 0); }
  100% { transform: translate3d(240%, 0, 0); }
}

@keyframes cloud-drift-2 {
  0% { transform: translate3d(220%, 0, 0); }
  100% { transform: translate3d(-140%, 0, 0); }
}

@keyframes cloud-drift-3 {
  0% { transform: translate3d(-100%, 0, 0); }
  100% { transform: translate3d(250%, 0, 0); }
}

/* ======================================================== */
/* 4. SANDSTORM KEYFRAMES & STYLES                         */
/* ======================================================== */
.sand-vignette {
  position: absolute;
  inset: 0;
  background: radial-gradient(ellipse at 50% 50%, rgba(217, 119, 6, 0.22) 0%, rgba(180, 83, 9, 0.45) 100%);
}

.sand-streak {
  position: absolute;
  height: 2px;
  border-radius: 1px;
  background: linear-gradient(to right, transparent, rgba(251, 191, 36, 0.65) 40%, rgba(245, 158, 11, 0.9) 80%, transparent 100%);
  animation: sand-sweep linear infinite;
  transform: rotate(-4deg);
}

.sand-streak-1 { top: 22%; width: 140px; animation-duration: 0.65s; animation-delay: 0.1s; }
.sand-streak-2 { top: 48%; width: 190px; animation-duration: 0.55s; animation-delay: 0.3s; }
.sand-streak-3 { top: 76%; width: 160px; animation-duration: 0.6s; animation-delay: 0.15s; }

.sand-particle {
  position: absolute;
  left: -20px;
  border-radius: 9999px;
  background-color: #fbbf24;
  box-shadow: 0 0 6px #d97706;
  animation: sand-fly linear infinite;
}

.sand-gust-wave {
  position: absolute;
  inset: 0;
  background: linear-gradient(105deg, transparent 30%, rgba(245, 158, 11, 0.18) 50%, transparent 70%);
  animation: sand-gust 4s ease-in-out infinite;
}

@keyframes sand-sweep {
  0% { transform: translate3d(-180px, 0, 0) rotate(-4deg); }
  100% { transform: translate3d(520px, 20px, 0) rotate(-4deg); }
}

@keyframes sand-fly {
  0% { transform: translate3d(0, 0, 0); }
  100% { transform: translate3d(560px, 40px, 0); }
}

@keyframes sand-gust {
  0%, 100% { opacity: 0.3; transform: scale(1); }
  50% { opacity: 0.8; transform: scale(1.04); }
}

/* ======================================================== */
/* 5. SNOW KEYFRAMES & STYLES                              */
/* ======================================================== */
.snowflake-container {
  position: absolute;
  top: -20px;
  animation: snow-drop linear infinite;
  will-change: transform;
}

.snowflake-dot {
  border-radius: 9999px;
  background-color: #ffffff;
  box-shadow: 0 0 5px rgba(255, 255, 255, 0.85);
  animation: snow-sway ease-in-out infinite alternate;
}

.snow-frost-base {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 45px;
  background: linear-gradient(to top, rgba(224, 242, 254, 0.22), transparent);
}

@keyframes snow-drop {
  0% { transform: translate3d(0, 0, 0); }
  100% { transform: translate3d(0, 480px, 0); }
}

@keyframes snow-sway {
  0% { transform: translate3d(-12px, 0, 0); }
  100% { transform: translate3d(12px, 0, 0); }
}

/* ======================================================== */
/* 6. CHILLY NIGHT KEYFRAMES & STYLES                      */
/* ======================================================== */
.chilly-star {
  position: absolute;
  border-radius: 9999px;
  background-color: #ffffff;
  box-shadow: 0 0 4px #e0e7ff;
  animation: star-flicker ease-in-out infinite alternate;
}

.chilly-sparkle {
  box-shadow: 0 0 7px #93c5fd, 0 0 12px #bfdbfe;
}

.chilly-frost-border {
  position: absolute;
  inset: 0;
  box-shadow: inset 0 0 42px rgba(186, 230, 253, 0.2);
  pointer-events: none;
}

.chilly-aurora {
  position: absolute;
  top: -20%;
  left: -20%;
  width: 140%;
  height: 70%;
  border-radius: 9999px;
  background: linear-gradient(110deg, transparent 20%, rgba(56, 189, 248, 0.12) 40%, rgba(99, 102, 241, 0.16) 60%, rgba(168, 85, 247, 0.12) 80%, transparent 100%);
  filter: blur(28px);
  animation: aurora-drift 14s ease-in-out infinite alternate;
}

@keyframes star-flicker {
  0% { transform: scale(0.7); opacity: 0.35; }
  100% { transform: scale(1.2); opacity: 1; }
}

@keyframes aurora-drift {
  0% { transform: translate3d(-20px, 0, 0) scale(0.95); opacity: 0.6; }
  100% { transform: translate3d(30px, 15px, 0) scale(1.08); opacity: 0.95; }
}

/* ======================================================== */
/* 7. THUNDERSTORM KEYFRAMES                                */
/* ======================================================== */
.thunder-lightning {
  position: absolute;
  inset: 0;
  background: radial-gradient(circle at 50% 30%, rgba(245, 208, 254, 0.55) 0%, transparent 80%);
  opacity: 0;
  animation: violent-lightning 8s ease-in-out infinite;
}

@keyframes violent-lightning {
  0%, 91%, 95%, 100% { opacity: 0; }
  92% { opacity: 0.95; }
  92.5% { opacity: 0.1; }
  93.2% { opacity: 0.8; }
  94% { opacity: 0.05; }
}
</style>

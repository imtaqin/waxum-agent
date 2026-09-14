<script setup lang="ts">
import { computed } from "vue";
import { motion } from "motion-v";

const props = defineProps<{
  state: "offline" | "idle" | "listening" | "thinking" | "speaking";
}>();

const COLORS: Record<typeof props.state, string> = {
  offline: "#3a4a56",
  idle: "#22d3ee",
  listening: "#ff2fb4",
  thinking: "#ffb020",
  speaking: "#5eead4",
};
const color = computed(() => COLORS[props.state]);

const ringSpeed = computed(() => {
  if (props.state === "listening") return 2.2;
  if (props.state === "thinking") return 1.2;
  if (props.state === "speaking") return 3;
  return 6;
});

// 48 tick marks around the dial, generated once — matches the reference
// HUD's radial gauge rim rather than a plain CSS circle.
const ticks = Array.from({ length: 48 }, (_, i) => {
  const angle = (i / 48) * 360;
  const long = i % 4 === 0;
  return { angle, long };
});
</script>

<template>
  <div class="relative w-full aspect-square max-w-[220px] mx-auto select-none">
    <svg viewBox="0 0 200 200" class="w-full h-full overflow-visible">
      <!-- outer tick dial -->
      <g :stroke="color" :style="{ opacity: state === 'offline' ? 0.25 : 0.55 }">
        <line
          v-for="(t, i) in ticks"
          :key="i"
          :transform="`rotate(${t.angle} 100 100)`"
          x1="100"
          :y1="t.long ? 6 : 10"
          x2="100"
          y2="14"
          :stroke-width="t.long ? 1.5 : 0.75" />
      </g>

      <!-- rotating rings -->
      <motion.circle
        cx="100"
        cy="100"
        r="78"
        fill="none"
        :stroke="color"
        stroke-width="1"
        stroke-dasharray="4 10"
        :style="{ opacity: 0.6 }"
        :animate="{ rotate: 360 }"
        :transition="{ duration: ringSpeed * 2.4, repeat: Infinity, ease: 'linear' }"
        style="transform-origin: 100px 100px" />
      <motion.circle
        cx="100"
        cy="100"
        r="64"
        fill="none"
        :stroke="color"
        stroke-width="1.5"
        stroke-dasharray="1 6"
        :style="{ opacity: 0.7 }"
        :animate="{ rotate: -360 }"
        :transition="{ duration: ringSpeed * 1.6, repeat: Infinity, ease: 'linear' }"
        style="transform-origin: 100px 100px" />
      <motion.circle
        cx="100"
        cy="100"
        r="50"
        fill="none"
        :stroke="color"
        stroke-width="2"
        :stroke-dasharray="state === 'offline' ? '4 4' : '40 200'"
        :animate="{ rotate: 360 }"
        :transition="{ duration: ringSpeed, repeat: Infinity, ease: 'linear' }"
        style="transform-origin: 100px 100px" />

      <!-- core glow -->
      <motion.circle
        cx="100"
        cy="100"
        r="34"
        :fill="color"
        :animate="{
          opacity: state === 'offline' ? [0.08, 0.08] : [0.18, 0.32, 0.18],
          scale: state === 'listening' || state === 'speaking' ? [1, 1.08, 1] : [1, 1.02, 1],
        }"
        :transition="{ duration: state === 'listening' ? 0.8 : 2.4, repeat: Infinity, ease: 'easeInOut' }"
        style="transform-origin: 100px 100px" />
      <circle cx="100" cy="100" r="20" :fill="color" :style="{ opacity: state === 'offline' ? 0.3 : 0.9 }" />
    </svg>

    <div class="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
      <span
        class="text-[9px] uppercase tracking-[0.3em] font-semibold"
        :style="{ color, textShadow: `0 0 10px ${color}` }">
        {{ state }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { motion } from "motion-v";

withDefaults(defineProps<{ active?: boolean }>(), { active: true });
</script>

<template>
  <div class="relative w-7 h-7 shrink-0">
    <motion.div
      class="absolute inset-0 rounded-full border border-dashed border-hud-500/50"
      :animate="active ? { rotate: 360 } : { rotate: 0 }"
      :transition="active ? { duration: 6, repeat: Infinity, ease: 'linear' } : {}" />
    <motion.div
      class="absolute inset-[3px] rounded-full border border-hud-500/30"
      :animate="active ? { rotate: -360 } : { rotate: 0 }"
      :transition="active ? { duration: 4, repeat: Infinity, ease: 'linear' } : {}" />
    <div
      class="absolute inset-0 rounded-full border border-hud-500/50 flex items-center justify-center text-hud-500 text-xs font-bold hud-glow-text"
      :class="active ? 'hud-pulse' : ''">
      W
    </div>
  </div>
</template>

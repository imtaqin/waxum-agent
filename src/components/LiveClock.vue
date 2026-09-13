<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";

const now = ref(new Date());
let timer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  timer = setInterval(() => (now.value = new Date()), 1000);
});
onUnmounted(() => {
  if (timer) clearInterval(timer);
});

function pad(n: number) {
  return n.toString().padStart(2, "0");
}
</script>

<template>
  <div class="text-right leading-tight select-none">
    <div class="text-xs font-semibold text-hud-400 hud-glow-text tabular-nums">
      {{ pad(now.getHours()) }}:{{ pad(now.getMinutes()) }}:{{ pad(now.getSeconds()) }}
    </div>
    <div class="text-[9px] uppercase tracking-widest text-hud-400/40">
      {{ now.toLocaleDateString(undefined, { day: "2-digit", month: "short" }) }}
    </div>
  </div>
</template>

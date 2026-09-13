<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import QRCode from "qrcode";
import { motion } from "motion-v";
import { onPairingEvent, startPairing, stopPairing } from "../lib/waxum";

const props = defineProps<{ baseUrl: string; token: string; sessionId: string }>();
const emit = defineEmits<{ close: []; paired: [] }>();

const qrDataUrl = ref<string | null>(null);
const pairCode = ref<string | null>(null);
const statusText = ref("menginisialisasi…");
const failed = ref(false);
let unlisten: (() => void) | null = null;

async function renderQr(code: string) {
  qrDataUrl.value = await QRCode.toDataURL(code, { margin: 1, width: 240 });
}

onMounted(async () => {
  unlisten = await onPairingEvent(async (e) => {
    switch (e.event) {
      case "qr_code":
        statusText.value = "pindai kode ini di WhatsApp > Perangkat Tertaut";
        pairCode.value = null;
        await renderQr(e.data.code);
        break;
      case "pair_code":
        statusText.value = "masukkan kode ini di WhatsApp > Perangkat Tertaut";
        pairCode.value = e.data.code;
        qrDataUrl.value = null;
        break;
      case "connected":
        statusText.value = "tertaut — menyinkronkan data…";
        break;
      case "ready":
        statusText.value = "modul aktif";
        emit("paired");
        break;
      case "error":
        statusText.value = `pairing failed: ${e.data.message ?? e.data.reason ?? "unknown error"}`;
        failed.value = true;
        break;
      case "timeout":
        statusText.value = "timed out waiting for a scan — try again";
        failed.value = true;
        break;
    }
  });
  await startPairing({ baseUrl: props.baseUrl, token: props.token }, props.sessionId).catch((err) => {
    statusText.value = `couldn't start pairing: ${err}`;
    failed.value = true;
  });
});

onUnmounted(() => {
  unlisten?.();
  stopPairing().catch(() => {});
});

function close() {
  emit("close");
}
</script>

<template>
  <motion.div
    :initial="{ opacity: 0 }"
    :animate="{ opacity: 1 }"
    :exit="{ opacity: 0 }"
    class="fixed inset-0 bg-black/80 flex items-center justify-center z-50 p-4">
    <motion.div
      :initial="{ opacity: 0, scale: 0.9, y: 16 }"
      :animate="{ opacity: 1, scale: 1, y: 0 }"
      :exit="{ opacity: 0, scale: 0.9, y: 16 }"
      :transition="{ type: 'spring', stiffness: 260, damping: 22 }"
      class="card w-full max-w-xs p-6 flex flex-col items-center gap-4">
      <motion.div
        v-if="qrDataUrl"
        :initial="{ opacity: 0, scale: 0.9 }"
        :animate="{ opacity: 1, scale: 1 }"
        class="bg-white p-2 rounded-lg shadow-hud-lg">
        <img :src="qrDataUrl" alt="pairing QR" width="200" height="200" />
      </motion.div>
      <div v-else-if="pairCode" class="text-2xl font-mono tracking-widest text-hud-400 hud-glow-text">
        {{ pairCode }}
      </div>
      <div v-else class="w-[200px] h-[200px] flex items-center justify-center text-hud-400/30 text-xs uppercase tracking-wide">
        menunggu kode…
      </div>

      <p class="text-xs text-center" :class="failed ? 'text-cyber-pink' : 'text-hud-400/60'">
        {{ statusText }}
      </p>

      <button class="btn-ghost w-full" @click="close">Cancel</button>
    </motion.div>
  </motion.div>
</template>

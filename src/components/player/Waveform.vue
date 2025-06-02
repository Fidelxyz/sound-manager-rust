<script setup lang="ts">
import { Channel } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onMounted, onUnmounted, watch } from "vue";

import { $dt } from "@primeuix/themes";
import WaveSurfer from "wavesurfer.js";
import Hover from "wavesurfer.js/dist/plugins/hover.esm.js";
import Timer from "wavesurfer.js/dist/timer.js";

import type { Entry, PlayerState } from "@/api";
import { api } from "@/api";
import { PlaybackTimer } from "@/utils/playbackTimer";

const { entry } = defineProps<{
  entry: Entry | null;
}>();
const emit = defineEmits<{
  seek: [pos: number];
}>();

let wavesurfer: WaveSurfer;
let waveformData: Float32Array;
let waveformLength = 0;
let waveformChannel: Channel<ArrayBuffer> | null = null;

let timer: Timer;
const playback_timer = new PlaybackTimer();

onMounted(() => {
  wavesurfer = WaveSurfer.create({
    container: "#waveform",
    waveColor: $dt("surface.100").value.dark.value,
    progressColor: $dt("surface.300").value.dark.value,
    height: 128,
    dragToSeek: true,
    plugins: [
      Hover.create({
        lineColor: "#ff0000",
        lineWidth: 1,
        labelBackground: "#555",
        labelColor: "#fff",
        labelSize: "11px",
      }),
    ],
    peaks: [[0]],
    duration: 1,
  });
  wavesurfer.on("interaction", (pos) => {
    console.debug("interaction", pos);
    emit("seek", pos);
  });

  timer = new Timer();
  timer.on("tick", () => {
    if (!wavesurfer.isSeeking()) {
      wavesurfer.setTime(playback_timer.getCurrentPos());
    }
  });
});

onUnmounted(() => {
  timer.destroy();
  wavesurfer.destroy();
});

async function requestWaveform() {
  console.debug("requestWaveform");

  const waveformDataLength = await api.prepareWaveform();
  waveformData = new Float32Array(waveformDataLength);
  waveformLength = 0;

  console.debug("waveform data length", waveformDataLength);

  // Clear the previous channel
  if (waveformChannel) waveformChannel.onmessage = () => {};
  // Create a new channel
  waveformChannel = new Channel<ArrayBuffer>();
  waveformChannel.onmessage = onReceiveWaveformData;
  api.requestWaveform(waveformChannel);
}

function onReceiveWaveformData(srcData: ArrayBuffer) {
  // console.debug("receive waveform", srcData.byteLength);

  const float32array = new Float32Array(srcData);

  // console.debug(
  //   "waveform length %d / %d",
  //   waveformLength + float32array.length,
  //   waveformData.length
  // );

  waveformData.set(float32array, waveformLength);
  waveformLength += float32array.length;

  wavesurfer.load("", [waveformData], entry?.duration || 0);
}

function clearWaveform() {
  console.debug("clearWaveform");
  if (waveformChannel) {
    waveformChannel.onmessage = () => {};
    waveformChannel = null;
  }
  wavesurfer.load("", [[0]], 1);
}

listen<PlayerState>("player_state_updated", (event) => {
  wavesurfer.setTime(event.payload.pos);
  if (event.payload.playing) {
    playback_timer.start(event.payload.pos);
    timer.stop();
    timer.start();
  } else {
    playback_timer.pause();
    playback_timer.setPos(event.payload.pos);
    timer.stop();
  }
});

watch(
  () => entry,
  (entry) => {
    if (entry) {
      requestWaveform();
    } else {
      clearWaveform();
    }
  },
);
</script>

<template>
  <div id="waveform" class="my-4"></div>
</template>

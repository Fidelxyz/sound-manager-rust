<script setup lang="ts">
import { onMounted, onUnmounted, watch } from "vue";
import { invoke, Channel } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { $dt } from "@primeuix/themes";
import WaveSurfer from "wavesurfer.js";
import Timer from "wavesurfer.js/dist/timer.js";
import Hover from "wavesurfer.js/dist/plugins/hover.esm.js";

import { Entry, PlayerState } from "../types";
import { PlaybackTimer } from "../lib/playback_timer";

const props = defineProps<{
  entry?: Entry;
}>();
const emit = defineEmits<{
  seek: [pos: number];
}>();

let wavesurfer: WaveSurfer;
let waveformData: Float32Array;
let waveformLength = 0;
let waveformChannel: Channel<ArrayBuffer> | undefined;

let timer: Timer;
let playback_timer = new PlaybackTimer();

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
    peaks: [],
    duration: 0,
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

  const waveformDataLength = await invoke<number>("prepare_waveform");
  waveformData = new Float32Array(waveformDataLength);
  waveformLength = 0;

  console.debug("waveform data length", waveformDataLength);

  // Clear the previous channel
  if (waveformChannel) waveformChannel.onmessage = () => {};
  // Create a new channel
  waveformChannel = new Channel<ArrayBuffer>();
  waveformChannel.onmessage = onReceiveWaveformData;
  invoke<number>("request_waveform", {
    channel: waveformChannel,
  });
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

  wavesurfer.load("", [waveformData], props.entry?.duration || 0);
}

listen<PlayerState>("player_state_updated", (event) => {
  console.debug("player_state_updated", event.payload);
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
  () => props.entry,
  (entry) => {
    if (entry) {
      requestWaveform();
    }
  },
  { deep: true }
);
</script>

<template>
  <div id="waveform" class="my-4"></div>
</template>

<script setup lang="ts">
import { ref, onUnmounted, watch } from "vue";
import { listen } from "@tauri-apps/api/event";

import { Button, Slider, ToggleSwitch } from "primevue";
import Waveform from "./Waveform.vue";

import type { Entry, PlayerState } from "../api";
import { api } from "../api";
import { error } from "../utils/message";

const props = defineProps<{
  entry?: Entry;
}>();

const activeEntry = ref<Entry>();

// options
const autoPlay = ref(true);
const skipSilence = ref(true);
const volume = ref(50);

// states
const playing = ref(false);
let playingPos = 0;
let seeking = false;

watch(
  () => props.entry,

  // on entry changed
  async (entry) => {
    if (!entry) return;

    pause();
    await api.setPlayerSource(entry.id).catch((e) => {
      error("设置播放源失败", e.message);
      console.error(e);
    });
    activeEntry.value = { ...entry };
    playingPos = 0;
    if (autoPlay.value) play();
  }
);

onUnmounted(() => {
  pause();
});

async function play() {
  console.debug("play");
  setVolume(volume.value);
  await api
    .play(playingPos, skipSilence.value)
    .then(() => {
      playing.value = true;
    })
    .catch((e) => {
      error("播放失败", e.message);
      console.error(e);
    });
}

function pause() {
  console.debug("pause");
  api.pause();
  playing.value = false;
}

async function seek(time: number) {
  if (seeking) return;
  console.debug("seek", time);
  seeking = true;
  playingPos = time;
  if (playing.value) {
    await play();
  }
  seeking = false;
}

function syncPlayingPos() {
  console.debug("updatePlayingPos");
  api
    .getPlayingPos()
    .then((pos) => {
      playingPos = pos;
    })
    .catch((e) => {
      error("获取播放进度失败", e.message);
      console.error(e);
    });
}

function togglePlayPause() {
  console.debug("togglePlayPause");
  if (!activeEntry.value?.path) return;
  if (playing.value) {
    pause();
    syncPlayingPos();
  } else {
    play();
  }
}

function setVolume(value: number) {
  console.debug("setVolume", value);
  api.setVolume(value / 100).catch((e) => {
    error("设置音量失败", e.message);
    console.error(e);
  });
}

listen<PlayerState>("player_state_updated", (event) => {
  console.debug("player_state_updated", event.payload);
  playing.value = event.payload.playing;
  playingPos = event.payload.pos;
});
</script>

<template>
  <div class="px-8 py-4">
    <Waveform :entry="activeEntry" @seek="seek" />

    <div class="flex">
      <div class="flex flex-1 align-center justify-start items-center gap-4">
        <label for="auto-play">自动播放</label>
        <ToggleSwitch v-model="autoPlay" inputId="auto-play"></ToggleSwitch>
        <label for="auto-play">跳过无声</label>
        <ToggleSwitch
          v-model="skipSilence"
          inputId="skip-silence"
        ></ToggleSwitch>
      </div>

      <div class="flex flex-1 align-center justify-center">
        <Button
          :icon="playing ? 'pi pi-pause' : 'pi pi-play'"
          rounded
          text
          aria-label="播放/暂停"
          @click="togglePlayPause"
        />
      </div>

      <div class="flex flex-1 align-center justify-end">
        <div class="flex items-center gap-4">
          <i class="pi pi-volume-up" />
          <Slider class="w-48" v-model="volume" @change="setVolume" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped></style>

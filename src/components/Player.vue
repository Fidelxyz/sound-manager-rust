<script setup lang="ts">
import { ref, onUnmounted, watch, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { Button, Slider, ToggleSwitch } from "primevue";

import Waveform from "./Waveform.vue";
import { Entry, PlayerState } from "../types";

const props = defineProps<{
  entry?: Entry;
}>();

const activeEntry = ref<Entry>();

// options
const autoPlay = ref(true);
const skipSilence = ref(true);
const volume = ref(50);

// states
let playing = ref(false);
let playingPos = 0;
let seeking = false;

watch(
  () => props.entry,

  // on entry changed
  async (entry) => {
    if (!entry) return;

    pause();
    await invoke("set_player_source", {
      entryId: entry.id,
    }).catch(console.error);
    activeEntry.value = { ...entry };
    playingPos = 0;
    if (autoPlay.value) play();
  },

  { deep: true }
);

onMounted(() => {
  setVolume(volume.value);
});

onUnmounted(() => {
  pause();
});

async function play() {
  console.debug("play");
  await invoke("play", {
    seek: playingPos,
    skipSilence: skipSilence.value,
  })
    .then(() => {
      playing.value = true;
    })
    .catch(console.error);
}

function pause() {
  console.debug("pause");
  invoke("pause");
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
  invoke<number>("get_playing_pos")
    .then((pos) => {
      playingPos = pos;
    })
    .catch(console.error);
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
  invoke("set_volume", {
    volume: value / 100,
  }).catch(console.error);
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

    <div class="player-control flex">
      <div class="flex flex-1 align-center justify-start item-center gap-4">
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

<script setup lang="ts">
import { listen } from "@tauri-apps/api/event";
import { Button, Slider, ToggleSwitch } from "primevue";
import { onUnmounted, ref, useTemplateRef, watch } from "vue";
import { api, type PlayerState } from "@/api";
import { useConfig } from "@/config";
import type { Entry } from "@/types";
import { error } from "@/utils/message";
import Spotter from "./Spotter.vue";
import Waveform from "./Waveform.vue";

const STEP_SECONDS = 5;

const spotter = useTemplateRef("spotter");

const { entry } = defineProps<{
  entry: Entry | null;
}>();

defineExpose({
  spotter,
  togglePlayPause,
  stepForward,
  stepBackward,
});

const activeEntry = ref<Entry | null>(null);

// options
const settings = useConfig("player", {
  autoPlay: true,
  skipSilence: true,
  volume: 50,
});

// states
const playing = ref(false);
/** The current playing position in seconds. */
let playingPos = 0;
let seeking = false;

onUnmounted(() => {
  pause();
});

watch(
  () => entry,

  // on entry changed
  async (entry) => {
    if (!entry) {
      stop();
      return;
    }

    pause();
    await api.setPlayerSource(entry.id).catch((e) => {
      error("设置播放源失败", e.message);
      console.error(e);
    });
    // activeEntry needs to be set after setPlayerSource,
    // bacause setPlayerSource should be called before generating waveform
    activeEntry.value = { ...entry };
    playingPos = 0;
    if (settings.value.autoPlay) play();
  },
);

async function play() {
  console.debug("play");
  setVolume(settings.value.volume);
  await api
    .play(settings.value.skipSilence)
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

function stop() {
  console.debug("stop");
  api.stop();
  activeEntry.value = null;
  playing.value = false;
  playingPos = 0;
}

/**
 * @param pos The position in seconds to seek to.
 */
function seek(pos: number) {
  if (seeking) return;
  console.debug("seek", pos);
  playingPos = pos;
  seeking = true;
  api.seek(pos).finally(() => {
    seeking = false;
  });
}

function stepForward() {
  console.debug("stepForward");
  seek(playingPos + STEP_SECONDS);
}

function stepBackward() {
  console.debug("stepBackward");
  seek(Math.max(0, playingPos - STEP_SECONDS));
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
  if (!entry) return;

  console.debug("togglePlayPause");
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
      <div class="align-center flex flex-1 items-center justify-start gap-4">
        <label class="leading-none" for="auto-play">自动播放</label>
        <ToggleSwitch v-model="settings.autoPlay" inputId="auto-play" />
        <label class="leading-none" for="skip-silence">跳过无声</label>
        <ToggleSwitch v-model="settings.skipSilence" inputId="skip-silence" />
      </div>

      <div class="align-center flex flex-1 justify-center">
        <Button
          :icon="playing ? 'pi pi-pause' : 'pi pi-play'"
          rounded
          text
          aria-label="播放/暂停"
          @click="togglePlayPause"
        />
      </div>

      <div class="align-center flex flex-1 justify-end gap-8">
        <div class="flex items-center gap-4">
          <i class="pi pi-volume-up" />
          <Slider class="w-48" v-model="settings.volume" @change="setVolume" />
        </div>

        <Spotter ref="spotter" :entry="entry" @pause="pause" />
      </div>
    </div>
  </div>
</template>

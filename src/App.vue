<script setup lang="ts">
import { ref } from "vue";

import { Splitter, SplitterPanel } from "primevue";

import AudioList from "./components/AudioList.vue";
import MetadataPanel from "./components/MetadataPanel.vue";
import TagList from "./components/TagList.vue";
import FolderList from "./components/FolderList.vue";
import Startup from "./components/Startup.vue";

import type { Entry } from "./types";
import Player from "./components/Player.vue";

const emit = defineEmits(["database-updated"]);

const loaded = ref(false);
const audioListRef = ref();
const tagListRef = ref();
const folderListRef = ref();
const activeEntry = ref<Entry>();

function onDatabaseLoaded() {
  console.log("Database loaded");
  loaded.value = true;
}

function onDatabaseUpdated() {
  console.debug("Database updated");
  audioListRef.value?.refresh();
  folderListRef.value?.refresh();
  tagListRef.value?.refresh();
}

function onEntrySelected(entry: any) {
  activeEntry.value = entry;
}

function onTagSelected(tagId: number) {
  console.debug("Selected tag:", tagId);
}

function onFolderSelected(folderId: number) {
  console.debug("Selected folder:", folderId);
}
</script>

<template>
  <main class="container max-w-none h-screen">
    <div v-if="loaded" class="h-full flex flex-col">
      <div class="flex-1 min-h-0">
        <Splitter class="h-full rounded-none!" :gutterSize="2">
          <SplitterPanel class="min-w-2xs" :size="15">
            <Splitter layout="vertical" class="h-full" :gutterSize="2">
              <SplitterPanel :minSize="20">
                <FolderList ref="folderListRef" @select="onFolderSelected" />
              </SplitterPanel>

              <SplitterPanel :minSize="20">
                <TagList ref="tagListRef" @select="onTagSelected" />
              </SplitterPanel>
            </Splitter>
          </SplitterPanel>

          <SplitterPanel :size="65">
            <AudioList ref="audioListRef" @select="onEntrySelected" />
          </SplitterPanel>

          <SplitterPanel class="min-w-xs" :size="20">
            <MetadataPanel :entry="activeEntry" />
          </SplitterPanel>
        </Splitter>
      </div>
      <div>
        <Player :entry="activeEntry" />
      </div>
    </div>
    <Startup v-else @database-loaded="onDatabaseLoaded" />
  </main>
</template>

<style scoped></style>

<style>
@import "tailwindcss";
@import "tailwindcss-primeui";
@import "primeicons/primeicons.css";

html {
  user-select: none !important;
  font-size: 14px;
  font-weight: 300;
}
</style>

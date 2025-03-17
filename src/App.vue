<script setup lang="ts">
import { ref } from "vue";

import { Splitter, SplitterPanel, Toast } from "primevue";

import AudioList from "./components/AudioList.vue";
import MetadataPanel from "./components/MetadataPanel.vue";
import TagList from "./components/TagList.vue";
import FolderList from "./components/FolderList.vue";
import Startup from "./components/Startup.vue";

import type { Entry, EntryTag, Filter } from "./api";
import { api } from "./api";
import Player from "./components/Player.vue";

const emit = defineEmits(["database-updated"]);

const audioList = ref();
const tagList = ref();
const folderList = ref();
const metadataPanel = ref();

const loaded = ref(false);
const activeEntry = ref<Entry>();
const tags = ref<EntryTag[]>([]);
const filter = ref<Filter>({
  search: "",
  tagIds: [],
  folderPath: "",
});

function onDatabaseLoaded() {
  console.log("Database loaded");
  loadTags();
  loaded.value = true;
}

function onTagsChanged() {
  console.debug("Tags changed");
  loadTags();
  metadataPanel.value?.refresh();
}

function onEntrySelected(entry: Entry) {
  activeEntry.value = entry;
}

function loadTags() {
  console.log("Load tags");
  api
    .getTags()
    .then((data) => {
      console.log(data);
      tags.value = data;
    })
    .catch((e) => {
      e("加载标签失败", e.message);
      console.error(e);
    });
}
</script>

<template>
  <main class="container max-w-none h-screen">
    <Toast />
    <div v-if="loaded" class="h-full flex flex-col">
      <div class="flex-1 min-h-0">
        <Splitter class="h-full rounded-none!" :gutterSize="2">
          <SplitterPanel class="min-w-2xs" :size="15">
            <Splitter layout="vertical" class="h-full" :gutterSize="2">
              <SplitterPanel :minSize="20">
                <FolderList ref="folderList" :filter="filter" />
              </SplitterPanel>

              <SplitterPanel :minSize="20">
                <TagList
                  ref="tagList"
                  :tags="tags"
                  :filter="filter"
                  @tags-changed="onTagsChanged"
                />
              </SplitterPanel>
            </Splitter>
          </SplitterPanel>

          <SplitterPanel :size="65">
            <AudioList
              ref="audioList"
              :filter="filter"
              :tags="tags"
              @select="onEntrySelected"
            />
          </SplitterPanel>

          <SplitterPanel class="min-w-xs" :size="20">
            <MetadataPanel
              ref="metadataPanel"
              :entry="activeEntry"
              :allTags="tags"
            />
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
@import "./assets/variables.css";
@import "./assets/global.css";

html {
  user-select: none !important;
  font-size: 14px;
  font-weight: 300;
}
</style>

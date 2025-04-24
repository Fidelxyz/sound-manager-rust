<script setup lang="ts">
import { ref } from "vue";
import { listen } from "@tauri-apps/api/event";

import { Splitter, SplitterPanel, Toast } from "primevue";
import type { TreeNode } from "primevue/treenode";

import AudioList from "./components/audio-list/AudioList.vue";
import MetadataPanel from "./components/metadata-panel/MetadataPanel.vue";
import TagList from "./components/tag-list/TagList.vue";
import FolderList from "./components/folder-list/FolderList.vue";
import Startup from "./components/Startup.vue";
import Player from "./components/player/Player.vue";

import type { Entry, TagNode, Filter, Folder } from "@/api";
import { api } from "@/api";
import { error } from "@/utils/message";

const emit = defineEmits(["database-updated"]);

const audioList = ref();
const tagList = ref();
const folderList = ref();
const metadataPanel = ref();

// data
const entries = ref<Entry[]>([]);
const folder = ref<Folder>();
const tags = ref<TreeNode[]>([]);

// state
const loaded = ref(false);
const activeEntry = ref<Entry>();
const filter = ref<Filter>({
  search: "",
  tagIds: [],
  folderPath: "",
});

function onDatabaseLoaded() {
  console.log("Database loaded");
  loadEntries();
  loadFolders();
  loadTags();
  loaded.value = true;
}

function onTagsChanged() {
  console.debug("Tags changed");
  loadTags();
  metadataPanel.value?.refresh();
}

function onFilesChanged() {
  console.debug("Files changed");
  loadEntries();
  loadFolders();
  metadataPanel.value?.refresh();
}

function loadEntries() {
  console.log("Load entries");
  api
    .getEntries()
    .then((data) => {
      console.log(data);
      entries.value = data;
    })
    .catch((e) => {
      error("加载文件失败", e.message);
      console.error(e);
    });
}

async function loadFolders() {
  console.log("Load folders");
  api
    .getFolder()
    .then((data) => {
      console.log(data);
      folder.value = data;
    })
    .catch((e) => {
      error("加载文件夹失败", e.message);
      console.error(e);
    });
}

function loadTags() {
  console.log("Load tags");
  api
    .getTags()
    .then((tagNodes) => {
      console.log(tagNodes);
      tags.value = toTagTree(tagNodes);
    })
    .catch((e) => {
      e("加载标签失败", e.message);
      console.error(e);
    });
}

function toTagTree(tags: TagNode[]): TreeNode[] {
  return tags.map((tagNode): TreeNode => {
    return {
      key: tagNode.tag.id.toString(),
      label: tagNode.tag.name,
      data: tagNode.tag,
      icon: `pi pi-tag tag-color-${tagNode.tag.color}`,
      children: toTagTree(tagNode.children),
    };
  });
}

listen("files_updated", onFilesChanged);
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
                <FolderList
                  ref="folderList"
                  :folder="folder"
                  v-model:filter="filter"
                />
              </SplitterPanel>

              <SplitterPanel :minSize="20">
                <TagList
                  ref="tagList"
                  :tags="tags"
                  v-model:filter="filter"
                  @tags-changed="onTagsChanged"
                />
              </SplitterPanel>
            </Splitter>
          </SplitterPanel>

          <SplitterPanel :size="65">
            <AudioList
              ref="audioList"
              :entries="entries"
              :tags="tags"
              v-model:filter="filter"
              v-model:activeEntry="activeEntry"
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

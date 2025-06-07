<script setup lang="ts">
import { listen } from "@tauri-apps/api/event";
import { onMounted, ref, useTemplateRef } from "vue";

import { Splitter, SplitterPanel } from "primevue";
import type { TreeNode } from "primevue/treenode";

import AudioList from "./audio-list/AudioList.vue";
import FolderList from "./folder-list/FolderList.vue";
import MetadataPanel from "./metadata-panel/MetadataPanel.vue";
import Player from "./player/Player.vue";
import TagList from "./tag-list/TagList.vue";

import type { Entry, Filter, FolderNode, TagNode } from "@/api";
import { api } from "@/api";
import { error } from "@/utils/message";

const metadataPanel = useTemplateRef("metadataPanel");
const audioList = useTemplateRef("audioList");

// data
const entries = ref<Entry[]>([]);
const folder = ref<FolderNode | null>(null);
const tags = ref<TreeNode[]>([]);

// state
const activeEntry = ref<Entry | null>(null);
const filter = ref<Filter>({
  search: "",
  tagIds: [],
  folderId: null,
});

defineExpose({
  audioList,
  activeEntry,
});

onMounted(() => {
  loadEntries();
  loadFolders();
  loadTags();
});

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
  console.info("Loading entries");
  api
    .getEntries()
    .then((data) => {
      console.debug("Entries", data);
      entries.value = data;
    })
    .catch((e) => {
      error("加载文件失败", e.message);
      console.error(e);
    });
}

async function loadFolders() {
  console.info("Loading folders");
  api
    .getFolder()
    .then((data) => {
      console.debug("Folder", data);
      folder.value = data;
    })
    .catch((e) => {
      error("加载文件夹失败", e.message);
      console.error(e);
    });
}

function loadTags() {
  console.info("Loading tags");
  api
    .getTags()
    .then((tagNodes) => {
      console.debug("Tags", tagNodes);
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
  <div class="flex h-full flex-col">
    <div class="min-h-0 flex-auto">
      <Splitter class="h-full rounded-none!" :gutterSize="2">
        <SplitterPanel class="min-w-2xs" :size="15">
          <Splitter layout="vertical" class="h-full" :gutterSize="2">
            <SplitterPanel :minSize="20">
              <FolderList :folder="folder" v-model:filter="filter" />
            </SplitterPanel>

            <SplitterPanel :minSize="20">
              <TagList
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
</template>

<script setup lang="ts">
import { ref } from "vue";

import FolderItem from "./FolderItem.vue";

import type { Filter, Folder, FolderNode } from "@/api";

const filter = defineModel<Filter>("filter", { required: true });

const { folder } = defineProps<{
  folder: FolderNode | null;
}>();

const selectedFolder = ref<Folder | null>(null);

function selectFolder(folder: Folder) {
  if (selectedFolder.value === folder) {
    console.info("Unselected folder:", folder.name);
    selectedFolder.value = null;
    filter.value.folderId = null;
    return;
  }
  console.info("Selected folder:", folder.name);
  selectedFolder.value = folder;
  filter.value.folderId = folder.id;
}
</script>

<template>
  <div class="bg-surface-800 flex h-full w-full flex-col px-8 pt-8">
    <div class="p-2 font-bold">文件夹</div>
    <ul class="flex-auto overflow-auto">
      <FolderItem
        v-if="folder"
        :folderNode="folder"
        :selectedFolder="selectedFolder"
        @select="selectFolder"
      />
    </ul>
  </div>
</template>

<style scoped></style>

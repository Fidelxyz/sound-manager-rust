<script setup lang="ts">
import { ref } from "vue";

import FolderItem from "./FolderItem.vue";

import type { Filter, Folder, FolderNode } from "@/api";

const filter = defineModel<Filter>("filter", { required: true });

const { folder } = defineProps<{
  folder: FolderNode | null;
}>();

const selectedFolder = ref<Folder>();

function selectFolder(folder: Folder) {
  if (selectedFolder.value === folder) {
    console.log("Unselect folder:", folder.name);
    selectedFolder.value = undefined;
    filter.value.folderId = null;
    return;
  }
  console.log("Select folder:", folder.name);
  selectedFolder.value = folder;
  filter.value.folderId = folder.id;
}
</script>

<template>
  <div class="w-full h-full p-8 bg-surface-800">
    <div class="font-bold p-2">文件夹</div>
    <ul>
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

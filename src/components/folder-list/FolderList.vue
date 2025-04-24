<script setup lang="ts">
import { ref } from "vue";

import FolderItem from "./FolderItem.vue";

import type { Folder, Filter } from "@/api";

const filter = defineModel<Filter>("filter", { required: true });

const { folder } = defineProps<{
  folder?: Folder;
}>();

const selectedFolder = ref<Folder>();

function selectFolder(folder: Folder) {
  if (selectedFolder.value === folder) {
    console.log("Unselect folder:", folder.name);
    selectedFolder.value = undefined;
    filter.value.folderPath = "";
    return;
  }
  console.log("Select folder:", folder.name);
  selectedFolder.value = folder;
  filter.value.folderPath = folder.path;
}
</script>

<template>
  <div class="w-full h-full p-8 bg-surface-800">
    <div class="font-bold p-2">文件夹</div>
    <ul>
      <FolderItem
        v-if="folder"
        :key="folder.path"
        :folder="folder"
        :selected-path="selectedFolder?.path"
        @select="selectFolder"
      />
    </ul>
  </div>
</template>

<style scoped></style>

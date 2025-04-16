<script setup lang="ts">
import { ref, onMounted } from "vue";

import FolderItem from "./FolderItem.vue";

import type { Folder, Filter } from "@/api";
import { api } from "@/api";
import { error } from "@/utils/message";

const { filter } = defineProps<{
  filter: Filter;
}>();

defineExpose({
  refresh: loadFolders,
});

const folder = ref<Folder>();
const selectedFolder = ref<Folder>();

onMounted(() => {
  loadFolders();
});

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

function selectFolder(folder: Folder) {
  if (selectedFolder.value === folder) {
    console.log("Unselect folder:", folder.name);
    selectedFolder.value = undefined;
    filter.folderPath = "";
    return;
  }
  console.log("Select folder:", folder.name);
  selectedFolder.value = folder;
  filter.folderPath = folder.path;
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

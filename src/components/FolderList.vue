<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

import FolderItem from "./FolderItem.vue";

import { Folder } from "../types";

const folder = ref<Folder>();
const selectedFolder = ref<Folder>();

const emit = defineEmits(["select"]);

async function loadFolders() {
  console.log("Load folders");
  invoke<Folder>("get_folder")
    .then((data) => {
      console.log(data);
      folder.value = data;
    })
    .catch((error) => {
      console.error(error);
    });
}

function selectFolder(folder: Folder) {
  if (selectedFolder.value === folder) {
    console.log("Unselect folder:", folder.name);
    selectedFolder.value = undefined;
    emit("select", undefined);
    return;
  }
  selectedFolder.value = folder;
  console.log("Select folder:", folder.name);
  emit("select", folder);
}

defineExpose({
  refresh: loadFolders,
});

onMounted(() => {
  loadFolders();
});
</script>

<template>
  <div class="w-full h-full p-8">
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

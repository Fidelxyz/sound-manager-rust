<script setup lang="ts">
import { Button } from "primevue";

import type { Folder, FolderNode } from "@/api";
import { computed } from "vue";

const {
  folderNode,
  selectedFolder,
  depth = 0,
} = defineProps<{
  folderNode: FolderNode;
  selectedFolder?: Folder;
  depth?: number;
}>();

const folder = computed(() => folderNode.folder);

const emit = defineEmits<{
  select: [folder: Folder];
}>();

function selectFolder(folder: Folder) {
  emit("select", folder);
}
</script>

<template>
  <li>
    <Button
      variant="text"
      class="w-full justify-start!"
      :class="{ active: folder === selectedFolder }"
      :style="{
        paddingLeft: `calc(var(--p-button-padding-x) + ${depth * 1}rem)`,
      }"
      :label="folder.name"
      @click.stop="selectFolder(folder)"
      :dt="{
        label: {
          font: {
            weight: 300,
          },
        },
      }"
      :pt="{
        label: {
          class: 'text-surface-100',
        },
      }"
    >
    </Button>
  </li>
  <FolderItem
    v-for="subFolder in folderNode.subFolders"
    :folderNode="subFolder"
    :depth="depth + 1"
    :selectedFolder="selectedFolder"
    @select="selectFolder"
  />
</template>

<style scoped>
button.active {
  background: var(--p-button-text-primary-active-background) !important;
}
</style>

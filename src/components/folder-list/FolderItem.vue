<script setup lang="ts">
import type { PropType } from "vue";

import { Button } from "primevue";

import type { Folder } from "@/api";

defineProps({
  folder: {
    type: Object as PropType<Folder>,
    required: true,
  },
  depth: {
    type: Number,
    default: 0,
  },
  selectedPath: String,
});

const emit = defineEmits<{
  select: [folder: Folder];
}>();

const selectFolder = (folder: Folder) => {
  emit("select", folder);
};
</script>

<template>
  <li>
    <Button
      variant="text"
      class="w-full justify-start!"
      :class="{ active: folder.path === selectedPath }"
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
    v-for="subFolder in folder.subFolders"
    :key="subFolder.path"
    :folder="subFolder"
    :depth="depth + 1"
    :selected-path="selectedPath"
    @select="selectFolder"
  />
</template>

<style scoped>
button.active {
  background: var(--p-button-text-primary-active-background) !important;
}
</style>

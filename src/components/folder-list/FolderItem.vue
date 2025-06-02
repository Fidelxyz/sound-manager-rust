<script setup lang="ts">
import { computed, onMounted, onUnmounted, useTemplateRef } from "vue";

import { Button } from "primevue";

import type { CleanupFn } from "@atlaskit/pragmatic-drag-and-drop/dist/types/internal-types";
import { dropTargetForExternal } from "@atlaskit/pragmatic-drag-and-drop/external/adapter";
import { containsFiles } from "@atlaskit/pragmatic-drag-and-drop/external/file";

import type { Folder, FolderNode } from "@/api";
import type { DropTargetData } from "@/types";

const {
  folderNode,
  selectedFolder,
  depth = 0,
} = defineProps<{
  folderNode: FolderNode;
  selectedFolder?: Folder | null;
  depth?: number;
}>();

const nodeContent = useTemplateRef("nodeContent");

const folder = computed(() => folderNode.folder);

const emit = defineEmits<{
  select: [folder: Folder];
}>();

onMounted(() => {
  registerDragAndDrop();
});

onUnmounted(() => {
  unregisterDragAndDrop();
});

function selectFolder(folder: Folder) {
  emit("select", folder);
}

// ========== Drag and Drop BEGIN ==========

let unregisterDropTarget: CleanupFn | null = null;

function registerDragAndDrop() {
  if (nodeContent.value) {
    unregisterDropTarget = dropTargetForExternal({
      element: nodeContent.value,
      canDrop: containsFiles,
      getData: () => {
        const data: DropTargetData = {
          type: "folder",
          folder: folder.value,
        };
        return data;
      },
    });
  }
}

function unregisterDragAndDrop() {
  if (unregisterDropTarget) {
    unregisterDropTarget();
    unregisterDropTarget = null;
  }
}

// ========== Drag and Drop END ==========
</script>

<template>
  <li ref="nodeContent">
    <Button
      variant="text"
      class="z-1999 w-full justify-start!"
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

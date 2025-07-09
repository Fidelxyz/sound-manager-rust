<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, useTemplateRef } from "vue";

import { Button } from "primevue";

import type { Instruction } from "@atlaskit/pragmatic-drag-and-drop-hitbox/tree-item";
import type { CleanupFn } from "@atlaskit/pragmatic-drag-and-drop/dist/types/internal-types";
import { dropTargetForElements } from "@atlaskit/pragmatic-drag-and-drop/element/adapter";

import DropIndicator from "@/components/dropindicator/DropIndicator.vue";

import type { Folder } from "@/api";
import type { DropTargetData, FolderNode } from "@/types";

const INDENT_PX = 14;

const { folderNode, depth = 0 } = defineProps<{
  folderNode: FolderNode;
  depth?: number;
}>();

const selectedFolder = defineModel<Folder | null>("selectedFolder", {
  default: null,
});

const emit = defineEmits<{
  contextmenu: [event: MouseEvent, folder: Folder];
}>();

const nodeContent = useTemplateRef("nodeContent");

const folder = computed(() => folderNode.folder);

onMounted(() => {
  registerDragAndDrop();
});

onUnmounted(() => {
  unregisterDragAndDrop();
});

function onClick(folder: Folder) {
  if (selectedFolder.value !== folder) {
    selectedFolder.value = folder;
  } else {
    selectedFolder.value = null;
  }
}

// ========== Drag and Drop BEGIN ==========

const dropTargetInstruction = ref<Instruction | null>(null);
let unregisterDropTarget: CleanupFn | null = null;

function registerDragAndDrop() {
  if (nodeContent.value) {
    unregisterDropTarget = dropTargetForElements({
      element: nodeContent.value,
      getData: () => {
        const data: DropTargetData = {
          type: "folder",
          folder: folder.value,
        };
        return data;
      },
      canDrop: ({ source }) =>
        (source.data.type === "folder" &&
          source.data.folder !== folder.value) ||
        source.data.type === "entry",
      onDragEnter: () => {
        dropTargetInstruction.value = {
          type: "make-child",
          currentLevel: depth,
          indentPerLevel: INDENT_PX,
        };
      },
      onDragLeave: () => {
        dropTargetInstruction.value = null;
      },
      onDrop: () => {
        dropTargetInstruction.value = null;
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
  <li ref="nodeContent" class="relative">
    <Button
      variant="text"
      class="w-full justify-start!"
      :class="{ active: folder === selectedFolder }"
      :style="{
        paddingLeft: `calc(var(--p-button-padding-x) + ${depth * INDENT_PX}px)`,
      }"
      :label="folder.name"
      @click.stop="onClick(folder)"
      @contextmenu="emit('contextmenu', $event, folder)"
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
    />
    <DropIndicator
      v-if="dropTargetInstruction"
      :instruction="dropTargetInstruction"
    />
  </li>
  <FolderItem
    v-for="subFolder in folderNode.subFolders"
    :folderNode="subFolder"
    :depth="depth + 1"
    v-model:selectedFolder="selectedFolder"
    @contextmenu="(event, folder) => emit('contextmenu', event, folder)"
  />
</template>

<style scoped>
button.active {
  background: var(--p-button-text-primary-active-background) !important;
}
</style>

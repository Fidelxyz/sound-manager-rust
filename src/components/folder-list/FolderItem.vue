<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, useTemplateRef } from "vue";

import { Button } from "primevue";

import type { Instruction } from "@atlaskit/pragmatic-drag-and-drop-hitbox/tree-item";
import type { CleanupFn } from "@atlaskit/pragmatic-drag-and-drop/dist/types/internal-types";
import {
  draggable,
  dropTargetForElements,
} from "@atlaskit/pragmatic-drag-and-drop/element/adapter";
import { combine } from "@atlaskit/pragmatic-drag-and-drop/combine";

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

const dragging = ref(false);

const dropTargetInstruction = ref<Instruction | null>(null);
let unregisterDragAndDrop: CleanupFn = () => {};

function registerDragAndDrop() {
  if (nodeContent.value) {
    unregisterDragAndDrop = combine(
      draggable({
        element: nodeContent.value,
        getInitialData: () => {
          const data: DropTargetData = {
            type: "folder",
            folder: folder.value,
          };
          return data;
        },
        onDragStart: () => {
          dragging.value = true;
        },
        onDrop: () => {
          dragging.value = false;
        },
      }),
      dropTargetForElements({
        element: nodeContent.value,
        getData: () => {
          const data: DropTargetData = {
            type: "folder",
            folder: folder.value,
          };
          return data;
        },
        canDrop: ({ source }) => {
          const sourceData = source.data as DropTargetData;
          switch (sourceData.type) {
            case "folder":
              return (
                sourceData.folder !== folder.value &&
                sourceData.folder.parentId !== folder.value.id
              );
            case "entry":
              return true;
            default:
              return false;
          }
        },
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
      }),
    );
  }
}

// ========== Drag and Drop END ==========
</script>

<template>
  <li ref="nodeContent" class="relative">
    <Button
      variant="text"
      class="w-full justify-start!"
      :class="{ active: folder === selectedFolder, 'opacity-50': dragging }"
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
    v-if="!dragging"
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

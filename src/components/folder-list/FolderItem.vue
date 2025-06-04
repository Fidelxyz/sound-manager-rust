<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, useTemplateRef } from "vue";

import { Button } from "primevue";

import type { Instruction } from "@atlaskit/pragmatic-drag-and-drop-hitbox/tree-item";
import { combine } from "@atlaskit/pragmatic-drag-and-drop/combine";
import type { CleanupFn } from "@atlaskit/pragmatic-drag-and-drop/dist/types/internal-types";
import { dropTargetForElements } from "@atlaskit/pragmatic-drag-and-drop/element/adapter";
import { dropTargetForExternal } from "@atlaskit/pragmatic-drag-and-drop/external/adapter";
import { containsFiles } from "@atlaskit/pragmatic-drag-and-drop/external/file";

import DropIndicator from "@/components/dropindicator/DropIndicator.vue";

import type { Folder, FolderNode } from "@/api";
import type { DropTargetData } from "@/types";

const INDENT_PX = 14;

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

const dropTargetInstruction = ref<Instruction | null>(null);
let unregisterDropTarget: CleanupFn | null = null;

function registerDragAndDrop() {
  if (nodeContent.value) {
    unregisterDropTarget = combine(
      dropTargetForElements({
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
      }),
      dropTargetForExternal({
        element: nodeContent.value,
        canDrop: containsFiles,
        getData: () => {
          const data: DropTargetData = {
            type: "folder",
            folder: folder.value,
          };
          return data;
        },
      }),
    );
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
      class="z-1999 w-full justify-start!"
      :class="{ active: folder === selectedFolder }"
      :style="{
        paddingLeft: `calc(var(--p-button-padding-x) + ${depth * INDENT_PX}px)`,
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
    :selectedFolder="selectedFolder"
    @select="selectFolder"
  />
</template>

<style scoped>
button.active {
  background: var(--p-button-text-primary-active-background) !important;
}
</style>

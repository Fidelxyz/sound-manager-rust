<script setup lang="ts">
import { combine } from "@atlaskit/pragmatic-drag-and-drop/combine";
import {
  draggable,
  dropTargetForElements,
} from "@atlaskit/pragmatic-drag-and-drop/element/adapter";
import type { Instruction } from "@atlaskit/pragmatic-drag-and-drop-hitbox/tree-item";
import { Button } from "primevue";
import { computed, ref, useTemplateRef } from "vue";
import type { Folder } from "@/api";
import CollapseHandler from "@/components/CollapseHandler.vue";
import DropIndicator from "@/components/drop-indicator/DropIndicator.vue";
import type { DropTargetData, FolderNode } from "@/types";
import { useDragAndDrop } from "@/utils/drag-and-drop";

const { folderNode, depth = 0 } = defineProps<{
  folderNode: FolderNode;
  depth?: number;
}>();

const emit = defineEmits<{
  contextmenu: [event: MouseEvent, folder: Folder];
}>();

const nodeContent = useTemplateRef("nodeContent");

const folder = computed(() => folderNode.folder);

const expanded = ref(true);

// ========= Selection ==========

const selectedFolder = defineModel<Folder | null>("selectedFolder", {
  default: null,
});

const selected = computed(() => {
  return selectedFolder.value === folder.value;
});

function onClick() {
  if (selected.value) {
    selectedFolder.value = null;
  } else {
    selectedFolder.value = folder.value;
  }
}

// ========== Drag and Drop BEGIN ==========

const dragging = ref(false);

const dropTargetInstruction = ref<Instruction | null>(null);

useDragAndDrop(() => {
  if (!nodeContent.value) return null;
  return combine(
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
          indentPerLevel: 0,
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
});

// ========== Drag and Drop END ==========
</script>

<template>
  <div ref="nodeContent" class="relative">
    <Button
      variant="text"
      class="w-full justify-start!"
      :class="{ active: folder === selectedFolder, 'opacity-50': dragging }"
      @click.stop="onClick"
      @contextmenu="emit('contextmenu', $event, folder)"
    >
      <CollapseHandler
        class="flex items-center"
        v-model="expanded"
        :hidden="folderNode.subFolders.length === 0"
      />

      <i class="pi pi-folder" />

      <span class="text-surface-100">{{ folder.name }}</span>
    </Button>
    <DropIndicator
      v-if="dropTargetInstruction"
      :instruction="dropTargetInstruction"
    />
  </div>
  <ul
    v-if="folderNode.subFolders.length > 0 && expanded && !dragging"
    class="pl-4"
  >
    <li v-for="subFolder in folderNode.subFolders">
      <FolderItem
        :folderNode="subFolder"
        :depth="depth + 1"
        v-model:selectedFolder="selectedFolder"
        @contextmenu="(event, folder) => emit('contextmenu', event, folder)"
      />
    </li>
  </ul>
</template>

<style scoped>
.active {
  background: var(--p-button-text-primary-active-background) !important;
}
</style>

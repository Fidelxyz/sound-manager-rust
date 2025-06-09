<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";

import type { CleanupFn } from "@atlaskit/pragmatic-drag-and-drop/dist/types/internal-types";
import { monitorForElements } from "@atlaskit/pragmatic-drag-and-drop/element/adapter";
import { useConfirm } from "primevue";

import FolderItem from "./FolderItem.vue";

import type { Entry, ErrorKind, Folder } from "@/api";
import { api } from "@/api";
import type { DropTargetData, Filter, FolderNode } from "@/types";
import { error, info } from "@/utils/message";

const filter = defineModel<Filter>("filter", { required: true });

const { folderTree } = defineProps<{
  folderTree: FolderNode | null;
}>();

const confirm = useConfirm();

onMounted(() => {
  registerDraggingMonitor();
});

onUnmounted(() => {
  if (unregisterDraggingMonitor) {
    unregisterDraggingMonitor();
    unregisterDraggingMonitor = null;
  }
});

const selectedFolder = ref<Folder | null>(null);

watch(selectedFolder, (newFolder) => {
  if (newFolder) {
    filter.value.folder = newFolder;
  } else {
    filter.value.folder = null;
  }
});

function moveEntryToFolder(entry: Entry, folder: Folder, force = false) {
  if (entry.folderId === folder.id) {
    console.info(
      `Entry ${entry.fileName} is already in folder ${folder.name}, no need to move.`,
    );
    info(
      "无需移动文件",
      `文件 ${entry.fileName} 已经在 ${folder.name} 文件夹中。`,
    );
    return;
  }

  api
    .moveFile(entry.id, folder.id, force)
    .then(() => {
      console.info(`Moved entry ${entry.fileName} to folder ${folder.name}.`);
      info(
        "移动文件成功",
        `已将文件 ${entry.fileName} 文件移动至 ${folder.name} 文件夹。`,
      );
    })
    .catch((e: ErrorKind) => {
      if (e.kind === "fileAlreadyExists") {
        confirm.require({
          header: "文件已存在",
          message: `位于 ${folder.name} 文件夹中的文件 ${entry.fileName} 已存在。确定要覆盖文件吗？`,
          icon: "pi pi-exclamation-circle",
          rejectProps: {
            label: "取消",
            severity: "secondary",
            outlined: true,
          },
          acceptProps: { label: "覆盖文件", severity: "danger" },
          accept: () => moveEntryToFolder(entry, folder, true),
        });
        return;
      }
      console.error("Failed to move file:", e);
      error("移动文件失败", e.message);
    });
}

// ========== Drag and Drop BEGIN ==========

let unregisterDraggingMonitor: CleanupFn | null = null;

function registerDraggingMonitor() {
  unregisterDraggingMonitor = monitorForElements({
    canMonitor: ({ source }) =>
      source.data.type === "folder" || source.data.type === "entry",
    onDrop({ location, source }) {
      console.debug(1);
      if (location.current.dropTargets.length === 0) return;

      const sourceData = source.data as DropTargetData;
      const targetData = location.current.dropTargets[0].data as DropTargetData;

      if (targetData.type !== "folder") return;

      if (sourceData.type === "folder") {
        // [TODO]
      } else if (sourceData.type === "entry") {
        const entry = sourceData.data;
        const folder = targetData.folder;
        moveEntryToFolder(entry, folder);
      }
    },
  });
}

// ========== Drag and Drop END ==========
</script>

<template>
  <div class="bg-surface-800 flex h-full w-full flex-col px-8 pt-8">
    <div class="p-2 font-bold">文件夹</div>
    <ul class="flex-auto overflow-auto">
      <FolderItem
        v-if="folderTree"
        :folderNode="folderTree"
        v-model:selectedFolder="selectedFolder"
      />
    </ul>
  </div>
</template>

<style scoped></style>

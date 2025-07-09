<script setup lang="ts">
import type { CleanupFn } from "@atlaskit/pragmatic-drag-and-drop/dist/types/internal-types";
import { monitorForElements } from "@atlaskit/pragmatic-drag-and-drop/element/adapter";
import { ContextMenu, useConfirm } from "primevue";
import { onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";

import type { Entry, ErrorKind, Folder } from "@/api";
import { api } from "@/api";
import type { DropTargetData, Filter, FolderNode } from "@/types";
import { error, info } from "@/utils/message";
import FolderItem from "./FolderItem.vue";

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
      console.error("Failed to move file", e);
      error("移动文件失败", e.message);
    });
}

function moveFolderToFolder(folder: Folder, newParent: Folder) {
  api
    .moveFolder(folder.id, newParent.id)
    .then(() => {
      console.info(`Moved folder ${folder.name} to folder ${newParent.name}.`);
      info(
        "移动文件夹成功",
        `已将文件夹 ${folder.name} 移动至 ${newParent.name} 文件夹。`,
      );
    })
    .catch((e: ErrorKind) => {
      if (e.kind === "folderAlreadyExists") {
        error("文件夹已存在", `文件夹 ${folder.name} 已经存在于目标文件夹。`);
        return;
      }

      console.error("Failed to move folder", e);
      error("移动文件夹失败", e.message);
    });
}

// ========== Drag and Drop BEGIN ==========

let unregisterDraggingMonitor: CleanupFn | null = null;

function registerDraggingMonitor() {
  unregisterDraggingMonitor = monitorForElements({
    canMonitor: ({ source }) =>
      source.data.type === "folder" || source.data.type === "entry",
    onDrop({ location, source }) {
      if (location.current.dropTargets.length === 0) return;

      const sourceData = source.data as DropTargetData;
      const targetData = location.current.dropTargets[0].data as DropTargetData;

      if (targetData.type !== "folder") return;

      if (sourceData.type === "folder") {
        const sourceFolder = sourceData.folder;
        const targetFolder = targetData.folder;
        moveFolderToFolder(sourceFolder, targetFolder);
      } else if (sourceData.type === "entry") {
        const entry = sourceData.data;
        const folder = targetData.folder;
        moveEntryToFolder(entry, folder);
      }
    },
  });
}

// ========== Drag and Drop END ==========

// ========== Context Menu BEGIN ==========

const contextMenu = useTemplateRef("contextMenu");
const contextMenuSelection = ref<Folder | null>(null);
const contextMenuItems = ref([
  {
    label: "在文件管理器中显示",
    icon: "pi pi-folder-open",
    command: () => {
      if (contextMenuSelection.value) {
        api.revealFolder(contextMenuSelection.value.id);
      }
    },
  },
]);

function onContextmenu(event: MouseEvent, folder: Folder) {
  if (contextMenu.value) {
    contextMenuSelection.value = folder;
    contextMenu.value.show(event);
  }
}

// ========== Context Menu END ==========
</script>

<template>
  <div class="bg-surface-800 flex h-full w-full flex-col px-8 pt-8">
    <div class="p-2 font-bold">文件夹</div>
    <div>
      <FolderItem
        v-if="folderTree"
        :folderNode="folderTree"
        v-model:selectedFolder="selectedFolder"
        @contextmenu="(event, folder) => onContextmenu(event, folder)"
      />
      <ContextMenu ref="contextMenu" :model="contextMenuItems" />
    </div>
  </div>
</template>

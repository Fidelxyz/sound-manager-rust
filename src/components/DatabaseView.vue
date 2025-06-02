<script setup lang="ts">
import { listen } from "@tauri-apps/api/event";
import { dirname } from "@tauri-apps/api/path";
import { create, exists } from "@tauri-apps/plugin-fs";
import { useCurrentElement } from "@vueuse/core";
import { onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";

import { BlockUI, Splitter, SplitterPanel, useConfirm } from "primevue";
import type { TreeNode } from "primevue/treenode";

import type { CleanupFn } from "@atlaskit/pragmatic-drag-and-drop/dist/types/internal-types";
import {
  dropTargetForExternal,
  monitorForExternal,
} from "@atlaskit/pragmatic-drag-and-drop/external/adapter";
import {
  containsFiles,
  getFiles,
} from "@atlaskit/pragmatic-drag-and-drop/external/file";

import AudioList from "./audio-list/AudioList.vue";
import FolderList from "./folder-list/FolderList.vue";
import MetadataPanel from "./metadata-panel/MetadataPanel.vue";
import Player from "./player/Player.vue";
import TagList from "./tag-list/TagList.vue";

import type { Entry, Filter, Folder, FolderNode, TagNode } from "@/api";
import { api } from "@/api";
import type { DropTargetData } from "@/types";
import { error, info } from "@/utils/message";
import { isAudioFile } from "@/utils/utils";

const { basePath } = defineProps<{
  basePath: string | null;
}>();

const el = useCurrentElement();
const metadataPanel = useTemplateRef("metadataPanel");
const audioList = useTemplateRef("audioList");
const confirm = useConfirm();

// data
const entries = ref<Entry[]>([]);
const folder = ref<FolderNode | null>(null);
const tags = ref<TreeNode[]>([]);

// state
const activeEntry = ref<Entry | null>(null);
const filter = ref<Filter>({
  search: "",
  tagIds: [],
  folderId: null,
});

defineExpose({
  audioList,
  activeEntry,
});

onMounted(() => {
  loadEntries();
  loadFolders();
  loadTags();
});

onUnmounted(() => {
  unregisterDragAndDrop();
});

function onTagsChanged() {
  console.debug("Tags changed");
  loadTags();
  metadataPanel.value?.refresh();
}

function onFilesChanged() {
  console.debug("Files changed");
  loadEntries();
  loadFolders();
  metadataPanel.value?.refresh();
}

function loadEntries() {
  console.info("Loading entries");
  api
    .getEntries()
    .then((data) => {
      console.debug("Entries", data);
      entries.value = data;
    })
    .catch((e) => {
      error("加载文件失败", e.message);
      console.error(e);
    });
}

async function loadFolders() {
  console.info("Loading folders");
  api
    .getFolder()
    .then((data) => {
      console.debug("Folder", data);
      folder.value = data;
    })
    .catch((e) => {
      error("加载文件夹失败", e.message);
      console.error(e);
    });
}

function loadTags() {
  console.info("Loading tags");
  api
    .getTags()
    .then((tagNodes) => {
      console.debug("Tags", tagNodes);
      tags.value = toTagTree(tagNodes);
    })
    .catch((e) => {
      e("加载标签失败", e.message);
      console.error(e);
    });
}

function toTagTree(tags: TagNode[]): TreeNode[] {
  return tags.map((tagNode): TreeNode => {
    return {
      key: tagNode.tag.id.toString(),
      label: tagNode.tag.name,
      data: tagNode.tag,
      icon: `pi pi-tag tag-color-${tagNode.tag.color}`,
      children: toTagTree(tagNode.children),
    };
  });
}

listen("files_updated", onFilesChanged);

// ========== Drag and Drop BEGIN ==========

let unregisterDropTarget: CleanupFn | null = null;
let unregisterDraggingMonitor: CleanupFn | null = null;
const dropTargetFolder = ref<Folder | null>(null);

function registerDragAndDrop() {
  const rootFolder = folder.value?.folder;
  if (el.value instanceof HTMLElement && rootFolder) {
    unregisterDropTarget = dropTargetForExternal({
      element: el.value,
      canDrop: containsFiles,
      getData: () => {
        const data: DropTargetData = {
          type: "folder",
          folder: rootFolder,
        };
        return data;
      },
    });
  }
  unregisterDraggingMonitor = monitorForExternal({
    canMonitor: containsFiles,
    onDropTargetChange: ({ location }) => {
      if (location.current.dropTargets.length === 0) {
        dropTargetFolder.value = null;
        return;
      }

      const data = location.current?.dropTargets[0].data as DropTargetData;
      if (data.type === "folder") {
        dropTargetFolder.value = data.folder;
      } else {
        dropTargetFolder.value = null;
      }
    },
    onDrop: async ({ location, source }) => {
      dropTargetFolder.value = null;

      if (location.current.dropTargets.length === 0) return;
      const data = location.current?.dropTargets[0].data as DropTargetData;
      if (data.type !== "folder") return;

      const files = getFiles({ source });
      for (const file of files) {
        await importFile(file, data.folder);
      }
    },
  });
}

function unregisterDragAndDrop() {
  if (unregisterDropTarget) {
    unregisterDropTarget();
    unregisterDropTarget = null;
  }
  if (unregisterDraggingMonitor) {
    unregisterDraggingMonitor();
    unregisterDraggingMonitor = null;
  }
}

watch(folder, () => {
  unregisterDragAndDrop();
  registerDragAndDrop();
});

async function importFile(file: File, dstFolder: Folder) {
  if (basePath === null) return;
  if (!(await isAudioFile(file.name))) {
    error(
      "不支持的文件类型",
      `文件 ${file.name} 不是支持的音频文件（wav、mp3、flac、ogg）。`,
    );
    return;
  }

  const dstDir = `${basePath}/${dstFolder?.path ?? folder.value?.folder.path}`;
  const dstPath = `${dstDir}/${file.name}`;

  console.info(`Importing file ${file.name} to ${dstDir}`);

  if (await exists(dstPath)) {
    confirm.require({
      header: "文件已存在",
      message: `位于 ${dstDir} 中的文件 ${file.name} 已存在。确定要覆盖文件吗？`,
      icon: "pi pi-exclamation-circle",
      rejectProps: { label: "取消", severity: "secondary", outlined: true },
      acceptProps: { label: "覆盖文件", severity: "danger" },
      accept: () => confirmImportFile(file, dstPath),
    });
    return;
  }

  confirmImportFile(file, dstPath);
}

async function confirmImportFile(file: File, dstPath: string) {
  const dstFile = await create(dstPath);
  try {
    dstFile.write(new Uint8Array(await file.arrayBuffer())).then(async () => {
      console.info(`File ${file.name} written to ${dstPath}`);
      info(
        "导入文件成功",
        `文件 ${file.name} 已导入至文件夹 ${await dirname(dstPath)}`,
      );
    });
  } catch (e) {
    console.error(e);
    error("导入文件失败", `无法写入文件 ${dstPath}：${e}`);
    return;
  } finally {
    dstFile.close();
  }
}

// ========== Drag and Drop END ==========
</script>

<template>
  <div class="h-full">
    <div
      v-if="dropTargetFolder"
      class="flex items-center justify-center absolute size-full z-1999"
    >
      <div>导入至 {{ dropTargetFolder?.name }}</div>
    </div>
    <BlockUI :blocked="dropTargetFolder !== null" class="flex flex-col h-full">
      <div class="flex-auto min-h-0">
        <Splitter class="h-full rounded-none!" :gutterSize="2">
          <SplitterPanel class="min-w-2xs" :size="15">
            <Splitter layout="vertical" class="h-full" :gutterSize="2">
              <SplitterPanel :minSize="20">
                <FolderList :folder="folder" v-model:filter="filter" />
              </SplitterPanel>

              <SplitterPanel :minSize="20">
                <TagList
                  :tags="tags"
                  v-model:filter="filter"
                  @tags-changed="onTagsChanged"
                />
              </SplitterPanel>
            </Splitter>
          </SplitterPanel>

          <SplitterPanel :size="65">
            <AudioList
              ref="audioList"
              :entries="entries"
              :tags="tags"
              v-model:filter="filter"
              v-model:activeEntry="activeEntry"
            />
          </SplitterPanel>

          <SplitterPanel class="min-w-xs" :size="20">
            <MetadataPanel
              ref="metadataPanel"
              :entry="activeEntry"
              :allTags="tags"
            />
          </SplitterPanel>
        </Splitter>
      </div>
      <div>
        <Player :entry="activeEntry" />
      </div>
    </BlockUI>
  </div>
</template>

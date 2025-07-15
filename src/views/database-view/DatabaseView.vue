<script setup lang="ts">
import { listen } from "@tauri-apps/api/event";
import { basename } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { Splitter, SplitterPanel, useConfirm } from "primevue";
import type { TreeNode } from "primevue/treenode";
import { onMounted, ref, useTemplateRef } from "vue";
import { api } from "@/api";
import type { Entry, Filter, Folder, FolderNode, Tag, TagNode } from "@/types";
import { error, info } from "@/utils/message";
import AudioList from "./audio-list/AudioList.vue";
import FolderList from "./folder-list/FolderList.vue";
import MenuBar from "./MenuBar.vue";
import MetadataPanel from "./metadata-panel/MetadataPanel.vue";
import Player from "./player/Player.vue";
import TagList from "./tag-list/TagList.vue";

// refs
const metadataPanel = useTemplateRef("metadataPanel");
const audioList = useTemplateRef("audioList");
const player = useTemplateRef("player");

// data
const entries = ref<Entry[]>([]);
const folderTree = ref<FolderNode | null>(null);
const tags = ref<Record<number, Tag>>({});
const tagTree = ref<TagNode[]>([]);
const tagTreeNodes = ref<TreeNode[]>([]);

// state
const activeEntry = ref<Entry | null>(null);
const filter = ref<Filter>({
  search: "",
  tags: [],
  includeChildTags: true,
  noTags: false,
  folder: null,
  includeSubfolders: true,
});

defineExpose({
  audioList,
  player,
  activeEntry,
  importFiles,
});

const confirm = useConfirm();

onMounted(() => {
  loadEntries();
  loadFolders();
  loadTags();
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
      folderTree.value = buildFolderTree(data);
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
    .then((newTags) => {
      console.debug("Tags", newTags);
      tags.value = newTags;
      tagTree.value = buildTagTree(newTags).children;
      tagTreeNodes.value = tagTreeToTreeNode(tagTree.value);
    })
    .catch((e) => {
      e("加载标签失败", e.message);
      console.error(e);
    });
}

function buildFolderTree(
  folders: Record<number, Folder>,
  folderId = -1,
): FolderNode {
  const folder = folders[folderId];
  const subFolders = Object.values(folder.subFolders).map((subFolderId) =>
    buildFolderTree(folders, subFolderId),
  );
  return { folder, subFolders };
}

function buildTagTree(tags: Record<number, Tag>, tagId = -1): TagNode {
  const tag = tags[tagId];
  const children = tag.children
    .map((childTagId) => buildTagTree(tags, childTagId))
    .sort((a, b) => a.tag.position - b.tag.position);
  return { tag, children };
}

function tagTreeToTreeNode(tagTree: TagNode[]): TreeNode[] {
  return tagTree.map((tagNode) => ({
    key: tagNode.tag.id.toString(),
    label: tagNode.tag.name,
    data: tagNode.tag,
    icon: `pi pi-tag tag-color-${tagNode.tag.color}`,
    children: tagTreeToTreeNode(tagNode.children),
  }));
}

async function importFiles() {
  const paths = await open({
    title: "导入文件",
    multiple: true,
    filters: [{ name: "音频文件", extensions: ["wav", "mp3", "flac", "ogg"] }],
  });
  if (!paths) return;

  for (const path of paths) {
    console.info("Importing file", path);
    confirmImportFile(path);
  }
}

function confirmImportFile(path: string, force = false) {
  api
    .importFile(path, force)
    .then(async () => {
      info("导入文件成功", `已导入文件 ${await basename(path)}。`);
      console.info("Imported file", path);
    })
    .catch(async (e) => {
      if (e.kind === "fileAlreadyExists") {
        confirm.require({
          header: "文件已存在",
          message: `位于 ${folderTree.value?.folder.name} 中的文件 ${await basename(path)} 已存在。确定要覆盖文件吗？`,
          icon: "pi pi-exclamation-circle",
          rejectProps: { label: "取消", severity: "secondary", outlined: true },
          acceptProps: { label: "覆盖文件", severity: "danger" },
          accept: () => confirmImportFile(path, true),
        });
        return;
      }

      error("导入文件失败", e.message);
      console.error(e);
    });
}

listen("files_updated", onFilesChanged);
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="min-h-0 flex-auto">
      <Splitter class="h-full rounded-none!" :gutterSize="2">
        <SplitterPanel class="flex min-w-2xs flex-col" :size="15">
          <MenuBar />

          <Splitter layout="vertical" class="h-full" :gutterSize="2">
            <SplitterPanel :minSize="20">
              <FolderList :folderTree="folderTree" v-model:filter="filter" />
            </SplitterPanel>

            <SplitterPanel :minSize="20">
              <TagList
                :tagTree="tagTree"
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
            :folderTree="folderTree"
            :tags="tags"
            :tagTreeNodes="tagTreeNodes"
            v-model:filter="filter"
            v-model:activeEntry="activeEntry"
          />
        </SplitterPanel>

        <SplitterPanel class="min-w-xs" :size="20">
          <MetadataPanel
            ref="metadataPanel"
            :entry="activeEntry"
            :tagTreeNodes="tagTreeNodes"
          />
        </SplitterPanel>
      </Splitter>
    </div>
    <div>
      <Player ref="player" :entry="activeEntry" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { monitorForElements } from "@atlaskit/pragmatic-drag-and-drop/element/adapter";
import { extractInstruction } from "@atlaskit/pragmatic-drag-and-drop-hitbox/tree-item";
import { $dt } from "@primeuix/themes";
import {
  Button,
  ContextMenu,
  IconField,
  InputIcon,
  InputText,
  useConfirm,
} from "primevue";
import type { MenuItem, MenuItemCommandEvent } from "primevue/menuitem";
import { ref, useTemplateRef } from "vue";
import { api, type ErrorKind } from "@/api";
import type { DropTargetData, Filter, Tag, TagNode } from "@/types";
import { useDragAndDrop } from "@/utils/drag-and-drop";
import { error } from "@/utils/message";
import {
  appendChildNode,
  insertNodeAbove,
  insertNodeBelow,
  takeNode,
} from "@/utils/tag-tree";
import TagItem from "./TagItem.vue";

const emit = defineEmits<{
  "tags-changed": [];
}>();

const filter = defineModel<Filter>("filter", { required: true });

const { tagTree } = defineProps<{
  tagTree: TagNode[];
}>();

// ========== New Tag ==========

const editingNewTag = ref(false);
const editingTag = ref<Tag | null>(null);
const editingTagName = ref("");

function newTag() {
  editingNewTag.value = true;
  editingTagName.value = "";
  setTimeout(() => {
    const input = document.getElementById("new-tag");
    if (input) {
      input.focus();
    }
  }, 0);
}

function completeEditingNewTag() {
  editingNewTag.value = false;

  if (editingTagName.value === "") {
    return;
  }

  api
    .newTag(editingTagName.value)
    .then(() => {
      emit("tags-changed");
    })
    .catch((e) => {
      console.error(e);
      if (e.kind === "tagAlreadyExists") {
        error("标签已存在", "请使用其他名称");
      } else {
        error("创建标签错误", e.message);
      }
    });
}

// ========== Reorder Tags ==========

useDragAndDrop(() =>
  monitorForElements({
    canMonitor: ({ source }) => source.data.type === "tag",
    onDrop({ location, source }) {
      if (location.current.dropTargets.length === 0) return;

      const sourceData = source.data as DropTargetData;
      const targetData = location.current.dropTargets[0].data as DropTargetData;
      if (sourceData.type !== "tag" || targetData.type !== "tag") return;

      const instruction = extractInstruction(targetData);

      // take the source node from the tree
      let sourceNode = null;
      if (
        instruction?.type === "reorder-above" ||
        instruction?.type === "reorder-below" ||
        instruction?.type === "make-child"
      ) {
        sourceNode = takeNode(sourceData.tagNode, tagTree);
      }
      if (!sourceNode) return;

      // insert the source node into the target node
      let inserted;
      switch (instruction?.type) {
        case "reorder-above":
          inserted = insertNodeAbove(sourceNode, targetData.tagNode, tagTree);
          break;
        case "reorder-below":
          inserted = insertNodeBelow(sourceNode, targetData.tagNode, tagTree);
          break;
        case "make-child":
          inserted = appendChildNode(sourceNode, targetData.tagNode, tagTree);
          break;
        default:
          return;
      }
      if (!inserted) return;

      reorderTag(sourceNode, inserted.parent, inserted.location);
    },
  }),
);

function reorderTag(tag: TagNode, newParent: TagNode | null, newPos: number) {
  console.debug("Reorder tag", tag, newParent, newPos);
  const newParentId = newParent ? newParent.tag.id : -1;
  api
    .reorderTag(tag.tag.id, newParentId, newPos)
    .then(() => {
      emit("tags-changed");
    })
    .catch((e) => {
      console.error(e);
      error("移动标签错误", e.message);
    });
}

function addTagToEntry(tagId: number, entryId: number) {
  api
    .addTagForEntry(entryId, tagId)
    .then(() => {
      emit("tags-changed");
    })
    .catch((e: ErrorKind) => {
      if (e.kind === "tagAlreadyExistsForEntry") return;

      console.error(e);
      error("添加标签到条目错误", e.message);
    });
}

// ========== Drag and Drop ==========

useDragAndDrop(() =>
  monitorForElements({
    canMonitor: ({ source }) => source.data.type === "entry",
    onDrop({ location, source }) {
      if (location.current.dropTargets.length === 0) return;

      const sourceData = source.data as DropTargetData;
      const targetData = location.current.dropTargets[0].data as DropTargetData;

      if (sourceData.type !== "entry" || targetData.type !== "tag") return;

      const entryId = sourceData.key;
      const tagId = targetData.tagNode.tag.id;

      addTagToEntry(tagId, entryId);
    },
  }),
);

// ========== Context Menu ==========

const confirm = useConfirm();
const contextMenu = useTemplateRef("contextMenu");
const contextMenuSelectedTag = ref<Tag>();
const contextMenuItems: MenuItem[] = [
  {
    label: "重命名",
    icon: "pi pi-pencil",
    command: renameTag,
  },
  {
    label: "色彩",
    icon: "pi pi-palette",
    items: [
      "灰色",
      "红色",
      "橙色",
      "黄色",
      "绿色",
      "青色",
      "蓝色",
      "紫色",
      "粉色",
    ].map((color, index) => ({
      label: color,
      icon: `pi pi-circle-fill tag-color-${index}`,
      command: setTagColor,
      color: index,
    })),
  },
  {
    label: "删除",
    icon: "pi pi-trash",
    command: deleteTag,
  },
];

function onContextmenu(event: MouseEvent, target: Tag) {
  contextMenuSelectedTag.value = target;
  contextMenu.value?.show(event);
}

function renameTag() {
  const tag = contextMenuSelectedTag.value;
  if (!tag) return;

  editingTag.value = tag;
}

function deleteTag() {
  const tag = contextMenuSelectedTag.value;
  if (!tag) return;

  confirm.require({
    header: "删除标签",
    message: `确定要删除标签 “${tag.name}” 吗？`,
    icon: "pi pi-trash",
    rejectProps: { label: "取消", severity: "secondary", outlined: true },
    acceptProps: { label: "删除", severity: "danger" },
    accept: () => confirmDeleteTag(tag),
  });
}

function confirmDeleteTag(tag: Tag) {
  console.debug("Delete tag", tag);
  api
    .deleteTag(tag.id)
    .then(() => {
      emit("tags-changed");
    })
    .catch((e) => {
      console.error(e);
      error("删除标签错误", e.message);
    });
}

function setTagColor(event: MenuItemCommandEvent) {
  const tag = contextMenuSelectedTag.value;
  if (!tag) return;
  const color: number = event.item.color;
  console.debug("Set tag color", tag, color);
  api
    .setTagColor(tag.id, color)
    .then(() => {
      emit("tags-changed");
    })
    .catch((e) => {
      console.error(e);
      error("设置标签颜色错误", e.message);
    });
}
</script>

<template>
  <div class="bg-surface-800 flex h-full w-full flex-col px-8 pt-8">
    <div class="flex items-center">
      <div class="mr-auto p-2 font-bold">标签</div>
      <Button
        icon="pi pi-plus"
        class="p-button-rounded p-button-text"
        size="small"
        @click="newTag"
      />
    </div>

    <div class="overflow-auto">
      <ContextMenu ref="contextMenu" :model="contextMenuItems" />

      <!-- Tag List -->
      <ul
        class="flex flex-col"
        :style="{
          gap: $dt('tree.gap').value,
        }"
      >
        <li v-for="(tagNode, index) of tagTree">
          <TagItem
            :tagNode="tagNode"
            :lastInGroup="index === tagTree.length - 1"
            v-model:selectedTags="filter.tags"
            v-model:editingTag="editingTag"
            @contextmenu="(event, target) => onContextmenu(event, target)"
            @tags-changed="emit('tags-changed')"
          />
        </li>
      </ul>

      <!-- New tag -->
      <IconField v-if="editingNewTag">
        <InputIcon class="pi pi-tag" />
        <InputText
          id="new-tag"
          v-model="editingTagName"
          icon="pi pi-tag"
          class="editing-input w-full"
          v-on:focusout="completeEditingNewTag"
          v-on:keydown.enter="completeEditingNewTag"
          autofocus
        />
      </IconField>
    </div>
  </div>
</template>

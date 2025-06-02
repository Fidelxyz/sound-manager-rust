<script setup lang="ts">
import { computed, ref, useTemplateRef } from "vue";

import {
  Button,
  ContextMenu,
  IconField,
  InputIcon,
  InputText,
  type TreeSelectionKeys,
  useConfirm,
} from "primevue";
import type { MenuItem, MenuItemCommandEvent } from "primevue/menuitem";
import type { TreeNode } from "primevue/treenode";

import type { ErrorKind, Filter, Tag } from "@/api";
import { api } from "@/api";
import { error } from "@/utils/message";
import Tree from "./tree";

const emit = defineEmits<{
  "tags-changed": [];
}>();

const filter = defineModel<Filter>("filter", { required: true });

const { tags } = defineProps<{
  tags: TreeNode[];
}>();

const selectedTags = computed({
  get: () => {
    const selectionKeys: TreeSelectionKeys = {};
    for (const tagId of filter.value.tagIds) {
      selectionKeys[tagId.toString()] = true;
    }
    return selectionKeys;
  },
  set: (selectionKeys: TreeSelectionKeys) => {
    console.debug("Set selected tag", selectionKeys);
    filter.value.tagIds = Object.keys(selectionKeys).map((id) =>
      Number.parseInt(id),
    );
  },
});

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

function completeRenameTag() {
  if (!editingTag.value) return;
  if (
    editingTagName.value === "" ||
    editingTagName.value === editingTag.value.name
  ) {
    editingTag.value = null;
    return;
  }

  api
    .renameTag(editingTag.value.id, editingTagName.value)
    .then(() => {
      emit("tags-changed");
    })
    .catch((e) => {
      console.error(e);
      if (e.kind === "tagAlreadyExists") {
        error("标签已存在", "请使用其他名称");
      } else {
        error("重命名标签错误", e.message);
      }
    })
    .finally(() => {
      editingTag.value = null;
    });
}

function reorderTags({
  sourceKey,
  targetParentKey,
  targetLocation,
}: {
  sourceKey: string;
  targetParentKey: string | null | undefined;
  targetLocation: number;
}) {
  if (targetParentKey === undefined) {
    console.error("targetParentKey is undefined");
    return;
  }

  const tagId = Number.parseInt(sourceKey);
  const parentId =
    targetParentKey !== null ? Number.parseInt(targetParentKey) : -1;

  api
    .reorderTag(tagId, parentId, targetLocation)
    .then(() => {
      emit("tags-changed");
    })
    .catch((e) => {
      console.error(e);
      error("移动标签错误", e.message);
    });
}

function addTagToEntry({
  tagKey,
  entryKey,
}: {
  tagKey: string;
  entryKey: number;
}) {
  const tagId = Number.parseInt(tagKey);
  const entryId = entryKey;

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

// ========== Context Menu BEGIN ==========

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
      {
        label: "灰色",
        icon: "pi pi-circle-fill tag-color-0",
        command: setTagColor,
        color: 0,
      },
      {
        label: "红色",
        icon: "pi pi-circle-fill tag-color-1",
        command: setTagColor,
        color: 1,
      },
      {
        label: "橙色",
        icon: "pi pi-circle-fill tag-color-2",
        command: setTagColor,
        color: 2,
      },
      {
        label: "黄色",
        icon: "pi pi-circle-fill tag-color-3",
        command: setTagColor,
        color: 3,
      },
      {
        label: "绿色",
        icon: "pi pi-circle-fill tag-color-4",
        command: setTagColor,
        color: 4,
      },
      {
        label: "青色",
        icon: "pi pi-circle-fill tag-color-5",
        command: setTagColor,
        color: 5,
      },
      {
        label: "蓝色",
        icon: "pi pi-circle-fill tag-color-6",
        command: setTagColor,
        color: 6,
      },
      {
        label: "紫色",
        icon: "pi pi-circle-fill tag-color-7",
        command: setTagColor,
        color: 7,
      },
      {
        label: "粉色",
        icon: "pi pi-circle-fill tag-color-8",
        command: setTagColor,
        color: 8,
      },
    ],
  },
  {
    label: "删除",
    icon: "pi pi-trash",
    command: deleteTag,
  },
];

function onTagRightClick(event: MouseEvent, tag: Tag) {
  contextMenuSelectedTag.value = tag;
  contextMenu.value?.show(event);
}

function renameTag() {
  const tag = contextMenuSelectedTag.value;
  if (!tag) return;

  editingTag.value = tag;
  editingTagName.value = tag.name;
  setTimeout(() => {
    const input = document.querySelector(".editing-input");
    if (input) {
      (input as HTMLElement).focus();
    }
  }, 0);
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

// ========== Context Menu END ==========
</script>

<template>
  <div class="flex flex-col w-full h-full px-8 pt-8 bg-surface-800">
    <div class="flex items-center">
      <div class="mr-auto font-bold p-2">标签</div>
      <Button
        icon="pi pi-plus"
        class="p-button-rounded p-button-text"
        size="small"
        @click="newTag"
      />
    </div>

    <div class="flex-auto overflow-auto">
      <ContextMenu ref="contextMenu" :model="contextMenuItems" />
      <Tree
        class="p-0!"
        :value="tags"
        v-model:selectionKeys="selectedTags"
        selectionMode="single"
        @node-reorder="reorderTags"
        @add-tag-to-entry="addTagToEntry"
        :pt="{
          root: {
            class: 'bg-transparent!',
          },
          nodeContent: ({ context }) => ({
            onContextmenu: (event: MouseEvent) =>
              onTagRightClick(event, context.node.data),
            class: {
              'py-0!': context.node.data === editingTag,
              'hover:bg-(--p-button-text-primary-hover-background)!': true,
            },
          }),
        }"
      >
        <template #default="{ node }">
          <span v-if="node.data !== editingTag">
            {{ node.label }}
          </span>

          <InputText
            v-else
            v-model="editingTagName"
            class="editing-input w-full"
            @focusout="completeRenameTag"
            @keydown.enter="completeRenameTag"
          />
        </template>
      </Tree>

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
        />
      </IconField>
    </div>
  </div>
</template>

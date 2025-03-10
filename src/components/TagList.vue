<script setup lang="ts">
import { ref } from "vue";

import {
  Button,
  InputText,
  ContextMenu,
  useConfirm,
  ConfirmDialog,
} from "primevue";

import { EntryTag } from "../types";
import { error } from "../utils/message";
import api from "../api";

const emit = defineEmits(["tags-changed"]);

const { tags } = defineProps<{
  tags: EntryTag[];
}>();

const selectedTagId = ref<number>();
const editingNewTag = ref(false);
const editingTag = ref<EntryTag>();
const editingTagName = ref("");

function selectTag(tagId: number) {
  if (selectedTagId.value === tagId) {
    selectedTagId.value = undefined;
    return;
  }
  selectedTagId.value = tagId;
}

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
    .then((tag) => {
      tags.push(tag);
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
  if (editingTag.value === undefined) return;
  if (
    editingTagName.value === "" ||
    editingTagName.value === editingTag.value.name
  ) {
    editingTag.value = undefined;
    return;
  }

  api
    .renameTag(editingTag.value.id, editingTagName.value)
    .then(() => {
      if (editingTag.value) {
        editingTag.value.name = editingTagName.value;
      }
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
      editingTag.value = undefined;
    });
}

// ========== Context Menu BEGIN ==========

const confirm = useConfirm();
const contextMenu = ref();
const contextMenuSelectedTag = ref<EntryTag>();
const contextMenuItems = ref([
  {
    label: "重命名",
    icon: "pi pi-pencil",
    command: renameTag,
  },
  {
    label: "删除",
    icon: "pi pi-trash",
    command: confirmDeleteTag,
  },
]);

function onTagRightClick(event: MouseEvent, tag: EntryTag) {
  contextMenuSelectedTag.value = tag;
  contextMenu.value.show(event);
}

function renameTag() {
  const tag = contextMenuSelectedTag.value;
  if (!tag) return;

  editingTag.value = tag;
  editingTagName.value = tag.name;
  setTimeout(() => {
    const input = document.querySelector(".editingtag");
    if (input) {
      (input as HTMLElement).focus();
    }
  }, 0);
}

function confirmDeleteTag() {
  const tag = contextMenuSelectedTag.value;
  if (!tag) return;

  confirm.require({
    header: "删除标签",
    message: `确定要删除标签 ${tag.name} 吗？`,
    icon: "pi pi-exclamation-circle",
    rejectProps: { label: "取消", severity: "secondary", outlined: true },
    acceptProps: { label: "删除", severity: "danger" },
    accept: () => deleteTag(tag),
  });
}

function deleteTag(tag: EntryTag) {
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

// ========== Context Menu END ==========
</script>

<template>
  <div class="w-full h-full p-8">
    <div class="flex items-center">
      <div class="mr-auto font-bold p-2">标签</div>
      <Button
        icon="pi pi-plus"
        class="p-button-rounded p-button-text"
        size="small"
        @click="newTag"
      />
    </div>
    <ul>
      <ContextMenu ref="contextMenu" :model="contextMenuItems" />
      <ConfirmDialog />
      <li v-for="tag in tags" :key="tag.id" @click.stop="selectTag(tag.id)">
        <Button
          v-if="tag.id !== editingTag?.id"
          variant="text"
          class="w-full justify-start!"
          :class="{ active: tag.id === selectedTagId }"
          :label="tag.name"
          @contextmenu="onTagRightClick($event, tag)"
        />
        <InputText
          v-else
          v-model="editingTagName"
          :class="{ editingtag: tag.id === editingTag?.id }"
          @focusout="completeRenameTag"
          @keydown.enter="completeRenameTag"
        />
      </li>
      <li v-if="editingNewTag">
        <InputText
          id="new-tag"
          v-model="editingTagName"
          v-on:focusout="completeEditingNewTag"
          v-on:keydown.enter="completeEditingNewTag"
        />
      </li>
    </ul>
  </div>
</template>

<style scoped>
button.active {
  background: var(--p-button-text-primary-active-background) !important;
}

:deep(.p-button-label) {
  font-weight: 300;
  color: var(--p-surface-100);
}
</style>

<script setup lang="ts">
import { Button, Popover, Tree, type TreeSelectionKeys } from "primevue";
import type { TreeNode } from "primevue/treenode";
import { onMounted, ref, useTemplateRef, watch } from "vue";
import { api, type ErrorKind } from "@/api";
import type { Entry, Tag } from "@/types";
import { error } from "@/utils/message";

const { entry, allTags } = defineProps<{
  entry?: Entry;
  allTags: TreeNode[];
}>();

const popover = useTemplateRef("popover");

const tags = ref<Tag[]>([]);
const selectedTag = ref<TreeSelectionKeys>();

watch(() => entry, refresh);

defineExpose({
  refresh: refresh,
});

onMounted(() => {
  refresh();
});

function refresh() {
  if (!entry) return;

  console.debug("Refresh", entry);
  tags.value = [];
  api
    .getTagsForEntry(entry.id)
    .then((data) => {
      console.debug("Get tags for entry", data);
      tags.value = data;
    })
    .catch((e) => {
      error("获取标签失败", e.message);
      console.error(e);
    });
}

function toggleTagSelector(event: MouseEvent) {
  popover.value?.toggle(event);
  setTimeout(() => {
    // if (treeSelect.value) {
    //   treeSelect.value.$el.querySelector("input")?.focus();
    // }
  }, 0);
}

function addTag(value: TreeSelectionKeys) {
  if (!entry) return;

  const tagId = Number.parseInt(Object.keys(value)[0]);
  console.info("Adding tag", tagId);
  api
    .addTagForEntry(entry.id, tagId)
    .then(() => {
      refresh();
    })
    .catch((e: ErrorKind) => {
      if (e.kind === "tagAlreadyExistsForEntry") return;

      console.error(e);
      error("添加标签失败", e.message);
    });

  popover.value?.hide();
}

function removeTag(tag: Tag) {
  if (!entry) return;

  console.debug("Remove tag", tag);
  api
    .removeTagForEntry(entry.id, tag.id)
    .then(() => {
      console.debug(entry);
      tags.value = tags.value.filter((oldTag) => oldTag.id !== tag.id);
    })
    .catch((e) => {
      error("删除标签失败", e.message);
      console.error(e);
    });
}

function onTagSelectorHide() {
  selectedTag.value = {};
}
</script>

<template>
  <div class="p-inputtext flex flex-wrap items-stretch gap-2 py-2!">
    <span v-if="tags.length === 0" class="text-surface-400 h-8 content-center">
      添加标签
    </span>

    <div
      v-for="tag in tags"
      class="flex h-8 items-center gap-0.5 rounded-md px-2.5"
      :class="`tag-bg-${tag.color}`"
    >
      <span class="ml-0.5 font-normal">{{ tag.name }}</span>
      <Button
        icon="pi pi-times"
        variant="text"
        size="small"
        class="size-6!"
        rounded
        @click="removeTag(tag)"
      />
    </div>

    <Button
      icon="pi pi-plus"
      class="ml-auto size-8!"
      severity="secondary"
      @click="toggleTagSelector"
    ></Button>

    <Popover
      class="tag-selector"
      ref="popover"
      dismissable
      closeOnEscape
      @hide="onTagSelectorHide"
    >
      <Tree
        v-model:selectionKeys="selectedTag"
        :value="allTags"
        selectionMode="single"
        class="p-0!"
        scrollHeight="36rem"
        filter
        @update:selectionKeys="addTag"
      />
    </Popover>
  </div>
</template>

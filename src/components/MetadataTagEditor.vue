<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { Tag, Button, Popover, Listbox } from "primevue";

import { Entry, EntryTag, ErrorKind } from "../types";
import { api } from "../api";
import { error } from "../utils/message";

const { entry, allTags } = defineProps<{
  entry?: Entry;
  allTags: EntryTag[];
}>();

const popoverRef = ref();

const tags = ref<EntryTag[]>([]);
const selectedTag = ref<EntryTag | null>();

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
  popoverRef.value?.toggle(event);
}

function addTag(tag: EntryTag) {
  if (!entry || !tag) return;

  console.log("Add tag", tag);
  api
    .addTagForEntry(entry.id, tag.id)
    .then(() => {
      console.debug(entry);
      tags.value.push(tag);
    })
    .catch((e: ErrorKind) => {
      console.error(e);
      error("添加标签失败", e.message);
    });

  popoverRef.value?.hide();
}

function removeTag(tag: EntryTag) {
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
  selectedTag.value = null;
}
</script>

<template>
  <div class="p-inputtext flex flex-wrap items-center items-stretch gap-2">
    <Tag v-for="tag in tags" class="items-center">
      <span>{{ tag.name }}</span>
      <Button
        icon="pi pi-times"
        variant="text"
        size="small"
        @click="removeTag(tag)"
      />
    </Tag>

    <Button
      icon="pi pi-plus"
      class="ml-auto"
      @click="toggleTagSelector"
    ></Button>

    <Popover class="tag-selector" ref="popoverRef" dismissable closeOnEscape>
      <Listbox
        v-model="selectedTag"
        :options="allTags"
        optionLabel="name"
        filter
        @update:modelValue="addTag"
        @hide="onTagSelectorHide"
      />
    </Popover>
  </div>
</template>

<style scoped></style>

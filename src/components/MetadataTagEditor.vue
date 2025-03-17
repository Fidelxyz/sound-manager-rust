<script setup lang="ts">
import { onMounted, ref, watch } from "vue";

import { Tag, Button, Popover, Listbox } from "primevue";

import type { Entry, EntryTag, ErrorKind } from "../api";
import { api } from "../api";
import { error } from "../utils/message";

const { entry, allTags } = defineProps<{
  entry?: Entry;
  allTags: EntryTag[];
}>();

const popover = ref();
const listbox = ref();

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
  popover.value?.toggle(event);
  setTimeout(() => {
    if (listbox.value) {
      listbox.value.$el.querySelector("input")?.focus();
    }
  }, 0);
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

  popover.value?.hide();
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
  <div class="p-inputtext flex flex-wrap items-stretch gap-2 py-2!">
    <span v-if="tags.length === 0" class="text-surface-400 content-center h-8"
      >添加标签</span
    >

    <Tag
      v-for="tag in tags"
      class="items-center rounded-md! h-8 px-2.5! gap-0.5!"
      :class="`tag-bg-${tag.color}`"
    >
      <span class="font-normal ml-0.5">{{ tag.name }}</span>
      <Button
        icon="pi pi-times"
        variant="text"
        size="small"
        class="size-6!"
        rounded
        @click="removeTag(tag)"
      />
    </Tag>

    <Button
      icon="pi pi-plus"
      class="ml-auto"
      @click="toggleTagSelector"
    ></Button>

    <Popover class="tag-selector" ref="popover" dismissable closeOnEscape>
      <Listbox
        ref="listbox"
        v-model="selectedTag"
        :options="allTags"
        optionLabel="name"
        filter
        emptyMessage="无标签"
        emptyFilterMessage="无匹配标签"
        @update:modelValue="addTag"
        @hide="onTagSelectorHide"
        :dt="{
          background: 'none',
          border: {
            color: 'none',
          },
        }"
      />
    </Popover>
  </div>
</template>

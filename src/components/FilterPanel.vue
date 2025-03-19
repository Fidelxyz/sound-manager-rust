<script setup lang="ts">
import { computed } from "vue";

import {
  InputText,
  FloatLabel,
  TreeSelect,
  Button,
  type TreeSelectionKeys,
} from "primevue";
import type { TreeNode } from "primevue/treenode";

import type { Filter, Entry } from "../api";

const { filter, entries, tags } = defineProps<{
  filter: Filter;
  entries: Entry[];
  tags: TreeNode[];
}>();

const selectedTags = computed({
  get: () => {
    const selectionKeys: TreeSelectionKeys = {};
    for (const tagId of filter.tagIds) {
      selectionKeys[tagId.toString()] = { checked: true };
    }
    return selectionKeys;
  },
  set: (selectionKeys: TreeSelectionKeys) => {
    const tagIds = [];
    for (const [tagId, state] of Object.entries(selectionKeys)) {
      if (state.checked) {
        tagIds.push(Number.parseInt(tagId));
      }
    }
    filter.tagIds = tagIds;
  },
});

const filterEnabled = computed(() => {
  return filter.search.length > 0 || filter.tagIds.length > 0;
});

function clearFilter() {
  filter.search = "";
  filter.tagIds = [];
}

function shuffle() {
  for (let i = entries.length - 1; i >= 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [entries[i], entries[j]] = [entries[j], entries[i]];
  }
}
</script>

<template>
  <div class="flex p-4">
    <div class="flex items-center gap-4">
      <FloatLabel variant="on">
        <InputText
          id="filter-search"
          v-model="filter.search"
          autocapitalize="off"
          class="w-64"
        />
        <label for="filter-search">搜索</label>
      </FloatLabel>

      <FloatLabel variant="on">
        <TreeSelect
          id="filter-tags"
          v-model="selectedTags"
          :options="tags"
          selectionMode="checkbox"
          class="w-48"
          showClear
        />
        <label for="filter-tags">标签</label>
      </FloatLabel>

      <Button
        v-if="filterEnabled"
        icon="pi pi-times"
        aria-label="清除过滤器"
        variant="text"
        size="small"
        rounded
        @click="clearFilter"
      />
    </div>

    <div class="flex justify-end ml-auto">
      <Button icon="pi pi-refresh" label="打乱" size="small" @click="shuffle" />
    </div>
  </div>
</template>

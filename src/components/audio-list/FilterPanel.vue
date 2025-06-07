<script setup lang="ts">
import { computed, ref } from "vue";

import { faShuffle } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/vue-fontawesome";
import {
  Button,
  FloatLabel,
  InputText,
  TreeSelect,
  type TreeSelectionKeys,
} from "primevue";
import type { TreeNode } from "primevue/treenode";

import type { Entry, Filter } from "@/api";
import { api } from "@/api";
import { error } from "@/utils/message";

const filter = defineModel<Filter>({ required: true });

const { entries, tags } = defineProps<{
  entries: Entry[];
  tags: TreeNode[];
}>();

const selectedTags = computed({
  get: () => {
    const selectionKeys: TreeSelectionKeys = {};
    for (const tagId of filter.value.tagIds) {
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
    filter.value.tagIds = tagIds;
  },
});

const filterEnabled = computed(() => {
  return filter.value.search.length > 0 || filter.value.tagIds.length > 0;
});

function clearFilter() {
  filter.value.search = "";
  filter.value.tagIds = [];
}

function shuffle() {
  for (let i = entries.length - 1; i >= 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [entries[i], entries[j]] = [entries[j], entries[i]];
  }
}

const refreshing = ref(false);

function refresh() {
  console.debug("Refreshing");
  refreshing.value = true;
  api
    .refresh()
    .catch((e) => {
      error("刷新失败", e.message);
    })
    .finally(() => {
      refreshing.value = false;
    });
}
</script>

<template>
  <div class="flex gap-4 p-4">
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
          class="w-48"
          v-model="selectedTags"
          :options="tags"
          selectionMode="checkbox"
          emptyMessage="无可用标签"
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

    <div class="ml-auto flex justify-end gap-2 text-nowrap">
      <Button label="打乱" severity="secondary" size="small" @click="shuffle">
        <template #icon>
          <FontAwesomeIcon :icon="faShuffle" />
        </template>
      </Button>

      <Button
        icon="pi pi-sync"
        label="刷新"
        severity="secondary"
        size="small"
        :loading="refreshing"
        @click="refresh"
      />
    </div>
  </div>
</template>

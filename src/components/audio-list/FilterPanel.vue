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

import type { Entry, Tag } from "@/api";
import { api } from "@/api";
import type { Filter } from "@/types";
import { error } from "@/utils/message";

const filter = defineModel<Filter>({ required: true });

const { entries, tags, tagTreeNodes } = defineProps<{
  entries: Entry[];
  tags: Record<number, Tag>;
  tagTreeNodes: TreeNode[];
}>();

const selectedTags = computed({
  get: () => {
    const selectionKeys: TreeSelectionKeys = {};
    for (const tag of filter.value.tags) {
      selectionKeys[tag.id] = { checked: true };
    }
    return selectionKeys;
  },
  set: (selectionKeys: TreeSelectionKeys) => {
    console.debug("Selected tags changed", selectionKeys);
    const filteredTags: Tag[] = [];
    for (const [tagId, state] of Object.entries(selectionKeys)) {
      if (state.checked) {
        filteredTags.push(tags[Number.parseInt(tagId)]);
      }
    }
    filter.value.tags = filteredTags;
  },
});

const filterEnabled = computed(() => {
  return (
    filter.value.search.length > 0 ||
    filter.value.tags.length > 0 ||
    filter.value.folder !== null
  );
});

function clearFilter() {
  filter.value = {
    search: "",
    tags: [],
    folder: null,
  };
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
          :options="tagTreeNodes"
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

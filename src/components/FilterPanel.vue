<script setup lang="ts">
import { ref, watch } from "vue";

import { InputText, FloatLabel, MultiSelect, Button } from "primevue";

import type { Filter, Entry, EntryTag } from "../api";

const { filter, entries, tags } = defineProps<{
  filter: Filter;
  entries: Entry[];
  tags: EntryTag[];
}>();

const filterEnabled = ref(false);

function clearFilter() {
  filter.search = "";
  filter.tagIds = [];
  filterEnabled.value = false;
}

watch(
  () => filter,
  (filter) => {
    filterEnabled.value = filter.search.length > 0 || filter.tagIds.length > 0;
  },
  { deep: true }
);

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
        <MultiSelect
          id="filter-tags"
          v-model="filter.tagIds"
          :options="tags"
          optionValue="id"
          optionLabel="name"
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

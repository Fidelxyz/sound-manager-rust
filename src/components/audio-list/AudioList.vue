<script setup lang="ts">
import { onMounted, ref, watch } from "vue";

import { FilterMatchMode } from "@primevue/core/api";
import {
  Column,
  type DataTableRowSelectEvent,
  type DataTableFilterMeta,
  type DataTableFilterMetaData,
} from "primevue";
import type { TreeNode } from "primevue/treenode";
import DataTable from "./datatable";

import type { Entry, Filter } from "@/api";
import { api } from "@/api";
import { error } from "@/utils/message";
import { formatDuration } from "@/utils/utils";
import FilterPanel from "./FilterPanel.vue";

const entries = ref<Entry[]>([]);
const activeEntry = ref<Entry>();

const emit = defineEmits<{
  select: [entry: Entry];
}>();

defineExpose({
  refresh: loadEntries,
});

const { filter, tags } = defineProps<{
  filter: Filter;
  tags: TreeNode[];
}>();

onMounted(() => {
  loadEntries();
});

function loadEntries() {
  console.log("Load entries");
  api
    .getEntries()
    .then((data) => {
      console.log(data);
      entries.value = data;
    })
    .catch((e) => {
      error("加载文件失败", e.message);
      console.error(e);
    });
}

function selectEntry(event: DataTableRowSelectEvent) {
  const entry = event.data as Entry;
  console.debug("Select entry", entry);

  entry.viewed = true;
  emit("select", entry);
}

// ========== Filter BEGIN ==========

const tableFilters = ref<DataTableFilterMeta>({
  id: { value: undefined, matchMode: FilterMatchMode.IN },
});

watch(
  () => filter,
  async (filter) => {
    let entry_ids = await api.filter(filter);
    console.debug("Filtered entries", entry_ids);
    if (Array.isArray(entry_ids) && entry_ids.length === 0) {
      entry_ids = [-1];
    }
    (tableFilters.value.id as DataTableFilterMetaData).value = entry_ids;
  },
  { deep: true },
);

// ========== Filter END ==========

function rowClass(data: Entry) {
  return [{ viewed: data?.viewed }];
}
</script>

<template>
  <FilterPanel :filter="filter" :entries="entries" :tags="tags"></FilterPanel>

  <DataTable
    :value="entries"
    v-model:selection="activeEntry"
    v-model:filters="tableFilters"
    datakey="id"
    class="h-full w-full text-nowrap"
    :rowClass="rowClass"
    scrollable
    scrollHeight="flex"
    resizableColumns
    removableSort
    tableClass="table-fixed"
    selectionMode="single"
    :metaKeySelection="true"
    :virtualScrollerOptions="{ itemSize: 32 }"
    @rowSelect="selectEntry"
    :pt="{
      tableContainer: {
        style: 'overflow-x: hidden !important',
      },
    }"
  >
    <Column class="w-1/3" field="title" header="标题" sortable>
      <template #body="slotProps">
        <span>{{ slotProps.data.title || slotProps.data.fileName }}</span>
      </template>
    </Column>
    <Column class="w-1/6" field="artist" header="艺术家" sortable></Column>
    <Column class="w-1/3" field="album" header="专辑" sortable></Column>
    <Column class="w-1/6" field="duration" header="时长" sortable>
      <template #body="slotProps">
        <span>{{
          slotProps.data.duration ? formatDuration(slotProps.data.duration) : ""
        }}</span>
      </template>
    </Column>
  </DataTable>
</template>

<style scoped>
:deep(.viewed) {
  color: var(--p-surface-300);
}
</style>

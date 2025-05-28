<script setup lang="ts">
import { onKeyStroke } from "@vueuse/core";
import { ref, useTemplateRef, watch } from "vue";

import { FilterMatchMode } from "@primevue/core/api";
import {
  Column,
  type DataTableFilterMeta,
  type DataTableFilterMetaData,
  type DataTableRowSelectEvent,
} from "primevue";
import type { TreeNode } from "primevue/treenode";
import DataTable from "./datatable";

import type { Entry, Filter } from "@/api";
import { api } from "@/api";
import { formatDuration } from "@/utils/utils";
import FilterPanel from "./FilterPanel.vue";

const { entries, tags } = defineProps<{
  entries: Entry[];
  tags: TreeNode[];
}>();

const filter = defineModel<Filter>("filter", { required: true });
const activeEntry = defineModel<Entry>("activeEntry");

function selectEntry(event: DataTableRowSelectEvent) {
  const entry = event.data as Entry;
  console.debug("Select entry", entry);
  entry.viewed = true;
}

const dataTable = useTemplateRef("dataTable");
onKeyStroke("ArrowUp", (event) => {
  if (activeEntry.value) {
    dataTable.value?.selectPrevRow(event);
  } else {
    dataTable.value?.selectRow(event, 0);
  }
  event.preventDefault();
});
onKeyStroke("ArrowDown", (event) => {
  if (activeEntry.value) {
    dataTable.value?.selectNextRow(event);
  } else {
    dataTable.value?.selectRow(event, 0);
  }
  event.preventDefault();
});

// ========== Filter BEGIN ==========

const tableFilters = ref<DataTableFilterMeta>({
  id: { value: undefined, matchMode: FilterMatchMode.IN },
});

watch(
  filter,
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
  <div class="flex flex-col h-full">
    <FilterPanel v-model="filter" :entries="entries" :tags="tags" />

    <DataTable
      ref="dataTable"
      :value="entries"
      v-model:selection="activeEntry"
      v-model:filters="tableFilters"
      dataKey="id"
      dragPreviewKey="fileName"
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
            slotProps.data.duration
              ? formatDuration(slotProps.data.duration)
              : ""
          }}</span>
        </template>
      </Column>
    </DataTable>
  </div>
</template>

<style scoped>
:deep(.viewed) {
  color: var(--p-surface-300);
}
</style>

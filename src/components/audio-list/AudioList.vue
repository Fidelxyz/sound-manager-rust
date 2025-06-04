<script setup lang="ts">
import { ref, useTemplateRef, watch } from "vue";

import { FilterMatchMode } from "@primevue/core/api";
import {
  Column,
  ContextMenu,
  type DataTableFilterMeta,
  type DataTableFilterMetaData,
  type DataTableRowContextMenuEvent,
  type DataTableRowSelectEvent,
  useConfirm,
} from "primevue";
import type { MenuItem } from "primevue/menuitem";
import type { TreeNode } from "primevue/treenode";
import DataTable from "./datatable";

import type { Entry, Filter } from "@/api";
import { api } from "@/api";
import { info } from "@/utils/message";
import { formatDuration } from "@/utils/utils";
import FilterPanel from "./FilterPanel.vue";

const confirm = useConfirm();

const { entries, tags } = defineProps<{
  entries: Entry[];
  tags: TreeNode[];
}>();

const filter = defineModel<Filter>("filter", { required: true });
const activeEntry = defineModel<Entry | null>("activeEntry", { default: null });

defineExpose({
  selectPrev,
  selectNext,
});

const dataTable = useTemplateRef("dataTable");

function selectPrev() {
  if (activeEntry.value) {
    dataTable.value?.selectPrevRow();
  } else {
    dataTable.value?.selectRow(undefined, 0);
  }
}

function selectNext() {
  if (activeEntry.value) {
    dataTable.value?.selectNextRow();
  } else {
    dataTable.value?.selectRow(undefined, 0);
  }
}

function onEntrySelected(event: DataTableRowSelectEvent) {
  const entry = event.data as Entry;
  console.debug("Select entry", entry);
  entry.viewed = true;
}

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

// ========== Context Menu BEGIN ==========

const contextMenu = useTemplateRef("contextMenu");
const contextMenuSelection = ref<Entry | null>(null);
const contextMenuItems: MenuItem[] = [
  {
    label: "删除",
    icon: "pi pi-trash",
    command: () => {
      if (contextMenuSelection.value) {
        deleteEntry(contextMenuSelection.value);
      }
    },
  },
];
function onRowContextMenu(event: DataTableRowContextMenuEvent) {
  contextMenu.value?.show(event.originalEvent);
}

function deleteEntry(entry: Entry) {
  confirm.require({
    header: "确认删除",
    message: `确定要删除 “${entry.fileName}” 吗？`,
    icon: "pi pi-trash",
    rejectProps: { label: "取消", severity: "secondary", outlined: true },
    acceptProps: { label: "删除", severity: "danger" },
    accept: () => confirmDeleteEntry(entry),
  });
}

function confirmDeleteEntry(entry: Entry) {
  if (activeEntry.value?.id === entry.id) {
    activeEntry.value = null;
  }

  api
    .deleteFile(entry.id)
    .then(() => {
      console.info("Deleted entry", entry);
      info("删除成功", `已删除 "${entry.fileName}。"`);
    })
    .catch((error) => {
      console.error("Failed to delete entry", entry, error);
      info("删除失败", `删除 "${entry.fileName}" 时出现错误：${error}。`);
    });
}

// ========== Context Menu END ==========
</script>

<template>
  <div class="flex h-full flex-col">
    <FilterPanel v-model="filter" :entries="entries" :tags="tags" />

    <div class="flex-auto overflow-hidden">
      <ContextMenu ref="contextMenu" :model="contextMenuItems" />
      <DataTable
        ref="dataTable"
        :value="entries"
        v-model:selection="activeEntry"
        v-model:contextMenuSelection="contextMenuSelection"
        v-model:filters="tableFilters"
        dataKey="id"
        dragPreviewKey="fileName"
        draggableType="entry"
        :rowClass="(data: Entry) => [{ viewed: data?.viewed }]"
        scrollable
        scrollHeight="flex"
        resizableColumns
        removableSort
        tableClass="table-fixed"
        selectionMode="single"
        :metaKeySelection="true"
        @rowSelect="onEntrySelected"
        @rowContextmenu="onRowContextMenu"
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
  </div>
</template>

<style scoped>
:deep(.viewed) {
  color: var(--p-surface-300);
}
</style>

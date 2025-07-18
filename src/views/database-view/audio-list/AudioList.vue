<script setup lang="ts">
import { $dt } from "@primeuix/themes";
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
import { ref, useTemplateRef, watch } from "vue";
import { api, type FilterArg } from "@/api";
import type { Entry, Filter, FolderNode, Tag } from "@/types";
import { info } from "@/utils/message";
import { formatDuration } from "@/utils/utils";
import DataTable from "./datatable";
import FilterPanel from "./FilterPanel.vue";

const confirm = useConfirm();

const { entries, folderTree, tags, tagTreeNodes } = defineProps<{
  entries: Entry[];
  folderTree: FolderNode | null;
  tags: Record<number, Tag>;
  tagTreeNodes: TreeNode[];
}>();

const activeEntry = defineModel<Entry | null>("activeEntry", { default: null });

defineExpose({
  selectPrev,
  selectNext,
});

const dataTable = useTemplateRef("dataTable");

function selectPrev() {
  dataTable.value?.selectPrevRow();
}

function selectNext() {
  dataTable.value?.selectNextRow();
}

function onEntrySelected(event: DataTableRowSelectEvent) {
  const entry = event.data as Entry;
  console.debug("Select entry", entry);
  entry.viewed = true;
}

// ========== Filter ==========

const filter = defineModel<Filter>("filter", { required: true });

const tableFilters = ref<DataTableFilterMeta>({
  id: { value: undefined, matchMode: FilterMatchMode.IN },
});

function toFilterArg(filter: Filter): FilterArg {
  return {
    search: filter.search,
    tagIds: filter.tags.map((tag) => tag.id),
    includeChildTags: filter.includeChildTags,
    noTags: filter.noTags,
    folderId: filter.folder ? filter.folder.id : null,
    includeSubfolders: filter.includeSubfolders,
  };
}

watch(
  [filter, () => entries],
  async ([filter, _]) => {
    const filterArg = toFilterArg(filter);
    console.debug("Applying filter", filter, filterArg);
    let entry_ids = await api.filter(filterArg);
    console.debug("Filtered entries", entry_ids);
    if (Array.isArray(entry_ids) && entry_ids.length === 0) {
      entry_ids = [-1];
    }
    (tableFilters.value.id as DataTableFilterMetaData).value = entry_ids;
  },
  { deep: true },
);

// ========== Context Menu ==========

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
  {
    label: "在文件管理器中显示",
    icon: "pi pi-folder-open",
    command: () => {
      if (contextMenuSelection.value) {
        api.revealEntry(contextMenuSelection.value.id);
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

// ========== Styling ==========

function rowStyle(data: Entry) {
  if (data.viewed) {
    return { color: $dt("surface.300").value.dark.value };
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <FilterPanel
      v-model="filter"
      :entries="entries"
      :folderTree="folderTree"
      :tags="tags"
      :tagTreeNodes="tagTreeNodes"
    />

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
        scrollable
        scrollHeight="flex"
        resizableColumns
        removableSort
        tableClass="table-fixed"
        selectionMode="single"
        :metaKeySelection="true"
        @rowSelect="onEntrySelected"
        @rowContextmenu="onRowContextMenu"
        :rowStyle="rowStyle"
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
        <Column class="w-1/6" field="artist" header="艺术家" sortable />
        <Column class="w-1/3" field="album" header="专辑" sortable />
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

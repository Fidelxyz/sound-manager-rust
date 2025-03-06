<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

import { DataTable, Column } from "primevue";

import type { Entry } from "../types";
import { formatDuration } from "../lib/utils";

const entries = ref<Entry[]>([]);
const selectedEntry = ref<Entry>();

const emit = defineEmits(["select"]);

async function loadEntries() {
  console.log("Load entries");
  invoke<Entry[]>("get_entries")
    .then((data) => {
      console.log(data);
      entries.value = data;
    })
    .catch((error) => {
      console.error(error);
    });
}

async function selectEntry(event: any) {
  emit("select", event.data);
}

defineExpose({
  refresh: loadEntries,
});

onMounted(() => {
  loadEntries();
});
</script>

<template>
  <DataTable
    scrollable
    scrollHeight="flex"
    resizableColumns
    class="h-full w-full text-nowrap"
    tableClass="table-fixed"
    :value="entries"
    v-model:selection="selectedEntry"
    selectionMode="single"
    :metaKeySelection="true"
    datakey="id"
    @rowSelect="selectEntry"
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
        <span>{{ slotProps.data.duration ? formatDuration(slotProps.data.duration) : "" }}</span>
      </template>
    </Column>
  </DataTable>
</template>

<style scoped>
:deep(.p-datatable-table-container) {
  overflow-x: hidden !important;
}
</style>

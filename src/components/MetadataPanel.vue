<script setup lang="ts">
import { ref, watch } from "vue";

import { Tag, InputText } from "primevue";

import { formatDuration } from "../lib/utils";
import { Entry } from "../types";

const props = defineProps<{
  entry?: Entry;
}>();

const emit = defineEmits(["update"]);

const activeEntry = ref({ ...props.entry });

watch(
  () => props.entry,
  (newEntry) => {
    if (newEntry) {
      activeEntry.value = { ...newEntry };
    }
  },
  { deep: true }
);
</script>

<template>
  <div class="metadata-panel w-full h-full px-6 py-8">
    <div v-if="entry">
      <InputText class="w-full" v-model="activeEntry.fileName" />
      <Tag
        v-for="tag in entry.tagIds"
        :key="tag"
        :value="tag"
        severity="info"
      />
      <table class="w-full border-separate border-spacing-y-2">
        <tbody>
          <tr class="metadata-row">
            <td class="metadata-label">标题</td>
            <td class="metadata-field">
              <InputText v-model="activeEntry.title" />
            </td>
          </tr>
          <tr class="metadata-row">
            <td class="metadata-label">艺术家</td>
            <td class="metadata-field">
              <InputText v-model="activeEntry.artist" />
            </td>
          </tr>
          <tr class="metadata-row">
            <td class="metadata-label">专辑</td>
            <td class="metadata-field">
              <InputText v-model="activeEntry.album" />
            </td>
          </tr>
          <tr class="metadata-row">
            <td class="metadata-label">时长</td>
            <td class="metadata-field">
              {{
                activeEntry.duration ? formatDuration(activeEntry.duration) : ""
              }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <div
      v-else
      class="flex items-center justify-center h-full text-surface-100"
    >
      未选择文件
    </div>
  </div>
</template>

<style scoped>
.metadata-panel {
  background-color: var(--p-surface-800);
}

.metadata-row {
  height: 2.4rem;
}

.metadata-label {
  width: 20%;
  color: var(--p-surface-400);
  padding-right: 1rem;
}

.metadata-field {
  color: var(--p-surface-200);
}

.metadata-field :deep(.p-inputtext) {
  width: 100%;
}

:deep(.p-inputtext) {
  color: var(--p-surface-50);
  border-color: transparent;
}

.tags-container {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}
</style>

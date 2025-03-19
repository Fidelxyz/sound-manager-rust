<script setup lang="ts">
import { ref } from "vue";

import { InputText } from "primevue";
import type { TreeNode } from "primevue/treenode";

import MetadataTagEditor from "./MetadataTagEditor.vue";
import { formatDuration } from "../utils/utils";
import type { Entry } from "../api";

const { entry, allTags } = defineProps<{
  entry?: Entry;
  allTags: TreeNode[];
}>();
const metadataTagEditor = ref();

const emit = defineEmits(["update"]);

function refresh() {
  metadataTagEditor.value?.refresh();
}

defineExpose({
  refresh: refresh,
});
</script>

<template>
  <div class="metadata-panel w-full h-full px-6 py-8">
    <div v-if="entry" class="metadata-content">
      <InputText class="w-full" v-model="entry.fileName" />

      <MetadataTagEditor
        ref="metadataTagEditor"
        :entry="entry"
        :allTags="allTags"
      />

      <table class="w-full border-separate border-spacing-y-2">
        <tbody>
          <tr class="metadata-row">
            <td class="metadata-label">标题</td>
            <td class="metadata-field">
              <InputText v-model="entry.title" />
            </td>
          </tr>
          <tr class="metadata-row">
            <td class="metadata-label">艺术家</td>
            <td class="metadata-field">
              <InputText v-model="entry.artist" />
            </td>
          </tr>
          <tr class="metadata-row">
            <td class="metadata-label">专辑</td>
            <td class="metadata-field">
              <InputText v-model="entry.album" />
            </td>
          </tr>
          <tr class="metadata-row">
            <td class="metadata-label">时长</td>
            <td class="metadata-field">
              {{ entry.duration ? formatDuration(entry.duration) : "" }}
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

.metadata-content > * {
  margin: 0.5rem 0;
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

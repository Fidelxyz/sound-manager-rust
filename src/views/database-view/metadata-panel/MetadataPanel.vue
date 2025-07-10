<script setup lang="ts">
import { InputText } from "primevue";
import type { TreeNode } from "primevue/treenode";
import { useTemplateRef } from "vue";
import type { Entry } from "@/types";
import { formatDuration } from "@/utils/utils";
import MetadataTagEditor from "./MetadataTagEditor.vue";

const { entry, tagTreeNodes } = defineProps<{
  entry?: Entry | null;
  tagTreeNodes: TreeNode[];
}>();
const metadataTagEditor = useTemplateRef("metadataTagEditor");

function refresh() {
  metadataTagEditor.value?.refresh();
}

defineExpose({
  refresh: refresh,
});
</script>

<template>
  <div class="bg-surface-800 h-full w-full px-6 py-8">
    <div v-if="entry" class="*:my-2">
      <InputText class="w-full" v-model="entry.fileName" />

      <MetadataTagEditor
        ref="metadataTagEditor"
        :entry="entry"
        :allTags="tagTreeNodes"
      />

      <table class="w-full border-separate border-spacing-y-2">
        <tbody class="*:h-[2.4rem]">
          <tr>
            <td class="metadata-label">标题</td>
            <td class="metadata-field">
              <InputText v-model="entry.title" />
            </td>
          </tr>
          <tr>
            <td class="metadata-label">艺术家</td>
            <td class="metadata-field">
              <InputText v-model="entry.artist" />
            </td>
          </tr>
          <tr>
            <td class="metadata-label">专辑</td>
            <td class="metadata-field">
              <InputText v-model="entry.album" />
            </td>
          </tr>
          <tr>
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
      class="text-surface-100 flex h-full items-center justify-center"
    >
      未选择文件
    </div>
  </div>
</template>

<style scoped>
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
</style>

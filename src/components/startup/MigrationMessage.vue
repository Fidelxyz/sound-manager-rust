<script setup lang="ts">
import { computed } from "vue";

import type { MigratorResult, MigratorLog } from "@/api";

import { Listbox, Button, Dialog, Tag } from "primevue";

const visible = defineModel<boolean>("visible");

const { result } = defineProps<{
  result?: MigratorResult;
}>();

const emit = defineEmits<{
  close: [];
}>();

const prompt = computed(() => {
  if (!result) return {};
  switch (result.success) {
    case true:
      return {
        header: "数据库迁移警告",
        message: "数据库迁移程序报告了一些警告，迁移后的数据可能不完整。",
      };
    case false:
      return {
        header: "数据库迁移错误",
        message: "数据库迁移失败。",
      };
  }
});
</script>

<template>
  <Dialog
    v-model:visible="visible"
    :header="prompt.header"
    class="w-6xl"
    :closable="false"
    modal
  >
    <div class="flex flex-col gap-4">
      <div>{{ prompt.message }}</div>
      <Listbox :options="result?.logs" scrollHeight="24rem">
        <template #option="{ option }: { option: MigratorLog }">
          <div class="flex gap-2">
            <Tag v-if="option.kind === 'warn'" severity="warn" value="警告" />
            <Tag
              v-else-if="option.kind === 'error'"
              severity="danger"
              value="错误"
            />
            <div>{{ option.message }}</div>
          </div>
        </template>
      </Listbox>
    </div>

    <div class="flex justify-end mt-6">
      <Button label="确认" icon="pi pi-check" @click="emit('close')" />
    </div>
  </Dialog>
</template>

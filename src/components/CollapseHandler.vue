<script setup lang="ts">
const expanded = defineModel<boolean>({ required: true });
const { hidden = false } = defineProps<{
  hidden: boolean;
}>();

function onClick(event: MouseEvent) {
  expanded.value = !expanded.value;
  event.stopPropagation();
}
</script>

<template>
  <button
    type="button"
    class="cursor-pointer"
    :class="{ invisible: hidden }"
    @click="onClick"
  >
    <i v-if="expanded" class="pi pi-chevron-down" />
    <i v-else class="pi pi-chevron-right" />
  </button>
</template>

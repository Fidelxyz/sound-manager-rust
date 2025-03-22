<script setup lang="ts">
import { computed } from "vue";

// TODO: Update default values
const {
  strokeColor = "#579DFF",
  strokeWidth = "2px",
  borderRadius = "3px", // TODO: update to border.radius (4px) token
  indent = "0px",
} = defineProps<{
  strokeColor?: string;
  strokeWidth?: string;
  borderRadius?: string;
  indent?: string;
}>();

// ========== Computed ==========

const styleVariables = computed(() => ({
  "--stroke-color": strokeColor,
  "--stroke-width": strokeWidth,
  "--border-radius": borderRadius,
  "--indent": indent,
}));
</script>

<template>
  <div class="drop-indicator-outline" :style="styleVariables" />
</template>

<style scoped>
.drop-indicator-outline {
  /* To make things a bit clearer we are making the box that the indicator in as
	big as the whole tree item */
  position: absolute;
  inset-block-start: 0;
  inset-block-end: 0;
  inset-inline-end: 0;
  inset-inline-start: var(--indent);

  /* We don't want to cause any additional 'dragenter' events */
  pointer-events: none;

  border: var(--stroke-width) solid var(--stroke-color);
  border-radius: var(--border-radius);
}
</style>

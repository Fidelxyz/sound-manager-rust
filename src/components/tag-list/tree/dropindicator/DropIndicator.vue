<script setup lang="ts">
import { computed } from "vue";

import type { Instruction } from "@atlaskit/pragmatic-drag-and-drop-hitbox/tree-item";

import Line from "./Line.vue";
import Outline from "./Outline.vue";

const { instruction } = defineProps<{
  instruction: Instruction | null;
}>();

// TODO strokeColor is determined by isBlocked
const isBlocked = computed(() => {
  return instruction?.type === "instruction-blocked";
});
const indent = computed(() => {
  switch (instruction?.type) {
    case "reorder-above":
    case "reorder-below":
    case "make-child":
      return `${instruction.currentLevel * instruction.indentPerLevel}px`;
    case "reparent":
      return `${instruction.desiredLevel * instruction.indentPerLevel}px`;
  }
});
</script>

<template>
  <Line
    v-if="instruction?.type === 'reorder-above'"
    edge="top"
    :indent="indent"
  />
  <Line
    v-if="instruction?.type === 'reorder-below'"
    edge="bottom"
    :indent="indent"
  />
  <Outline v-if="instruction?.type === 'make-child'" :indent="indent" />
  <Line
    v-if="instruction?.type === 'reparent'"
    edge="bottom"
    :indent="`${instruction.desiredLevel * instruction.indentPerLevel}px`"
    stroke
  />
</template>

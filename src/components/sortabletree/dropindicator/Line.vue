<script setup lang="ts">
import { computed } from "vue";

import type { Edge } from "@atlaskit/pragmatic-drag-and-drop-hitbox/types";

const {
  edge,
  gap = "0px",
  indent = "0px",
  strokeColor = "#579DFF",
  strokeWidth = "2px",
  type = "terminal",
} = defineProps<{
  edge: Edge;
  gap?: string;
  indent?: string;
  strokeColor?: string;
  strokeWidth?: string;
  type?: LineType;
}>();

// ========== Types ==========

type Orientation = "horizontal" | "vertical";

const edgeToOrientationMap: Record<Edge, Orientation> = {
  top: "horizontal",
  bottom: "horizontal",
  left: "vertical",
  right: "vertical",
};

type LineType = "terminal" | "no-terminal" | "terminal-no-bleed";

const lineStartFrom: {
  [TKey in LineType]: ({ indent }: { indent: string }) => string;
} = {
  // - half the terminal bleeding out the containing element
  // - half the terminal inside the containing element (we need to position the line next to this)
  terminal: ({ indent }) => `calc(var(--terminal-radius) + ${indent})`,

  // The full terminal is inside the containing element (we need to position the line next to this)
  "terminal-no-bleed": ({ indent }) =>
    `calc(var(--terminal-diameter) + ${indent})`,

  // No terminal to worry about, line should take up all the space
  "no-terminal": ({ indent }) => indent,
};

// ========== Computed ==========

const orientation = computed(() => edgeToOrientationMap[edge]);

const styleVariables = computed(() => ({
  "--stroke-color": strokeColor,
  "--stroke-width": strokeWidth,
  // Shift line and terminal on the main access to account for gaps between items
  "--main-axis-offset": `calc(-0.5 * (${gap} + var(--stroke-width)))`,

  // ## Line
  // If there is a terminal, we want the line to start from next to it
  "--line-main-axis-start": lineStartFrom[type]({ indent }),

  // ## Terminal
  "--terminal-display": type === "no-terminal" ? "none" : "block",
  "--terminal-diameter": "calc(var(--stroke-width) * 4)",
  "--terminal-radius": "calc(var(--terminal-diameter) / 2)",

  // The line is positioned to account for the the terminal (--line-main-axis-start).
  // The terminal is rendered relative to the line (it's a `::before`)
  // We need to pull the terminal backwards so it sits before the start of the line
  "--terminal-main-axis-start": "calc(-1 * var(--terminal-diameter))",

  // Pull the terminal backwards on the cross axis (eg 'up' on 'vertical')
  // so the center of the terminal lines up with the center of the line
  "--terminal-cross-axis-offset":
    "calc(calc(var(--stroke-width) - var(--terminal-diameter)) / 2)",
}));
</script>

<template>
  <div
    class="drop-indicator-line"
    :class="`drop-indicator-${orientation} drop-indicator-${edge}`"
    :style="styleVariables"
  />
</template>

<style scoped>
.drop-indicator-line {
  display: block;
  position: absolute;
  z-index: 1;
  /* Blocking pointer events to prevent the line from triggering drag events */
  pointer-events: none;
  background-color: var(--stroke-color);
}
.drop-indicator-line::before {
  display: var(--terminal-display);
  content: "";
  position: absolute;
  box-sizing: border-box;
  width: var(--terminal-diameter);
  height: var(--terminal-diameter);
  border-width: var(--stroke-width);
  border-style: solid;
  border-color: var(--stroke-color);
  border-radius: 50%;
}

/* Orientation Styles */
.drop-indicator-horizontal {
  height: var(--stroke-width);
  inset-inline-start: var(--line-main-axis-start);
  inset-inline-end: 0;
}
.drop-indicator-horizontal::before {
  inset-inline-start: var(--terminal-main-axis-start);
}

/* For now, vertical lines will always have the terminal on the top.
   Need to investigate whether we want the terminal on the bottom
   for bottom to top languages. */
.drop-indicator-vertical {
  width: var(--stroke-width);
  top: var(--line-main-axis-start);
  bottom: 0;
}
.drop-indicator-vertical::before {
  top: var(--terminal-main-axis-start);
}

/* Edge Styles */
.drop-indicator-top {
  top: var(--main-axis-offset);
}
.drop-indicator-top::before {
  top: var(--terminal-cross-axis-offset);
}

.drop-indicator-right {
  right: var(--main-axis-offset);
}
.drop-indicator-right::before {
  right: var(--terminal-cross-axis-offset);
}

.drop-indicator-bottom {
  bottom: var(--main-axis-offset);
}
.drop-indicator-bottom::before {
  bottom: var(--terminal-cross-axis-offset);
}

.drop-indicator-left {
  left: var(--main-axis-offset);
}
.drop-indicator-left::before {
  left: var(--terminal-cross-axis-offset);
}
</style>

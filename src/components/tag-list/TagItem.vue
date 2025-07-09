<script setup lang="ts">
import { combine } from "@atlaskit/pragmatic-drag-and-drop/combine";
import {
  draggable,
  dropTargetForElements,
} from "@atlaskit/pragmatic-drag-and-drop/element/adapter";
import {
  attachInstruction,
  extractInstruction,
  Instruction,
} from "@atlaskit/pragmatic-drag-and-drop-hitbox/tree-item";
import { unrefElement } from "@vueuse/core";
import { Button, InputText } from "primevue";
import { type Component, computed, ref, useTemplateRef, watch } from "vue";
import { api } from "@/api";
import type { DropTargetData, Tag, TagNode } from "@/types";
import { useDragAndDrop } from "@/utils/drag-and-drop";
import { error } from "@/utils/message";
import DropIndicator from "../dropindicator/DropIndicator.vue";

const { tagNode, lastInGroup } = defineProps<{
  tagNode: TagNode;
  lastInGroup: boolean;
}>();
const selectedTags = defineModel<Tag[]>("selectedTags", {
  required: true,
});
const emit = defineEmits<{
  contextmenu: [event: MouseEvent, target: Tag];
  "tags-changed": [];
}>();

const tag = computed(() => tagNode.tag);

// ========== Expansion ==========

const expanded = ref(true);

function toggle() {
  expanded.value = !expanded.value;
}

// ========== Editing ==========

const editingInput = useTemplateRef<Component>("editingInput");
const editingTag = defineModel<Tag | null>("editingTag", { required: true });
const editing = computed(() => editingTag.value === tag.value);
const editingName = ref("");

watch(editing, (isEditing) => {
  if (isEditing) {
    editingName.value = tag.value.name;
    setTimeout(() => {
      if (editingInput.value) {
        unrefElement(editingInput.value).focus();
      }
    }, 0);
  }
});

function renameTag() {
  if (editingName.value.trim() === "" || editingName.value === tag.value.name) {
    editingTag.value = null;
    return;
  }

  api
    .renameTag(tag.value.id, editingName.value)
    .then(() => emit("tags-changed"))
    .catch((e) => {
      console.error(e);
      if (e.kind === "tagAlreadyExists") {
        error("标签已存在", "请使用其他名称");
      } else {
        error("重命名标签错误", e.message);
      }
    })
    .finally(() => {
      editingTag.value = null;
    });
}

// ========== Drag and Drop ==========

const nodeContent = useTemplateRef<HTMLElement>("nodeContent");

const dragging = ref(false);
const dropTargetInstruction = ref<Instruction | null>(null);

useDragAndDrop(() => {
  if (!nodeContent.value) return () => {};
  return combine(
    draggable({
      element: nodeContent.value,
      getInitialData: () => {
        const data: DropTargetData = {
          type: "tag",
          tagNode,
        };
        return data;
      },
      onDragStart: () => {
        dragging.value = true;
      },
      onDrop: () => {
        dragging.value = false;
      },
    }),
    dropTargetForElements({
      element: nodeContent.value,
      getData: ({ input, element }) => {
        const data: DropTargetData = {
          type: "tag",
          tagNode,
        };
        return attachInstruction(data, {
          input,
          element,
          currentLevel: 1,
          indentPerLevel: 0,
          mode: (() => {
            if (tagNode.children.length > 0 && expanded.value) {
              return "expanded";
            } else if (lastInGroup) {
              return "last-in-group";
            } else {
              return "standard";
            }
          })(),
        });
      },
      canDrop: ({ source }) => {
        const sourceData = source.data as DropTargetData;
        switch (sourceData.type) {
          case "tag":
            return (
              sourceData.tagNode.tag.id !== tag.value.id &&
              sourceData.tagNode.tag.parentId !== tag.value.id
            );
          case "entry":
            return true;
          default:
            return false;
        }
      },
      onDrag: ({ self, source }) => {
        const sourceData = source.data as DropTargetData;
        dropTargetInstruction.value =
          sourceData.type === "tag"
            ? extractInstruction(self.data)
            : { type: "make-child", currentLevel: 1, indentPerLevel: 0 };
      },
      onDragLeave: () => {
        dropTargetInstruction.value = null;
      },
      onDrop: () => {
        dropTargetInstruction.value = null;
      },
    }),
  );
});
</script>

<template>
  <!-- Node Content -->
  <div ref="nodeContent" class="relative">
    <Button
      variant="text"
      class="w-full justify-start!"
      :class="{
        active: selectedTags.includes(tag),
        'opacity-50': dragging,
        'py-0! pr-0!': editing,
      }"
      @contextmenu="emit('contextmenu', $event, tag)"
    >
      <!-- Toggle Button -->
      <button
        type="button"
        class="flex cursor-pointer items-center"
        :class="{ invisible: tagNode.children.length === 0 }"
        @click="toggle"
      >
        <i v-if="expanded" class="pi pi-chevron-down" />
        <i v-else class="pi pi-chevron-right" />
      </button>

      <!-- Tag Icon -->
      <i class="pi pi-tag" :class="`tag-color-${tag.color}`" />

      <InputText
        ref="editingInput"
        v-if="editing"
        class="input-text w-full"
        style="
          padding-inline: calc(var(--p-inputtext-padding-x) - 1px);
          padding-block: calc(var(--p-inputtext-padding-y) - 1px);
        "
        v-model="editingName"
        @focusout="renameTag"
        @keydown.enter="renameTag"
      />
      <span v-else class="text-surface-100">{{ tag.name }}</span>
    </Button>

    <DropIndicator
      v-if="dropTargetInstruction"
      :instruction="dropTargetInstruction"
    />
  </div>

  <!-- Children -->
  <ul v-if="tagNode.children && expanded && !dragging" class="pl-4">
    <li>
      <TagItem
        v-for="(child, index) in tagNode.children"
        :tagNode="child"
        :lastInGroup="index === tagNode.children.length - 1"
        v-model:selectedTags="selectedTags"
        v-model:editingTag="editingTag"
        @contextmenu="(event, target) => emit('contextmenu', event, target)"
        @tags-changed="emit('tags-changed')"
      />
    </li>
  </ul>
</template>

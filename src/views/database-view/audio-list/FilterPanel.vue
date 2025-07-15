<script setup lang="ts">
import { faShuffle } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/vue-fontawesome";
import {
  Button,
  FloatLabel,
  InputText,
  SelectButton,
  ToggleButton,
  type TreeSelectionKeys,
} from "primevue";
import type { TreeNode } from "primevue/treenode";
import { computed, ref } from "vue";
import { api } from "@/api";
import type { Entry, Filter, FolderNode, Tag } from "@/types";
import { error } from "@/utils/message";
import { useWindowDragRegion } from "@/utils/window-drag-region";
import TreeSelect from "./treeselect";

const filter = defineModel<Filter>({ required: true });

const { entries, folderTree, tags, tagTreeNodes } = defineProps<{
  entries: Entry[];
  folderTree: FolderNode | null;
  tags: Record<number, Tag>;
  tagTreeNodes: TreeNode[];
}>();

const selectedTags = computed({
  get: () => {
    const selectionKeys: TreeSelectionKeys = {};
    for (const tag of filter.value.tags) {
      selectionKeys[tag.id] = { checked: true, partialChecked: false };

      let parentTag = tags[tag.parentId];
      while (parentTag.id !== -1) {
        if (selectionKeys[parentTag.id]) break;
        selectionKeys[parentTag.id] = { checked: false, partialChecked: true };
        parentTag = tags[parentTag.parentId];
      }
    }

    console.debug("Get selected tags", selectionKeys);
    return selectionKeys;
  },
  set: (selectionKeys: TreeSelectionKeys) => {
    console.debug("Selected tags changed", selectionKeys);
    const filteredTags: Tag[] = [];
    for (const [tagId, state] of Object.entries(selectionKeys)) {
      if (state.checked) {
        filteredTags.push(tags[Number.parseInt(tagId)]);
      }
    }
    console.debug("Selected tags:", filteredTags);
    filter.value.tags = filteredTags;
  },
});

// ========== Filter By Tags Options BEGIN ==========

enum FilterByTagsOption {
  IncludeChildTags = 0,
  NoTags = 1,
}

const filterByTagsOptions = computed<
  { label: string; value: FilterByTagsOption; disabled: boolean }[]
>(() => [
  {
    label: "包含子标签",
    value: FilterByTagsOption.IncludeChildTags,
    disabled: filter.value.noTags,
  },
  {
    label: "无标签",
    value: FilterByTagsOption.NoTags,
    disabled: false,
  },
]);

const filterByTagsOptionValues = computed({
  get: () => {
    const options: FilterByTagsOption[] = [];
    if (filter.value.includeChildTags) {
      options.push(FilterByTagsOption.IncludeChildTags);
    }
    if (filter.value.noTags) {
      options.push(FilterByTagsOption.NoTags);
    }
    return options;
  },
  set: (value: FilterByTagsOption[]) => {
    filter.value.includeChildTags = value.includes(
      FilterByTagsOption.IncludeChildTags,
    );
    filter.value.noTags = value.includes(FilterByTagsOption.NoTags);
  },
});

const filterEnabled = computed(() => {
  return (
    filter.value.search.length > 0 ||
    filter.value.tags.length > 0 ||
    filter.value.noTags ||
    filter.value.folder !== null
  );
});

// ========== Filter By Tags Options END ==========

function clearFilter() {
  filter.value.search = "";
  filter.value.tags = [];
  filter.value.noTags = false;
  filter.value.folder = null;
}

function shuffle() {
  for (let i = entries.length - 1; i >= 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [entries[i], entries[j]] = [entries[j], entries[i]];
  }
}

const refreshing = ref(false);

function refresh() {
  console.debug("Refreshing");
  refreshing.value = true;
  api
    .refresh()
    .catch((e) => {
      error("刷新失败", e.message);
    })
    .finally(() => {
      refreshing.value = false;
    });
}

useWindowDragRegion("input, a, button, .p-inputwrapper");
</script>

<template>
  <div class="flex gap-4 p-4">
    <div class="flex items-center gap-4">
      <div class="flex items-center gap-2">
        <h2 class="font-semibold">
          {{ filter.folder?.name ?? folderTree?.folder.name }}
        </h2>
        <ToggleButton
          v-model="filter.includeSubfolders"
          onLabel="包含子文件夹"
          offLabel="不包含子文件夹"
          size="small"
          :pt="{
            content: {
              class: 'px-2!',
            },
          }"
        />
      </div>

      <FloatLabel variant="on">
        <InputText
          id="filter-search"
          v-model="filter.search"
          autocapitalize="off"
          class="w-64"
        />
        <label for="filter-search">搜索</label>
      </FloatLabel>

      <div class="flex items-center gap-2">
        <FloatLabel variant="on">
          <TreeSelect
            id="filter-tags"
            class="w-48"
            v-model="selectedTags"
            :options="tagTreeNodes"
            selectionMode="checkbox"
            emptyMessage="无可用标签"
            :disabled="filter.noTags"
          />
          <label for="filter-tags">标签</label>
        </FloatLabel>
        <SelectButton
          v-model="filterByTagsOptionValues"
          :options="filterByTagsOptions"
          optionLabel="label"
          optionValue="value"
          optionDisabled="disabled"
          multiple
          size="small"
          :pt="{
            pcToggleButton: {
              content: {
                class: 'px-2!',
              },
            },
          }"
        />
      </div>

      <Button
        v-if="filterEnabled"
        icon="pi pi-times"
        aria-label="清除过滤器"
        variant="text"
        size="small"
        rounded
        @click="clearFilter"
      />
    </div>

    <div class="ml-auto flex justify-end gap-2 text-nowrap">
      <Button label="打乱" severity="secondary" size="small" @click="shuffle">
        <template #icon>
          <FontAwesomeIcon :icon="faShuffle" />
        </template>
      </Button>

      <Button
        icon="pi pi-sync"
        label="刷新"
        severity="secondary"
        size="small"
        :loading="refreshing"
        @click="refresh"
      />
    </div>
  </div>
</template>

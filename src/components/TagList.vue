<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

import { Button } from "primevue";

import { Tag } from "../types";

const tags = ref<Tag[]>([]);
const selectedTagId = ref<number>();

const emit = defineEmits(["select"]);

async function loadTags() {
  console.log("Load tags");
  invoke<Tag[]>("get_folder")
    .then((data) => {
      console.log(data);
      tags.value = data;
    })
    .catch((error) => {
      console.error(error);
    });
}

async function selectTag(tagId: number) {
  if (selectedTagId.value === tagId) {
    selectedTagId.value = undefined;
    emit("select", undefined);
    return;
  }
  selectedTagId.value = tagId;
  emit("select", tagId);
}

defineExpose({
  refresh: loadTags,
});

onMounted(() => {
  loadTags();
});
</script>

<template>
  <div class="w-full h-full p-8">
    <div class="font-bold p-2">标签</div>
    <ul>
      <li
        v-for="tag in tags"
        :key="tag.id"
        :class="{ active: selectedTagId === tag.id }"
        @click="selectTag(tag.id)"
      >
        <Button variant="text">{{ tag.name }}</Button>
      </li>
    </ul>
  </div>
</template>

<style scoped></style>

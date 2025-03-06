<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import { Button } from "primevue";

import type { ErrorKind } from "../types";

const emit = defineEmits(["database-loaded"]);

const loading = ref(false);

async function handleOpenDatabase() {
  console.log("Open Database");
  const path = await open({
    multiple: false,
    directory: true,
  });
  if (!path) return;

  loading.value = true;
  invoke("open_database", {
    path: path,
  })
    .then(() => {
      emit("database-loaded");
    })
    .catch((error: ErrorKind) => {
      console.error(error);
    })
    .finally(() => {
      loading.value = false;
    });
}

async function handleCreateDatabase() {
  console.log("Create Database");
  const path = await open({
    multiple: false,
    directory: true,
  });
  if (!path) return;

  invoke("create_database", {
    path: path,
  })
    .then(() => {
      emit("database-loaded");
    })
    .catch((error: ErrorKind) => {
      console.error(error);
    })
    .finally(() => {
      loading.value = false;
    });
}
</script>

<template>
  <div class="flex items-center justify-center h-full">
    <div class="*:m-1">
      <Button
        label="打开数据库"
        :loading="loading"
        @click="handleOpenDatabase"
      />
      <Button
        label="创建数据库"
        :loading="loading"
        @click="handleCreateDatabase"
      />
    </div>
  </div>
</template>

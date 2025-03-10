<script setup lang="ts">
import { ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "primevue";
import { api } from "../api";

import type { ErrorKind } from "../types";
import { error } from "../utils/message";

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
  api
    .openDatabase(path)
    .then(() => {
      emit("database-loaded");
    })
    .catch((e: ErrorKind) => {
      console.error(e);
      if (e.kind === "databaseNotFound") {
        error("数据库不存在", "请创建数据库");
      } else {
        error("打开数据库错误", e.message);
      }
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

  loading.value = true;
  api
    .createDatabase(path)
    .then(() => {
      emit("database-loaded");
    })
    .catch((e: ErrorKind) => {
      console.error(e);
      if (e.kind === "databaseAlreadyExists") {
        error("数据库已存在", "请打开数据库");
      } else {
        error("创建数据库错误", e.message);
      }
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

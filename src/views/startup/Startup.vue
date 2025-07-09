<script setup lang="ts">
import { ref } from "vue";

import { open } from "@tauri-apps/plugin-dialog";

import { Button, useConfirm } from "primevue";

import type { ErrorKind, MigrateFrom, MigratorResult } from "@/api";
import { api } from "@/api";
import { error } from "@/utils/message";
import { basename } from "@tauri-apps/api/path";
import MigrationMessage from "./MigrationMessage.vue";

const emit = defineEmits<{
  "database-loaded": [path: string];
}>();

const loading = ref(false);

const confirm = useConfirm();

async function openDatabase(path?: string) {
  console.info("Opening Database");

  const path_ =
    path ||
    (await open({
      title: "打开数据库",
      multiple: false,
      directory: true,
      recursive: true,
    }));
  if (!path_) return;

  loading.value = true;
  api
    .openDatabase(path_)
    .then(() => {
      emit("database-loaded", path_);
    })
    .catch(async (e: ErrorKind) => {
      if (e.kind === "databaseNotFound") {
        console.warn(e);

        const folder_name = await basename(path_);
        confirm.require({
          header: "数据库不存在",
          message: `是否为 ${folder_name} 创建数据库？`,
          icon: "pi pi-question-circle",
          rejectProps: { label: "取消", severity: "secondary", outlined: true },
          acceptProps: { label: "创建数据库", severity: "success" },
          accept: () => createDatabase(path_),
        });
      } else {
        console.error(e);
        error("打开数据库错误", e.message);
      }
    })
    .finally(() => {
      loading.value = false;
    });
}

async function createDatabase(path?: string) {
  console.info("Creating Database");

  const path_ =
    path ||
    (await open({
      title: "选择创建数据库文件夹",
      multiple: false,
      directory: true,
      recursive: true,
    }));
  if (!path_) return;

  loading.value = true;
  api
    .createDatabase(path_)
    .then(() => {
      emit("database-loaded", path_);
    })
    .catch((e: ErrorKind) => {
      console.error(e);
      if (e.kind === "databaseAlreadyExists") {
        console.warn(e);
        onDatabaseExists(path_);
      } else {
        error("创建数据库错误", e.message);
      }
    })
    .finally(() => {
      loading.value = false;
    });
}

async function onDatabaseExists(path: string) {
  const folder_name = await basename(path);
  confirm.require({
    header: "数据库已存在",
    message: `是否打开数据库 ${folder_name} ？`,
    icon: "pi pi-question-circle",
    rejectProps: { label: "取消", severity: "secondary", outlined: true },
    acceptProps: { label: "打开数据库", severity: "success" },
    accept: () => openDatabase(path),
  });
}

// ========== Migration ==========

const migrationMessageVisible = ref(false);
const migratorResult = ref<MigratorResult>();
const migratedPath = ref<string>();

async function migrateFrom(from: MigrateFrom) {
  console.info("Migrating From Billfish");
  const path = await open({
    multiple: false,
    directory: true,
    recursive: true,
  });
  if (!path) return;

  loading.value = true;

  const result = await api.migrateDatabase(path, from).catch((e: ErrorKind) => {
    console.error(e);
    onDatabaseExists(path);
  });
  if (!result) {
    loading.value = false;
    return;
  }

  migratorResult.value = result;
  migratedPath.value = path;

  // show migration message if there are any logs
  if (migratorResult.value.logs.length > 0) {
    showMigrationMessage();
  } else {
    const databasePath = migratedPath.value;
    await api
      .openDatabase(databasePath)
      .then(() => {
        emit("database-loaded", databasePath);
      })
      .catch((e: ErrorKind) => {
        console.error(e);
        error("打开数据库错误", e.message);
      });
  }

  loading.value = false;
}

function showMigrationMessage() {
  migrationMessageVisible.value = true;
}

function closeMigrationMessage() {
  migrationMessageVisible.value = false;
  if (migratorResult.value?.success && migratedPath.value) {
    loading.value = true;
    const databasePath = migratedPath.value;
    api
      .openDatabase(databasePath)
      .then(() => {
        emit("database-loaded", databasePath);
      })
      .catch((e: ErrorKind) => {
        console.error(e);
        error("打开数据库错误", e.message);
      })
      .finally(() => {
        loading.value = false;
      });
  }
}
</script>

<template>
  <div class="flex h-full items-center justify-center gap-2">
    <Button
      label="打开数据库"
      icon="pi pi-folder-open"
      :loading="loading"
      @click="openDatabase()"
    />
    <Button
      label="创建数据库"
      icon="pi pi-folder-plus"
      :loading="loading"
      @click="createDatabase()"
    />
    <Button
      label="从 Billfish 迁移"
      icon="pi pi-file-import"
      :loading="loading"
      @click="migrateFrom('billfish')"
    />
  </div>

  <MigrationMessage
    v-model:visible="migrationMessageVisible"
    :result="migratorResult"
    @close="closeMigrationMessage"
  />
</template>

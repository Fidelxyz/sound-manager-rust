<script setup lang="ts">
import { Menu, PredefinedMenuItem, Submenu } from "@tauri-apps/api/menu";
import { open } from "@tauri-apps/plugin-dialog";
import { onMounted, onUnmounted, ref } from "vue";

import { ConfirmDialog, Toast } from "primevue";

import DatabaseView from "./components/DatabaseView.vue";
import Startup from "./components/startup/Startup.vue";

import type { ErrorKind } from "@/api";
import { api } from "@/api";
import { error } from "@/utils/message";

// state
const databaseOpen = ref(false);
const databasePath = ref<string>();

onMounted(async () => {
  const titleSubmenu = await Submenu.new({
    text: "Sound Manager",
    items: [
      await PredefinedMenuItem.new({
        text: "服务",
        item: "Services",
      }),
      await PredefinedMenuItem.new({
        item: "Separator",
      }),
      await PredefinedMenuItem.new({
        text: "隐藏 Sound Manager",
        item: "Hide",
      }),
      await PredefinedMenuItem.new({
        text: "隐藏其他",
        item: "HideOthers",
      }),
      await PredefinedMenuItem.new({
        text: "全部显示",
        item: "ShowAll",
      }),
      await PredefinedMenuItem.new({
        item: "Separator",
      }),
      await PredefinedMenuItem.new({
        text: "退出 Sound Manager",
        item: "Quit",
      }),
    ],
  });
  const fileSubmenu = await Submenu.new({
    text: "文件",
    items: [
      {
        id: "open",
        text: "打开数据库",
        action: openDatabase,
      },
      {
        id: "create",
        text: "创建数据库",
        action: createDatabase,
      },
      {
        id: "refresh",
        text: "刷新",
        action: refresh,
      },
    ],
  });
  const menu = await Menu.new({
    items: [titleSubmenu, fileSubmenu],
  });
  menu.setAsAppMenu();

  // make sure database unloaded after refresh
  closeDatabase();
});

onUnmounted(() => {
  closeDatabase();
});

async function openDatabase() {
  console.info("Opening Database");
  const path = await open({
    multiple: false,
    directory: true,
    recursive: true,
  });
  if (!path) return;

  if (databaseOpen.value) {
    await api.closeDatabase();
    databaseOpen.value = false;
  }

  api
    .openDatabase(path)
    .then(() => {
      onDatabaseLoaded(path);
    })
    .catch((e: ErrorKind) => {
      console.error(e);
      if (e.kind === "databaseNotFound") {
        error("数据库不存在", "请创建数据库");
      } else {
        error("打开数据库错误", e.message);
      }
    });
}

async function createDatabase() {
  console.info("Creating Database");
  const path = await open({
    multiple: false,
    directory: true,
    recursive: true,
  });
  if (!path) return;

  if (databaseOpen.value) {
    await api.closeDatabase();
    databaseOpen.value = false;
  }

  api
    .createDatabase(path)
    .then(() => {
      onDatabaseLoaded(path);
    })
    .catch((e: ErrorKind) => {
      console.error(e);
      if (e.kind === "databaseAlreadyExists") {
        error("数据库已存在", "请打开数据库");
      } else {
        error("创建数据库错误", e.message);
      }
    });
}

function closeDatabase() {
  api.closeDatabase().catch((e) => {
    console.error(e);
    error("关闭数据库失败", e.message);
  });
}

function refresh() {
  console.debug("Refreshing");
  api.refresh().catch((e) => {
    error("刷新失败", e.message);
  });
}

function onDatabaseLoaded(path: string) {
  console.info("Database loaded");
  databaseOpen.value = true;
  databasePath.value = path;
}
</script>

<template>
  <main class="container max-w-none h-screen">
    <Toast />
    <ConfirmDialog />

    <DatabaseView v-if="databaseOpen" :basePath="databasePath" />
    <Startup v-else @database-loaded="onDatabaseLoaded" />
  </main>
</template>

<style>
@import "tailwindcss";
@import "tailwindcss-primeui";
@import "primeicons/primeicons.css";
@import "./assets/variables.css";
@import "./assets/global.css";

html {
  user-select: none !important;
  font-size: 14px;
  font-weight: 300;
}

body {
  overflow: hidden;
}

*::-webkit-scrollbar {
  width: 8px;
}

*::-webkit-scrollbar-track {
  background: transparent;
}

*::-webkit-scrollbar-thumb {
  background: var(--p-surface-500);
  border-radius: 4px;
}

/* Hide outline when pressing arrow keys */
.p-splitterpanel {
  outline: none;
}
</style>

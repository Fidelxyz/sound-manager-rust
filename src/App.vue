<script setup lang="ts">
import { Menu, type MenuItem, type Submenu } from "@tauri-apps/api/menu";
import { open } from "@tauri-apps/plugin-dialog";
import { onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";

import { ConfirmDialog, Toast } from "primevue";

import DatabaseView from "./components/DatabaseView.vue";
import Startup from "./components/startup/Startup.vue";

import type { ErrorKind } from "@/api";
import { api } from "@/api";
import { error } from "@/utils/message";

// refs
const databaseView = useTemplateRef("databaseView");

// state
const databaseOpen = ref(false);
const databasePath = ref<string | null>(null);

/// ========== Menu BEGIN ==========

async function setupMenu() {
  const menu = await Menu.new({
    items: [
      {
        // ===== Application menu =====
        id: "app",
        text: "Sound Manager",
        items: [
          {
            text: "服务",
            item: "Services",
          },
          { item: "Separator" },
          {
            text: "隐藏 Sound Manager",
            item: "Hide",
          },
          {
            text: "隐藏其他",
            item: "HideOthers",
          },
          {
            text: "全部显示",
            item: "ShowAll",
          },
          { item: "Separator" },
          {
            text: "退出 Sound Manager",
            item: "Quit",
          },
        ],
      },
      {
        // ===== File menu =====
        id: "file",
        text: "文件",
        items: [
          {
            text: "打开数据库",
            action: openDatabase,
            accelerator: "Cmd+O",
          },
          {
            text: "创建数据库",
            action: createDatabase,
            accelerator: "Cmd+N",
          },
          { item: "Separator" },
          {
            id: "refresh",
            text: "刷新",
            action: refresh,
            accelerator: "Cmd+R",
            enabled: databaseOpen.value,
          },
          {
            id: "spot",
            text: "发送至…",
            accelerator: "S",
            enabled: false,
          },
        ],
      },
      {
        // ===== Edit menu =====
        id: "edit",
        text: "编辑",
        items: [
          {
            text: "撤销",
            item: "Undo",
          },
          {
            text: "重做",
            item: "Redo",
          },
          { item: "Separator" },
          {
            text: "剪切",
            item: "Cut",
          },
          {
            text: "复制",
            item: "Copy",
          },
          {
            text: "粘贴",
            item: "Paste",
          },
          {
            text: "全选",
            item: "SelectAll",
          },
        ],
      },
      {
        // ===== Select menu =====
        id: "select",
        text: "选择",
        items: [
          {
            id: "previous",
            text: "上一个",
            action: () => databaseView.value?.audioList?.selectPrev(),
            accelerator: "ArrowUp",
            enabled: databaseOpen.value,
          },
          {
            id: "next",
            text: "下一个",
            action: () => databaseView.value?.audioList?.selectNext(),
            accelerator: "ArrowDown",
            enabled: databaseOpen.value,
          },
        ],
      },
    ],
  });
  menu.setAsAppMenu();
  return menu;
}
let menuItems: Record<"refresh" | "previous" | "next" | "spot", MenuItem>;
setupMenu().then(async (menu) => {
  const fileSubmenu = (await menu.get("file")) as Submenu;
  const selectSubmenu = (await menu.get("select")) as Submenu;
  menuItems = {
    refresh: (await fileSubmenu.get("refresh")) as MenuItem,
    previous: (await selectSubmenu.get("previous")) as MenuItem,
    next: (await selectSubmenu.get("next")) as MenuItem,
    spot: (await fileSubmenu.get("spot")) as MenuItem,
  };
});

watch(databaseOpen, async (databaseOpen) => {
  if (!menuItems) return;
  menuItems.refresh.setEnabled(databaseOpen);
  menuItems.previous.setEnabled(databaseOpen);
  menuItems.next.setEnabled(databaseOpen);
});
watch(
  () => !!databaseView.value?.activeEntry,
  (hasActiveEntry) => {
    if (!menuItems) return;
    menuItems.spot.setEnabled(hasActiveEntry);
  },
);

// ========== Menu END ==========

onMounted(() => {
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
  <main class="container h-screen max-w-none">
    <Toast />
    <ConfirmDialog />

    <DatabaseView
      v-if="databaseOpen"
      ref="databaseView"
      :basePath="databasePath"
    />
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

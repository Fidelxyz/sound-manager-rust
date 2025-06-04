<script setup lang="ts">
import { basename } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { onKeyStroke } from "@vueuse/core";
import { computed, ref, watch } from "vue";

import {
  Button,
  ButtonGroup,
  Dialog,
  InputGroup,
  InputText,
  ToggleSwitch,
  useConfirm,
} from "primevue";

import type { Entry, ErrorKind } from "@/api";
import { api } from "@/api";
import { useConfig } from "@/config";
import { error, info } from "@/utils/message";

const { entry } = defineProps<{
  entry: Entry | null;
}>();

const emit = defineEmits<{
  pause: [];
}>();

type SpotSettings = {
  saveEnabled: boolean;
  savePath: string | null;
  openInApplicationEnabled: boolean;
  openInApplication: string | null;
};

const settings = useConfig<SpotSettings>("spot", {
  saveEnabled: false,
  savePath: null,
  openInApplicationEnabled: false,
  openInApplication: null,
});
const tempSettings = ref<SpotSettings>(settings.value);
const settingsOpened = ref(false);

const confirm = useConfirm();

const saveFolderName = ref<string | null>(null);
const openInApplicationName = ref<string | null>(null);
const spotToName = computed(() => {
  if (settings.value.openInApplicationEnabled) {
    return openInApplicationName.value ?? "…";
  } else if (settings.value.saveEnabled) {
    return saveFolderName.value ?? "…";
  }
  return "…";
});

watch(settings, async (newSettings) => {
  if (newSettings.openInApplication !== null) {
    // extract the application name from the path
    const match = newSettings.openInApplication.match(
      /([^\\/]+?)(?:\.app|\.exe)?$/i,
    );
    openInApplicationName.value = match ? match[1] : null;
  } else {
    openInApplicationName.value = null;
  }

  if (newSettings.savePath !== null) {
    // extract the folder name from the path
    saveFolderName.value = await basename(newSettings.savePath);
  } else {
    saveFolderName.value = null;
  }
});

function spot() {
  // validate settings
  if (
    (!settings.value.saveEnabled && !settings.value.openInApplicationEnabled) ||
    (settings.value.saveEnabled && !settings.value.savePath) ||
    (settings.value.openInApplicationEnabled &&
      !settings.value.openInApplication)
  ) {
    openSpotSettings();
    return;
  }

  if (!entry) return;

  console.debug(
    "spot",
    entry.id,
    settings.value.savePath,
    settings.value.openInApplication,
  );

  const savePath =
    settings.value.saveEnabled && settings.value.savePath !== null
      ? settings.value.savePath
      : undefined;
  const openInApplication =
    settings.value.openInApplicationEnabled &&
    settings.value.openInApplication !== null
      ? settings.value.openInApplication
      : undefined;
  confirmSpot(entry, savePath, openInApplication);
}

function confirmSpot(
  entry: Entry,
  savePath?: string,
  openInApplication?: string,
  force = false,
) {
  api
    .spot(entry.id, savePath, openInApplication, force)
    .then(() => {
      if (settings.value.openInApplicationEnabled) {
        info(
          "发送至应用成功",
          `已将 ${entry.fileName} 发送至 ${openInApplicationName.value}。`,
        );
        emit("pause");
      } else {
        info(
          "另存为成功",
          `已将 ${entry.fileName} 保存至 ${saveFolderName.value} 文件夹。`,
        );
      }
    })

    .catch((e: ErrorKind) => {
      if (!force && e.kind === "fileAlreadyExists") {
        confirm.require({
          header: "文件已存在",
          message: `位于 ${savePath} 中的文件 ${entry.fileName} 已存在。确定要覆盖文件吗？`,
          icon: "pi pi-exclamation-circle",
          rejectProps: { label: "取消", severity: "secondary", outlined: true },
          acceptProps: { label: "覆盖文件", severity: "danger" },
          accept: () => confirmSpot(entry, savePath, openInApplication, true),
        });
        return;
      }
      console.error(e);
      error("发送至应用失败", e.message);
    });
}

function openSpotSettings() {
  tempSettings.value = { ...settings.value };
  settingsOpened.value = true;
}

function saveSettings() {
  settings.value = { ...tempSettings.value };
  settingsOpened.value = false;
}

async function selectSavePath() {
  const path = await open({
    multiple: false,
    directory: true,
  });
  if (!path) return;

  tempSettings.value.savePath = path;
}

async function selectOpenInApplication() {
  const path = await open({
    multiple: false,
    directory: false,
  });
  if (!path) return;

  tempSettings.value.openInApplication = path;
}

onKeyStroke("s", () => {
  if (document.activeElement?.tagName.toLowerCase() === "input") return;

  if (!entry) return;
  spot();
});
</script>

<template>
  <ButtonGroup>
    <Button
      :label="`发送至 ${spotToName}`"
      icon="pi pi-file-export"
      @click="spot"
      :disabled="!entry"
    />
    <Button
      icon="pi pi-cog"
      aria-label="“发送至……”设置"
      @click="openSpotSettings"
    />

    <Dialog
      v-model:visible="settingsOpened"
      header="“发送至……”设置"
      class="w-2xl"
      modal
    >
      <table class="border-separate border-spacing-y-2 whitespace-nowrap">
        <tbody class="*:h-[2.4rem] **:align-middle">
          <tr>
            <td class="setting-label">
              <label for="save-copy">保存文件至新位置</label>
            </td>
            <td class="setting-value">
              <ToggleSwitch
                v-model="tempSettings.saveEnabled"
                inputId="save-copy"
              />
            </td>
          </tr>

          <tr v-if="tempSettings.saveEnabled">
            <td class="setting-label">
              <label for="save-path">保存至位置</label>
            </td>
            <td class="setting-value">
              <InputGroup inputId="save-path">
                <InputText
                  type="text"
                  v-model="tempSettings.savePath"
                  placeholder="未选择路径"
                  disabled
                  :pt="{
                    root: {
                      class: 'w-2xl',
                    },
                  }"
                />
                <Button label="选择路径" @click="selectSavePath" />
              </InputGroup>
            </td>
          </tr>

          <tr>
            <td class="setting-label">
              <label for="save-copy">发送文件至应用</label>
            </td>
            <td class="setting-value">
              <ToggleSwitch
                v-model="tempSettings.openInApplicationEnabled"
                inputId="save-copy"
              />
            </td>
          </tr>

          <tr v-if="tempSettings.openInApplicationEnabled">
            <td class="setting-label">
              <label for="open-in-application">发送至应用</label>
            </td>
            <td class="setting-value">
              <InputGroup inputId="open-in-application">
                <InputText
                  type="text"
                  v-model="tempSettings.openInApplication"
                  placeholder="未选择应用"
                  disabled
                />
                <Button label="选择应用" @click="selectOpenInApplication" />
              </InputGroup>
            </td>
          </tr>
        </tbody>
      </table>

      <div class="mt-6 flex justify-end gap-2">
        <Button
          type="button"
          label="取消"
          severity="secondary"
          @click="settingsOpened = false"
        />
        <Button type="button" label="保存" @click="saveSettings" />
      </div>
    </Dialog>
  </ButtonGroup>
</template>

<style scoped>
.setting-label {
  padding-right: 1rem;
}

.setting-value {
  width: 28rem;
}
</style>

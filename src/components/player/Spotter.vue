<script setup lang="ts">
import { ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";

import {
  Button,
  ButtonGroup,
  Dialog,
  InputText,
  InputGroup,
  ToggleSwitch,
  useConfirm,
} from "primevue";

import type { Entry, ErrorKind } from "@/api";
import { api } from "@/api";
import { error } from "@/utils/message";

const { entry } = defineProps<{
  entry?: Entry;
}>();

type SpotSettings = {
  saveCopy: boolean;
  savePath: string | null;
  openInApplication: string | null;
};

const settings = ref<SpotSettings>({
  saveCopy: false,
  savePath: null,
  openInApplication: null,
});
const tempSettings = ref<SpotSettings>(settings.value);
const openInApplicationName = ref<string | null>(null);
const settingsOpened = ref(false);
const confirm = useConfirm();

function spot() {
  if (
    (settings.value.saveCopy === false || settings.value.savePath === null) &&
    settings.value.openInApplication === null
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

  const entryId = entry.id;
  const savePath =
    settings.value.saveCopy !== false && settings.value.savePath !== null
      ? settings.value.savePath
      : undefined;
  const openInApplication =
    settings.value.openInApplication !== null
      ? settings.value.openInApplication
      : undefined;

  api.spot(entryId, savePath, openInApplication).catch((e: ErrorKind) => {
    if (e.kind === "fileAlreadyExists") {
      confirm.require({
        header: "文件已存在",
        message: `文件 ${entry.fileName} 位于 ${savePath} 已存在。确定要覆盖文件吗？`,
        icon: "pi pi-exclamation-circle",
        rejectProps: { label: "取消", severity: "secondary", outlined: true },
        acceptProps: { label: "覆盖文件", severity: "danger" },
        accept: () => confirmSpot(entryId, savePath, openInApplication),
      });
      return;
    }
    console.error(e);
    error("发送至应用失败", e.message);
  });
}

function confirmSpot(
  entryId: number,
  savePath?: string,
  openInApplication?: string,
) {
  api.spot(entryId, savePath, openInApplication, true).catch((e: ErrorKind) => {
    if (e.kind === "fileAlreadyExists") {
      confirmSpot(entryId, savePath, openInApplication);
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
  if (settings.value.openInApplication !== null) {
    // extract the application name from the path
    const match = settings.value.openInApplication.match(
      /([^\\/]+?)(?:\.app|\.exe)?$/i,
    );
    openInApplicationName.value = match ? match[1] : null;
  } else {
    openInApplicationName.value = null;
  }
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
</script>

<template>
  <ButtonGroup>
    <Button
      :label="`发送至 ${openInApplicationName ?? '…'}`"
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
      <table class="whitespace-nowrap border-separate border-spacing-y-2">
        <tbody class="*:h-[2.4rem] **:align-middle">
          <tr>
            <td class="setting-label">
              <label for="save-copy">保存文件至新位置</label>
            </td>
            <td class="setting-value">
              <ToggleSwitch
                v-model="tempSettings.saveCopy"
                inputId="save-copy"
              />
            </td>
          </tr>

          <tr v-if="tempSettings.saveCopy">
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

      <div class="flex justify-end gap-2 mt-6">
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

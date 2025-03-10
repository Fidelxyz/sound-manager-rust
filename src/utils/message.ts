import { ToastEventBus } from "primevue";

const lifeTime = 3000;

export function info(summary?: string, detail?: string): void {
  ToastEventBus.emit("add", {
    severity: "info",
    summary: summary,
    detail: detail,
    life: lifeTime,
  });
}

export function warn(summary: string, detail: string): void {
  ToastEventBus.emit("add", {
    severity: "warn",
    summary: summary,
    detail: detail,
    life: lifeTime,
  });
}

export function error(summary: string, detail: string): void {
  ToastEventBus.emit("add", {
    severity: "error",
    summary: summary,
    detail: detail,
    life: lifeTime,
  });
}

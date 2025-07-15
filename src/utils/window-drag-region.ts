import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCurrentElement, useEventListener } from "@vueuse/core";

export function useWindowDragRegion(
  noDragSelector: string = "input, a, button",
) {
  const appWindow = getCurrentWindow();
  const el = useCurrentElement<HTMLElement>();
  useEventListener(el, "mousedown", async (e: MouseEvent) => {
    if ((e.target as Element).closest(noDragSelector)) return; // a non-draggable element either in target or its ancestors
    await appWindow.startDragging();
  });
}

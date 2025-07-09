import type { CleanupFn } from "@atlaskit/pragmatic-drag-and-drop/dist/types/internal-types";
import { onMounted, onUnmounted } from "vue";

export function useDragAndDrop(registerFn: () => CleanupFn | null) {
  let cleanupFn: CleanupFn | null = null;

  onMounted(() => {
    cleanupFn = registerFn();
  });

  onUnmounted(() => {
    if (cleanupFn) {
      cleanupFn();
      cleanupFn = null;
    }
  });
}

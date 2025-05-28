import type { Folder } from "@/api";

export type DropTargetData =
  | {
      type: "tag";
      key: any;
    }
  | {
      type: "folder";
      folder: Folder;
    };

import type { Folder } from "@/api";

export type DropTargetData =
  | {
      type: "tag";
      key: string;
    }
  | {
      type: "entry";
      key: number;
    }
  | {
      type: "folder";
      folder: Folder;
    };

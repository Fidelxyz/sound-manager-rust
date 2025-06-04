import type { Entry, Folder } from "@/api";

export type DropTargetData =
  | {
      type: "tag";
      key: string;
    }
  | {
      type: "entry";
      key: number;
      data: Entry;
    }
  | {
      type: "folder";
      folder: Folder;
    };

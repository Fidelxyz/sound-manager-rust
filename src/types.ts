import type { Entry, Folder, Tag } from "@/api";

export type FolderNode = {
  folder: Folder;
  subFolders: FolderNode[];
};

export type TagNode = {
  tag: Tag;
  children: TagNode[];
};

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

export type Filter = {
  search: string;
  tags: Tag[];
  includeChildTags: boolean;
  folder: Folder | null;
  includeSubfolders: boolean;
};

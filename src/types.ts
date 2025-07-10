import type { Folder, Entry as RawEntry, Tag } from "@/api";

export type { Folder, Tag };

export type Entry = RawEntry & {
  viewed?: boolean;
};

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
      tagNode: TagNode;
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
  noTags: boolean;
  folder: Folder | null;
  includeSubfolders: boolean;
};

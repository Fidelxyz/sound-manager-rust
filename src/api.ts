import { type Channel, invoke } from "@tauri-apps/api/core";

export type ErrorKind = {
  kind:
    | "databaseNotFound"
    | "databaseAlreadyExists"
    | "tagAlreadyExists"
    | "tagAlreadyExistsForEntry"
    | "fileAlreadyExists"
    | "folderAlreadyExists"
    | "other";
  message: string;
};

export type Entry = {
  id: number;
  fileName: string;
  folderId: number;
  title?: string;
  artist?: string;
  album?: string;
  duration?: number;
  viewed?: boolean;
};

export type Folder = {
  id: number;
  name: string;
  parentId: number;
  subFolders: Record<string, number>;
};

export type Tag = {
  id: number;
  name: string;
  color: number;
  parentId: number;
  position: number;
  children: number[];
};

export type PlayerState = {
  playing: boolean;
  pos: number;
};

export type FilterArg = {
  search: string;
  tagIds: number[];
  includeChildTags: boolean;
  noTags: boolean;
  folderId: number | null;
  includeSubfolders: boolean;
};

// ========== Migrator ==========

export type MigrateFrom = "billfish";

export type MigratorLog = {
  kind: "warn" | "error";
  message: string;
};

export type MigratorResult = {
  success: boolean;
  logs: MigratorLog[];
};

export const api = {
  openDatabase(path: string): Promise<void> {
    return invoke("open_database", { path });
  },

  createDatabase(path: string): Promise<void> {
    return invoke("create_database", { path });
  },

  closeDatabase(): Promise<void> {
    return invoke("close_database");
  },

  migrateDatabase(
    path: string,
    fromType: MigrateFrom,
  ): Promise<MigratorResult> {
    return invoke("migrate_database", { path, fromType });
  },

  refresh(): Promise<void> {
    return invoke("refresh");
  },

  getEntries(): Promise<Entry[]> {
    return invoke("get_entries");
  },

  getTags(): Promise<Record<number, Tag>> {
    return invoke("get_tags");
  },

  getFolder(): Promise<Record<number, Folder>> {
    return invoke("get_folder");
  },

  newTag(name: string): Promise<number> {
    return invoke("new_tag", { name });
  },

  deleteTag(tagId: number): Promise<void> {
    return invoke("delete_tag", { tagId });
  },

  renameTag(tagId: number, name: string): Promise<void> {
    return invoke("rename_tag", { tagId, name });
  },

  reorderTag(
    tagId: number,
    newParentId: number,
    newPos: number,
  ): Promise<void> {
    return invoke("reorder_tag", { tagId, newParentId, newPos });
  },

  setTagColor(tagId: number, color: number): Promise<void> {
    return invoke("set_tag_color", { tagId, color });
  },

  getTagsForEntry(entryId: number): Promise<Tag[]> {
    return invoke("get_tags_for_entry", { entryId });
  },

  addTagForEntry(entryId: number, tagId: number): Promise<void> {
    return invoke("add_tag_for_entry", { entryId, tagId });
  },

  removeTagForEntry(entryId: number, tagId: number): Promise<void> {
    return invoke("remove_tag_for_entry", { entryId, tagId });
  },

  filter(filter: FilterArg): Promise<number[] | undefined> {
    return invoke("filter", { filter });
  },

  setPlayerSource(entryId: number): Promise<void> {
    return invoke("set_player_source", { entryId });
  },

  seek(pos: number): Promise<void> {
    return invoke("seek", { pos });
  },

  play(skipSilence: boolean): Promise<void> {
    return invoke("play", { skipSilence });
  },

  pause(): Promise<void> {
    return invoke("pause");
  },

  stop(): Promise<void> {
    return invoke("stop");
  },

  getPlayingPos(): Promise<number> {
    return invoke("get_playing_pos");
  },

  setVolume(volume: number): Promise<void> {
    return invoke("set_volume", { volume });
  },

  prepareWaveform(): Promise<number> {
    return invoke("prepare_waveform");
  },

  requestWaveform(channel: Channel<ArrayBuffer>): Promise<number> {
    return invoke("request_waveform", { channel });
  },

  importFile(path: string, force = false): Promise<void> {
    return invoke("import_file", { path, force });
  },

  deleteFile(entryId: number): Promise<void> {
    return invoke("delete_file", { entryId });
  },

  moveFile(entryId: number, folderId: number, force = false): Promise<void> {
    return invoke("move_file", { entryId, folderId, force });
  },

  moveFolder(folderId: number, newParentId: number): Promise<void> {
    return invoke("move_folder", { folderId, newParentId });
  },

  spot(
    entryId: number,
    savePath?: string,
    openInApplication?: string,
    force = false,
  ): Promise<void> {
    return invoke("spot", {
      entryId,
      savePath,
      openInApplication,
      force,
    });
  },

  revealEntry(entryId: number): Promise<void> {
    return invoke("reveal_entry", { entryId });
  },

  revealFolder(folderId: number): Promise<void> {
    return invoke("reveal_folder", { folderId });
  },
};

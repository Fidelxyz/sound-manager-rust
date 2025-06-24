import { type Channel, invoke } from "@tauri-apps/api/core";

export type ErrorKind = {
  kind:
    | "databaseNotFound"
    | "databaseAlreadyExists"
    | "tagAlreadyExists"
    | "tagAlreadyExistsForEntry"
    | "fileAlreadyExists"
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
    return invoke<Entry[]>("get_entries");
  },

  getTags(): Promise<Record<number, Tag>> {
    return invoke<Record<number, Tag>>("get_tags");
  },

  getFolder(): Promise<Record<number, Folder>> {
    return invoke<Record<number, Folder>>("get_folder");
  },

  newTag(name: string): Promise<number> {
    return invoke<number>("new_tag", { name });
  },

  deleteTag(tagId: number): Promise<void> {
    return invoke<void>("delete_tag", { tagId });
  },

  renameTag(tagId: number, name: string): Promise<void> {
    return invoke<void>("rename_tag", { tagId, name });
  },

  reorderTag(
    tagId: number,
    newParentId: number,
    newPos: number,
  ): Promise<void> {
    return invoke<void>("reorder_tag", { tagId, newParentId, newPos });
  },

  setTagColor(tagId: number, color: number): Promise<void> {
    return invoke<void>("set_tag_color", { tagId, color });
  },

  getTagsForEntry(entryId: number): Promise<Tag[]> {
    return invoke<Tag[]>("get_tags_for_entry", { entryId });
  },

  addTagForEntry(entryId: number, tagId: number): Promise<void> {
    return invoke<void>("add_tag_for_entry", { entryId, tagId });
  },

  removeTagForEntry(entryId: number, tagId: number): Promise<void> {
    return invoke<void>("remove_tag_for_entry", { entryId, tagId });
  },

  filter(filter: FilterArg): Promise<number[] | undefined> {
    return invoke<number[] | undefined>("filter", { filter });
  },

  setPlayerSource(entryId: number): Promise<void> {
    return invoke<void>("set_player_source", { entryId });
  },

  play(seek: number, skipSilence: boolean): Promise<void> {
    return invoke("play", { seek, skipSilence });
  },

  pause(): Promise<void> {
    return invoke("pause");
  },

  stop(): Promise<void> {
    return invoke("stop");
  },

  getPlayingPos(): Promise<number> {
    return invoke<number>("get_playing_pos");
  },

  setVolume(volume: number): Promise<void> {
    return invoke("set_volume", { volume });
  },

  prepareWaveform(): Promise<number> {
    return invoke<number>("prepare_waveform");
  },

  requestWaveform(channel: Channel<ArrayBuffer>): Promise<number> {
    return invoke<number>("request_waveform", { channel });
  },

  importFile(path: string, force = false): Promise<void> {
    return invoke<void>("import_file", { path, force });
  },

  deleteFile(entryId: number): Promise<void> {
    return invoke<void>("delete_file", { entryId });
  },

  moveFile(entryId: number, folderId: number, force = false): Promise<void> {
    return invoke<void>("move_file", { entryId, folderId, force });
  },

  spot(
    entryId: number,
    savePath?: string,
    openInApplication?: string,
    force = false,
  ): Promise<void> {
    return invoke<void>("spot", {
      entryId,
      savePath,
      openInApplication,
      force,
    });
  },
};

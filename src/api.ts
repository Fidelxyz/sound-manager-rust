import { invoke, Channel } from "@tauri-apps/api/core";
import { Entry, EntryTag, Folder } from "./types";
import { rename } from "fs";

export const api = {
  openDatabase(path: string): Promise<void> {
    return invoke("open_database", { path });
  },

  createDatabase(path: string): Promise<void> {
    return invoke("create_database", { path });
  },

  getEntries(): Promise<Entry[]> {
    return invoke<Entry[]>("get_entries");
  },

  getTags(): Promise<EntryTag[]> {
    return invoke<EntryTag[]>("get_tags");
  },

  getFolder(): Promise<Folder> {
    return invoke<Folder>("get_folder");
  },

  newTag(name: string): Promise<EntryTag> {
    return invoke<EntryTag>("new_tag", { name });
  },

  deleteTag(tagId: number): Promise<void> {
    return invoke<void>("delete_tag", { tagId });
  },

  renameTag(tagId: number, name: string): Promise<void> {
    return invoke<void>("rename_tag", { tagId, name });
  },

  getTagsForEntry(entryId: number): Promise<EntryTag[]> {
    return invoke<EntryTag[]>("get_tags_for_entry", { entryId });
  },

  addTagForEntry(entryId: number, tagId: number): Promise<void> {
    return invoke<void>("add_tag_for_entry", { entryId, tagId });
  },

  removeTagForEntry(entryId: number, tagId: number): Promise<void> {
    return invoke<void>("remove_tag_for_entry", { entryId, tagId });
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
};

export default api;

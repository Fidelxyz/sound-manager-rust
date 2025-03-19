import { invoke, type Channel } from "@tauri-apps/api/core";

export type ErrorKind = {
	kind:
		| "databaseNotFound"
		| "databaseAlreadyExists"
		| "entryNotFound"
		| "tagNotFound"
		| "tagAlreadyExists"
		| "tagNotFoundForEntry"
		| "tagAlreadyExistsForEntry"
		| "database"
		| "player"
		| "waveform";
	message: string;
};

export type Entry = {
	id: number;
	path: string;
	fileName: string;
	title?: string;
	artist?: string;
	album?: string;
	duration?: number;
	viewed?: boolean;
};

export type Tag = {
	id: number;
	name: string;
	color: number;
};

export type TagNode = {
	tag: Tag;
	children: TagNode[];
};

export type Folder = {
	path: string;
	name: string;
	subFolders: Folder[];
};

export type PlayerState = {
	playing: boolean;
	pos: number;
};

export type Filter = {
	search: string;
	tagIds: number[];
	folderPath: string;
};

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

	getTags(): Promise<TagNode[]> {
		return invoke<TagNode[]>("get_tags");
	},

	getFolder(): Promise<Folder> {
		return invoke<Folder>("get_folder");
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

	reorderTag(tagId: number, newPos: number): Promise<void> {
		return invoke<void>("reorder_tag", { tagId, newPos });
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

	filter(filter: Filter): Promise<number[] | undefined> {
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

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
  title: string?;
  artist: string?;
  album: string?;
  duration: number?;
};

export type EntryTag = {
  id: number;
  name: string;
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

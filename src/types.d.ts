export type ErrorKind = {
  kind: "entry" | "database";
  message: string;
};

export type Entry = {
  id: number;
  path: string;
  fileName: string;
  tagIds: number[];
  title: string?;
  artist: string?;
  album: string?;
  duration: number?;
};

export type Tag = {
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

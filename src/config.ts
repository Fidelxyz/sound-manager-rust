import { LazyStore } from "@tauri-apps/plugin-store";

const store = new LazyStore("config.json");

type NonUndefined<T> = T extends undefined ? never : T;

abstract class Config<T> {
  protected readonly key: string;
  protected readonly defaultValue: NonUndefined<T>;

  constructor(key: string, defaultValue: NonUndefined<T>) {
    this.key = key;
    this.defaultValue = defaultValue;
  }

  abstract load(): Promise<T>;

  async save(value: NonUndefined<T>): Promise<void> {
    await store.set(this.key, value);
  }
}

class ObjectConfig<T extends Object> extends Config<T> {
  override async load(): Promise<T> {
    const value = await store.get<T>(this.key);
    return { ...this.defaultValue, ...value };
  }
}

class ValueConfig<T> extends Config<T> {
  override async load(): Promise<T> {
    const value = await store.get<T>(this.key);
    return value !== undefined ? value : this.defaultValue;
  }
}

export function useConfig<T>(
  key: string,
  defaultValue: NonUndefined<T>,
): Config<T> {
  if (typeof defaultValue === "object") {
    return new ObjectConfig(key, defaultValue as T & Object) as Config<T>;
  } else {
    return new ValueConfig(key, defaultValue);
  }
}

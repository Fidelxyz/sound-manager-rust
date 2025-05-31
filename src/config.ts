import { LazyStore } from "@tauri-apps/plugin-store";
import { ref, watch } from "vue";

const store = new LazyStore("config.json");

abstract class Config<T> {
  protected readonly key: string;
  protected readonly defaultValue: T;
  /**
   * The value of the config, which is a Vue ref to allow reactivity.
   * Changes to this value will be automatically saved to the store.
   */
  public readonly value;

  public constructor(key: string, defaultValue: T) {
    this.key = key;
    this.defaultValue = defaultValue;
    this.value = ref(defaultValue); // No need to clone, since we will then recreate an object in load()

    this.load().then(() => {
      // watch for write changes
      watch(
        this.value,
        (newValue) => {
          // save changes to the store
          this.save(newValue);
        },
        { deep: true },
      );
    });
  }

  protected abstract load(): Promise<void>;

  private async save(value: T): Promise<void> {
    await store.set(this.key, value);
  }
}

class ObjectConfig<T extends Object> extends Config<T> {
  protected override async load() {
    const storeValue = await store.get<T>(this.key);
    this.value.value = { ...this.defaultValue, ...storeValue };
  }
}

class ValueConfig<T> extends Config<T> {
  protected override async load() {
    const storeValue = await store.get<T>(this.key);
    this.value.value =
      storeValue !== undefined ? storeValue : this.defaultValue;
  }
}

export function useConfig<T>(key: string, defaultValue: T) {
  const config: Config<T> =
    typeof defaultValue === "object"
      ? new ObjectConfig(key, defaultValue as T & Object)
      : new ValueConfig(key, defaultValue);
  return config.value;
}

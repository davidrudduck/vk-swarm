import '@testing-library/jest-dom';

// Node >= 24 defines global `localStorage`/`sessionStorage` getters that return
// undefined unless --localstorage-file is passed. Vitest's jsdom environment
// skips populating globals that already exist in Node, so under Node 24+ tests
// inherit Node's broken stub instead of jsdom's Storage. Install an in-memory
// Storage when the real one is missing; on environments where jsdom's Storage
// is populated normally this is a no-op.
class MemoryStorage {
  [name: string]: unknown;
  private store = new Map<string, string>();
  get length() {
    return this.store.size;
  }
  clear() {
    this.store.clear();
  }
  getItem(key: string) {
    return this.store.get(key) ?? null;
  }
  key(index: number) {
    return [...this.store.keys()][index] ?? null;
  }
  removeItem(key: string) {
    this.store.delete(key);
  }
  setItem(key: string, value: string) {
    this.store.set(key, String(value));
  }
}

if (typeof window !== 'undefined') {
  for (const name of ['localStorage', 'sessionStorage'] as const) {
    if (typeof window[name] === 'undefined') {
      Object.defineProperty(globalThis, name, {
        value: new MemoryStorage(),
        configurable: true,
        writable: true,
      });
    }
  }
}

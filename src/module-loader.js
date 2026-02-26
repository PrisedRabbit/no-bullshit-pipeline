// Global module loader for non-ESM script bundle.
// Provides deterministic readiness signaling across app scripts.

(function initModuleLoader() {
  if (window.NBPModuleLoader) return;

  const ready = new Map();
  const waiters = new Map();

  function resolveWaiters(name, api) {
    const list = waiters.get(name) || [];
    waiters.delete(name);
    for (const item of list) item.resolve(api);
  }

  function rejectWaiters(name, err) {
    const list = waiters.get(name) || [];
    waiters.delete(name);
    for (const item of list) item.reject(err);
  }

  const loader = {
    register(name, api) {
      if (!name) return;
      ready.set(name, api || {});
      resolveWaiters(name, api || {});
      window.dispatchEvent(new CustomEvent('nbp:module-ready', { detail: { name } }));
    },

    isReady(name) {
      return ready.has(name);
    },

    get(name) {
      return ready.get(name);
    },

    waitFor(name, timeoutMs) {
      if (ready.has(name)) return Promise.resolve(ready.get(name));

      return new Promise((resolve, reject) => {
        const list = waiters.get(name) || [];
        const waiter = { resolve, reject };
        list.push(waiter);
        waiters.set(name, list);

        if (typeof timeoutMs === 'number' && timeoutMs > 0) {
          setTimeout(() => {
            const current = waiters.get(name) || [];
            const idx = current.indexOf(waiter);
            if (idx >= 0) {
              current.splice(idx, 1);
              if (current.length === 0) waiters.delete(name);
              reject(new Error('Module timeout: ' + name));
            }
          }, timeoutMs);
        }
      });
    },

    waitForAll(names, timeoutMs) {
      return Promise.all((names || []).map((name) => loader.waitFor(name, timeoutMs)));
    },

    fail(name, message) {
      rejectWaiters(name, new Error(message || ('Module failed: ' + name)));
    },
  };

  window.NBPModuleLoader = loader;
})();

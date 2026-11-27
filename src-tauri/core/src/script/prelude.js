// Prélude des scripts d'Arnod (docs/plan-v4.md, R9) : vars, response, assert, console.
// Les données arrivent en JSON dans __input ; __output() rend ce que le script a posé.
(function () {
  const input = JSON.parse(__input);
  const store = Object.assign({}, input.vars || {});
  const set = {};
  const logs = [];

  const text = (value) =>
    typeof value === "string" ? value : value === undefined ? "" : JSON.stringify(value);

  // Variables imbriquées : « order.total » se lit vars.order.total.
  function tree() {
    const root = {};
    for (const name of Object.keys(store)) {
      const parts = name.split(".");
      let node = root;
      for (let i = 0; i < parts.length - 1; i++) {
        const part = parts[i];
        if (typeof node[part] !== "object" || node[part] === null) node[part] = {};
        node = node[part];
      }
      const last = parts[parts.length - 1];
      if (typeof node[last] !== "object" || node[last] === null) node[last] = store[name];
    }
    return root;
  }

  const vars = new Proxy(
    {},
    {
      get(_target, key) {
        if (key === "set")
          return (name, value) => {
            store[String(name)] = text(value);
            set[String(name)] = text(value);
          };
        if (key === "get") return (name) => store[String(name)];
        if (key === "has")
          return (name) => Object.prototype.hasOwnProperty.call(store, String(name));
        if (typeof key !== "string") return undefined;
        if (Object.prototype.hasOwnProperty.call(store, key)) return store[key];
        return tree()[key];
      },
      has(_target, key) {
        return Object.prototype.hasOwnProperty.call(store, String(key)) || String(key) in tree();
      },
      ownKeys() {
        return Object.keys(store);
      },
      getOwnPropertyDescriptor(_target, key) {
        if (!Object.prototype.hasOwnProperty.call(store, String(key))) return undefined;
        return { value: store[String(key)], enumerable: true, configurable: true };
      },
    },
  );

  globalThis.vars = vars;
  globalThis.response = input.response;
  globalThis.assert = function (condition, message) {
    if (!condition) {
      throw new Error(message ? "Assertion fausse : " + message : "Assertion fausse");
    }
  };
  globalThis.console = {
    log: (...items) => logs.push(items.map(text).join(" ")),
    info: (...items) => logs.push(items.map(text).join(" ")),
    warn: (...items) => logs.push(items.map(text).join(" ")),
    error: (...items) => logs.push(items.map(text).join(" ")),
  };
  globalThis.__arnod = { input, store, set, logs, text };
  globalThis.__output = function () {
    const out = { set, logs };
    if (globalThis.__bruno) Object.assign(out, globalThis.__bruno());
    return JSON.stringify(out);
  };
})();

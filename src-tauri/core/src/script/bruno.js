// API de Bruno pour les scripts gardés à l'import (docs/plan-v4.md, R10) : bru, req, res, test,
// expect (sous-ensemble de Chai). Après prelude.js.
(function () {
  const { input, store, set, text } = globalThis.__arnod;
  const tests = [];
  let request = input.request ? JSON.parse(JSON.stringify(input.request)) : null;
  let changed = false;

  globalThis.bru = {
    getVar: (name) => store[String(name)],
    setVar: (name, value) => {
      store[String(name)] = text(value);
      set[String(name)] = text(value);
    },
    getEnvVar: (name) => store[String(name)],
    setEnvVar: (name, value) => {
      store[String(name)] = text(value);
      set[String(name)] = text(value);
    },
    hasVar: (name) => Object.prototype.hasOwnProperty.call(store, String(name)),
    deleteVar: (name) => {
      delete store[String(name)];
    },
    getRequestVar: (name) => store[String(name)],
    getCollectionVar: (name) => store[String(name)],
    getFolderVar: (name) => store[String(name)],
    interpolate: (value) =>
      String(value).replace(/\{\{\s*([^}\s]+)\s*\}\}/g, (all, name) =>
        Object.prototype.hasOwnProperty.call(store, name) ? store[name] : all,
      ),
    sleep: () => undefined,
    setNextRequest: () => undefined,
    runner: { skipRequest: () => undefined, stopExecution: () => undefined },
  };

  const header = (list, name) => {
    const found = (list || []).find((h) => h[0].toLowerCase() === String(name).toLowerCase());
    return found ? found[1] : undefined;
  };

  if (request) {
    const bodyValue = () => {
      if (request.body === null || request.body === undefined) return undefined;
      try {
        return JSON.parse(request.body);
      } catch (_) {
        return request.body;
      }
    };
    globalThis.req = {
      getUrl: () => request.url,
      setUrl: (url) => {
        request.url = String(url);
        changed = true;
      },
      getMethod: () => request.method,
      setMethod: (method) => {
        request.method = String(method).toUpperCase();
        changed = true;
      },
      getHeader: (name) => header(request.headers, name),
      getHeaders: () => Object.fromEntries((request.headers || []).map((h) => [h[0], h[1]])),
      setHeader: (name, value) => {
        const list = (request.headers || []).filter(
          (h) => h[0].toLowerCase() !== String(name).toLowerCase(),
        );
        list.push([String(name), text(value)]);
        request.headers = list;
        changed = true;
      },
      deleteHeader: (name) => {
        request.headers = (request.headers || []).filter(
          (h) => h[0].toLowerCase() !== String(name).toLowerCase(),
        );
        changed = true;
      },
      getBody: bodyValue,
      setBody: (body) => {
        request.body = text(body);
        changed = true;
      },
      get url() {
        return request.url;
      },
      get method() {
        return request.method;
      },
      get body() {
        return bodyValue();
      },
      get headers() {
        return Object.fromEntries((request.headers || []).map((h) => [h[0], h[1]]));
      },
    };
  }

  const response = input.response;
  if (response) {
    globalThis.res = {
      status: response.status,
      headers: response.headers,
      body: response.body,
      responseTime: response.duration,
      getStatus: () => response.status,
      getHeader: (name) => response.headers[String(name).toLowerCase()],
      getHeaders: () => response.headers,
      getBody: () => response.body,
      getResponseTime: () => response.duration,
    };
  }

  // expect : sous-ensemble de Chai, messages en français.
  const show = (value) => {
    try {
      return typeof value === "string" ? JSON.stringify(value) : JSON.stringify(value) ?? String(value);
    } catch (_) {
      return String(value);
    }
  };
  const deepEqual = (a, b) => JSON.stringify(a) === JSON.stringify(b);
  const typeOf = (value) =>
    value === null ? "null" : Array.isArray(value) ? "array" : typeof value;

  function expect(actual) {
    let negate = false;
    const check = (ok, message) => {
      if (negate ? ok : !ok) throw new Error((negate ? "non attendu : " : "") + message);
    };
    const api = {
      get to() {
        return api;
      },
      get be() {
        return api;
      },
      get been() {
        return api;
      },
      get is() {
        return api;
      },
      get that() {
        return api;
      },
      get and() {
        return api;
      },
      get have() {
        return api;
      },
      get with() {
        return api;
      },
      get deep() {
        return api;
      },
      get not() {
        negate = !negate;
        return api;
      },
      equal(expected) {
        check(actual === expected, `attendu ${show(expected)}, reçu ${show(actual)}`);
        return api;
      },
      equals(expected) {
        return api.equal(expected);
      },
      eq(expected) {
        return api.equal(expected);
      },
      eql(expected) {
        check(deepEqual(actual, expected), `attendu ${show(expected)}, reçu ${show(actual)}`);
        return api;
      },
      a(type) {
        check(typeOf(actual) === String(type).toLowerCase(), `attendu un ${type}, reçu ${show(actual)}`);
        return api;
      },
      an(type) {
        return api.a(type);
      },
      property(name, value) {
        const has = actual !== null && actual !== undefined && Object.prototype.hasOwnProperty.call(Object(actual), name);
        if (arguments.length > 1) {
          check(has && deepEqual(actual[name], value), `attendu ${name} = ${show(value)}, reçu ${show(has ? actual[name] : undefined)}`);
        } else {
          check(has, `propriété « ${name} » absente`);
        }
        return api;
      },
      include(item) {
        const ok =
          typeof actual === "string"
            ? actual.includes(item)
            : Array.isArray(actual)
              ? actual.some((x) => deepEqual(x, item))
              : actual && typeof actual === "object"
                ? Object.keys(item || {}).every((k) => deepEqual(actual[k], item[k]))
                : false;
        check(ok, `${show(actual)} devait contenir ${show(item)}`);
        return api;
      },
      contain(item) {
        return api.include(item);
      },
      contains(item) {
        return api.include(item);
      },
      lengthOf(n) {
        const length = actual === null || actual === undefined ? undefined : actual.length;
        check(length === n, `longueur attendue ${n}, reçue ${show(length)}`);
        return api;
      },
      length(n) {
        return api.lengthOf(n);
      },
      above(n) {
        check(actual > n, `${show(actual)} devait être supérieur à ${n}`);
        return api;
      },
      greaterThan(n) {
        return api.above(n);
      },
      below(n) {
        check(actual < n, `${show(actual)} devait être inférieur à ${n}`);
        return api;
      },
      lessThan(n) {
        return api.below(n);
      },
      least(n) {
        check(actual >= n, `${show(actual)} devait valoir au moins ${n}`);
        return api;
      },
      most(n) {
        check(actual <= n, `${show(actual)} devait valoir au plus ${n}`);
        return api;
      },
      oneOf(list) {
        check((list || []).some((x) => deepEqual(x, actual)), `${show(actual)} devait être parmi ${show(list)}`);
        return api;
      },
      match(pattern) {
        check(new RegExp(pattern).test(String(actual)), `${show(actual)} ne correspond pas à ${pattern}`);
        return api;
      },
      get true() {
        check(actual === true, `attendu true, reçu ${show(actual)}`);
        return api;
      },
      get false() {
        check(actual === false, `attendu false, reçu ${show(actual)}`);
        return api;
      },
      get null() {
        check(actual === null, `attendu null, reçu ${show(actual)}`);
        return api;
      },
      get undefined() {
        check(actual === undefined, `attendu undefined, reçu ${show(actual)}`);
        return api;
      },
      get ok() {
        check(Boolean(actual), `attendu une valeur vraie, reçu ${show(actual)}`);
        return api;
      },
      get exist() {
        check(actual !== null && actual !== undefined, `valeur absente`);
        return api;
      },
      get empty() {
        const empty =
          actual === "" ||
          (Array.isArray(actual) && actual.length === 0) ||
          (actual && typeof actual === "object" && Object.keys(actual).length === 0);
        check(Boolean(empty), `${show(actual)} devait être vide`);
        return api;
      },
    };
    return api;
  }
  globalThis.expect = expect;
  globalThis.test = function (name, fn) {
    try {
      fn();
      tests.push({ name: String(name), passed: true, error: null });
    } catch (error) {
      tests.push({
        name: String(name),
        passed: false,
        error: error && error.message ? String(error.message) : String(error),
      });
    }
  };
  globalThis.__bruno = function () {
    return { tests, request: changed ? request : null };
  };
})();

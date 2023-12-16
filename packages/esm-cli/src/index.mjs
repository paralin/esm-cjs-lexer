import fs from "./fs.mjs";
import { enc, globToRegExp, isNEString, isObject } from "./util.mjs";

/**
 * Creates a fetch handler for serving hot applications.
 * @param {import("../types").ServeOptions} options
 * @returns {(req: Request) => Promise<Response>}
 */
export const serveHot = (options) => {
  const { root = ".", fallback = "index.html", env = {} } = options;
  const w = fs.watch(root);
  const contentCache = new Map(); // todo: use `caches` api if possible
  const resolveEnv = (value, name) => {
    return value.replace(/\$\{name\}/gi, name).replace(
      /\$\{ENV\.(.+?)\}/gi,
      (_, key) => env[key.trim()] || "",
    );
  };

  return async (req) => {
    const url = new URL(req.url);
    const pathname = decodeURIComponent(url.pathname);

    if (req.method === "POST") {
      if (pathname === "/@hot-content") {
        try {
          const {
            name,
            cacheTtl,
            url,
            method,
            body,
            authorization,
            headers,
            pickKeys,
            omitKeys,
          } = await req.json();
          if (!isNEString(name) || !isNEString(url)) {
            return new Response("Invalid request", { status: 400 });
          }
          const u = resolveEnv(url, name);
          const h = new Headers(headers);
          h.forEach((value, key) => {
            h.set(key, resolveEnv(value, name));
          });
          if (authorization) {
            h.set("authorization", resolveEnv(authorization, name));
          }
          const args = JSON.stringify([
            u,
            method,
            body,
            pickKeys,
            omitKeys,
            ...h.entries(),
          ]);
          const cached = contentCache.get(name);
          if (cached) {
            if (cached.args === args && cached.expires > Date.now()) {
              return Response.json(cached.value);
            }
            // delete expired cache
            contentCache.delete(name);
          }
          const res = await fetch(u, { method, body, headers: h });
          if (!res.ok) {
            return res;
          }
          let value = await res.json();
          if (isObject(value)) {
            if (Array.isArray(pickKeys) && pickKeys.length) {
              const result = {};
              for (const key of pickKeys) {
                result[key] = value[key];
              }
              value = result;
            }
            if (Array.isArray(omitKeys) && omitKeys.length) {
              for (const key of omitKeys) {
                delete value[key];
              }
            }
          }
          if (Number.isInteger(cacheTtl)) {
            contentCache.set(name, {
              args,
              value,
              expires: Date.now() + cacheTtl * 1000,
            });
          }
          return Response.json(value);
        } catch (e) {
          return new Response(e.message, { status: 500 });
        }
      }
      if (pathname === "/@hot-glob") {
        try {
          const headers = new Headers({
            "content-type": "hot/glob",
            "content-index": "2",
          });
          const { pattern: glob } = await req.json();
          if (!isNEString(glob)) {
            return new Response("[]", { headers });
          }
          const entries = await fs.ls(root);
          const matched = entries.filter((entry) =>
            glob.includes(entry) || entry.match(globToRegExp(glob))
          );
          if (!matched.length) {
            return new Response("[]", { headers });
          }
          const names = enc.encode(JSON.stringify(matched) + "\n");
          const sizes = await Promise.all(matched.map(async (filename) => {
            const stat = await fs.stat(root + "/" + filename);
            return stat.size;
          }));
          headers.set("content-index", [names.length, ...sizes].join(","));
          let currentFile;
          return new Response(
            new ReadableStream({
              start(controller) {
                const enqueue = (chunk) => controller.enqueue(chunk);
                const pipe = async () => {
                  const filename = matched.shift();
                  if (!filename) {
                    controller.close();
                    return;
                  }
                  currentFile = await fs.open(root + "/" + filename);
                  const reader = currentFile.body.getReader();
                  const pump = async () => {
                    const { done, value } = await reader.read();
                    if (done) {
                      currentFile.close();
                      pipe();
                      return;
                    }
                    enqueue(new Uint8Array(value));
                    pump();
                  };
                  pump();
                };
                enqueue(names);
                pipe();
              },
              cancel() {
                currentFile?.close();
              },
            }),
            { headers },
          );
        } catch (e) {
          return new Response(e.message, { status: 500 });
        }
      }
    }

    if (req.method !== "GET" && req.method !== "HEAD") {
      return new Response("Method not allowed", { status: 405 });
    }

    if (pathname === "/@hot-notify") {
      let dispose;
      return new Response(
        new ReadableStream({
          start(controller) {
            const enqueue = (chunk) => controller.enqueue(chunk);
            dispose = w((type, name) => {
              enqueue(enc.encode("event: fs-notify\ndata: "));
              enqueue(enc.encode(JSON.stringify({ type, name })));
              enqueue(enc.encode("\n\n"));
            });
            enqueue(enc.encode(": hot notify stream\n\n"));
          },
          cancel() {
            dispose?.();
          },
        }),
        {
          headers: {
            "transfer-encoding": "chunked",
            "content-type": "text/event-stream",
          },
        },
      );
    }

    if (pathname === "/@hot-index") {
      const entries = await fs.ls(root);
      return Response.json(entries);
    }

    let file = pathname.includes(".") ? await fs.open(root + pathname) : null;
    if (!file && pathname === "/sw.js") {
      const hotUrl = new URL("https://esm.sh/v135/hot");
      return new Response(`import hot from "${hotUrl.href}";hot.listen();`, {
        headers: {
          "content-type": "application/javascript; charset=utf-8",
          "last-modified": new Date().toUTCString(),
        },
      });
    }
    if (!file) {
      switch (pathname) {
        case "/apple-touch-icon-precomposed.png":
        case "/apple-touch-icon.png":
        case "/robots.txt":
        case "/favicon.ico":
          return new Response("Not found", { status: 404 });
      }
      const list = [
        pathname + ".html",
        pathname + "/index.html",
        "/404.html",
        "/" + fallback,
      ];
      for (const filename of list) {
        file = await fs.open(root + filename);
        if (file) break;
      }
    }
    if (file) {
      const headers = new Headers({
        "transfer-encoding": "chunked",
        "content-type": file.contentType,
        "content-length": file.size.toString(),
      });
      if (file.lastModified) {
        headers.set("last-modified", new Date(file.lastModified).toUTCString());
      }
      if (req.method === "HEAD") {
        file.close();
        return new Response(null, { headers });
      }
      return new Response(
        new ReadableStream({
          start(controller) {
            const reader = file.body.getReader();
            const pump = async () => {
              const { done, value } = await reader.read();
              if (done) {
                controller.close();
                file.close();
                return;
              }
              controller.enqueue(new Uint8Array(value));
              pump();
            };
            pump();
          },
          cancel() {
            file.close();
          },
        }),
        { headers },
      );
    }
    return new Response("Not Found", { status: 404 });
  };
};
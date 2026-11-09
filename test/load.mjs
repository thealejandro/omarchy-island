// Loads the island's QML JavaScript libraries (.pragma library) in node, so
// the pure logic can be tested without Quickshell. `.import "X.js" as Y`
// lines are resolved the same way, and every top-level function and var of
// the file ends up on the returned object.
import { readFileSync } from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import vm from "node:vm"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const cache = new Map()

export function load(name) {
  if (cache.has(name)) return cache.get(name)
  const ctx = vm.createContext({})
  cache.set(name, ctx)
  const src = readFileSync(path.join(root, name), "utf8")
    .replace(/^\.pragma library\s*$/m, "")
    .replace(/^\.import\s+"([^"]+)"\s+as\s+(\w+)\s*$/gm, (_, file, alias) => {
      ctx[alias] = load(file)
      return ""
    })
  vm.runInContext(src, ctx, { filename: name })
  return ctx
}

export function readJson(name) {
  return JSON.parse(readFileSync(path.join(root, name), "utf8"))
}

// Values made in another vm context fail deepStrictEqual on their prototype.
export function plain(v) {
  return JSON.parse(JSON.stringify(v))
}

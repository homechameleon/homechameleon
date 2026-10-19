import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { gzipSync } from "node:zlib";

const limit = 300 * 1024;
const skip = new Set([".woff", ".woff2", ".ttf", ".otf", ".png", ".jpg", ".jpeg", ".webp", ".gif", ".txt"]);
const root = new URL("../dist/", import.meta.url);

let raw = 0;
let gz = 0;
const rows = [];

function walk(dir) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      walk(path);
      continue;
    }
    const ext = entry.slice(entry.lastIndexOf(".")).toLowerCase();
    if (skip.has(ext)) continue;
    const bytes = readFileSync(path);
    const packed = gzipSync(bytes).length;
    raw += bytes.length;
    gz += packed;
    rows.push(`${packed}\t${bytes.length}\t${path}`);
  }
}

walk(root.pathname);
rows.sort();
for (const row of rows) console.log(row);
console.log(`raw=${raw} gz=${gz} limit=${limit}`);
if (gz > limit) {
  console.error(`app gzip size ${gz} is over ${limit}`);
  process.exit(1);
}

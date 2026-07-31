// Copies xterm.js assets from node_modules into vendor/ (source tree).
// Trunk's copy-dir pipeline then moves vendor/ into dist/ at build time.
// Run: cd nexus-console/ui && npm install && node vendor-xterm.mjs

import { mkdirSync, copyFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const vendorDir = resolve(__dirname, "vendor");
mkdirSync(vendorDir, { recursive: true });

const copies = [
    ["node_modules/@xterm/xterm/lib/xterm.js", "vendor/xterm.js"],
    ["node_modules/@xterm/xterm/lib/xterm.js.map", "vendor/xterm.js.map"],
    ["node_modules/@xterm/xterm/css/xterm.css", "vendor/xterm.css"],
    ["node_modules/@xterm/addon-fit/lib/addon-fit.js", "vendor/addon-fit.js"],
];

for (const [src, dst] of copies) {
    const srcPath = resolve(__dirname, src);
    const dstPath = resolve(__dirname, dst);
    if (!existsSync(srcPath)) {
        console.error(`vendor-xterm: missing source ${srcPath} — run 'npm install' first`);
        process.exit(1);
    }
    copyFileSync(srcPath, dstPath);
    console.log(`vendor-xterm: ${src} -> ${dst}`);
}

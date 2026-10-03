// 校验三处版本号一致:package.json / src-tauri/tauri.conf.json / src-tauri/Cargo.toml
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf-8"));
const conf = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf-8"));
const cargo = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf-8");
const cargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? "";

const versions = [
  ["package.json", pkg.version],
  ["tauri.conf.json", conf.version],
  ["Cargo.toml", cargoVersion],
];

const bad = versions.filter(([, v]) => v !== pkg.version);
if (bad.length > 0 || !pkg.version) {
  console.error("版本号不一致:");
  for (const [f, v] of versions) console.error(`  ${f}: ${v}`);
  process.exit(1);
}
console.log(`版本一致: v${pkg.version}`);

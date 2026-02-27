import { spawnSync } from "node:child_process";

const rustOnly = process.argv.includes("--rust-only");

const checks = [
  {
    id: "cargo",
    command: "cargo",
    args: ["--version"],
    required: true,
    help: "Install Rust toolchain: https://rustup.rs",
  },
  {
    id: "rustc",
    command: "rustc",
    args: ["--version"],
    required: true,
    help: "Install Rust toolchain: https://rustup.rs",
  },
  {
    id: "tauri",
    command: "tauri",
    args: ["--version"],
    required: !rustOnly,
    help: "Install JS deps with `bun install` (includes @tauri-apps/cli).",
  },
  {
    id: "bun",
    command: "bun",
    args: ["--version"],
    required: !rustOnly,
    help: "Install Bun: https://bun.sh/docs/installation",
  },
];

function runCheck(check) {
  const result = spawnSync(check.command, check.args, { encoding: "utf8" });
  const ok = result.status === 0;
  const output = (result.stdout || result.stderr || "").trim();
  return { ok, output };
}

let hasFailures = false;

for (const check of checks) {
  if (!check.required) {
    continue;
  }
  const { ok, output } = runCheck(check);
  if (ok) {
    console.log(`[ok] ${check.id}: ${output}`);
    continue;
  }
  hasFailures = true;
  console.error(`[missing] ${check.id}: not available in PATH`);
  console.error(`         ${check.help}`);
}

if (hasFailures) {
  console.error("\nDoctor failed: install missing tools and retry.");
  process.exit(1);
}

console.log("Doctor passed.");

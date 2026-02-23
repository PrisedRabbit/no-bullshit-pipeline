#!/usr/bin/env python3
"""docker.py — run hltm-loop in Docker with credential forwarding.

Handles macOS Keychain extraction so subscription-based Claude Code auth
works inside the container.

Usage:
    docker.py -m opus -p dev.md -e claude-code -n 20
    docker.py --build -m opus -p dev.md -e claude-code -n 20
"""

import hashlib
import os
import platform
import signal
import subprocess
import sys
import tempfile
from pathlib import Path

DEFAULT_IMAGE = "hltm-loop:latest"
SCRIPT_DIR = Path(os.path.realpath(__file__)).parent
PROJECT_ROOT = SCRIPT_DIR.parent


def ensure_image(image, build):
    if not build:
        result = subprocess.run(
            ["docker", "image", "inspect", image],
            capture_output=True, check=False,
        )
        if result.returncode == 0:
            return 0
        print(f"image '{image}' not found, building...", file=sys.stderr)

    print(f"building image '{image}' from {PROJECT_ROOT}...", file=sys.stderr)
    cmd = ["docker", "build",
           "--build-arg", f"USER_UID={os.getuid()}",
           "-t", image]
    if build:
        cmd.append("--no-cache")
    cmd.append(str(PROJECT_ROOT))
    proc = subprocess.Popen(cmd)
    try:
        proc.wait()
    except KeyboardInterrupt:
        print("\n  ⏹ build interrupted", file=sys.stderr, flush=True)
        proc.kill()
        proc.wait()
        return 130
    if proc.returncode != 0:
        print("docker build failed", file=sys.stderr)
    return proc.returncode


def keychain_service_name(claude_home):
    resolved = claude_home.expanduser().resolve()
    default = Path.home() / ".claude"
    if resolved == default or resolved == default.resolve():
        return "Claude Code-credentials"
    digest = hashlib.sha256(str(resolved).encode()).hexdigest()[:8]
    return f"Claude Code-credentials-{digest}"


def extract_macos_credentials(claude_home):
    if platform.system() != "Darwin":
        return None
    if (claude_home / ".credentials.json").exists():
        return None

    service = keychain_service_name(claude_home)

    def _find():
        try:
            r = subprocess.run(
                ["security", "find-generic-password", "-s", service, "-w"],
                capture_output=True, text=True, check=False,
            )
            if r.returncode == 0 and r.stdout.strip():
                return r.stdout.strip()
        except OSError:
            pass
        return None

    creds = _find()
    if not creds:
        subprocess.run(["security", "unlock-keychain"], capture_output=True, check=False)
        creds = _find()

    if not creds:
        return None

    fd, tmp_path = tempfile.mkstemp()
    try:
        with os.fdopen(fd, "w") as f:
            f.write(creds + "\n")
    except OSError:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass
        return None
    return Path(tmp_path)


def build_volumes(creds_temp, claude_home):
    home = Path.home()
    pwd_env = os.environ.get("PWD")
    cwd = Path(pwd_env) if pwd_env else Path(os.getcwd())
    vols = []

    def add(src, dst, ro=False):
        suffix = ":ro" if ro else ""
        vols.extend(["-v", f"{src}:{dst}{suffix}"])

    if claude_home.is_dir():
        add(claude_home.resolve(), "/mnt/claude", ro=True)

    add(cwd, "/workspace")

    if creds_temp:
        add(creds_temp, "/mnt/claude-credentials.json", ro=True)

    codex_dir = home / ".codex"
    if codex_dir.is_dir():
        add(codex_dir.resolve(), "/mnt/codex", ro=True)

    gitconfig = home / ".gitconfig"
    if gitconfig.exists():
        add(gitconfig.resolve(), "/home/hltm/.gitconfig", ro=True)

    # host-builder bridge for macOS native builds
    bridge = Path(os.environ.get("TMPDIR", "/tmp")) / "hltm-bridge"
    if bridge.is_dir():
        add(bridge, "/tmp/hltm-bridge")

    return vols


def run_docker(image, volumes, args):
    cmd = ["docker", "run", "--init", "-t"]
    if sys.stdin.isatty():
        cmd.append("-i")
    cmd.append("--rm")

    for key in ("ANTHROPIC_API_KEY", "OPENAI_API_KEY"):
        val = os.environ.get(key)
        if val:
            cmd.extend(["-e", f"{key}={val}"])

    cmd.extend(volumes)
    cmd.extend(["-w", "/workspace"])
    cmd.extend([image, "/workspace/.hltm/hltm-loop.sh"])
    cmd.extend(args)

    def sigint_handler(signum, frame):
        print("\n  ⏹ ctrl+c caught (docker.py)", file=sys.stderr, flush=True)
        try:
            proc.send_signal(signal.SIGINT)
        except (ProcessLookupError, OSError):
            pass

    proc = subprocess.Popen(cmd)
    prev = signal.signal(signal.SIGINT, sigint_handler)
    rc = proc.wait()
    signal.signal(signal.SIGINT, prev)
    return rc


def main():
    image = os.environ.get("HLTM_IMAGE", DEFAULT_IMAGE)
    args = sys.argv[1:]

    build = "--build" in args
    if build:
        args = [a for a in args if a != "--build"]

    rc = ensure_image(image, build)
    if rc != 0:
        return rc

    claude_config = os.environ.get("CLAUDE_CONFIG_DIR")
    claude_home = Path(claude_config).expanduser().resolve() if claude_config else Path.home() / ".claude"

    creds_temp = extract_macos_credentials(claude_home)
    if creds_temp:
        print("extracted Claude credentials from macOS Keychain", file=sys.stderr)

    def cleanup():
        if creds_temp:
            try:
                creds_temp.unlink(missing_ok=True)
            except OSError:
                pass

    def term_handler(signum, frame):
        cleanup()
        sys.exit(128 + signum)

    signal.signal(signal.SIGTERM, term_handler)

    try:
        volumes = build_volumes(creds_temp, claude_home)
        return run_docker(image, volumes, args)
    finally:
        cleanup()


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)

#!/usr/bin/env python3
"""Measure Mac-Word's start-up time and memory.

Usage:
  python3 scripts/bench-memory.py <path-to-wordcraft-binary> [--sample]

The app is launched with an isolated profile and control channel; start-up ends at the first
answered `ui.inspect`, which the app answers from its frame loop. A `ui.screenshot` is not used:
the capture keeps a window-sized GPU texture (17 MB at the default size) that ordinary use never
allocates. Idle RSS, physical footprint and the graphics/malloc breakdown are printed
after a few idle seconds. Requires macOS (`ps`, `footprint`, `vmmap`).
"""
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time

def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    app = os.path.abspath(sys.argv[1])
    sample = "--sample" in sys.argv[2:]
    port = 7991
    # `--profile=DIR` keeps a profile between runs, to measure a warm start (font catalog cache).
    given_profile = next((a.split("=", 1)[1] for a in sys.argv[2:] if a.startswith("--profile=")), None)
    profile = given_profile or tempfile.mkdtemp(prefix="mac-word-bench-")
    os.makedirs(profile, exist_ok=True)
    env = dict(os.environ, WORDCRAFT_PROFILE_DIR=profile)
    args = [app, "--control", str(port)] + (["--sample"] if sample else [])
    t0 = time.monotonic()
    proc = subprocess.Popen(args, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    keyfile = os.path.join(profile, f"control-key.local-{port}")
    key = None
    try:
        while time.monotonic() - t0 < 60:
            if proc.poll() is not None:
                print(f"app exited with {proc.returncode}")
                return 1
            try:
                if os.path.exists(keyfile):
                    key = open(keyfile, encoding="utf-8").read().strip()
                    if key:
                        break
            except OSError:
                pass
            time.sleep(0.01)
        if not key:
            print("no control key appeared")
            return 1
        key_time = time.monotonic() - t0

        def send(method, params=None):
            sock = socket.create_connection(("127.0.0.1", port), timeout=60)
            stream = sock.makefile("rw")
            stream.write(json.dumps({"id": 1, "key": key, "method": method, "params": params or {}}) + "\n")
            stream.flush()
            line = stream.readline()
            sock.close()
            return json.loads(line)

        first = None
        while time.monotonic() - t0 < 60:
            try:
                if send("ui.inspect").get("ok"):
                    first = time.monotonic() - t0
                    break
            except Exception:
                pass
            time.sleep(0.05)
        if first is None:
            print("no first frame")
            return 1
        time.sleep(4)
        rss = int(subprocess.check_output(["ps", "-o", "rss=", "-p", str(proc.pid)]).decode().strip()) / 1024.0
        footprint = subprocess.run(["footprint", "-p", str(proc.pid)], capture_output=True, text=True, timeout=60).stdout
        vmmap = subprocess.run(["vmmap", "-summary", str(proc.pid)], capture_output=True, text=True, timeout=60).stdout
        phys = next((line.strip() for line in footprint.splitlines() if line.strip().startswith("phys_footprint:")), "phys_footprint: ?")
        print(f"key file:            {key_time:.3f} s")
        print(f"first frame:         {first:.3f} s")
        print(f"idle RSS:            {rss:.1f} MB")
        print(f"physical footprint:  {phys}")
        for line in footprint.splitlines():
            if any(k in line for k in ("Owned physical footprint", "IOSurface", "Malloc Large", "Malloc Small", "Malloc Metadata")):
                print(f"  {line.strip()}")
        for line in vmmap.splitlines():
            if "MALLOC ZONE" in line or line.strip().startswith("DefaultMallocZone"):
                print(f"  {line.strip()}")
        # Leave through the app so it removes its control key and exits cleanly.
        try:
            send("app.quit")
        except Exception:
            pass
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.terminate()
        return 0
    finally:
        if proc.poll() is None:
            proc.terminate()
        if not given_profile:
            shutil.rmtree(profile, ignore_errors=True)

if __name__ == "__main__":
    sys.exit(main())

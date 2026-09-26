#!/usr/bin/env python3
"""One-command supervisor for the optional LabContext router and transports."""

from __future__ import annotations

import argparse
import os
import shlex
import shutil
import signal
import socket
import subprocess
import sys
import time
from pathlib import Path
from typing import NoReturn

from labcontext_router import DEFAULT_CONFIG_PATH, RouterError, load_config, parse_listen_addr


DEFAULT_ENV_PATH = Path("~/.config/labcontext/launcher.env").expanduser()
SCRIPT_DIR = Path(__file__).resolve().parent
ROUTER_SCRIPT = SCRIPT_DIR / "labcontext_router.py"


def fail(message: str) -> NoReturn:
    raise SystemExit(f"labcontext: {message}")


def parse_bool(value: str | None, default: bool = False) -> bool:
    if value is None:
        return default
    return value.strip().lower() in {"1", "true", "yes", "on"}


def load_env(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if not path.is_file():
        return values
    for number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line[7:].lstrip()
        if "=" not in line:
            fail(f"invalid line {number} in {path}")
        key, value = line.split("=", 1)
        key = key.strip()
        if not key.replace("_", "").isalnum():
            fail(f"invalid variable name on line {number} in {path}")
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
            value = value[1:-1]
        values[key] = os.path.expanduser(value)
    return values


def process_environment(values: dict[str, str]) -> dict[str, str]:
    environment = os.environ.copy()
    environment.update(values)
    return environment


def render_provider(provider_id: str, port: int, token_file: str) -> str:
    label = "This computer" if provider_id == "local" else "Research server"
    return f'''[[providers]]
id = "{provider_id}"
label = "{label}"
mcp_url = "http://127.0.0.1:{port}/mcp"
admin_url = "http://127.0.0.1:{port}/admin"
admin_token_file = "{token_file}"
enabled = true
timeout_seconds = 20
'''


def init_config(args: argparse.Namespace) -> int:
    config_path = args.config.expanduser()
    env_path = args.env_file.expanduser()
    existing = [path for path in (config_path, env_path) if path.exists()]
    if existing and not args.force:
        fail("refusing to overwrite existing configuration: " + ", ".join(map(str, existing)))
    config_path.parent.mkdir(parents=True, exist_ok=True)
    env_path.parent.mkdir(parents=True, exist_ok=True)
    providers: list[str] = []
    if args.mode in {"local", "hybrid"}:
        local_port = 1456 if args.mode == "hybrid" else 1455
        providers.append(render_provider("local", local_port, "~/.local/state/labcontext-local/admin.token"))
    if args.mode in {"server", "hybrid"}:
        providers.append(render_provider("server", 1455, "~/.local/state/labcontext/admin.token"))
    config_path.write_text(
        '''[router]
listen_addr = "127.0.0.1:1460"
cors_origins = ["http://127.0.0.1:48761", "http://localhost:48761"]

''' + "\n".join(providers),
        encoding="utf-8",
    )
    enable_ssh = args.mode in {"server", "hybrid"} and bool(args.ssh_host)
    env_path.write_text(
        "\n".join([
            f"LABCONTEXT_MODE={args.mode}",
            f"LABCONTEXT_ROUTER_CONFIG={config_path}",
            f"LABCONTEXT_ENABLE_SSH={'1' if enable_ssh else '0'}",
            f"LABCONTEXT_SSH_HOST={args.ssh_host or ''}",
            "LABCONTEXT_SSH_BATCH_MODE=1",
            "LABCONTEXT_SSH_SERVER_ALIVE_INTERVAL=30",
            "LABCONTEXT_SSH_SERVER_ALIVE_COUNT_MAX=3",
            "LABCONTEXT_SERVER_MCP_LOCAL_PORT=1455",
            "LABCONTEXT_SERVER_MCP_REMOTE_PORT=1455",
            "LABCONTEXT_SERVER_WEB_LOCAL_PORT=48761",
            "LABCONTEXT_SERVER_WEB_REMOTE_PORT=48761",
            f"LABCONTEXT_ENABLE_TUNNEL={'1' if args.tunnel_profile else '0'}",
            f"LABCONTEXT_TUNNEL_PROFILE={args.tunnel_profile or 'labcontext'}",
            "LABCONTEXT_TUNNEL_CLIENT=tunnel-client",
            "# LABCONTEXT_SECRET_ENV_FILE=~/.config/labcontext/secrets.env",
            "# Optional SSH settings: identity/config files and a reverse proxy forwarding pair.",
            "# LABCONTEXT_SSH_IDENTITY_FILE=~/.ssh/id_ed25519",
            "# LABCONTEXT_SSH_CONFIG_FILE=~/.ssh/config",
            "# LABCONTEXT_SERVER_PROXY_REMOTE_PORT=17987",
            "# LABCONTEXT_LOCAL_PROXY_PORT=7897",
            "# Optional: start an independently installed local Provider with the same command.",
            "# LABCONTEXT_LOCAL_PROVIDER_COMMAND=labcontext-provider --listen 127.0.0.1:1456",
            "",
            "# Keep CONTROL_PLANE_API_KEY in this chmod-600 file or inject it from your secret manager.",
            "# CONTROL_PLANE_API_KEY=replace-me",
            "",
        ]),
        encoding="utf-8",
    )
    os.chmod(env_path, 0o600)
    print(f"created {config_path}")
    print(f"created {env_path} (mode {args.mode})")
    print("edit provider ports/token paths if needed, then run: labcontext doctor && labcontext")
    return 0


def require_program(name: str) -> str:
    resolved = shutil.which(name)
    if not resolved:
        fail(f"required program is not installed or not on PATH: {name}")
    return resolved


def local_provider_command(values: dict[str, str]) -> list[str] | None:
    raw = values.get("LABCONTEXT_LOCAL_PROVIDER_COMMAND", "").strip()
    if not raw:
        return None
    command = shlex.split(raw)
    if not command:
        return None
    command[0] = require_program(command[0])
    return command


def doctor(config_path: Path, values: dict[str, str]) -> int:
    config = load_config(config_path)
    address = parse_listen_addr(config.listen_addr)
    print(f"router config: ok ({config_path})")
    print(f"router endpoint: http://{address[0]}:{address[1]}/mcp")
    print("providers: " + ", ".join(provider.id for provider in config.providers if provider.enabled))
    provider_command = local_provider_command(values)
    print(f"local Provider process: {'configured' if provider_command else 'externally managed'}")
    if parse_bool(values.get("LABCONTEXT_ENABLE_SSH")):
        require_program("ssh")
        if not values.get("LABCONTEXT_SSH_HOST"):
            fail("LABCONTEXT_ENABLE_SSH=1 requires LABCONTEXT_SSH_HOST")
        print(f"SSH bridge: configured ({values['LABCONTEXT_SSH_HOST']})")
    else:
        print("SSH bridge: disabled")
    if parse_bool(values.get("LABCONTEXT_ENABLE_TUNNEL")):
        require_program(values.get("LABCONTEXT_TUNNEL_CLIENT", "tunnel-client"))
        print(f"Secure MCP Tunnel: configured ({values.get('LABCONTEXT_TUNNEL_PROFILE', 'labcontext')})")
    else:
        print("Secure MCP Tunnel: disabled")
    return 0


def ssh_command(values: dict[str, str]) -> list[str]:
    command = [require_program("ssh")]
    config_file = values.get("LABCONTEXT_SSH_CONFIG_FILE")
    if config_file:
        command.extend(["-F", config_file])
    command.extend(["-N", "-o", "ExitOnForwardFailure=yes"])
    if parse_bool(values.get("LABCONTEXT_SSH_BATCH_MODE"), default=True):
        command.extend(["-o", "BatchMode=yes"])
    alive_interval = values.get("LABCONTEXT_SSH_SERVER_ALIVE_INTERVAL", "30")
    alive_count = values.get("LABCONTEXT_SSH_SERVER_ALIVE_COUNT_MAX", "3")
    command.extend([
        "-o", f"ServerAliveInterval={alive_interval}",
        "-o", f"ServerAliveCountMax={alive_count}",
    ])
    identity = values.get("LABCONTEXT_SSH_IDENTITY_FILE")
    if identity:
        command.extend(["-i", identity])
    local_mcp = values.get("LABCONTEXT_SERVER_MCP_LOCAL_PORT", "1455")
    remote_mcp = values.get("LABCONTEXT_SERVER_MCP_REMOTE_PORT", "1455")
    command.extend(["-L", f"127.0.0.1:{local_mcp}:127.0.0.1:{remote_mcp}"])
    local_web = values.get("LABCONTEXT_SERVER_WEB_LOCAL_PORT")
    remote_web = values.get("LABCONTEXT_SERVER_WEB_REMOTE_PORT", "48761")
    if local_web:
        command.extend(["-L", f"127.0.0.1:{local_web}:127.0.0.1:{remote_web}"])
    remote_proxy = values.get("LABCONTEXT_SERVER_PROXY_REMOTE_PORT")
    local_proxy = values.get("LABCONTEXT_LOCAL_PROXY_PORT")
    if bool(remote_proxy) != bool(local_proxy):
        fail("reverse proxy forwarding requires both LABCONTEXT_SERVER_PROXY_REMOTE_PORT and LABCONTEXT_LOCAL_PROXY_PORT")
    if remote_proxy and local_proxy:
        command.extend(["-R", f"127.0.0.1:{remote_proxy}:127.0.0.1:{local_proxy}"])
    command.append(values["LABCONTEXT_SSH_HOST"])
    return command


def wait_for_router(address: tuple[str, int], process: subprocess.Popen[bytes]) -> None:
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if process.poll() is not None:
            fail(f"router exited during startup with status {process.returncode}")
        try:
            with socket.create_connection(address, timeout=0.2):
                return
        except OSError:
            time.sleep(0.1)
    fail(f"router did not start on {address[0]}:{address[1]} within 10 seconds")


def stop_processes(processes: list[tuple[str, subprocess.Popen[bytes]]]) -> None:
    for _, process in reversed(processes):
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
    deadline = time.monotonic() + 4
    for _, process in reversed(processes):
        remaining = max(0.0, deadline - time.monotonic())
        try:
            process.wait(timeout=remaining)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass


def run(config_path: Path, values: dict[str, str]) -> int:
    config = load_config(config_path)
    address = parse_listen_addr(config.listen_addr)
    environment = process_environment(values)
    processes: list[tuple[str, subprocess.Popen[bytes]]] = []
    shutting_down = False

    def handle_signal(_signum: int, _frame: object) -> None:
        nonlocal shutting_down
        shutting_down = True

    signal.signal(signal.SIGINT, handle_signal)
    signal.signal(signal.SIGTERM, handle_signal)
    try:
        provider_command = local_provider_command(values)
        if provider_command:
            processes.append(("local Provider", subprocess.Popen(
                provider_command, env=environment, start_new_session=True,
            )))
        if parse_bool(values.get("LABCONTEXT_ENABLE_SSH")):
            if not values.get("LABCONTEXT_SSH_HOST"):
                fail("LABCONTEXT_ENABLE_SSH=1 requires LABCONTEXT_SSH_HOST")
            processes.append(("SSH bridge", subprocess.Popen(
                ssh_command(values), env=environment, start_new_session=True,
            )))
        router = subprocess.Popen(
            [sys.executable, str(ROUTER_SCRIPT), "--config", str(config_path)],
            env=environment,
            start_new_session=True,
        )
        processes.append(("LabContext Router", router))
        wait_for_router(address, router)
        if parse_bool(values.get("LABCONTEXT_ENABLE_TUNNEL")):
            client = require_program(values.get("LABCONTEXT_TUNNEL_CLIENT", "tunnel-client"))
            profile = values.get("LABCONTEXT_TUNNEL_PROFILE", "labcontext")
            processes.append(("Secure MCP Tunnel", subprocess.Popen(
                [client, "run", "--profile", profile],
                env=environment,
                start_new_session=True,
            )))
        labels = " + ".join(label for label, _ in processes)
        print(f"labcontext: running {labels}; press Ctrl-C to stop")
        while not shutting_down:
            for label, process in processes:
                status = process.poll()
                if status is not None:
                    fail(f"{label} exited unexpectedly with status {status}")
            time.sleep(0.5)
    finally:
        stop_processes(processes)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Run optional LabContext providers behind one MCP endpoint")
    parser.add_argument("--env-file", type=Path, default=DEFAULT_ENV_PATH)
    parser.add_argument("--config", type=Path)
    subparsers = parser.add_subparsers(dest="command")
    init_parser = subparsers.add_parser("init", help="create a local, server, or hybrid configuration")
    init_parser.add_argument("mode", choices=("local", "server", "hybrid"))
    init_parser.add_argument("--ssh-host", help="SSH host or alias for the server provider")
    init_parser.add_argument("--tunnel-profile", help="tunnel-client profile that points to the router")
    init_parser.add_argument("--force", action="store_true")
    subparsers.add_parser("doctor", help="validate configuration and dependencies")
    args = parser.parse_args()
    values = load_env(args.env_file.expanduser())
    secret_env_file = values.get("LABCONTEXT_SECRET_ENV_FILE")
    if secret_env_file:
        secret_values = load_env(Path(secret_env_file).expanduser())
        secret_values.update(values)
        values = secret_values
    config_path = (args.config or Path(values.get("LABCONTEXT_ROUTER_CONFIG", str(DEFAULT_CONFIG_PATH)))).expanduser()
    if args.command == "init":
        args.config = config_path
        return init_config(args)
    if args.command == "doctor":
        return doctor(config_path, values)
    return run(config_path, values)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RouterError, OSError, ValueError) as exc:
        fail(str(exc))

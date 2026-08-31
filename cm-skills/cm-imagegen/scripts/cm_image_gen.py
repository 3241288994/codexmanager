import argparse
import base64
import json
import mimetypes
import os
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

DEFAULT_MODEL = "gpt-image-2"
DEFAULT_SIZE = "1024x1024"
DEFAULT_RESPONSE_FORMAT = "b64_json"


def home() -> Path:
    return Path(os.environ.get("USERPROFILE") or os.environ.get("HOME") or str(Path.home()))


def codex_home() -> Path:
    return Path(os.environ.get("CODEX_HOME") or home() / ".codex")


def load_toml_basic(path: Path) -> dict:
    data = {}
    if not path.exists():
        return data
    section = []
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            section = [part.strip() for part in line[1:-1].split(".") if part.strip()]
            continue
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        value = value.strip().split(" #", 1)[0].strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
            value = value[1:-1]
        cursor = data
        for part in section:
            cursor = cursor.setdefault(part, {})
        cursor[key] = value
    return data


def provider_base_url() -> str:
    override = os.environ.get("CODEXMANAGER_IMAGE_BASE_URL")
    if override and override.strip():
        return override.strip().rstrip("/")
    config_path = codex_home() / "config.toml"
    cfg = load_toml_basic(config_path)
    provider_name = str(cfg.get("model_provider") or "").strip()
    if not provider_name:
        raise SystemExit(f"No model_provider found in {config_path}.")
    providers = cfg.get("model_providers") or {}
    if not isinstance(providers, dict):
        raise SystemExit(f"No model_providers table found in {config_path}.")
    provider = providers.get(provider_name) or {}
    if not isinstance(provider, dict):
        raise SystemExit(f"model_provider {provider_name!r} is not defined in {config_path}.")
    base_url = str(provider.get("base_url") or "").strip()
    if not base_url:
        raise SystemExit(f"model_provider {provider_name!r} has no base_url in {config_path}.")
    return base_url.rstrip("/")


def load_api_key() -> str:
    auth_path = codex_home() / "auth.json"
    if auth_path.exists():
        try:
            data = json.loads(auth_path.read_text(encoding="utf-8"))
            value = data.get("OPENAI_API_KEY")
            if isinstance(value, str) and value.strip():
                return value.strip()
        except Exception as exc:
            raise SystemExit(f"Failed to read auth.json: {exc}")
    raise SystemExit(f"No OPENAI_API_KEY found in {auth_path}.")


def sanitize_filename(name: str) -> str:
    name = re.sub(r"[\\/:*?\"<>|\r\n\t]+", "-", name).strip(" .-")
    return name or f"image-{int(time.time())}"


def output_dir() -> Path:
    env = os.environ.get("CODEXMANAGER_IMAGE_OUTPUT_DIR")
    if env and env.strip():
        return Path(env.strip())
    return Path.cwd() / "generated-images"


def request_json(url: str, api_key: str, payload: dict, timeout: int) -> dict:
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=body,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "Accept": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
            return json.loads(raw)
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        raise SystemExit(f"HTTP {exc.code} from CodexManager Images API: {raw}")
    except urllib.error.URLError as exc:
        raise SystemExit(f"Failed to reach CodexManager Images API: {exc}")


def data_url_from_file(path: Path) -> str:
    if not path.exists() or not path.is_file():
        raise SystemExit(f"Input image not found: {path}")
    mime_type = mimetypes.guess_type(str(path))[0] or "image/png"
    b64 = base64.b64encode(path.read_bytes()).decode("ascii")
    return f"data:{mime_type};base64,{b64}"


def save_image_from_item(item: dict, out_dir: Path, filename: str | None, index: int) -> Path:
    b64 = item.get("b64_json")
    if not isinstance(b64, str) or not b64.strip():
        url = item.get("url")
        raise SystemExit(f"Response item has no b64_json. URL responses are not saved by this script: {url}")
    raw = base64.b64decode(b64)
    ext = ".png"
    if raw.startswith(b"\xff\xd8"):
        ext = ".jpg"
    elif raw.startswith(b"RIFF") and b"WEBP" in raw[:16]:
        ext = ".webp"
    base = sanitize_filename(filename or f"codexmanager-image-{int(time.time())}")
    if not base.lower().endswith((".png", ".jpg", ".jpeg", ".webp")):
        base = f"{base}{ext}"
    if index > 0:
        stem = Path(base).stem
        suffix = Path(base).suffix
        base = f"{stem}-{index + 1}{suffix}"
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / base
    if path.exists():
        stem = path.stem
        suffix = path.suffix
        path = out_dir / f"{stem}-{int(time.time())}{suffix}"
    path.write_bytes(raw)
    return path


def generate(args: argparse.Namespace) -> int:
    base_url = provider_base_url()
    api_key = load_api_key()
    model = args.model or os.environ.get("CODEXMANAGER_IMAGE_MODEL") or DEFAULT_MODEL
    url = f"{base_url.rstrip('/')}/images/generations"
    payload = {
        "model": model,
        "prompt": args.prompt,
        "size": args.size,
        "response_format": args.response_format,
    }
    if args.n is not None:
        payload["n"] = args.n
    if args.quality:
        payload["quality"] = args.quality
    if args.background:
        payload["background"] = args.background
    if args.output_format:
        payload["output_format"] = args.output_format
    data = request_json(url, api_key, payload, args.timeout)
    items = data.get("data")
    if not isinstance(items, list) or not items:
        raise SystemExit(f"Images API returned no data items: {json.dumps(data, ensure_ascii=False)[:1000]}")
    out_dir = Path(args.out_dir) if args.out_dir else output_dir()
    paths = [save_image_from_item(item, out_dir, args.filename, index) for index, item in enumerate(items)]
    result = {
        "ok": True,
        "base_url": base_url,
        "model": model,
        "paths": [str(path.resolve()) for path in paths],
        "usage": data.get("usage"),
        "created": data.get("created"),
    }
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


def edit(args: argparse.Namespace) -> int:
    base_url = provider_base_url()
    api_key = load_api_key()
    model = args.model or os.environ.get("CODEXMANAGER_IMAGE_MODEL") or DEFAULT_MODEL
    url = f"{base_url.rstrip('/')}/images/edits"
    image_paths = [Path(value).expanduser() for value in args.image]
    payload = {
        "model": model,
        "prompt": args.prompt,
        "images": [{"image_url": data_url_from_file(path)} for path in image_paths],
        "size": args.size,
        "response_format": args.response_format,
    }
    if args.mask:
        payload["mask"] = {"image_url": data_url_from_file(Path(args.mask).expanduser())}
    if args.n is not None:
        payload["n"] = args.n
    if args.quality:
        payload["quality"] = args.quality
    if args.background:
        payload["background"] = args.background
    if args.output_format:
        payload["output_format"] = args.output_format
    data = request_json(url, api_key, payload, args.timeout)
    items = data.get("data")
    if not isinstance(items, list) or not items:
        raise SystemExit(f"Images API returned no data items: {json.dumps(data, ensure_ascii=False)[:1000]}")
    out_dir = Path(args.out_dir) if args.out_dir else output_dir()
    paths = [save_image_from_item(item, out_dir, args.filename, index) for index, item in enumerate(items)]
    result = {
        "ok": True,
        "operation": "edit",
        "base_url": base_url,
        "model": model,
        "source_images": [str(path.resolve()) for path in image_paths],
        "paths": [str(path.resolve()) for path in paths],
        "usage": data.get("usage"),
        "created": data.get("created"),
    }
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate images through CodexManager Images API.")
    sub = parser.add_subparsers(dest="command", required=True)
    gen = sub.add_parser("generate")
    gen.add_argument("--prompt", required=True)
    gen.add_argument("--model")
    gen.add_argument("--size", default=DEFAULT_SIZE)
    gen.add_argument("--response-format", default=DEFAULT_RESPONSE_FORMAT, choices=["b64_json", "url"])
    gen.add_argument("--out-dir")
    gen.add_argument("--filename")
    gen.add_argument("--n", type=int)
    gen.add_argument("--quality")
    gen.add_argument("--background")
    gen.add_argument("--output-format", choices=["png", "jpeg", "webp"])
    gen.add_argument("--timeout", type=int, default=600)
    gen.set_defaults(func=generate)
    edit_parser = sub.add_parser("edit")
    edit_parser.add_argument("--prompt", required=True)
    edit_parser.add_argument("--image", action="append", required=True)
    edit_parser.add_argument("--mask")
    edit_parser.add_argument("--model")
    edit_parser.add_argument("--size", default=DEFAULT_SIZE)
    edit_parser.add_argument("--response-format", default=DEFAULT_RESPONSE_FORMAT, choices=["b64_json", "url"])
    edit_parser.add_argument("--out-dir")
    edit_parser.add_argument("--filename")
    edit_parser.add_argument("--n", type=int)
    edit_parser.add_argument("--quality")
    edit_parser.add_argument("--background")
    edit_parser.add_argument("--output-format", choices=["png", "jpeg", "webp"])
    edit_parser.add_argument("--timeout", type=int, default=600)
    edit_parser.set_defaults(func=edit)
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())

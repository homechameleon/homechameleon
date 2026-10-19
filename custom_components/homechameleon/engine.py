"""Run homechameleon-core.wasm under wasmtime.

The module is the wasm-bindgen build the browser uses. It has no imports.
String arguments follow the wasm-bindgen stack ABI.
"""

from __future__ import annotations

import base64
import io
import threading
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageFilter, UnidentifiedImageError

from .const import MAX_IMAGE_BYTES

WASM_PATH = Path(__file__).parent / "core.wasm"
EXTRACT_EDGE = 128

# Pillow's decompression-bomb ceiling. Decoding always runs under this cap.
Image.MAX_IMAGE_PIXELS = 178_956_970

_LOCK = threading.Lock()
_ENGINE = None
_MODULE = None


class EngineError(Exception):
    """The core rejected the request."""


@dataclass
class DecodedImage:
    """One uploaded image, downscaled for the core, plus the source for a wallpaper."""

    rgba: bytes
    width: int
    height: int
    source: Image.Image
    weight: float
    use_as_background: bool


def fnv32(data: bytes) -> str:
    """FNV-1a 32-bit, matching the editor's wallpaper file name."""
    hash_value = 0x811C9DC5
    for byte in data:
        hash_value ^= byte
        hash_value = (hash_value * 0x01000193) & 0xFFFFFFFF
    return f"{hash_value:08x}"


def slugify(name: str) -> str:
    """Theme file stem: lowercase `[a-z0-9_-]`, same rules as the core."""
    out: list[str] = []
    dash = False
    for char in name:
        if char.isascii() and char.isalnum():
            out.append(char.lower())
            dash = False
        elif (char in "-_" or char.isspace()) and out and not dash:
            out.append("_" if char == "_" else "-")
            dash = True
    while out and out[-1] in "-_":
        out.pop()
    return "".join(out) or "theme"


def decode_upload(data: bytes) -> Image.Image:
    """Decode an uploaded image under Pillow's pixel cap."""
    if len(data) > MAX_IMAGE_BYTES:
        raise EngineError(f"image is {len(data)} bytes; the limit is {MAX_IMAGE_BYTES}")
    try:
        with Image.open(io.BytesIO(data)) as image:
            image.load()
            return image.convert("RGBA")
    except Image.DecompressionBombError as err:
        raise EngineError("image exceeds the pixel limit") from err
    except (UnidentifiedImageError, OSError) as err:
        raise EngineError("image could not be decoded") from err


def downscale(image: Image.Image, edge: int = EXTRACT_EDGE) -> tuple[bytes, int, int]:
    """Longest edge to `edge` pixels. Returns raw RGBA."""
    rgba = image.convert("RGBA")
    width, height = rgba.size
    longest = max(width, height)
    if longest > edge:
        scale = edge / longest
        width = max(1, round(width * scale))
        height = max(1, round(height * scale))
        rgba = rgba.resize((width, height), Image.Resampling.LANCZOS)
    return rgba.tobytes(), rgba.width, rgba.height


def encode_wallpaper(image: Image.Image, *, blur: int, dim: float, max_edge: int) -> bytes:
    """Resize, optional blur and dim, then WebP."""
    picture = image.convert("RGB")
    longest = max(picture.size)
    limit = max(1, int(max_edge))
    if longest > limit:
        scale = limit / longest
        picture = picture.resize(
            (max(1, round(picture.width * scale)), max(1, round(picture.height * scale))),
            Image.Resampling.LANCZOS,
        )
    if blur > 0:
        picture = picture.filter(ImageFilter.GaussianBlur(radius=blur))
    if dim > 0:
        black = Image.new("RGB", picture.size, (0, 0, 0))
        picture = Image.blend(picture, black, min(1.0, float(dim)))
    buffer = io.BytesIO()
    picture.save(buffer, format="WEBP", quality=80, method=4)
    return buffer.getvalue()


def _module():
    global _ENGINE, _MODULE
    from wasmtime import Engine, Module

    with _LOCK:
        if _MODULE is None:
            _ENGINE = Engine()
            _MODULE = Module.from_file(_ENGINE, str(WASM_PATH))
        return _ENGINE, _MODULE


def _call(export: str, payload: str | None) -> str:
    from wasmtime import Instance, Store

    engine, module = _module()
    store = Store(engine)
    instance = Instance(store, module, [])
    exports = instance.exports(store)
    memory = exports["memory"]
    stack = exports["__wbindgen_add_to_stack_pointer"]
    free = exports["__wbindgen_export3"]
    retptr = stack(store, -16)
    if payload is not None:
        raw = payload.encode("utf-8")
        ptr = exports["__wbindgen_export"](store, len(raw), 1)
        memory.write(store, raw, ptr)
        exports[export](store, retptr, ptr, len(raw))
    else:
        exports[export](store, retptr)
    header = memory.read(store, retptr, retptr + 8)
    start = int.from_bytes(header[0:4], "little", signed=True)
    length = int.from_bytes(header[4:8], "little", signed=True)
    if length < 0 or start < 0:
        stack(store, 16)
        raise EngineError("the core returned an empty result")
    text = bytes(memory.read(store, start, start + length)).decode("utf-8")
    stack(store, 16)
    free(store, start, length, 1)
    return text


def generate_json(request: str) -> str:
    """JSON in, JSON out. Same bytes as the browser core."""
    return _call("generate_json", request)


def styles_json() -> str:
    """Preset list from the core."""
    return _call("styles_json", None)


def b64encode(data: bytes) -> str:
    return base64.b64encode(data).decode("ascii")

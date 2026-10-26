"""HTTP views for generate, preview, styles, apply, and delete."""

from __future__ import annotations

import base64
import json
from pathlib import Path
from typing import Any

from aiohttp import web
from homeassistant.components.http import HomeAssistantView
from homeassistant.core import HomeAssistant

from .const import MAX_IMAGE_BYTES, MAX_IMAGES, MAX_YAML_BYTES
from .dashboards import (
    BindError,
    async_check_view,
    async_create_snapshot,
    async_delete_snapshot,
    async_finish_apply,
    async_get_snapshot,
    async_list_dashboards,
    async_list_snapshots,
    async_restore_snapshot,
    async_undo,
    async_undo_status,
    parse_view_target,
)
from .engine import (
    DecodedImage,
    EngineError,
    b64encode,
    decode_upload,
    downscale,
    encode_wallpaper,
    fnv32,
    generate_json,
    slugify,
    styles_json,
)
from .files import (
    PathError,
    async_reload_themes,
    atomic_write,
    background_path,
    delete_theme_files,
    theme_yaml_path,
)


def async_register_views(hass: HomeAssistant) -> None:
    """Register every HomeChameleon view. All of them require an admin."""
    for view in (
        GenerateView,
        PreviewView,
        StylesView,
        ApplyView,
        DeleteView,
        DashboardsView,
        SnapshotsView,
        SnapshotItemView,
        SnapshotRestoreView,
        UndoView,
    ):
        hass.http.register_view(view)


def _admin_or_response(request: web.Request) -> web.Response | None:
    user = request.get("hass_user")
    if user is None or not user.is_admin:
        return HomeAssistantView.json({"error": "admin only"}, status_code=403)
    return None


class _BodyError(Exception):
    def __init__(self, message: str, status: int) -> None:
        super().__init__(message)
        self.status = status


async def _read_payload(request: web.Request) -> dict[str, Any]:
    """JSON body, or multipart with a `request` field and image file parts."""
    if request.content_type == "application/json":
        try:
            body = await request.json()
        except json.JSONDecodeError as err:
            raise _BodyError("request JSON could not be read", 400) from err
        if not isinstance(body, dict):
            raise _BodyError("request JSON must be an object", 400)
        return body
    if request.content_type != "multipart/form-data":
        raise _BodyError("send JSON or multipart form data", 400)

    payload: dict[str, Any] | None = None
    files: list[bytes] = []
    reader = await request.multipart()
    while True:
        part = await reader.next()
        if part is None:
            break
        data = await part.read()
        if part.name in {"request", "json"}:
            try:
                parsed = json.loads(data)
            except json.JSONDecodeError as err:
                raise _BodyError("request JSON could not be read", 400) from err
            if not isinstance(parsed, dict):
                raise _BodyError("request JSON must be an object", 400)
            payload = parsed
            continue
        if part.name in {"images", "image", "file"}:
            if len(data) > MAX_IMAGE_BYTES:
                raise _BodyError("image is over 10 MB", 413)
            files.append(data)
    if payload is None:
        payload = {}
    images = payload.get("images")
    if not isinstance(images, list):
        images = [{} for _ in files]
        payload["images"] = images
    file_index = 0
    for image in images:
        if not isinstance(image, dict):
            continue
        if image.get("data"):
            continue
        if file_index >= len(files):
            break
        image["data"] = base64.b64encode(files[file_index]).decode("ascii")
        file_index += 1
    while file_index < len(files):
        images.append({"data": base64.b64encode(files[file_index]).decode("ascii"), "weight": 1})
        file_index += 1
    return payload


def _decode_images(body: dict[str, Any]) -> list[DecodedImage]:
    images = body.get("images") or []
    if not isinstance(images, list):
        raise _BodyError("images must be an array", 400)
    if len(images) > MAX_IMAGES:
        raise _BodyError(f"{len(images)} images were sent; the limit is {MAX_IMAGES}", 400)
    decoded: list[DecodedImage] = []
    for image in images:
        if not isinstance(image, dict) or not image.get("data"):
            continue
        try:
            raw = base64.b64decode(image["data"], validate=False)
        except (ValueError, TypeError) as err:
            raise _BodyError("image data is not base64", 400) from err
        if len(raw) > MAX_IMAGE_BYTES:
            raise _BodyError("image is over 10 MB", 413)
        try:
            picture = decode_upload(raw)
        except EngineError as err:
            message = str(err)
            status = 413 if "10" in message and "bytes" in message else 400
            raise _BodyError(message, status) from err
        rgba, width, height = downscale(picture)
        weight = image.get("weight", 1)
        try:
            weight = float(weight)
        except (TypeError, ValueError):
            weight = 1.0
        decoded.append(
            DecodedImage(
                rgba=rgba,
                width=width,
                height=height,
                source=picture,
                weight=weight,
                use_as_background=bool(image.get("use_as_background")),
            )
        )
    return decoded


def _core_request(body: dict[str, Any], decoded: list[DecodedImage]) -> tuple[dict[str, Any], bytes | None, str]:
    name = body.get("name") or "theme"
    if not isinstance(name, str) or not name.strip():
        name = "theme"
    slug = slugify(name)
    background = body.get("background") if isinstance(body.get("background"), dict) else {}
    enabled = bool(background.get("enabled", False))
    blur = int(background.get("blur") or 0)
    dim = float(background.get("dim") or 0)
    max_edge = int(background.get("max_edge") or 2560)
    wallpaper: bytes | None = None
    file_name: str | None = None
    if enabled and decoded:
        chosen = next((item for item in decoded if item.use_as_background), decoded[0])
        wallpaper = encode_wallpaper(chosen.source, blur=blur, dim=dim, max_edge=max_edge)
        if len(wallpaper) > MAX_IMAGE_BYTES:
            raise _BodyError("image is over 10 MB", 413)
        file_name = f"{slug}/bg-{fnv32(chosen.rgba)}.webp"
    core: dict[str, Any] = {
        "name": name,
        "style": body.get("style") or "sheer",
        "mode": body.get("mode") or "dark",
        "images": [
            {
                "width": item.width,
                "height": item.height,
                "rgba": b64encode(item.rgba),
                "weight": item.weight,
                "use_as_background": item.use_as_background,
            }
            for item in decoded
        ],
        "background": {
            "enabled": enabled,
            "blur": blur,
            "dim": dim,
            "max_edge": max_edge,
            "file_name": file_name,
        },
    }
    if isinstance(body.get("palette"), dict):
        core["palette"] = body["palette"]
    if isinstance(body.get("options"), dict):
        core["options"] = body["options"]
    if isinstance(body.get("extraction"), str):
        core["extraction"] = body["extraction"]
    if isinstance(body.get("tweaks"), dict):
        core["tweaks"] = body["tweaks"]
    return core, wallpaper, slug


def _public(result: dict[str, Any], *, files: list[str], applied: bool, default_set: bool, warnings: list[str]) -> dict[str, Any]:
    palette: dict[str, Any] = {}
    if isinstance(result.get("light"), dict):
        palette["light"] = result["light"]
    if isinstance(result.get("dark"), dict):
        palette["dark"] = result["dark"]
    merged = list(result.get("warnings") or []) + warnings
    return {
        "theme": result.get("theme") or result.get("slug"),
        "files": files,
        "palette": palette,
        "contrast": result.get("contrast") or {"pass": False, "checks": []},
        "applied": applied,
        "default_set": default_set,
        "warnings": merged,
    }


def _dashboard_warning(body: dict[str, Any]) -> list[str]:
    if "dashboard" in body:
        return ["Starter dashboards are not built by this version."]
    return []


async def _generate(hass: HomeAssistant, body: dict[str, Any], *, write: bool) -> dict[str, Any]:
    applying = write and body.get("apply", True)
    target = parse_view_target(body) if applying else None
    if target is not None:
        await async_check_view(hass, target)
    decoded = _decode_images(body)
    core, wallpaper, slug = _core_request(body, decoded)
    try:
        raw = await hass.async_add_executor_job(generate_json, json.dumps(core))
    except EngineError as err:
        raise _BodyError(str(err), 400) from err
    result = json.loads(raw)
    if result.get("error"):
        message = str(result["error"])
        status = 400
        if "unknown style" in message:
            status = 400
        raise _BodyError(message, status)
    warnings = _dashboard_warning(body)
    enabled = bool(core["background"]["enabled"])
    files: list[str] = []
    if write and body.get("apply", True):
        yaml_path = theme_yaml_path(hass.config.config_dir, slug)
        www_root_existed = Path(hass.config.path("www")).exists()
        await hass.async_add_executor_job(atomic_write, yaml_path, result["yaml"].encode("utf-8"))
        files.append(f"themes/{slug}.yaml")
        if enabled and wallpaper is not None:
            relative = core["background"]["file_name"]
            asset = background_path(hass.config.config_dir, slug, relative)
            await hass.async_add_executor_job(atomic_write, asset, wallpaper)
            files.append(f"www/homechameleon/{relative}")
            if not www_root_existed:
                warnings.append("Restart Home Assistant once so files under /local/ are served.")
        elif enabled:
            warnings.append("Background is on, but no image was sent, so no wallpaper was written.")
        note = await async_reload_themes(hass)
        if note:
            warnings.append(note)
        bound = await async_finish_apply(
            hass,
            theme=str(result.get("theme") or slug),
            view=target,
            set_default=bool(body.get("set_default")),
        )
        warnings.extend(bound.pop("warnings"))
        public = _public(
            result,
            files=files,
            applied=True,
            default_set=bool(bound.pop("default_set")),
            warnings=warnings,
        )
        public.update(bound)
        return public
    return _public(result, files=files, applied=False, default_set=False, warnings=warnings)


class GenerateView(HomeAssistantView):
    """POST /api/homechameleon/generate."""

    url = "/api/homechameleon/generate"
    name = "api:homechameleon:generate"
    requires_auth = True

    async def post(self, request: web.Request) -> web.Response:
        """Generate, and write when apply is true."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        try:
            body = await _read_payload(request)
            if "apply" not in body:
                body["apply"] = True
            result = await _generate(request.app["hass"], body, write=True)
        except _BodyError as err:
            return self.json({"error": str(err)}, status_code=err.status)
        except BindError as err:
            return self.json({"error": str(err)}, status_code=err.status)
        except PathError as err:
            return self.json({"error": str(err)}, status_code=400)
        return self.json(result)


class PreviewView(HomeAssistantView):
    """POST /api/homechameleon/preview. Writes nothing."""

    url = "/api/homechameleon/preview"
    name = "api:homechameleon:preview"
    requires_auth = True

    async def post(self, request: web.Request) -> web.Response:
        """Return the theme JSON, including the YAML, and leave the disk alone."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        try:
            body = await _read_payload(request)
            body["apply"] = False
            body["set_default"] = False
            hass: HomeAssistant = request.app["hass"]
            decoded = _decode_images(body)
            core, _wallpaper, _slug = _core_request(body, decoded)
            raw = await hass.async_add_executor_job(generate_json, json.dumps(core))
            result = json.loads(raw)
            if result.get("error"):
                return self.json({"error": result["error"]}, status_code=400)
            public = _public(
                result,
                files=[],
                applied=False,
                default_set=False,
                warnings=_dashboard_warning(body),
            )
            public["yaml"] = result.get("yaml", "")
        except _BodyError as err:
            return self.json({"error": str(err)}, status_code=err.status)
        except EngineError as err:
            return self.json({"error": str(err)}, status_code=400)
        return self.json(public)


class StylesView(HomeAssistantView):
    """GET /api/homechameleon/styles."""

    url = "/api/homechameleon/styles"
    name = "api:homechameleon:styles"
    requires_auth = True

    async def get(self, request: web.Request) -> web.Response:
        """Return the launch styles from the core."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        raw = await hass.async_add_executor_job(styles_json)
        return self.json({"styles": json.loads(raw)})


class ApplyView(HomeAssistantView):
    """POST /api/homechameleon/apply. Writes YAML the browser already built."""

    url = "/api/homechameleon/apply"
    name = "api:homechameleon:apply"
    requires_auth = True

    async def post(self, request: web.Request) -> web.Response:
        """Write a theme the panel already generated."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        try:
            body = await request.json()
        except json.JSONDecodeError:
            return self.json({"error": "request JSON could not be read"}, status_code=400)
        if not isinstance(body, dict):
            return self.json({"error": "request JSON must be an object"}, status_code=400)
        yaml = body.get("yaml")
        if not isinstance(yaml, str) or not yaml.strip():
            return self.json({"error": "yaml is required"}, status_code=400)
        if len(yaml.encode("utf-8")) > MAX_YAML_BYTES:
            return self.json({"error": "yaml is too large"}, status_code=400)
        try:
            target = parse_view_target(body)
            if target is not None:
                await async_check_view(hass, target)
        except BindError as err:
            return self.json({"error": str(err)}, status_code=err.status)
        raw_name = body.get("name") if isinstance(body.get("name"), str) else "theme"
        display = raw_name.strip() or "theme"
        slug = slugify(display)
        background = body.get("background") if isinstance(body.get("background"), dict) else {}
        enabled = bool(background.get("enabled", False))
        warnings = _dashboard_warning(body)
        if not enabled and ("lovelace-background:" in yaml or "/local/" in yaml):
            return self.json(
                {"error": "background is off, so the theme cannot reference a wallpaper"},
                status_code=400,
            )
        try:
            yaml_path = theme_yaml_path(hass.config.config_dir, slug)
            www_existed = Path(hass.config.path("www")).exists()
            await hass.async_add_executor_job(atomic_write, yaml_path, yaml.encode("utf-8"))
            files = [f"themes/{slug}.yaml"]
            if enabled:
                raw_b64 = background.get("data")
                relative = background.get("path") or ""
                if not isinstance(raw_b64, str) or not isinstance(relative, str):
                    return self.json({"error": "background image is missing"}, status_code=400)
                try:
                    raw = base64.b64decode(raw_b64, validate=False)
                except (ValueError, TypeError):
                    return self.json({"error": "background image is not base64"}, status_code=400)
                if len(raw) > MAX_IMAGE_BYTES:
                    return self.json({"error": "image is over 10 MB"}, status_code=413)
                asset = background_path(hass.config.config_dir, slug, relative)
                await hass.async_add_executor_job(atomic_write, asset, raw)
                files.append(f"www/homechameleon/{slug}/{asset.name}")
                if not www_existed:
                    warnings.append("Restart Home Assistant once so files under /local/ are served.")
        except PathError as err:
            return self.json({"error": str(err)}, status_code=400)
        note = await async_reload_themes(hass)
        if note:
            warnings.append(note)
        try:
            bound = await async_finish_apply(
                hass,
                theme=display,
                view=target,
                set_default=bool(body.get("set_default")),
            )
        except BindError as err:
            return self.json({"error": str(err)}, status_code=err.status)
        warnings.extend(bound.pop("warnings"))
        payload = {
            "theme": display,
            "files": files,
            "palette": {},
            "contrast": {"pass": True, "checks": []},
            "applied": True,
            "default_set": bool(bound.pop("default_set")),
            "warnings": warnings,
        }
        payload.update(bound)
        return self.json(payload)


class DeleteView(HomeAssistantView):
    """DELETE /api/homechameleon/theme/<slug>."""

    url = "/api/homechameleon/theme/{slug}"
    name = "api:homechameleon:delete"
    requires_auth = True

    async def delete(self, request: web.Request, slug: str) -> web.Response:
        """Delete one theme file and its wallpaper folder."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        try:
            found = await hass.async_add_executor_job(delete_theme_files, hass.config.config_dir, slug)
        except PathError as err:
            return self.json({"error": str(err)}, status_code=400)
        if not found:
            return self.json({"error": "theme not found"}, status_code=404)
        note = await async_reload_themes(hass)
        warnings = [note] if note else []
        return self.json({"theme": slug, "deleted": True, "warnings": warnings})


class DashboardsView(HomeAssistantView):
    """GET /api/homechameleon/dashboards."""

    url = "/api/homechameleon/dashboards"
    name = "api:homechameleon:dashboards"
    requires_auth = True

    async def get(self, request: web.Request) -> web.Response:
        """List dashboards and the theme on each view."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        return self.json({"dashboards": await async_list_dashboards(hass)})


class SnapshotsView(HomeAssistantView):
    """GET and POST /api/homechameleon/snapshots."""

    url = "/api/homechameleon/snapshots"
    name = "api:homechameleon:snapshots"
    requires_auth = True

    async def get(self, request: web.Request) -> web.Response:
        """List dashboard snapshots."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        return self.json({"snapshots": await async_list_snapshots(hass)})

    async def post(self, request: web.Request) -> web.Response:
        """Store a copy of one dashboard config."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        try:
            body = await request.json()
        except json.JSONDecodeError:
            return self.json({"error": "request JSON could not be read"}, status_code=400)
        if not isinstance(body, dict):
            return self.json({"error": "request JSON must be an object"}, status_code=400)
        try:
            summary = await async_create_snapshot(hass, body.get("url_path"))
        except BindError as err:
            return self.json({"error": str(err)}, status_code=err.status)
        return self.json(summary)


class SnapshotItemView(HomeAssistantView):
    """GET and DELETE /api/homechameleon/snapshots/<id>."""

    url = "/api/homechameleon/snapshots/{snapshot_id}"
    name = "api:homechameleon:snapshot"
    requires_auth = True

    async def get(self, request: web.Request, snapshot_id: str) -> web.Response:
        """Return one snapshot, including the dashboard config."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        try:
            document = await async_get_snapshot(hass, snapshot_id)
        except BindError as err:
            return self.json({"error": str(err)}, status_code=err.status)
        return self.json(document)

    async def delete(self, request: web.Request, snapshot_id: str) -> web.Response:
        """Delete one snapshot file."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        try:
            await async_delete_snapshot(hass, snapshot_id)
        except BindError as err:
            return self.json({"error": str(err)}, status_code=err.status)
        return self.json({"id": snapshot_id, "deleted": True})


class SnapshotRestoreView(HomeAssistantView):
    """POST /api/homechameleon/snapshots/<id>/restore."""

    url = "/api/homechameleon/snapshots/{snapshot_id}/restore"
    name = "api:homechameleon:snapshot-restore"
    requires_auth = True

    async def post(self, request: web.Request, snapshot_id: str) -> web.Response:
        """Restore a dashboard from a snapshot."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        try:
            result = await async_restore_snapshot(hass, snapshot_id)
        except BindError as err:
            return self.json({"error": str(err)}, status_code=err.status)
        return self.json(result)


class UndoView(HomeAssistantView):
    """GET and POST /api/homechameleon/undo."""

    url = "/api/homechameleon/undo"
    name = "api:homechameleon:undo"
    requires_auth = True

    async def get(self, request: web.Request) -> web.Response:
        """Say whether the last theme assignment can be restored."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        return self.json(await async_undo_status(hass))

    async def post(self, request: web.Request) -> web.Response:
        """Restore the recorded global, per-user, and per-view themes."""
        if (denied := _admin_or_response(request)) is not None:
            return denied
        hass: HomeAssistant = request.app["hass"]
        try:
            result = await async_undo(hass)
        except BindError as err:
            return self.json({"error": str(err)}, status_code=err.status)
        return self.json(result)

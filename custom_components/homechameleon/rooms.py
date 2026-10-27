"""Build a Lovelace view for one Home Assistant area.

The result is a view document. Nothing here writes a dashboard.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from homeassistant.const import EntityCategory
from homeassistant.core import HomeAssistant
from homeassistant.helpers import area_registry as ar
from homeassistant.helpers import device_registry as dr
from homeassistant.helpers import entity_registry as er

from .engine import slugify

# AlarmControlPanelEntityFeature bits. Trigger is intentionally unused.
_ARM_HOME = 1
_ARM_AWAY = 2
_ARM_NIGHT = 4

_INCLUDED = {
    "light",
    "climate",
    "media_player",
    "cover",
    "sensor",
    "binary_sensor",
    "scene",
    "lock",
    "alarm_control_panel",
    "camera",
}

_GROUP_ORDER = (
    ("lights", "Lights"),
    ("climate", "Climate"),
    ("media", "Media"),
    ("covers", "Covers"),
    ("sensors", "Sensors"),
    ("scenes", "Scenes"),
    ("security", "Security"),
    ("cameras", "Cameras"),
)


class RoomError(Exception):
    """The room view could not be built."""

    def __init__(self, message: str, status: int) -> None:
        super().__init__(message)
        self.status = status


@dataclass(frozen=True)
class RoomEntity:
    """One entity the view may place."""

    entity_id: str
    name: str
    domain: str
    device_class: str | None = None
    state: str | None = None
    unit: str | None = None
    current_temperature: str | None = None
    current_humidity: str | None = None
    temperature_unit: str | None = None
    supported_features: int = 0


def choose_packs(detected: set[str]) -> list[str]:
    """Pick at most one optional card pack. Mushroom wins when both are present."""
    if "mushroom" in detected:
        return ["mushroom"]
    if "bubble-card" in detected:
        return ["bubble-card"]
    return []


def build_room_view(
    *,
    area_id: str,
    name: str,
    entities: list[RoomEntity],
    packs: list[str],
    temperature_entity_id: str | None = None,
    humidity_entity_id: str | None = None,
    both_packs: bool = False,
) -> dict[str, Any]:
    """Return the view, YAML, and a preview model. `packs` only adds optional cards."""
    kept: list[RoomEntity] = []
    skipped: list[RoomEntity] = []
    for entity in entities:
        if entity.domain in _INCLUDED:
            kept.append(entity)
        else:
            skipped.append(entity)

    temperature = _header_sensor(kept, temperature_entity_id, "temperature")
    humidity = _header_sensor(kept, humidity_entity_id, "humidity")
    header_ids = {
        entity.entity_id
        for entity in (temperature, humidity)
        if entity is not None and entity.domain == "sensor"
    }
    grouped: dict[str, list[RoomEntity]] = {key: [] for key, _title in _GROUP_ORDER}
    for entity in kept:
        if entity.entity_id in header_ids:
            continue
        key = _group_for(entity)
        if key is None:
            skipped.append(entity)
            continue
        grouped[key].append(entity)
    for bucket in grouped.values():
        bucket.sort(key=_by_name)

    used: set[str] = set()
    header_cards: list[dict[str, Any]] = [
        {"type": "heading", "heading": name, "heading_style": "title"}
    ]
    header_sensors: list[RoomEntity] = []
    seen_header: set[str] = set()
    for entity in (temperature, humidity):
        if entity is None or entity.domain != "sensor" or entity.entity_id in seen_header:
            continue
        seen_header.add(entity.entity_id)
        header_sensors.append(entity)
    if "mushroom" in packs and header_sensors:
        header_cards.append(
            {
                "type": "custom:mushroom-chips-card",
                "chips": [{"type": "entity", "entity": entity.entity_id} for entity in header_sensors],
            }
        )
        used.add("mushroom")
    else:
        header_cards.extend(_core_tile(entity) for entity in header_sensors)
    lights = grouped["lights"]
    if lights:
        header_cards.append(
            {
                "type": "button",
                "name": "All lights",
                "icon": "mdi:lightbulb-group",
                "tap_action": {
                    "action": "perform-action",
                    "perform_action": "light.toggle",
                    "target": {"area_id": area_id},
                },
                "hold_action": {"action": "none"},
            }
        )

    sections = [{"type": "grid", "cards": header_cards}]
    preview_sections: list[dict[str, Any]] = []
    for key, title in _GROUP_ORDER:
        bucket = grouped[key]
        if not bucket:
            continue
        cards = [_card_for(entity, packs, used) for entity in bucket]
        sections.append(
            {"type": "grid", "cards": [{"type": "heading", "heading": title}, *cards]}
        )
        preview_sections.append(
            {
                "id": key,
                "title": title,
                "cards": [_preview_card(entity, packs) for entity in bucket],
            }
        )

    enhancements = [pack for pack in ("mushroom", "bubble-card") if pack in used]
    warnings: list[str] = []
    if both_packs and "mushroom" in enhancements:
        warnings.append(
            "Mushroom and Bubble Card are both installed. This view uses Mushroom cards "
            "where they replace a core card. Locks, garage covers, alarms, and cameras "
            "stay on core cards."
        )
    omitted = _omitted_warning(skipped)
    if omitted:
        warnings.append(omitted)
    if not lights and not any(grouped.values()) and temperature is None and humidity is None:
        warnings.append("This area has no entities to place on the view.")

    view = {
        "title": name,
        "path": _view_path(name, area_id),
        "type": "sections",
        "max_columns": 2,
        "sections": sections,
    }
    return {
        "area_id": area_id,
        "name": name,
        "view": view,
        "yaml": emit_yaml(view),
        "card_set": "enhanced" if enhancements else "core",
        "enhancements": enhancements,
        "warnings": warnings,
        "wrote": False,
        "preview": {
            "header": {
                "title": name,
            "temperature": _preview_fact(temperature, packs, "temperature"),
            "humidity": _preview_fact(humidity, packs, "humidity"),
                "lights_toggle": bool(lights),
            },
            "sections": preview_sections,
        },
    }


def emit_yaml(value: Any) -> str:
    """Dump a JSON-shaped document with stable quotes."""
    lines: list[str] = []
    _emit(lines, value, 0)
    return "\n".join(lines) + "\n"


async def async_list_areas(hass: HomeAssistant) -> dict[str, Any]:
    """List areas and how many view entities each one has."""
    area_reg = ar.async_get(hass)
    rows = []
    for area in sorted(area_reg.async_list_areas(), key=lambda item: item.name.casefold()):
        entities = _collect_entities(hass, area)
        placed = sum(1 for entity in entities if entity.domain in _INCLUDED)
        rows.append({"id": area.id, "name": area.name, "entities": placed})
    return {"areas": rows}


async def async_room_response(hass: HomeAssistant, body: dict[str, Any]) -> dict[str, Any]:
    """Resolve an area, read its entities, and return a view. Writes nothing."""
    area = _resolve_area(ar.async_get(hass), body)
    entities = _collect_entities(hass, area)
    found = await async_detect_packs(hass)
    packs = choose_packs(found)
    return build_room_view(
        area_id=area.id,
        name=area.name,
        entities=entities,
        packs=packs,
        temperature_entity_id=area.temperature_entity_id,
        humidity_entity_id=area.humidity_entity_id,
        both_packs="mushroom" in found and "bubble-card" in found,
    )


async def async_detect_packs(hass: HomeAssistant) -> set[str]:
    """Find Mushroom and Bubble Card without loading extra integrations."""
    found: set[str] = set()
    if "mushroom" in hass.config.components:
        found.add("mushroom")
    found |= _packs_from_lovelace(hass)
    found |= await hass.async_add_executor_job(_packs_from_disk, hass.config.config_dir)
    return found


def _resolve_area(registry: ar.AreaRegistry, body: dict[str, Any]) -> ar.AreaEntry:
    area_id = body.get("area_id")
    if isinstance(area_id, str) and area_id.strip():
        area = registry.async_get_area(area_id.strip())
        if area is None:
            raise RoomError("Area not found", 404)
        return area
    raw_name = body.get("area")
    if isinstance(raw_name, str) and raw_name.strip():
        wanted = raw_name.strip()
        area = registry.async_get_area_by_name(wanted)
        if area is not None:
            return area
        folded = wanted.casefold()
        for candidate in registry.async_list_areas():
            if candidate.name.casefold() == folded:
                return candidate
        raise RoomError("Area not found", 404)
    raise RoomError("area_id is required", 400)


def _collect_entities(hass: HomeAssistant, area: ar.AreaEntry) -> list[RoomEntity]:
    entity_reg = er.async_get(hass)
    device_reg = dr.async_get(hass)
    found: dict[str, er.RegistryEntry] = {}
    for entry in er.async_entries_for_area(entity_reg, area.id):
        found[entry.entity_id] = entry
    for device in dr.async_entries_for_area(device_reg, area.id):
        for entry in er.async_entries_for_device(entity_reg, device.id):
            if entry.area_id not in (None, area.id):
                continue
            found[entry.entity_id] = entry
    for entity_id in (area.temperature_entity_id, area.humidity_entity_id):
        if not entity_id or entity_id in found:
            continue
        entry = entity_reg.async_get(entity_id)
        if entry is not None:
            found[entity_id] = entry
    entities = [_to_room_entity(hass, entry) for entry in found.values()]
    entities = [entity for entity in entities if entity is not None]
    entities.sort(key=_by_name)
    return entities


def _to_room_entity(hass: HomeAssistant, entry: er.RegistryEntry) -> RoomEntity | None:
    if entry.disabled_by is not None or entry.hidden_by is not None:
        return None
    if entry.entity_category is not None and entry.entity_category in {
        EntityCategory.CONFIG,
        EntityCategory.DIAGNOSTIC,
    }:
        return None
    state = hass.states.get(entry.entity_id)
    attributes = state.attributes if state is not None else {}
    name = entry.name or entry.original_name
    if not name and state is not None:
        friendly = attributes.get("friendly_name")
        if isinstance(friendly, str) and friendly.strip():
            name = friendly.strip()
    if not name:
        name = entry.entity_id.split(".", 1)[-1].replace("_", " ")
    device_class = entry.device_class or entry.original_device_class
    if device_class is None:
        raw_class = attributes.get("device_class")
        device_class = raw_class if isinstance(raw_class, str) else None
    unit = attributes.get("unit_of_measurement")
    if not isinstance(unit, str):
        unit = entry.unit_of_measurement
    features = entry.supported_features or 0
    raw_features = attributes.get("supported_features")
    if isinstance(raw_features, int):
        features = raw_features
    current_temperature = _attr_text(attributes, "current_temperature")
    current_humidity = _attr_text(attributes, "current_humidity")
    temperature_unit = attributes.get("temperature_unit")
    if not isinstance(temperature_unit, str):
        temperature_unit = unit if entry.domain == "sensor" else None
    return RoomEntity(
        entity_id=entry.entity_id,
        name=name,
        domain=entry.domain,
        device_class=device_class,
        state=state.state if state is not None else None,
        unit=unit if isinstance(unit, str) else None,
        current_temperature=current_temperature,
        current_humidity=current_humidity,
        temperature_unit=temperature_unit if isinstance(temperature_unit, str) else None,
        supported_features=features,
    )


def _attr_text(attributes: Any, key: str) -> str | None:
    if not isinstance(attributes, dict):
        return None
    value = attributes.get(key)
    if value is None or isinstance(value, bool):
        return None
    if isinstance(value, (int, float, str)):
        text = str(value).strip()
        return text or None
    return None


def _packs_from_lovelace(hass: HomeAssistant) -> set[str]:
    """Read resources already stored under the lovelace data key.

    The key is the string "lovelace". Looking it up directly avoids importing
    that component, which would make it a manifest dependency.
    """
    found: set[str] = set()
    data = hass.data.get("lovelace")
    resources = getattr(data, "resources", None)
    if resources is None or not getattr(resources, "loaded", False):
        return found
    try:
        items = resources.async_items() or []
    except (AttributeError, TypeError, RuntimeError):
        return found
    for item in items:
        if isinstance(item, dict):
            _note_pack(found, str(item.get("url", "")))
    return found


def _packs_from_disk(config_dir: str) -> set[str]:
    found: set[str] = set()
    storage = Path(config_dir, ".storage", "lovelace_resources")
    if storage.is_file():
        try:
            payload = json.loads(storage.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError):
            payload = {}
        items = payload.get("data", {}).get("items", []) if isinstance(payload, dict) else []
        if isinstance(items, list):
            for item in items:
                if isinstance(item, dict):
                    _note_pack(found, str(item.get("url", "")))
    for folder in ("www/community", "www/hacsfiles", "custom_components"):
        root = Path(config_dir, folder)
        if not root.is_dir():
            continue
        try:
            children = list(root.iterdir())
        except OSError:
            continue
        for child in children:
            _note_pack(found, child.name)
    return found


def _note_pack(found: set[str], text: str) -> None:
    name = text.lower()
    if "mushroom" in name:
        found.add("mushroom")
    if "bubble-card" in name or "bubble_card" in name or "bubblecard" in name:
        found.add("bubble-card")


def _header_sensor(
    entities: list[RoomEntity], entity_id: str | None, device_class: str
) -> RoomEntity | None:
    if entity_id:
        match = next((entity for entity in entities if entity.entity_id == entity_id), None)
        if match is not None and match.domain == "sensor":
            return match
    candidates = [
        entity
        for entity in entities
        if entity.domain == "sensor" and entity.device_class == device_class
    ]
    if candidates:
        return sorted(candidates, key=_by_name)[0]
    if device_class == "temperature":
        climates = [entity for entity in entities if entity.domain == "climate" and entity.current_temperature]
        return sorted(climates, key=_by_name)[0] if climates else None
    climates = [entity for entity in entities if entity.domain == "climate" and entity.current_humidity]
    return sorted(climates, key=_by_name)[0] if climates else None


def _group_for(entity: RoomEntity) -> str | None:
    if entity.domain == "camera":
        return "cameras"
    if entity.domain in {"lock", "alarm_control_panel"}:
        return "security"
    if entity.domain == "light":
        return "lights"
    if entity.domain == "climate":
        return "climate"
    if entity.domain == "media_player":
        return "media"
    if entity.domain == "cover":
        return "covers"
    if entity.domain in {"sensor", "binary_sensor"}:
        return "sensors"
    if entity.domain == "scene":
        return "scenes"
    return None


def _is_garage(entity: RoomEntity) -> bool:
    if entity.domain != "cover":
        return False
    if entity.device_class in {"garage", "garage_door"}:
        return True
    return "garage" in f"{entity.name} {entity.entity_id}".lower()


def _is_security(entity: RoomEntity) -> bool:
    return entity.domain in {"lock", "alarm_control_panel", "camera"} or _is_garage(entity)


def _interaction(entity: RoomEntity) -> str:
    if entity.domain == "camera":
        return "view"
    if entity.domain in {"lock", "alarm_control_panel"} or _is_garage(entity):
        return "confirm"
    return "direct"


def _detail(entity: RoomEntity) -> str | None:
    if entity.domain == "camera":
        return "View only."
    if entity.domain == "lock":
        return "Asks before changing the lock."
    if entity.domain == "alarm_control_panel":
        return "Arm and disarm ask before they run."
    if _is_garage(entity):
        return "Open and close ask before they run."
    return None


def _card_for(entity: RoomEntity, packs: list[str], used: set[str]) -> dict[str, Any]:
    if _is_security(entity):
        return _security_card(entity)
    if "mushroom" in packs:
        custom = _mushroom_card(entity)
        if custom is not None:
            used.add("mushroom")
            return custom
    if "bubble-card" in packs:
        custom = _bubble_card(entity)
        if custom is not None:
            used.add("bubble-card")
            return custom
    return _core_card(entity)


def _security_card(entity: RoomEntity) -> dict[str, Any]:
    if entity.domain == "camera":
        return {
            "type": "picture-entity",
            "entity": entity.entity_id,
            "name": entity.name,
            "camera_view": "auto",
            "show_name": True,
            "show_state": True,
            "tap_action": {"action": "none"},
            "hold_action": {"action": "none"},
            "double_tap_action": {"action": "none"},
        }
    if entity.domain == "alarm_control_panel":
        return _alarm_card(entity)
    text = (
        f"Open or close {entity.name}?"
        if _is_garage(entity)
        else f"Change {entity.name}?"
    )
    return {
        "type": "tile",
        "entity": entity.entity_id,
        "show_state": True,
        "tap_action": {
            "action": "toggle",
            "confirmation": {"text": text},
        },
        "hold_action": {"action": "none"},
        "double_tap_action": {"action": "none"},
    }


def _alarm_card(entity: RoomEntity) -> dict[str, Any]:
    buttons: list[dict[str, Any]] = []
    features = entity.supported_features
    unknown = features == 0
    modes = (
        ("Arm home", "alarm_control_panel.alarm_arm_home", _ARM_HOME, f"Arm {entity.name} to home?"),
        ("Arm away", "alarm_control_panel.alarm_arm_away", _ARM_AWAY, f"Arm {entity.name} to away?"),
        ("Arm night", "alarm_control_panel.alarm_arm_night", _ARM_NIGHT, f"Arm {entity.name} to night?"),
    )
    for label, service, bit, text in modes:
        if unknown and bit != _ARM_HOME:
            continue
        if not unknown and not features & bit:
            continue
        buttons.append(_confirm_button(label, service, entity.entity_id, text))
    buttons.append(
        _confirm_button("Disarm", "alarm_control_panel.alarm_disarm", entity.entity_id, f"Disarm {entity.name}?")
    )
    cards: list[dict[str, Any]] = [
        {
            "type": "tile",
            "entity": entity.entity_id,
            "show_state": True,
            "tap_action": {"action": "none"},
            "hold_action": {"action": "none"},
            "double_tap_action": {"action": "none"},
        }
    ]
    if len(buttons) == 1:
        cards.append(buttons[0])
    else:
        cards.append({"type": "horizontal-stack", "cards": buttons})
    return {"type": "vertical-stack", "cards": cards}


def _confirm_button(label: str, service: str, entity_id: str, text: str) -> dict[str, Any]:
    return {
        "type": "button",
        "name": label,
        "tap_action": {
            "action": "perform-action",
            "perform_action": service,
            "target": {"entity_id": entity_id},
            "confirmation": {"text": text},
        },
        "hold_action": {"action": "none"},
        "double_tap_action": {"action": "none"},
    }


def _core_card(entity: RoomEntity) -> dict[str, Any]:
    if entity.domain == "climate":
        return {"type": "thermostat", "entity": entity.entity_id}
    if entity.domain == "media_player":
        return {"type": "media-control", "entity": entity.entity_id}
    if entity.domain == "scene":
        return {
            "type": "button",
            "name": entity.name,
            "tap_action": {
                "action": "perform-action",
                "perform_action": "scene.turn_on",
                "target": {"entity_id": entity.entity_id},
            },
            "hold_action": {"action": "none"},
        }
    if entity.domain == "light":
        return {
            "type": "tile",
            "entity": entity.entity_id,
            "show_state": True,
            "features": [{"type": "light-brightness"}],
        }
    if entity.domain == "cover":
        return {
            "type": "tile",
            "entity": entity.entity_id,
            "show_state": True,
            "features": [{"type": "cover-open-close"}],
        }
    return _core_tile(entity)


def _core_tile(entity: RoomEntity) -> dict[str, Any]:
    return {"type": "tile", "entity": entity.entity_id, "show_state": True}


def _mushroom_card(entity: RoomEntity) -> dict[str, Any] | None:
    if entity.domain == "light":
        return {
            "type": "custom:mushroom-light-card",
            "entity": entity.entity_id,
            "use_light_color": True,
            "show_brightness_control": True,
        }
    if entity.domain == "climate":
        return {
            "type": "custom:mushroom-climate-card",
            "entity": entity.entity_id,
            "show_temperature_control": True,
        }
    if entity.domain == "cover":
        return {
            "type": "custom:mushroom-cover-card",
            "entity": entity.entity_id,
            "show_buttons_control": True,
        }
    if entity.domain == "media_player":
        return {
            "type": "custom:mushroom-media-player-card",
            "entity": entity.entity_id,
            "show_volume_level": True,
        }
    if entity.domain in {"sensor", "binary_sensor"}:
        return {"type": "custom:mushroom-entity-card", "entity": entity.entity_id}
    if entity.domain == "scene":
        return {
            "type": "custom:mushroom-entity-card",
            "entity": entity.entity_id,
            "tap_action": {
                "action": "perform-action",
                "perform_action": "scene.turn_on",
                "target": {"entity_id": entity.entity_id},
            },
        }
    return None


def _bubble_card(entity: RoomEntity) -> dict[str, Any] | None:
    if entity.domain == "light":
        return {
            "type": "custom:bubble-card",
            "card_type": "button",
            "button_type": "slider",
            "entity": entity.entity_id,
        }
    if entity.domain == "scene":
        return {
            "type": "custom:bubble-card",
            "card_type": "button",
            "entity": entity.entity_id,
            "tap_action": {
                "action": "perform-action",
                "perform_action": "scene.turn_on",
                "target": {"entity_id": entity.entity_id},
            },
        }
    return None


def _preview_card(entity: RoomEntity, packs: list[str]) -> dict[str, Any]:
    sample: set[str] = set()
    card = _card_for(entity, packs, sample)
    card_type = str(card.get("type", "tile"))
    if entity.domain == "alarm_control_panel":
        card_type = "tile"
    row = {
        "entity_id": entity.entity_id,
        "name": entity.name,
        "domain": entity.domain,
        "state": _preview_state(entity),
        "unit": _preview_unit(entity),
        "interaction": _interaction(entity),
        "card": card_type,
    }
    detail = _detail(entity)
    if detail:
        row["detail"] = detail
    return row


def _preview_fact(entity: RoomEntity | None, packs: list[str], role: str) -> dict[str, Any] | None:
    if entity is None:
        return None
    if entity.domain == "climate":
        if role == "humidity":
            state = entity.current_humidity
            unit = entity.unit
        else:
            state = entity.current_temperature
            unit = entity.temperature_unit
        return {
            "entity_id": entity.entity_id,
            "name": entity.name,
            "state": state,
            "unit": unit,
            "card": "thermostat",
        }
    card = "custom:mushroom-chips-card" if "mushroom" in packs else "tile"
    return {
        "entity_id": entity.entity_id,
        "name": entity.name,
        "state": entity.state,
        "unit": entity.unit,
        "card": card,
    }


def _preview_state(entity: RoomEntity) -> str | None:
    if entity.domain == "climate" and entity.current_temperature:
        return entity.current_temperature
    return entity.state


def _preview_unit(entity: RoomEntity) -> str | None:
    if entity.domain == "climate" and entity.current_temperature:
        return entity.temperature_unit
    return entity.unit


def _by_name(entity: RoomEntity) -> tuple[str, str]:
    return (entity.name.casefold(), entity.entity_id)


def _view_path(name: str, area_id: str) -> str:
    slug = slugify(name)
    if slug != "theme" or name.strip().casefold() == "theme":
        return slug
    alt = slugify(area_id)
    return alt if alt != "theme" else "room"


def _omitted_warning(skipped: list[RoomEntity]) -> str | None:
    counts: dict[str, int] = {}
    for entity in skipped:
        counts[entity.domain] = counts.get(entity.domain, 0) + 1
    if not counts:
        return None
    detail = ", ".join(f"{domain} ({count})" for domain, count in sorted(counts.items()))
    return f"Some entities in this area are not on the view: {detail}."


def _emit(lines: list[str], value: Any, indent: int) -> None:
    pad = " " * indent
    if isinstance(value, dict):
        if not value:
            lines.append(f"{pad}{{}}")
            return
        for key, item in value.items():
            rendered = _key(str(key))
            if isinstance(item, dict) and item:
                lines.append(f"{pad}{rendered}:")
                _emit(lines, item, indent + 2)
            elif isinstance(item, list) and item:
                lines.append(f"{pad}{rendered}:")
                _emit(lines, item, indent + 2)
            else:
                lines.append(f"{pad}{rendered}: {_scalar(item)}")
        return
    if isinstance(value, list):
        if not value:
            lines.append(f"{pad}[]")
            return
        for item in value:
            if isinstance(item, dict) and item:
                pairs = list(item.items())
                first_key, first_value = pairs[0]
                if isinstance(first_value, (dict, list)) and first_value:
                    lines.append(f"{pad}- {_key(str(first_key))}:")
                    _emit(lines, first_value, indent + 4)
                else:
                    lines.append(f"{pad}- {_key(str(first_key))}: {_scalar(first_value)}")
                for key, child in pairs[1:]:
                    rendered = _key(str(key))
                    if isinstance(child, (dict, list)) and child:
                        lines.append(f"{pad}  {rendered}:")
                        _emit(lines, child, indent + 4)
                    else:
                        lines.append(f"{pad}  {rendered}: {_scalar(child)}")
            else:
                lines.append(f"{pad}- {_scalar(item)}")
        return
    lines.append(f"{pad}{_scalar(value)}")


def _key(key: str) -> str:
    if key and all(char.isalnum() or char in "_-" for char in key):
        return key
    return _quote(key)


def _scalar(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if value is None:
        return "null"
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return str(value)
    if isinstance(value, dict):
        return "{}"
    if isinstance(value, list):
        return "[]"
    return _quote(str(value))


def _quote(text: str) -> str:
    escaped = text.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'

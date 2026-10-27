"""Room view generation: grouping, security policy, and no dashboard writes."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
import yaml

from custom_components.homechameleon.rooms import RoomEntity, build_room_view, choose_packs
from homeassistant.const import EntityCategory
from homeassistant.helpers import area_registry as ar
from homeassistant.helpers import device_registry as dr
from homeassistant.helpers import entity_registry as er


def _entity(
    entity_id: str,
    name: str,
    *,
    device_class: str | None = None,
    state: str | None = "on",
    unit: str | None = None,
    current_temperature: str | None = None,
    current_humidity: str | None = None,
    temperature_unit: str | None = None,
    supported_features: int = 0,
) -> RoomEntity:
    return RoomEntity(
        entity_id=entity_id,
        name=name,
        domain=entity_id.split(".", 1)[0],
        device_class=device_class,
        state=state,
        unit=unit,
        current_temperature=current_temperature,
        current_humidity=current_humidity,
        temperature_unit=temperature_unit,
        supported_features=supported_features,
    )


def _sample_room() -> list[RoomEntity]:
    return [
        _entity("light.ceiling", "Ceiling"),
        _entity("light.lamp", "Lamp", state="off"),
        _entity("climate.living_room", "Thermostat", state="heat", current_temperature="19.0", temperature_unit="°C"),
        _entity("media_player.speaker", "Speaker", state="playing"),
        _entity("cover.blinds", "Blinds", device_class="blind", state="open"),
        _entity("cover.garage_door", "Garage door", device_class="garage", state="closed"),
        _entity("sensor.living_room_temperature", "Living room temperature", device_class="temperature", state="21.5", unit="°C"),
        _entity("sensor.living_room_humidity", "Living room humidity", device_class="humidity", state="44", unit="%"),
        _entity("sensor.other_temperature", "Shelf temperature", device_class="temperature", state="18", unit="°C"),
        _entity("binary_sensor.motion", "Motion", device_class="motion", state="off"),
        _entity("scene.movie", "Movie", state="scening"),
        _entity("lock.front_door", "Front door", state="locked"),
        _entity("alarm_control_panel.home", "Home alarm", state="armed_home", supported_features=1),
        _entity("camera.front", "Front camera", state="idle"),
        _entity("switch.kettle", "Kettle", state="off"),
        _entity("fan.ceiling", "Fan", state="off"),
    ]


def _headings(view: dict) -> list[str]:
    titles = []
    for section in view["sections"]:
        for card in section["cards"]:
            if card.get("type") == "heading":
                titles.append(card["heading"])
    return titles


def _walk(node):
    if isinstance(node, dict):
        yield node
        for value in node.values():
            yield from _walk(value)
    elif isinstance(node, list):
        for item in node:
            yield from _walk(item)


def _cards_for(view: dict, entity_id: str) -> list[dict]:
    return [node for node in _walk(view) if node.get("entity") == entity_id]


def test_room_view_groups_core_cards_and_keeps_security_quiet():
    """Entities land in domain groups. Security cards confirm or stay view only."""
    result = build_room_view(
        area_id="living_room",
        name="Living room",
        entities=_sample_room(),
        packs=[],
        temperature_entity_id="sensor.living_room_temperature",
        humidity_entity_id="sensor.living_room_humidity",
    )
    assert result["wrote"] is False
    assert result["card_set"] == "core"
    assert result["enhancements"] == []
    view = result["view"]
    assert view["type"] == "sections"
    assert view["path"] == "living-room"
    assert _headings(view) == [
        "Living room",
        "Lights",
        "Climate",
        "Media",
        "Covers",
        "Sensors",
        "Scenes",
        "Security",
        "Cameras",
    ]
    assert yaml.safe_load(result["yaml"]) == view
    assert "custom:" not in result["yaml"]
    assert "alarm-panel" not in result["yaml"]

    header = view["sections"][0]["cards"]
    assert header[0]["heading"] == "Living room"
    assert [card["entity"] for card in header if card.get("entity")] == [
        "sensor.living_room_temperature",
        "sensor.living_room_humidity",
    ]
    lights_all = header[-1]
    assert lights_all["name"] == "All lights"
    assert lights_all["tap_action"]["perform_action"] == "light.toggle"
    assert lights_all["tap_action"]["target"] == {"area_id": "living_room"}
    assert "confirmation" not in lights_all["tap_action"]

    assert _cards_for(view, "sensor.living_room_temperature") == [
        {"type": "tile", "entity": "sensor.living_room_temperature", "show_state": True}
    ]
    assert [card["entity"] for card in _cards_for(view, "sensor.other_temperature")] == [
        "sensor.other_temperature"
    ]

    ceiling = _cards_for(view, "light.ceiling")[0]
    assert ceiling["type"] == "tile"
    assert ceiling["features"] == [{"type": "light-brightness"}]
    assert _cards_for(view, "climate.living_room")[0]["type"] == "thermostat"
    assert _cards_for(view, "media_player.speaker")[0]["type"] == "media-control"

    blinds = _cards_for(view, "cover.blinds")[0]
    assert blinds["features"] == [{"type": "cover-open-close"}]
    assert "confirmation" not in json.dumps(blinds)

    garage = _cards_for(view, "cover.garage_door")[0]
    assert garage["type"] == "tile"
    assert "features" not in garage
    assert garage["tap_action"]["confirmation"]["text"] == "Open or close Garage door?"
    assert garage["hold_action"] == {"action": "none"}

    lock = _cards_for(view, "lock.front_door")[0]
    assert lock["tap_action"]["action"] == "toggle"
    assert "Change Front door?" in lock["tap_action"]["confirmation"]["text"]

    alarm_nodes = [
        node
        for node in _walk(view)
        if node.get("perform_action", "").startswith("alarm_control_panel.")
    ]
    assert [node["perform_action"] for node in alarm_nodes] == [
        "alarm_control_panel.alarm_arm_home",
        "alarm_control_panel.alarm_disarm",
    ]
    assert all("confirmation" in node for node in alarm_nodes)
    assert not any(node.get("type") == "alarm-panel" for node in _walk(view))

    camera = _cards_for(view, "camera.front")[0]
    assert camera["type"] == "picture-entity"
    assert camera["tap_action"] == {"action": "none"}
    assert camera["hold_action"] == {"action": "none"}
    assert camera["double_tap_action"] == {"action": "none"}

    preview = result["preview"]
    assert preview["header"]["lights_toggle"] is True
    assert preview["header"]["temperature"]["state"] == "21.5"
    assert preview["header"]["temperature"]["unit"] == "°C"
    assert [section["id"] for section in preview["sections"]] == [
        "lights",
        "climate",
        "media",
        "covers",
        "sensors",
        "scenes",
        "security",
        "cameras",
    ]
    interactions = {
        card["entity_id"]: card["interaction"]
        for section in preview["sections"]
        for card in section["cards"]
    }
    assert interactions["camera.front"] == "view"
    assert interactions["lock.front_door"] == "confirm"
    assert interactions["cover.garage_door"] == "confirm"
    assert interactions["alarm_control_panel.home"] == "confirm"
    assert interactions["light.ceiling"] == "direct"
    assert interactions["cover.blinds"] == "direct"
    assert "switch" in result["warnings"][0]
    assert "fan" in result["warnings"][0]
    assert " HA " not in f" {json.dumps(result)} "


def test_optional_cards_replace_core_cards_except_security():
    """Mushroom and Bubble Card are optional. Security domains stay on core cards."""
    entities = _sample_room()
    mushroom = build_room_view(
        area_id="living_room",
        name="Living room",
        entities=entities,
        packs=["mushroom"],
        temperature_entity_id="sensor.living_room_temperature",
        humidity_entity_id="sensor.living_room_humidity",
        both_packs=True,
    )
    assert mushroom["card_set"] == "enhanced"
    assert mushroom["enhancements"] == ["mushroom"]
    assert any("Mushroom and Bubble Card" in note for note in mushroom["warnings"])
    view = mushroom["view"]
    assert _cards_for(view, "light.ceiling")[0]["type"] == "custom:mushroom-light-card"
    assert _cards_for(view, "climate.living_room")[0]["type"] == "custom:mushroom-climate-card"
    assert _cards_for(view, "cover.blinds")[0]["type"] == "custom:mushroom-cover-card"
    assert _cards_for(view, "cover.garage_door")[0]["type"] == "tile"
    assert _cards_for(view, "lock.front_door")[0]["type"] == "tile"
    assert _cards_for(view, "camera.front")[0]["type"] == "picture-entity"
    assert view["sections"][0]["cards"][1]["type"] == "custom:mushroom-chips-card"
    assert "custom:bubble-card" not in mushroom["yaml"]

    bubble = build_room_view(area_id="living_room", name="Living room", entities=entities, packs=["bubble-card"])
    assert bubble["enhancements"] == ["bubble-card"]
    bubble_view = bubble["view"]
    assert _cards_for(bubble_view, "light.ceiling")[0]["type"] == "custom:bubble-card"
    assert _cards_for(bubble_view, "scene.movie")[0]["type"] == "custom:bubble-card"
    assert _cards_for(bubble_view, "climate.living_room")[0]["type"] == "thermostat"
    assert _cards_for(bubble_view, "lock.front_door")[0]["type"] == "tile"


def test_header_can_use_climate_when_no_temperature_sensor_exists():
    """A climate reading fills the header without a second copy of that card."""
    result = build_room_view(
        area_id="den",
        name="Den",
        entities=[
            _entity(
                "climate.den",
                "Den thermostat",
                state="heat",
                current_temperature="20",
                current_humidity="40",
                temperature_unit="°C",
            )
        ],
        packs=[],
    )
    assert result["preview"]["header"]["temperature"]["state"] == "20"
    assert result["preview"]["header"]["humidity"]["state"] == "40"
    assert result["preview"]["header"]["lights_toggle"] is False
    assert _headings(result["view"]) == ["Den", "Climate"]
    assert "All lights" not in result["yaml"]


def test_choose_packs_prefers_mushroom():
    """One optional pack is enough, and Mushroom takes the place of Bubble Card."""
    assert choose_packs(set()) == []
    assert choose_packs({"bubble-card"}) == ["bubble-card"]
    assert choose_packs({"mushroom", "bubble-card"}) == ["mushroom"]


def test_non_ascii_area_name_uses_the_area_id_as_the_path():
    """A name that cannot be a path still produces a stable view path."""
    result = build_room_view(area_id="sun_room", name="客厅", entities=[], packs=[])
    assert result["view"]["path"] == "sun_room"
    assert result["warnings"] == ["This area has no entities to place on the view."]


def _add_entity(
    hass,
    domain: str,
    unique: str,
    name: str,
    *,
    area_id: str | None = None,
    device_id: str | None = None,
    device_class: str | None = None,
    disabled: bool = False,
    category: EntityCategory | None = None,
    features: int | None = None,
):
    registry = er.async_get(hass)
    kwargs = {}
    if device_class is not None:
        kwargs["original_device_class"] = device_class
    if device_id is not None:
        kwargs["device_id"] = device_id
    if disabled:
        kwargs["disabled_by"] = er.RegistryEntryDisabler.USER
    if category is not None:
        kwargs["entity_category"] = category
    if features is not None:
        kwargs["supported_features"] = features
    entry = registry.async_get_or_create(
        domain,
        "test",
        unique,
        suggested_object_id=unique,
        original_name=name,
        **kwargs,
    )
    if area_id is not None:
        entry = registry.async_update_entity(entry.entity_id, area_id=area_id)
    return entry


async def test_room_endpoint_reads_the_area_and_writes_nothing(hass, setup_hc, hass_client):
    """The admin endpoint returns a view for the area and leaves dashboards alone."""
    area = ar.async_get(hass).async_create("Living room")
    kitchen = ar.async_get(hass).async_create("Kitchen")
    device = dr.async_get(hass).async_get_or_create(
        config_entry_id=setup_hc.entry_id,
        identifiers={("homechameleon_test", "lamp")},
        name="Floor lamp",
    )
    dr.async_get(hass).async_update_device(device.id, area_id=area.id)

    _add_entity(hass, "light", "ceiling", "Ceiling", area_id=area.id)
    _add_entity(hass, "light", "lamp", "Lamp", device_id=device.id)
    _add_entity(hass, "light", "kitchen", "Kitchen light", area_id=kitchen.id)
    _add_entity(hass, "light", "hidden_bulb", "Hidden bulb", area_id=area.id, disabled=True)
    _add_entity(
        hass,
        "sensor",
        "living_room_temperature",
        "Living room temperature",
        area_id=area.id,
        device_class="temperature",
    )
    _add_entity(
        hass,
        "sensor",
        "living_room_humidity",
        "Living room humidity",
        area_id=area.id,
        device_class="humidity",
    )
    _add_entity(
        hass,
        "sensor",
        "signal",
        "Signal",
        area_id=area.id,
        category=EntityCategory.DIAGNOSTIC,
    )
    _add_entity(hass, "cover", "garage_door", "Garage door", area_id=area.id, device_class="garage")
    _add_entity(hass, "camera", "front", "Front camera", area_id=area.id)
    _add_entity(hass, "lock", "front_door", "Front door", area_id=area.id)
    _add_entity(hass, "switch", "kettle", "Kettle", area_id=area.id)
    hass.states.async_set(
        "sensor.living_room_temperature",
        "21.5",
        {"device_class": "temperature", "unit_of_measurement": "°C"},
    )
    hass.states.async_set(
        "sensor.living_room_humidity",
        "44",
        {"device_class": "humidity", "unit_of_measurement": "%"},
    )
    hass.states.async_set("light.ceiling", "on", {})
    hass.states.async_set("light.lamp", "off", {})
    ar.async_get(hass).async_update(
        area.id,
        temperature_entity_id="sensor.living_room_temperature",
        humidity_entity_id="sensor.living_room_humidity",
    )

    themes = sorted(path.name for path in Path(hass.config.path("themes")).glob("*"))
    client = await hass_client()
    listed = await client.get("/api/homechameleon/areas")
    assert listed.status == 200
    areas = await listed.json()
    living = next(item for item in areas["areas"] if item["name"] == "Living room")
    assert living["id"] == area.id
    assert living["entities"] >= 5

    response = await client.post("/api/homechameleon/room", json={"area_id": area.id})
    assert response.status == 200
    payload = await response.json()
    assert payload["wrote"] is False
    assert payload["name"] == "Living room"
    assert payload["card_set"] == "core"
    ids = [card["entity_id"] for section in payload["preview"]["sections"] for card in section["cards"]]
    assert "light.lamp" in ids
    assert "light.kitchen" not in ids
    assert "light.hidden_bulb" not in ids
    assert "sensor.signal" not in ids
    assert "switch.kettle" not in ids
    assert ids[-1] == "camera.front"
    assert payload["preview"]["header"]["temperature"]["state"] == "21.5"
    assert yaml.safe_load(payload["yaml"]) == payload["view"]
    assert sorted(path.name for path in Path(hass.config.path("themes")).glob("*")) == themes
    assert not Path(hass.config.path(".storage", "lovelace")).exists()
    assert not Path(hass.config.path(".storage", "lovelace_dashboards")).exists()
    assert hass.data["homechameleon_service_calls"] == []

    by_name = await client.post("/api/homechameleon/room", json={"area": "living room"})
    assert by_name.status == 200
    assert (await by_name.json())["area_id"] == area.id

    missing = await client.post("/api/homechameleon/room", json={"area_id": "nope"})
    assert missing.status == 404
    empty = await client.post("/api/homechameleon/room", json={})
    assert empty.status == 400


async def test_room_routes_are_admin_only(hass, setup_hc, hass_client, hass_client_no_auth, hass_read_only_access_token):
    """Anonymous and non-admin callers cannot list areas or build a view."""
    admin = await hass_client()
    assert (await admin.get("/api/homechameleon/areas")).status == 200

    anonymous = await hass_client_no_auth()
    assert (await anonymous.get("/api/homechameleon/areas")).status == 401
    assert (await anonymous.post("/api/homechameleon/room", json={"area_id": "x"})).status == 401

    reader = await hass_client(hass_read_only_access_token)
    assert (await reader.get("/api/homechameleon/areas")).status == 403
    assert (await reader.post("/api/homechameleon/room", json={"area_id": "x"})).status == 403


async def test_resource_list_on_the_lovelace_data_key_selects_mushroom(hass, setup_hc, hass_client):
    """A loaded resource list is read from hass.data without importing that component."""
    area = ar.async_get(hass).async_create("Study")
    _add_entity(hass, "light", "study", "Study", area_id=area.id)
    hass.states.async_set("light.study", "on", {})

    class _Resources:
        loaded = True

        @staticmethod
        def async_items():
            return [{"url": "/local/community/lovelace-mushroom/mushroom.js"}]

    class _Lovelace:
        resources = _Resources()

    hass.data["lovelace"] = _Lovelace()
    client = await hass_client()
    response = await client.post("/api/homechameleon/room", json={"area_id": area.id})
    assert response.status == 200
    payload = await response.json()
    assert payload["enhancements"] == ["mushroom"]
    source = Path(__file__).resolve().parents[1] / "custom_components/homechameleon/rooms.py"
    assert "homeassistant.components.lovelace" not in source.read_text(encoding="utf-8")


async def test_installed_cards_are_detected_for_the_view(hass, setup_hc, hass_client):
    """A detected Mushroom install changes non-security cards only."""
    area = ar.async_get(hass).async_create("Office")
    _add_entity(hass, "light", "desk", "Desk", area_id=area.id)
    _add_entity(hass, "lock", "office_door", "Office door", area_id=area.id)
    hass.states.async_set("light.desk", "on", {})
    www = Path(hass.config.path("www"))
    www_existed = www.exists()
    bubble = www / "community" / "Bubble-Card"
    bubble.mkdir(parents=True, exist_ok=True)
    hass.config.components.add("mushroom")

    try:
        client = await hass_client()
        response = await client.post("/api/homechameleon/room", json={"area": "Office"})
        assert response.status == 200
        payload = await response.json()
        assert payload["enhancements"] == ["mushroom"]
        assert any("Bubble Card" in note for note in payload["warnings"])
        types = [node.get("type") for node in _walk(payload["view"])]
        assert "custom:mushroom-light-card" in types
        assert "custom:bubble-card" not in types
        lock = _cards_for(payload["view"], "lock.office_door")[0]
        assert lock["type"] == "tile"
        assert "confirmation" in lock["tap_action"]
        assert not Path(hass.config.path(".storage", "lovelace")).exists()
    finally:
        if bubble.exists():
            bubble.rmdir()
        community = bubble.parent
        if community.exists() and not any(community.iterdir()):
            community.rmdir()
        if not www_existed and www.exists() and not any(www.iterdir()):
            www.rmdir()

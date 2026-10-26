import { describe, expect, it } from "vitest";

import { ROOM_NOTE, cardSetLabel, formatState, interactionLabel, parseRoom } from "./room";

const preview = {
  header: {
    title: "Living room",
    temperature: { entity_id: "sensor.temp", name: "Temperature", state: "21.5", unit: "°C", card: "tile" },
    humidity: null,
    lights_toggle: true,
  },
  sections: [
    {
      id: "cameras",
      title: "Cameras",
      cards: [
        {
          entity_id: "camera.front",
          name: "Front camera",
          domain: "camera",
          state: "idle",
          unit: null,
          interaction: "view",
          card: "picture-entity",
          detail: "View only.",
        },
      ],
    },
  ],
};

describe("room preview payload", () => {
  it("keeps the preview note in product language", () => {
    expect(ROOM_NOTE).toContain("dashboard");
    expect(ROOM_NOTE).not.toMatch(/\bHA\b/);
    expect(interactionLabel("confirm")).toBe("Confirm");
    expect(interactionLabel("view")).toBe("View only");
    expect(interactionLabel("direct")).toBeNull();
    expect(formatState("21.5", "°C")).toBe("21.5 °C");
    expect(formatState(null, "°C")).toBe("No state");
  });

  it("accepts a view that was not written", () => {
    const room = parseRoom({
      area_id: "living_room",
      name: "Living room",
      view: { title: "Living room", type: "sections" },
      yaml: "title: \"Living room\"\n",
      card_set: "core",
      enhancements: [],
      warnings: [],
      wrote: false,
      preview,
    });
    expect(room?.preview.header.title).toBe("Living room");
    expect(room?.wrote).toBe(false);
    expect(cardSetLabel(room!)).toBe("Core cards.");
  });

  it("rejects a payload that claims the view was written", () => {
    expect(
      parseRoom({
        area_id: "living_room",
        name: "Living room",
        view: { title: "Living room" },
        yaml: "title: \"Living room\"\n",
        card_set: "core",
        enhancements: [],
        warnings: [],
        wrote: true,
        preview,
      }),
    ).toBeNull();
  });
});

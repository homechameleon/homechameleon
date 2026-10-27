export type RoomInteraction = "direct" | "confirm" | "view";

export interface RoomFact {
  entity_id: string;
  name: string;
  state: string | null;
  unit: string | null;
  card: string;
}

export interface RoomCard {
  entity_id: string;
  name: string;
  domain: string;
  state: string | null;
  unit: string | null;
  interaction: RoomInteraction;
  card: string;
  detail?: string;
}

export interface RoomSection {
  id: string;
  title: string;
  cards: RoomCard[];
}

export interface RoomPreview {
  header: {
    title: string;
    temperature: RoomFact | null;
    humidity: RoomFact | null;
    lights_toggle: boolean;
  };
  sections: RoomSection[];
}

export interface RoomPayload {
  area_id: string;
  name: string;
  view: Record<string, unknown>;
  yaml: string;
  card_set: string;
  enhancements: string[];
  warnings: string[];
  wrote: false;
  preview: RoomPreview;
}

export interface AreaSummary {
  id: string;
  name: string;
  entities: number;
}

export const ROOM_NOTE = "This preview is not applied. HomeChameleon does not write the view to a dashboard.";

export function formatState(state: string | null, unit: string | null): string {
  if (!state) return "No state";
  if (unit && state !== "unknown" && state !== "unavailable") return `${state} ${unit}`;
  return state;
}

export function interactionLabel(interaction: RoomInteraction): string | null {
  if (interaction === "confirm") return "Confirm";
  if (interaction === "view") return "View only";
  return null;
}

export function cardSetLabel(payload: RoomPayload): string {
  if (payload.enhancements.length === 0) return "Core cards.";
  const names = payload.enhancements.map((pack) => (pack === "bubble-card" ? "Bubble Card" : "Mushroom"));
  return `${names.join(" and ")} cards are included. Locks, garage covers, alarms, and cameras stay on core cards.`;
}

export function parseAreas(value: unknown): AreaSummary[] | null {
  if (!value || typeof value !== "object") return null;
  const areas = (value as { areas?: unknown }).areas;
  if (!Array.isArray(areas)) return null;
  const parsed: AreaSummary[] = [];
  for (const area of areas) {
    if (!area || typeof area !== "object") return null;
    const row = area as { id?: unknown; name?: unknown; entities?: unknown };
    if (typeof row.id !== "string" || typeof row.name !== "string" || typeof row.entities !== "number") return null;
    parsed.push({ id: row.id, name: row.name, entities: row.entities });
  }
  return parsed;
}

export function parseRoom(value: unknown): RoomPayload | null {
  if (!value || typeof value !== "object") return null;
  const body = value as Record<string, unknown>;
  if (body.wrote !== false) return null;
  if (typeof body.area_id !== "string" || typeof body.name !== "string" || typeof body.yaml !== "string") return null;
  if (!body.view || typeof body.view !== "object" || Array.isArray(body.view)) return null;
  if (typeof body.card_set !== "string" || !Array.isArray(body.enhancements) || !Array.isArray(body.warnings)) return null;
  if (!body.enhancements.every((item) => typeof item === "string")) return null;
  if (!body.warnings.every((item) => typeof item === "string")) return null;
  const preview = parsePreview(body.preview);
  if (!preview) return null;
  return {
    area_id: body.area_id,
    name: body.name,
    view: body.view as Record<string, unknown>,
    yaml: body.yaml,
    card_set: body.card_set,
    enhancements: body.enhancements as string[],
    warnings: body.warnings as string[],
    wrote: false,
    preview,
  };
}

function parsePreview(value: unknown): RoomPreview | null {
  if (!value || typeof value !== "object") return null;
  const preview = value as { header?: unknown; sections?: unknown };
  if (!preview.header || typeof preview.header !== "object" || !Array.isArray(preview.sections)) return null;
  const header = preview.header as Record<string, unknown>;
  if (typeof header.title !== "string" || typeof header.lights_toggle !== "boolean") return null;
  const temperature = parseFact(header.temperature);
  const humidity = parseFact(header.humidity);
  if (temperature === undefined || humidity === undefined) return null;
  const sections: RoomSection[] = [];
  for (const section of preview.sections) {
    const parsed = parseSection(section);
    if (!parsed) return null;
    sections.push(parsed);
  }
  return { header: { title: header.title, temperature, humidity, lights_toggle: header.lights_toggle }, sections };
}

function parseFact(value: unknown): RoomFact | null | undefined {
  if (value === null) return null;
  if (!value || typeof value !== "object") return undefined;
  const fact = value as Record<string, unknown>;
  if (typeof fact.entity_id !== "string" || typeof fact.name !== "string" || typeof fact.card !== "string") return undefined;
  if (fact.state !== null && typeof fact.state !== "string") return undefined;
  if (fact.unit !== null && typeof fact.unit !== "string") return undefined;
  return {
    entity_id: fact.entity_id,
    name: fact.name,
    state: (fact.state as string | null) ?? null,
    unit: (fact.unit as string | null) ?? null,
    card: fact.card,
  };
}

function parseSection(value: unknown): RoomSection | null {
  if (!value || typeof value !== "object") return null;
  const section = value as Record<string, unknown>;
  if (typeof section.id !== "string" || typeof section.title !== "string" || !Array.isArray(section.cards)) return null;
  const cards: RoomCard[] = [];
  for (const card of section.cards) {
    const parsed = parseCard(card);
    if (!parsed) return null;
    cards.push(parsed);
  }
  return { id: section.id, title: section.title, cards };
}

function parseCard(value: unknown): RoomCard | null {
  if (!value || typeof value !== "object") return null;
  const card = value as Record<string, unknown>;
  if (typeof card.entity_id !== "string" || typeof card.name !== "string" || typeof card.domain !== "string") return null;
  if (typeof card.card !== "string") return null;
  if (card.interaction !== "direct" && card.interaction !== "confirm" && card.interaction !== "view") return null;
  if (card.state !== null && typeof card.state !== "string") return null;
  if (card.unit !== null && typeof card.unit !== "string") return null;
  if (card.detail !== undefined && typeof card.detail !== "string") return null;
  return {
    entity_id: card.entity_id,
    name: card.name,
    domain: card.domain,
    state: (card.state as string | null) ?? null,
    unit: (card.unit as string | null) ?? null,
    interaction: card.interaction,
    card: card.card,
    detail: typeof card.detail === "string" ? card.detail : undefined,
  };
}

export async function fetchAreas(): Promise<AreaSummary[]> {
  const response = await fetch("/api/homechameleon/areas", { credentials: "same-origin" });
  const payload = (await response.json().catch(() => null)) as { error?: string } | null;
  if (!response.ok) throw new Error(payload?.error || "Areas could not be read");
  const areas = parseAreas(payload);
  if (!areas) throw new Error("Areas could not be read");
  return areas;
}

export async function fetchRoom(areaId: string): Promise<RoomPayload> {
  const response = await fetch("/api/homechameleon/room", {
    method: "POST",
    credentials: "same-origin",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ area_id: areaId }),
  });
  const payload = (await response.json().catch(() => null)) as { error?: string } | null;
  if (!response.ok) throw new Error(payload?.error || "The room view could not be built");
  const room = parseRoom(payload);
  if (!room) throw new Error("The room view could not be built");
  return room;
}

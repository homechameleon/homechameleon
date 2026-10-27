import { LitElement, html } from "lit";
import { customElement, state } from "lit/decorators.js";
import { unsafeCSS } from "lit";
import { repeat } from "lit/directives/repeat.js";

import appCss from "./app.css?inline";
import "./preview";
import "./room-preview";
import { CoreBridge } from "./bridge";
import { decodeImage, encodeWallpaper, MAX_BYTES, MAX_IMAGES } from "./images";
import {
  type DeviceId,
  type ExtractionId,
  type ModeId,
  type PreviewSide,
  type RoleKey,
  ROLE_KEYS,
  ROLE_LABEL,
  STYLE_DEFAULTS,
  STYLE_IDS,
  type StyleId,
  type TabId,
  blurApplies,
  styleLabel,
  STYLE_BLURB,
} from "./names";
import {
  type EditorSnapshot,
  applyLocks,
  applyLocksToYaml,
  buildRequest,
  cssForSide,
  decodePaletteParam,
  detectPalette,
  fnv32,
  freshSnapshot,
  slugify,
  stitchCss,
  stitchYaml,
  varCount,
  viewTargetFromDashboard,
} from "./model";
import { type GenResult, type RoleColors, parseResult, rampColors } from "./result";
import { buildZip, cardModSnippet } from "./zip";
import {
  ROOM_NOTE,
  type AreaSummary,
  type RoomPayload,
  cardSetLabel,
  fetchAreas,
  fetchRoom,
} from "./room";

const BLUEPRINTS_KEY = "homechameleon.blueprints";

type HostMode = "panel" | "addon" | "standalone";

function hostMode(): HostMode {
  const params = new URLSearchParams(window.location.search);
  if (params.get("addon") === "1") return "addon";
  const meta = document.querySelector('meta[name="homechameleon-host"]');
  if (meta instanceof HTMLMetaElement && meta.content === "addon") return "addon";
  if (params.get("panel") === "1") return "panel";
  return "standalone";
}

function bytesToBase64(bytes: Uint8Array): string {
  let text = "";
  const chunk = 0x8000;
  for (let index = 0; index < bytes.length; index += chunk) {
    text += String.fromCharCode(...bytes.subarray(index, index + chunk));
  }
  return btoa(text);
}

interface BoardImage {
  id: string;
  name: string;
  weight: number;
  useAsBackground: boolean;
  width: number;
  height: number;
  rgba: string;
  raw: Uint8Array;
  previewUrl: string;
  source: Blob;
}

interface Hist {
  snap: EditorSnapshot;
  imageIds: string[];
}

interface Blueprint {
  id: string;
  savedAt: number;
  name: string;
  snap: EditorSnapshot;
}

type Job = "fast" | "full" | "extract";

const SLIDERS: { key: keyof EditorSnapshot["tweaks"]; label: string; min: number; max: number; step: number }[] = [
  { key: "roundness", label: "Roundness", min: 0, max: 40, step: 1 },
  { key: "blur", label: "Blur strength", min: 0, max: 40, step: 1 },
  { key: "transparency", label: "Transparency", min: 0, max: 100, step: 1 },
  { key: "glow", label: "Glow / sheen", min: 0, max: 100, step: 1 },
  { key: "saturation", label: "Saturation", min: -100, max: 100, step: 1 },
  { key: "temperature", label: "Temperature", min: -50, max: 50, step: 1 },
];

@customElement("hc-app")
export class HcApp extends LitElement {
  static styles = unsafeCSS(appCss);

  @state() private snap = freshSnapshot();
  @state() private imageIds: string[] = [];
  @state() private tab: TabId = "editor";
  @state() private device: DeviceId = "desktop";
  @state() private previewCss = "";
  @state() private exportYaml = "";
  @state() private exportCss = "";
  @state() private roles: Partial<Record<PreviewSide, RoleColors>> = {};
  @state() private paletteJson = "";
  @state() private contrastPass = true;
  @state() private contrastRatio = "";
  @state() private warnings: string[] = [];
  @state() private error = "";
  @state() private notice = "";
  @state() private blueprints: Blueprint[] = [];
  @state() private pastCount = 0;
  @state() private futureCount = 0;
  @state() private dragOver = false;
  @state() private paletteText = "";
  @state() private applying = false;
  @state() private applyDefault = false;
  @state() private applyTarget = "";
  @state() private viewChoices: { key: string; label: string }[] = [];
  @state() private undoReady = false;
  @state() private areas: AreaSummary[] = [];
  @state() private areaId = "";
  @state() private room: RoomPayload | null = null;
  @state() private roomError = "";
  @state() private roomLoading = false;

  private readonly hostMode = hostMode();
  private get panelMode(): boolean {
    return this.hostMode === "panel";
  }

  private readonly bridge = new CoreBridge();
  private readonly bank = new Map<string, BoardImage>();
  private past: Hist[] = [];
  private future: Hist[] = [];
  private cacheKey = "";
  private cache: Partial<Record<PreviewSide, string>> = {};
  private cssParts: Partial<Record<PreviewSide, string>> = {};
  private yamlParts: Partial<Record<PreviewSide, string>> = {};
  private basePreview = "";
  private baseYaml = "";
  private baseCss = "";
  private bgBytes: Uint8Array | null = null;
  private bgFileName: string | null = null;
  private bgKey = "";
  private dirty: Job | null = null;
  private frame = 0;
  private tail: Promise<void> = Promise.resolve();
  private slideCommitted = false;
  private lastGenMs = 0;
  private genCount = 0;

  connectedCallback(): void {
    super.connectedCallback();
    this.blueprints = readBlueprints();
    const params = new URLSearchParams(location.search);
    const palette = params.get("palette");
    if (palette) {
      const detected = detectPalette(decodePaletteParam(palette));
      if (detected) {
        this.snap = { ...this.snap, importedFormat: detected.format, importedData: detected.data };
        this.paletteText = detected.data;
      }
    }
    this.schedule(this.snap.importedData ? "full" : "fast");
    if (this.panelMode) void this.loadPanelState();
    window.addEventListener("keydown", this.onKey);
  }

  disconnectedCallback(): void {
    window.removeEventListener("keydown", this.onKey);
    super.disconnectedCallback();
  }

  protected updated(): void {
    if (this.previewCss) this.setAttribute("data-ready", "");
    else this.removeAttribute("data-ready");
    this.setAttribute("data-gen-ms", String(this.lastGenMs));
    this.setAttribute("data-gen-count", String(this.genCount));
  }

  private readonly onKey = (event: KeyboardEvent) => {
    if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== "z") return;
    event.preventDefault();
    if (event.shiftKey) this.redo();
    else this.undo();
  };

  render() {
    const images = this.images();
    return html`
      <div class="shell">
        <header>
          <div class="brand">
            <strong>HomeChameleon</strong>
            <span>for Home Assistant</span>
          </div>
          <input class="name" aria-label="Theme name" .value=${this.snap.name} @change=${this.onName} />
          <div class="tabs" role="tablist">
            ${this.tabs().map(
              (tab) => html`
                <button role="tab" aria-selected=${this.tab === tab} @click=${() => this.openTab(tab)}>
                  ${tab === "editor" ? "Editor" : tab === "wallpaper" ? "Wallpaper" : tab === "blueprints" ? "Blueprints" : tab === "room" ? "Room" : "YAML"}
                </button>
              `,
            )}
          </div>
          <span class="spacer"></span>
          <button class="ghost" ?disabled=${this.pastCount === 0} @click=${this.undo}>Undo</button>
          <button class="ghost" ?disabled=${this.futureCount === 0} @click=${this.redo}>Redo</button>
          <button class="ghost" @click=${this.saveBlueprint}>Save blueprint</button>
          <button class="primary" ?disabled=${!this.exportYaml} @click=${() => void this.exportZip()}>Export zip</button>
          ${this.hostMode === "standalone"
            ? null
            : html`
                ${this.panelMode
                  ? html`
                      <select class="apply-target" aria-label="Apply to" @change=${this.onApplyTarget}>
                        <option value="" ?selected=${this.applyTarget === ""}>Theme files only</option>
                        ${this.viewChoices.map(
                          (choice) => html`
                            <option value=${choice.key} ?selected=${this.applyTarget === choice.key}>${choice.label}</option>
                          `,
                        )}
                      </select>
                    `
                  : null}
                <label class="apply-default">
                  <input
                    type="checkbox"
                    .checked=${this.applyDefault}
                    @change=${(event: Event) => {
                      this.applyDefault = (event.target as HTMLInputElement).checked;
                    }}
                  />
                  Use as the default theme
                </label>
                ${this.panelMode
                  ? html`
                      <button class="ghost" ?disabled=${!this.undoReady || this.applying} @click=${() => void this.undoTheme()}>
                        Undo theme
                      </button>
                    `
                  : null}
                <button class="primary" ?disabled=${!this.exportYaml || this.applying} @click=${() => void this.applyTheme()}>
                  Apply
                </button>
              `}
        </header>
        <div class="body">
          <aside>
            ${this.tab === "editor" ? this.editorPanel(images) : null}
            ${this.tab === "wallpaper" ? this.wallpaperPanel(images) : null}
            ${this.tab === "blueprints" ? this.blueprintPanel() : null}
            ${this.tab === "room" ? this.roomPanel() : null}
            ${this.tab === "yaml" ? html`<pre class="yaml">${this.lockedYaml() || "The theme file appears after a photo or a palette."}</pre>` : null}
            ${this.error ? html`<p class="error">${this.error}</p>` : null}
            ${this.notice ? html`<p class="warn">${this.notice}</p>` : null}
          </aside>
          <section class="stage-wrap">
            <div class="preview-bar">
              <span>${this.tab === "room" ? "Room preview" : "Live preview"}</span>
              <span class="spacer"></span>
              <div class="devices" role="group" aria-label="Preview device">
                ${(["desktop", "tablet", "phone"] as const).map(
                  (device) => html`
                    <button aria-pressed=${this.device === device} @click=${() => (this.device = device)}>
                      ${device === "desktop" ? "Desktop" : device === "tablet" ? "Tablet" : "Phone"}
                    </button>
                  `,
                )}
              </div>
              ${this.snap.mode === "both"
                ? html`
                    <div class="devices" role="group" aria-label="Preview side">
                      ${(["dark", "light"] as const).map(
                        (side) => html`
                          <button aria-pressed=${this.snap.previewSide === side} @click=${() => this.setPreviewSide(side)}>
                            ${side === "dark" ? "Dark" : "Light"}
                          </button>
                        `,
                      )}
                    </div>
                  `
                : null}
            </div>
            ${this.tab === "room"
              ? html`
                  <div class="room-compare ${this.roomSides().length === 1 ? "single" : ""}">
                    ${this.roomSides().map(
                      (side) => html`
                        <hc-room-preview
                          .cssText=${this.sideCss(side)}
                          .device=${this.device}
                          .side=${side}
                          .preview=${this.room?.preview ?? null}
                        ></hc-room-preview>
                      `,
                    )}
                  </div>
                  <p class="room-footnote">${ROOM_NOTE}</p>
                `
              : html`
                  <hc-preview
                    .cssText=${this.previewCss}
                    .wallpaper=${images.find((image) => image.useAsBackground)?.previewUrl ?? images[0]?.previewUrl ?? null}
                    .device=${this.device}
                    .styleId=${this.snap.style}
                    .backgroundOn=${this.snap.backgroundEnabled}
                  ></hc-preview>
                `}
          </section>
        </div>
        <footer>
          <span>${this.contrastLine()}</span>
          <span>${this.previewCss ? `${varCount(this.previewCss)} theme variables` : ""}</span>
          <span>${this.warnings[0] ?? ""}</span>
        </footer>
      </div>
    `;
  }

  private editorPanel(images: BoardImage[]) {
    const paletteOn = Boolean(this.snap.importedData);
    return html`
      <section>
        <h2>Images</h2>
        <div
          class="drop ${this.dragOver ? "over" : ""}"
          @dragover=${this.onDragOver}
          @dragleave=${() => (this.dragOver = false)}
          @drop=${this.onDrop}
        >
          Drop photos or a palette file
          <div><input type="file" accept="image/*,.toml,.json,.txt,.yaml,.yml" multiple @change=${this.onFiles} /></div>
        </div>
        <div class="thumbs">
          ${repeat(
            images,
            (image) => image.id,
            (image) => html`
              <div class="thumb">
                <img src=${image.previewUrl} alt="" />
                <div>
                  <label>Weight <input type="range" min="0.2" max="4" step="0.1" .value=${String(image.weight)} @input=${(event: Event) => this.onWeight(image.id, event)} @change=${() => { this.slideCommitted = false; this.schedule("extract"); }} /></label>
                  <label><input type="radio" name="bg" .checked=${image.useAsBackground} @change=${() => this.markBackground(image.id)} /> Wallpaper</label>
                  <button class="ghost" @click=${() => this.removeImage(image.id)}>Remove</button>
                </div>
              </div>
            `,
          )}
        </div>
      </section>
      <section>
        <h2>Extraction</h2>
        <div class="extract">
          ${(["normal", "pastel", "material", "colorful", "muted"] as const).map(
            (mode) => html`
              <button aria-pressed=${this.snap.extraction === mode} ?disabled=${paletteOn} @click=${() => this.setExtraction(mode)}>
                ${mode[0].toUpperCase()}${mode.slice(1)}
              </button>
            `,
          )}
        </div>
        ${paletteOn ? html`<p class="note">Colors come from the loaded palette. Photos are not used for colors.</p>` : null}
      </section>
      <section>
        <h2>Palette</h2>
        <textarea
          aria-label="Palette text"
          .value=${this.paletteText}
          placeholder="Paste Omarchy colors.toml, Base16, a hex list, or homechameleon-palette JSON"
          @change=${this.onPaletteText}
        ></textarea>
        ${paletteOn ? html`<button class="ghost" @click=${this.clearPalette}>Use photos</button>` : null}
      </section>
      <section>
        <h2>Roles</h2>
        <div class="swatches">
          ${ROLE_KEYS.map((role) => this.swatch(role))}
        </div>
        ${this.ramp().length
          ? html`<div class="ramp" aria-hidden="true">${this.ramp().map((hex) => html`<i style=${`background:${hex}`}></i>`)}</div>`
          : null}
      </section>
      <section>
        <h2>Style</h2>
        <div class="styles">
          ${STYLE_IDS.map(
            (id) => html`
              <button aria-pressed=${this.snap.style === id} @click=${() => this.pickStyle(id)}>
                ${styleLabel(id)}
                <small>${STYLE_BLURB[id]}</small>
              </button>
            `,
          )}
        </div>
      </section>
      <section>
        <h2>Mode and tweaks</h2>
        <div class="modes" role="group" aria-label="Color mode">
          ${(["dark", "light", "both"] as const).map(
            (mode) => html`
              <button aria-pressed=${this.snap.mode === mode} @click=${() => this.setMode(mode)}>
                ${mode === "both" ? "Auto" : mode === "dark" ? "Dark" : "Light"}
              </button>
            `,
          )}
        </div>
        ${SLIDERS.map((slider) => this.slider(slider))}
        <div class="checks">
          <label><input type="checkbox" .checked=${this.snap.bubble} @change=${(event: Event) => this.setFlag("bubble", event)} /> Bubble Card variables</label>
          <label><input type="checkbox" .checked=${this.snap.mushroom} @change=${(event: Event) => this.setFlag("mushroom", event)} /> Mushroom variables</label>
          <label><input type="checkbox" .checked=${this.snap.cardMod} @change=${(event: Event) => this.setFlag("cardMod", event)} /> card-mod snippet</label>
        </div>
      </section>
    `;
  }

  private wallpaperPanel(images: BoardImage[]) {
    return html`
      <section>
        <h2>Wallpaper</h2>
        <label>
          <input type="checkbox" .checked=${this.snap.backgroundEnabled} @change=${(event: Event) => void this.setBackground(event)} />
          Include a background image
        </label>
        <p class="note">
          Off by default. When it is on, the theme points at <code>/local/homechameleon/…</code>.
          Home Assistant serves files under <code>/local/</code> without a login, so anyone who can reach the instance and guess the file name can download that wallpaper.
        </p>
        ${images.length === 0 ? html`<p class="note">Drop a photo on the Editor tab to use it as the wallpaper.</p>` : null}
        ${images.map(
          (image) => html`
            <label>
              <input type="radio" name="wall" .checked=${image.useAsBackground} @change=${() => this.markBackground(image.id)} />
              ${image.name}
            </label>
          `,
        )}
      </section>
    `;
  }

  private blueprintPanel() {
    return html`
      <section>
        <h2>Blueprints</h2>
        <p class="note">Saved in this browser. Photos are not stored with them.</p>
        ${this.blueprints.length === 0 ? html`<p class="note">None saved yet.</p>` : null}
        ${this.blueprints.map(
          (blueprint) => html`
            <div class="blueprint">
              <div>
                <strong>${blueprint.name}</strong>
                <div class="note">${blueprint.snap.style} · ${blueprint.snap.mode}</div>
              </div>
              <div>
                <button class="ghost" @click=${() => this.loadBlueprint(blueprint)}>Load</button>
                <button class="ghost" @click=${() => this.deleteBlueprint(blueprint.id)}>Delete</button>
              </div>
            </div>
          `,
        )}
      </section>
    `;
  }

  private roomPanel() {
    return html`
      <section>
        <h2>Room</h2>
        <p class="note">${ROOM_NOTE}</p>
        <label class="area-pick">
          Area
          <select aria-label="Area" @change=${this.onArea}>
            <option value="" ?selected=${this.areaId === ""}>Choose an area</option>
            ${this.areas.map(
              (area) => html`<option value=${area.id} ?selected=${this.areaId === area.id}>${area.name}</option>`,
            )}
          </select>
        </label>
        ${this.roomLoading ? html`<p class="note">Reading the area…</p>` : null}
        ${this.roomError ? html`<p class="error">${this.roomError}</p>` : null}
        ${this.roomSides().length < 2
          ? html`<p class="note">Choose Auto to compare light and dark on this room.</p>`
          : null}
        ${this.room ? html`<p class="note">${cardSetLabel(this.room)}</p>` : null}
        ${this.room?.warnings.length ? html`<p class="warn">${this.room.warnings[0]}</p>` : null}
        ${this.room ? html`<pre class="yaml">${this.room.yaml}</pre>` : null}
      </section>
    `;
  }

  private tabs(): TabId[] {
    const tabs: TabId[] = ["editor", "wallpaper", "blueprints"];
    if (this.panelMode) tabs.push("room");
    tabs.push("yaml");
    return tabs;
  }

  private openTab(tab: TabId): void {
    this.tab = tab;
    if (tab === "room" && this.areas.length === 0 && !this.roomLoading) void this.loadAreas();
  }

  private roomSides(): PreviewSide[] {
    const sides = (["dark", "light"] as const).filter((side) => Boolean(this.cssParts[side]));
    return sides.length ? [...sides] : [this.snap.previewSide];
  }

  private sideCss(side: PreviewSide): string {
    const raw = this.cssParts[side] ?? "";
    const body = raw ? cssForSide(raw, side) : this.previewCss;
    return applyLocks(body, this.snap.locks);
  }

  private async loadAreas(): Promise<void> {
    this.roomLoading = true;
    this.roomError = "";
    try {
      this.areas = await fetchAreas();
    } catch (error) {
      this.roomError = error instanceof Error ? error.message : "Areas could not be read";
    } finally {
      this.roomLoading = false;
    }
  }

  private async onArea(event: Event): Promise<void> {
    const areaId = (event.target as HTMLSelectElement).value;
    this.areaId = areaId;
    this.room = null;
    if (!areaId) return;
    this.roomLoading = true;
    this.roomError = "";
    try {
      this.room = await fetchRoom(areaId);
    } catch (error) {
      this.roomError = error instanceof Error ? error.message : "The room view could not be built";
    } finally {
      this.roomLoading = false;
    }
  }

  private swatch(role: RoleKey) {
    const color = this.shownColor(role);
    const locked = Boolean(this.snap.locks[role]);
    return html`
      <label class="swatch">
        ${ROLE_LABEL[role]}
        <input type="color" .value=${color} @change=${(event: Event) => this.lockColor(role, event)} />
        <span class="lock">
          <input type="checkbox" .checked=${locked} @change=${() => this.toggleLock(role)} />
          Lock
        </span>
      </label>
    `;
  }

  private slider(slider: (typeof SLIDERS)[number]) {
    const disabled = slider.key === "blur" && !blurApplies(this.snap.style);
    const value = this.snap.tweaks[slider.key];
    return html`
      <label class="slider-row">
        <span>${slider.label} <b>${value}</b></span>
        <input
          type="range"
          aria-label=${slider.label}
          min=${slider.min}
          max=${slider.max}
          step=${slider.step}
          .value=${String(value)}
          ?disabled=${disabled}
          @input=${(event: Event) => this.onSlider(slider.key, event)}
          @change=${() => this.endSlider()}
        />
      </label>
    `;
  }

  private images(): BoardImage[] {
    return this.imageIds.map((id) => this.bank.get(id)).filter((image): image is BoardImage => Boolean(image));
  }

  private shownColor(role: RoleKey): string {
    return this.snap.locks[role] ?? this.roles[this.snap.previewSide]?.[role] ?? "#808080";
  }

  private ramp(): string[] {
    return rampColors(this.paletteJson);
  }

  private contrastLine(): string {
    if (!this.previewCss) return "Drop a photo or open a palette";
    const ratio = this.contrastRatio ? ` text/surface ${this.contrastRatio}:1` : "";
    return `${this.contrastPass ? "WCAG AA" : "WCAG AA failed"}${ratio}`;
  }

  private lockedYaml(): string {
    return this.baseYaml ? applyLocksToYaml(this.baseYaml, this.snap.locks) : "";
  }

  private commit(): void {
    this.past.push({ snap: structuredClone(this.snap), imageIds: [...this.imageIds] });
    if (this.past.length > 50) this.past.shift();
    this.future = [];
    this.pastCount = this.past.length;
    this.futureCount = 0;
  }

  private undo(): void {
    const entry = this.past.pop();
    if (!entry) return;
    this.future.push({ snap: structuredClone(this.snap), imageIds: [...this.imageIds] });
    this.restore(entry);
  }

  private redo(): void {
    const entry = this.future.pop();
    if (!entry) return;
    this.past.push({ snap: structuredClone(this.snap), imageIds: [...this.imageIds] });
    this.restore(entry);
  }

  private restore(entry: Hist): void {
    this.snap = entry.snap;
    this.imageIds = [...entry.imageIds];
    this.paletteText = entry.snap.importedData ?? "";
    this.pastCount = this.past.length;
    this.futureCount = this.future.length;
    this.cacheKey = "";
    this.schedule(entry.snap.importedData || entry.imageIds.length ? "extract" : "full");
  }

  private schedule(kind: Job): void {
    this.dirty = rank(this.dirty, kind);
    if (this.frame) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      void this.enqueue();
    });
  }

  private enqueue(): Promise<void> {
    const run = this.tail.then(async () => {
      while (this.dirty) {
        const kind = this.dirty;
        this.dirty = null;
        try {
          await this.execute(kind);
        } catch (error) {
          this.error = error instanceof Error ? error.message : "Something went wrong";
        }
      }
    });
    this.tail = run.then(
      () => undefined,
      () => undefined,
    );
    return run;
  }

  private async execute(kind: Job): Promise<void> {
    if (!this.snap.importedData && this.imageIds.length === 0) {
      this.previewCss = "";
      this.exportYaml = "";
      this.baseYaml = "";
      this.error = "";
      return;
    }
    if (this.snap.backgroundEnabled) await this.ensureWallpaper();
    if (!this.snap.importedData && (kind === "extract" || this.cacheKey !== this.sourceKey())) {
      const ok = await this.rebuildCache();
      if (!ok) return;
    }
    if (kind === "fast") await this.generateSides([this.snap.previewSide], false);
    else await this.generateSides(sidesFor(this.snap.mode), true);
  }

  private sourceKey(): string {
    return `${this.snap.extraction}|${this.images()
      .map((image) => `${image.id}:${image.weight}:${image.rgba.length}`)
      .join(",")}`;
  }

  private async rebuildCache(): Promise<boolean> {
    const neutral: EditorSnapshot = {
      ...this.snap,
      tweaks: { ...this.snap.tweaks, saturation: 0, temperature: 0 },
      importedFormat: null,
      importedData: null,
    };
    const next: Partial<Record<PreviewSide, string>> = {};
    for (const side of ["dark", "light"] as const) {
      const result = await this.call({ ...neutral, mode: side, previewSide: side }, this.images(), null);
      if (!result || result.error || !result.palette_json) {
        this.error = result?.error ?? "Could not read the photos";
        return false;
      }
      next[side] = result.palette_json;
    }
    this.cache = next;
    this.cacheKey = this.sourceKey();
    this.error = "";
    return true;
  }

  private async generateSides(sides: PreviewSide[], replaceExport: boolean): Promise<void> {
    const cssParts = { ...this.cssParts };
    const yamlParts = { ...this.yamlParts };
    let combined: GenResult | null = null;
    if (this.snap.importedFormat && this.snap.importedData && replaceExport && this.snap.mode === "both") {
      combined = await this.call(this.snap, [], null);
      if (!this.take(combined)) return;
    } else {
      for (const side of sides) {
        const view: EditorSnapshot = {
          ...this.snap,
          mode: side,
          previewSide: side,
          importedFormat: this.snap.importedData ? this.snap.importedFormat : "homechameleon-palette",
          importedData: this.snap.importedData ?? this.cache[side] ?? null,
        };
        const result = await this.call(view, [], null);
        if (!this.take(result)) return;
        cssParts[side] = result.css;
        yamlParts[side] = result.yaml;
        if (result.light) this.roles = { ...this.roles, light: result.light };
        if (result.dark) this.roles = { ...this.roles, dark: result.dark };
        this.paletteJson = result.palette_json;
        this.noteContrast(result);
      }
    }
    if (combined) {
      this.cssParts = { light: combined.css, dark: combined.css };
      this.yamlParts = { light: combined.yaml, dark: combined.yaml };
      this.baseCss = combined.css;
      this.baseYaml = combined.yaml;
      this.basePreview = cssForSide(combined.css, this.snap.previewSide);
    } else {
      const sideCss = cssParts[this.snap.previewSide] ?? "";
      this.basePreview = cssForSide(sideCss, this.snap.previewSide);
      if (replaceExport) {
        const keptCss: Partial<Record<PreviewSide, string>> = {};
        const keptYaml: Partial<Record<PreviewSide, string>> = {};
        for (const side of sides) {
          keptCss[side] = cssParts[side];
          keptYaml[side] = yamlParts[side];
        }
        this.cssParts = keptCss;
        this.yamlParts = keptYaml;
        this.baseCss = stitchCss(keptCss);
        this.baseYaml = stitchYaml(keptYaml);
      } else {
        this.cssParts = cssParts;
        this.yamlParts = yamlParts;
      }
    }
    this.previewCss = applyLocks(this.basePreview, this.snap.locks);
    this.exportCss = applyLocks(this.baseCss, this.snap.locks);
    this.exportYaml = applyLocksToYaml(this.baseYaml, this.snap.locks);
    this.error = "";
  }

  private take(result: GenResult | null): result is GenResult {
    if (!result || result.error) {
      this.error = result?.error ?? "The theme could not be built";
      return false;
    }
    if (result.light) this.roles = { ...this.roles, light: result.light };
    if (result.dark) this.roles = { ...this.roles, dark: result.dark };
    this.paletteJson = result.palette_json;
    this.warnings = result.warnings;
    this.noteContrast(result);
    return true;
  }

  private noteContrast(result: GenResult): void {
    this.contrastPass = result.contrast.pass;
    const check = result.contrast.checks.find((item) => item.pair === "text/surface" && item.mode === this.snap.previewSide)
      ?? result.contrast.checks.find((item) => item.pair === "text/surface");
    this.contrastRatio = check ? check.ratio.toFixed(1) : "";
  }

  private async call(
    snap: EditorSnapshot,
    images: BoardImage[],
    palette: { format: "homechameleon-palette"; data: string } | null,
  ): Promise<GenResult | null> {
    const request = buildRequest(
      snap,
      images.map((image) => ({
        id: image.id,
        name: image.name,
        weight: image.weight,
        useAsBackground: image.useAsBackground,
        width: image.width,
        height: image.height,
        rgba: image.rgba,
      })),
      this.snap.backgroundEnabled ? this.bgFileName : null,
      false,
      palette,
    );
    const reply = await this.bridge.generate(JSON.stringify(request));
    this.lastGenMs = Math.round(reply.ms * 10) / 10;
    this.genCount += 1;
    return parseResult(reply.raw);
  }

  private async ensureWallpaper(): Promise<void> {
    const image = this.images().find((item) => item.useAsBackground) ?? this.images()[0];
    if (!image) {
      this.bgBytes = null;
      this.bgFileName = null;
      return;
    }
    const slug = slugify(this.snap.name);
    const key = `${image.id}:${slug}`;
    if (this.bgKey === key && this.bgBytes && this.bgFileName) return;
    const encoded = await encodeWallpaper(image.source);
    this.bgBytes = encoded.bytes;
    this.bgFileName = `${slug}/bg-${fnv32(image.raw)}.${encoded.ext}`;
    this.bgKey = key;
  }

  private onApplyTarget = (event: Event): void => {
    this.applyTarget = (event.target as HTMLSelectElement).value;
  };

  private async loadPanelState(): Promise<void> {
    try {
      const response = await fetch("/api/homechameleon/dashboards", { credentials: "same-origin" });
      if (response.ok) {
        const payload = (await response.json()) as {
          dashboards?: {
            url_path?: string;
            title?: string;
            views?: { index?: number; path?: string | null; title?: string }[];
          }[];
        };
        const choices: { key: string; label: string }[] = [];
        for (const board of payload.dashboards ?? []) {
          if (!board.url_path) continue;
          for (const view of board.views ?? []) {
            const index = typeof view.index === "number" ? view.index : 0;
            const target = viewTargetFromDashboard({
              urlPath: board.url_path,
              path: view.path ?? null,
              index,
            });
            const viewTitle = view.title || view.path || `View ${index + 1}`;
            choices.push({
              key: JSON.stringify(target),
              label: `${board.title || board.url_path} / ${viewTitle}`,
            });
          }
        }
        this.viewChoices = choices;
        if (!choices.some((choice) => choice.key === this.applyTarget)) this.applyTarget = "";
      }
    } catch {
      /* The editor is open outside Home Assistant. */
    }
    try {
      const response = await fetch("/api/homechameleon/undo", { credentials: "same-origin" });
      if (response.ok) {
        const payload = (await response.json()) as { available?: boolean };
        this.undoReady = Boolean(payload.available);
      }
    } catch {
      /* The editor is open outside Home Assistant. */
    }
  }

  private async applyTheme(): Promise<void> {
    await this.flush("full");
    const yaml = this.lockedYaml();
    if (!yaml) return;
    if (this.snap.backgroundEnabled) await this.ensureWallpaper();
    const background =
      this.snap.backgroundEnabled && this.bgBytes && this.bgFileName
        ? { enabled: true, path: this.bgFileName, data: bytesToBase64(this.bgBytes) }
        : { enabled: false };
    let view: ReturnType<typeof viewTargetFromDashboard> | null = null;
    if (this.applyTarget) {
      try {
        view = JSON.parse(this.applyTarget) as ReturnType<typeof viewTargetFromDashboard>;
      } catch {
        this.notice = "Pick a dashboard view again";
        return;
      }
    }
    this.applying = true;
    this.notice = "";
    try {
      const response = await fetch(this.applyEndpoint(), {
        method: "POST",
        credentials: "same-origin",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: this.snap.name,
          yaml,
          set_default: this.applyDefault,
          background,
          ...(view ? { view } : {}),
        }),
      });
      const payload = (await response.json()) as {
        error?: string;
        theme?: string;
        warnings?: string[];
        undo?: boolean;
        view?: unknown;
      };
      if (!response.ok) {
        this.notice = payload.error ?? "Apply failed";
        return;
      }
      this.undoReady = Boolean(payload.undo);
      this.notice = payload.theme
        ? payload.view
          ? `Applied ${payload.theme} to this view`
          : `Applied ${payload.theme}`
        : "Applied";
      if (payload.warnings?.length) this.warnings = [...this.warnings, ...payload.warnings];
      void this.loadPanelState();
    } catch (error) {
      this.notice = error instanceof Error ? error.message : "Apply failed";
    } finally {
      this.applying = false;
    }
  }

  private async undoTheme(): Promise<void> {
    this.applying = true;
    this.notice = "";
    try {
      const response = await fetch("/api/homechameleon/undo", {
        method: "POST",
        credentials: "same-origin",
      });
      const payload = (await response.json()) as { error?: string; warnings?: string[] };
      if (!response.ok) {
        this.notice = payload.error ?? "Could not restore the previous theme";
        return;
      }
      this.undoReady = false;
      this.notice = "Restored the previous theme";
      if (payload.warnings?.length) this.warnings = [...this.warnings, ...payload.warnings];
      void this.loadPanelState();
    } catch (error) {
      this.notice = error instanceof Error ? error.message : "Could not restore the previous theme";
    } finally {
      this.applying = false;
    }
  }

  private applyEndpoint(): string {
    if (this.hostMode === "addon") return new URL("api/apply", window.location.href).href;
    return "/api/homechameleon/apply";
  }

  private async exportZip(): Promise<void> {
    await this.flush("full");
    const yaml = this.lockedYaml();
    if (!yaml) return;
    if (this.snap.backgroundEnabled) await this.ensureWallpaper();
    const slug = slugify(this.snap.name);
    const snippet = this.snap.cardMod ? cardModSnippet(yaml) : null;
    const bytes = buildZip({
      slug,
      yaml,
      background:
        this.snap.backgroundEnabled && this.bgBytes && this.bgFileName
          ? { path: this.bgFileName, bytes: this.bgBytes }
          : null,
      cardModCss: snippet,
    });
    const blob = new Blob([Uint8Array.from(bytes)], { type: "application/zip" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `homechameleon-${slug}.zip`;
    link.click();
    URL.revokeObjectURL(url);
  }

  private flush(kind: Job): Promise<void> {
    this.dirty = rank(this.dirty, kind);
    if (this.frame) cancelAnimationFrame(this.frame);
    this.frame = 0;
    return this.enqueue();
  }

  private onName(event: Event): void {
    const value = (event.target as HTMLInputElement).value;
    this.commit();
    this.snap = { ...this.snap, name: value };
    this.bgKey = "";
    this.schedule("full");
  }

  private setMode(mode: ModeId): void {
    this.commit();
    const previewSide: PreviewSide = mode === "light" ? "light" : mode === "dark" ? "dark" : this.snap.previewSide;
    this.snap = { ...this.snap, mode, previewSide };
    this.schedule("full");
  }

  private setPreviewSide(side: PreviewSide): void {
    this.snap = { ...this.snap, previewSide: side };
    this.schedule("fast");
  }

  private setExtraction(extraction: ExtractionId): void {
    if (this.snap.importedData) return;
    this.commit();
    this.snap = { ...this.snap, extraction };
    this.schedule("extract");
  }

  private pickStyle(style: StyleId): void {
    this.commit();
    this.snap = {
      ...this.snap,
      style,
      tweaks: {
        ...STYLE_DEFAULTS[style],
        saturation: this.snap.tweaks.saturation,
        temperature: this.snap.tweaks.temperature,
      },
    };
    this.schedule("full");
  }

  private setFlag(flag: "bubble" | "mushroom" | "cardMod", event: Event): void {
    this.commit();
    const checked = (event.target as HTMLInputElement).checked;
    this.snap = { ...this.snap, [flag]: checked };
    this.schedule("full");
  }

  private async setBackground(event: Event): Promise<void> {
    this.commit();
    const checked = (event.target as HTMLInputElement).checked;
    this.snap = { ...this.snap, backgroundEnabled: checked };
    if (checked) await this.ensureWallpaper();
    this.schedule("full");
  }

  private onSlider(key: keyof EditorSnapshot["tweaks"], event: Event): void {
    if (!this.slideCommitted) {
      this.commit();
      this.slideCommitted = true;
    }
    const value = Number((event.target as HTMLInputElement).value);
    this.snap = { ...this.snap, tweaks: { ...this.snap.tweaks, [key]: value } };
    this.schedule("fast");
  }

  private endSlider(): void {
    this.slideCommitted = false;
    this.schedule("full");
  }

  private lockColor(role: RoleKey, event: Event): void {
    const value = (event.target as HTMLInputElement).value;
    this.commit();
    this.snap = { ...this.snap, locks: { ...this.snap.locks, [role]: value } };
    this.applyLockPaint();
  }

  private toggleLock(role: RoleKey): void {
    this.commit();
    const locks = { ...this.snap.locks };
    if (locks[role]) delete locks[role];
    else locks[role] = this.shownColor(role);
    this.snap = { ...this.snap, locks };
    this.applyLockPaint();
  }

  private applyLockPaint(): void {
    if (!this.basePreview) return;
    this.previewCss = applyLocks(this.basePreview, this.snap.locks);
    this.exportCss = applyLocks(this.baseCss, this.snap.locks);
    this.exportYaml = applyLocksToYaml(this.baseYaml, this.snap.locks);
  }

  private onWeight(id: string, event: Event): void {
    const image = this.bank.get(id);
    if (!image) return;
    if (!this.slideCommitted) {
      this.commit();
      this.slideCommitted = true;
    }
    this.bank.set(id, { ...image, weight: Number((event.target as HTMLInputElement).value) });
    this.imageIds = [...this.imageIds];
  }

  private markBackground(id: string): void {
    this.commit();
    for (const [key, image] of this.bank) {
      this.bank.set(key, { ...image, useAsBackground: key === id });
    }
    this.imageIds = [...this.imageIds];
    this.bgKey = "";
    if (this.snap.backgroundEnabled) this.schedule("full");
  }

  private removeImage(id: string): void {
    this.commit();
    this.imageIds = this.imageIds.filter((item) => item !== id);
    const first = this.imageIds[0];
    if (first && !this.images().some((image) => image.useAsBackground)) {
      const image = this.bank.get(first);
      if (image) this.bank.set(first, { ...image, useAsBackground: true });
    }
    this.bgKey = "";
    this.schedule("extract");
  }

  private onDragOver(event: DragEvent): void {
    event.preventDefault();
    this.dragOver = true;
  }

  private async onDrop(event: DragEvent): Promise<void> {
    event.preventDefault();
    this.dragOver = false;
    const files = [...(event.dataTransfer?.files ?? [])];
    await this.ingest(files);
  }

  private async onFiles(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement;
    await this.ingest([...(input.files ?? [])]);
    input.value = "";
  }

  private async ingest(files: File[]): Promise<void> {
    for (const file of files) {
      if (file.type.startsWith("image/")) await this.addImage(file);
      else await this.importText(await file.text());
    }
  }

  private async addImage(file: File): Promise<void> {
    if (file.size > MAX_BYTES) {
      this.notice = "Each photo must be 10 MB or smaller.";
      return;
    }
    if (this.imageIds.length >= MAX_IMAGES) {
      this.notice = "Eight photos is the limit.";
      return;
    }
    try {
      const decoded = await decodeImage(file);
      this.commit();
      const id = crypto.randomUUID();
      const image: BoardImage = {
        id,
        name: file.name || "Photo",
        weight: 1,
        useAsBackground: this.imageIds.length === 0,
        width: decoded.width,
        height: decoded.height,
        rgba: decoded.rgba,
        raw: decoded.raw,
        previewUrl: URL.createObjectURL(file),
        source: file,
      };
      this.bank.set(id, image);
      this.imageIds = [...this.imageIds, id];
      this.notice = "";
      this.schedule("extract");
    } catch {
      this.notice = "Could not read that image.";
    }
  }

  private onPaletteText(event: Event): void {
    void this.importText((event.target as HTMLTextAreaElement).value);
  }

  private async importText(text: string): Promise<void> {
    const detected = detectPalette(text);
    if (!detected) {
      this.notice = "That file is not a palette this editor can read.";
      return;
    }
    this.commit();
    this.paletteText = detected.data;
    this.snap = { ...this.snap, importedFormat: detected.format, importedData: detected.data };
    this.notice = "";
    this.schedule("full");
  }

  private clearPalette(): void {
    this.commit();
    this.paletteText = "";
    this.snap = { ...this.snap, importedFormat: null, importedData: null };
    this.schedule(this.imageIds.length ? "extract" : "full");
  }

  private saveBlueprint(): void {
    const blueprint: Blueprint = {
      id: crypto.randomUUID(),
      savedAt: Date.now(),
      name: this.snap.name.trim() || "HomeChameleon",
      snap: structuredClone(this.snap),
    };
    this.blueprints = [blueprint, ...this.blueprints].slice(0, 40);
    localStorage.setItem(BLUEPRINTS_KEY, JSON.stringify(this.blueprints));
  }

  private loadBlueprint(blueprint: Blueprint): void {
    this.commit();
    this.snap = structuredClone(blueprint.snap);
    this.paletteText = blueprint.snap.importedData ?? "";
    this.cacheKey = "";
    this.schedule(blueprint.snap.importedData || this.imageIds.length ? "extract" : "full");
  }

  private deleteBlueprint(id: string): void {
    this.blueprints = this.blueprints.filter((item) => item.id !== id);
    localStorage.setItem(BLUEPRINTS_KEY, JSON.stringify(this.blueprints));
  }
}

function sidesFor(mode: ModeId): PreviewSide[] {
  if (mode === "light") return ["light"];
  if (mode === "dark") return ["dark"];
  return ["light", "dark"];
}

function rank(prev: Job | null, next: Job): Job {
  const order = { fast: 1, full: 2, extract: 3 };
  if (!prev) return next;
  return order[next] > order[prev] ? next : prev;
}

function readBlueprints(): Blueprint[] {
  try {
    const raw = localStorage.getItem(BLUEPRINTS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as Blueprint[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "hc-app": HcApp;
  }
}

import { LitElement, html } from "lit";
import { customElement, property, state } from "lit/decorators.js";
import { unsafeCSS } from "lit";

import previewCss from "./preview.css?inline";
import type { DeviceId, StyleId } from "./names";

const fontHref = new URL("fonts/Antonio-Bold.woff2", document.baseURI).href;
const FONT_STYLE_ID = "homechameleon-antonio";

@customElement("hc-preview")
export class HcPreview extends LitElement {
  static styles = unsafeCSS(previewCss);

  override connectedCallback(): void {
    super.connectedCallback();
    if (document.getElementById(FONT_STYLE_ID)) return;
    const style = document.createElement("style");
    style.id = FONT_STYLE_ID;
    style.textContent = `@font-face{font-family:Antonio;src:url("${fontHref}") format("woff2");font-weight:700;font-display:swap;}`;
    document.head.appendChild(style);
  }

  @property() cssText = "";
  @property() wallpaper: string | null = null;
  @property() device: DeviceId = "desktop";
  @property() styleId: StyleId = "sheer";
  @property({ type: Boolean }) backgroundOn = false;
  @state() private pop = true;

  protected updated(): void {
    const stage = this.renderRoot.querySelector(".stage");
    if (!(stage instanceof HTMLElement)) return;
    stage.style.cssText = this.cssText;
    if (this.backgroundOn && this.wallpaper) {
      stage.style.backgroundImage = `url("${this.wallpaper}")`;
      stage.style.backgroundSize = "cover";
      stage.style.backgroundPosition = "center";
    }
  }

  render() {
    return html`
      <div class="viewport">
        <div class="frame" data-device=${this.device}>
          ${this.cssText
            ? html`
                <div class="stage" data-style=${this.styleId} data-preview>
                  <nav class="sidebar" aria-label="Sample sidebar">
                    ${["Overview", "Rooms", "Climate", "Energy"].map(
                      (label, index) => html`
                        <button class="side-btn" aria-current=${index === 0 ? "page" : "false"}>${label}</button>
                      `,
                    )}
                  </nav>
                  <div class="main">
                    <div class="top">
                      <h2 class="view-title">Home</h2>
                      <div class="tabs" role="tablist" aria-label="Sample views">
                        ${["Overview", "Living room", "Kitchen", "Energy"].map(
                          (label, index) => html`
                            <button class="tab" role="tab" aria-current=${index === 0 ? "page" : "false"}>${label}</button>
                          `,
                        )}
                      </div>
                    </div>
                    <div class="chips">
                      <span class="chip">19.0 °C</span>
                      <span class="chip">2 lights on</span>
                      <span class="chip">Locked</span>
                      <span class="chip">Home</span>
                      <span class="chip">380 W</span>
                    </div>
                    <div class="grid">
                      <article class="card">
                        <h3>Living room</h3>
                        <p class="muted">4 devices</p>
                        <div class="tiles">
                          <div class="tile"><strong>Ceiling</strong><span class="muted">On · 70%</span></div>
                          <div class="tile"><strong>Floor lamp</strong><span class="muted">Off</span></div>
                          <div class="tile">
                            <strong>Thermostat</strong>
                            <span class="muted">Heat · 19 °C</span>
                            <div class="feature">
                              <button type="button" aria-label="Decrease setpoint">−</button>
                              <span>19°</span>
                              <button type="button" aria-label="Increase setpoint">+</button>
                            </div>
                          </div>
                          <div class="tile"><strong>Front door</strong><span class="muted">Locked</span></div>
                        </div>
                      </article>
                      <article class="card">
                        <h3>Lights</h3>
                        <div class="ring-wrap">
                          <div class="ring"><span>80%</span></div>
                          <div>Pendant</div>
                          <span class="muted">Warm white</span>
                        </div>
                      </article>
                      <article class="card">
                        <h3>Kitchen</h3>
                        <div class="rows">
                          <div class="row"><span>Island</span><span>On</span></div>
                          <div class="row"><span>Cabinets</span><span>On</span></div>
                          <div class="row"><span>Kettle</span><span>Off</span></div>
                          <div class="row"><span>Hood</span><span>Off</span></div>
                        </div>
                      </article>
                      <article class="card">
                        <h3>Weather</h3>
                        <div>18°</div>
                        <p class="muted">Partly cloudy · humidity 64% · wind 9 km/h</p>
                        <div class="forecast">
                          ${[
                            ["Mon", "21°"],
                            ["Tue", "19°"],
                            ["Wed", "15°"],
                            ["Thu", "17°"],
                            ["Fri", "22°"],
                          ].map(([day, temp]) => html`<div><div>${day}</div><strong>${temp}</strong></div>`)}
                        </div>
                      </article>
                      <article class="card">
                        <h3>Energy today</h3>
                        <strong>5.8 kWh</strong>
                        <svg class="energy" viewBox="0 0 200 72" aria-hidden="true">
                          <path d="M0 50 C 20 48, 30 40, 50 42 S 80 58, 100 36 S 140 20, 160 28 200 22 200 18"></path>
                        </svg>
                      </article>
                      <article class="card">
                        <h3>Living room speaker</h3>
                        <div class="media">
                          <div>
                            <strong>Night Window</strong>
                            <div class="muted">Field Notes</div>
                          </div>
                          <div class="transport">
                            <button type="button" aria-label="Previous">‹</button>
                            <button type="button" aria-label="Pause">❚❚</button>
                            <button type="button" aria-label="Next">›</button>
                          </div>
                        </div>
                      </article>
                    </div>
                    <div class="scenes" aria-label="Scenes">
                      <span class="scene">Day</span>
                      <span class="scene">Evening</span>
                      <span class="scene">Night</span>
                    </div>
                    ${this.pop
                      ? html`
                          <article class="pop" aria-label="Bubble pop-up">
                            <header>
                              <h3>Bubble pop-up</h3>
                              <button type="button" @click=${() => (this.pop = false)} aria-label="Close pop-up">×</button>
                            </header>
                            <p class="muted">Sample pop-up card</p>
                            <div class="days">
                              <span>Mon</span><span>Tue</span><span>Wed</span><span>Thu</span><span>Fri</span>
                            </div>
                          </article>
                        `
                      : null}
                  </div>
                </div>
              `
            : html`<p class="empty">Drop a photo or open a palette to preview a theme.</p>`}
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "hc-preview": HcPreview;
  }
}

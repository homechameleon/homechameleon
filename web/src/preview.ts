import { LitElement, html } from "lit";
import { customElement, property } from "lit/decorators.js";
import { unsafeCSS } from "lit";

import previewCss from "./preview.css?inline";
import type { DeviceId, StyleId } from "./names";

const fontHref = new URL("fonts/Antonio-Bold.woff2", document.baseURI).href;
const FONT_STYLE_ID = "homechameleon-antonio";

function bars(label: string) {
  return html`
    <div class="lcars" aria-hidden="true">
      <i class="bar a"></i>
      <i class="bar b"></i>
      <i class="bar c"></i>
      <i class="bar d">${label}</i>
    </div>
  `;
}

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
  @property({ type: Boolean, reflect: true }) fit = false;

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
                      <span class="code">04</span>
                      <div class="tabs" role="tablist" aria-label="Sample views">
                        ${["Overview", "Lights", "Climate"].map(
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
                      <span class="chip">380 W</span>
                    </div>
                    <div class="grid">
                      <article class="card entities">
                        ${bars("01")}
                        <h3>Entities</h3>
                        <div class="entity"><span>Ceiling</span><span class="muted">On · 70%</span></div>
                        <div class="entity"><span>Floor lamp</span><span class="muted">Off</span></div>
                        <div class="entity"><span>Front door</span><span class="muted">Locked</span></div>
                        <div class="entity"><span>Hall motion</span><span class="muted">Clear</span></div>
                      </article>
                      <article class="card light-card">
                        ${bars("14")}
                        <h3>Light</h3>
                        <div class="light-row">
                          <span class="blob" aria-hidden="true"></span>
                          <div>
                            <strong>Pendant</strong>
                            <div class="muted">Warm white · 80%</div>
                          </div>
                          <button class="switch" type="button" role="switch" aria-checked="true" aria-label="Pendant"><i></i></button>
                        </div>
                      </article>
                      <article class="card thermo">
                        ${bars("19")}
                        <h3>Thermostat</h3>
                        <div class="setpoint">
                          <button type="button" aria-label="Decrease setpoint">−</button>
                          <strong>19°</strong>
                          <button type="button" aria-label="Increase setpoint">+</button>
                        </div>
                        <p class="muted">Heat · living room</p>
                      </article>
                      <article class="card media-card">
                        ${bars("08")}
                        <h3>Media</h3>
                        <div class="media">
                          <div class="art" aria-hidden="true"></div>
                          <div>
                            <strong>Night Window</strong>
                            <div class="muted">Field Notes</div>
                          </div>
                        </div>
                        <div class="transport">
                          <button type="button" aria-label="Previous">‹</button>
                          <button type="button" aria-label="Pause">❚❚</button>
                          <button type="button" aria-label="Next">›</button>
                        </div>
                      </article>
                    </div>
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

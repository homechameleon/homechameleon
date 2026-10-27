import { LitElement, html } from "lit";
import { customElement, property } from "lit/decorators.js";
import { unsafeCSS } from "lit";

import roomCss from "./room-preview.css?inline";
import type { DeviceId } from "./names";
import { type RoomPreview, formatState, interactionLabel } from "./room";

@customElement("hc-room-preview")
export class HcRoomPreview extends LitElement {
  static styles = unsafeCSS(roomCss);

  @property() cssText = "";
  @property() device: DeviceId = "desktop";
  @property() side: "dark" | "light" = "dark";
  @property({ attribute: false }) preview: RoomPreview | null = null;

  protected updated(): void {
    const sheet = this.renderRoot.querySelector("[data-room-sheet]");
    if (sheet instanceof HTMLElement) sheet.style.cssText = this.cssText;
  }

  render() {
    const preview = this.preview;
    const sideLabel = this.side === "dark" ? "Dark" : "Light";
    return html`
      <div class="viewport">
        <p class="side-label">${sideLabel}</p>
        <div class="frame" data-device=${this.device}>
          ${preview
            ? html`
                <div class="sheet" data-room-sheet aria-label="${sideLabel} theme">
                  <header class="banner">
                    <h2>${preview.header.title}</h2>
                    <div class="facts">
                      ${preview.header.temperature
                        ? html`<span>${preview.header.temperature.name} ${formatState(preview.header.temperature.state, preview.header.temperature.unit)}</span>`
                        : null}
                      ${preview.header.humidity
                        ? html`<span>${preview.header.humidity.name} ${formatState(preview.header.humidity.state, preview.header.humidity.unit)}</span>`
                        : null}
                      ${preview.header.lights_toggle
                        ? html`<button type="button" disabled>All lights</button>`
                        : null}
                    </div>
                  </header>
                  ${preview.sections.map(
                    (section) => html`
                      <section aria-label=${section.title}>
                        <h3>${section.title}</h3>
                        ${section.cards.map((card) => {
                          const policy = interactionLabel(card.interaction);
                          return html`
                            <article class="rcard" data-interaction=${card.interaction}>
                              <div>
                                <strong>${card.name}</strong>
                                <span class="muted">${card.card}</span>
                                ${card.detail ? html`<span class="muted">${card.detail}</span>` : null}
                              </div>
                              <div class="end">
                                <span>${formatState(card.state, card.unit)}</span>
                                ${policy ? html`<span class="badge">${policy}</span>` : null}
                              </div>
                            </article>
                          `;
                        })}
                      </section>
                    `,
                  )}
                </div>
              `
            : html`<p class="empty">Choose an area to preview this room.</p>`}
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "hc-room-preview": HcRoomPreview;
  }
}

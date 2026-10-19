import init, { generate_json } from "./generated/homechameleon_core.js";

let ready: Promise<unknown> | null = null;

function ensure(): Promise<unknown> {
  ready ??= init();
  return ready;
}

self.onmessage = (event: MessageEvent<{ id: number; request: string }>) => {
  const { id, request } = event.data;
  void ensure().then(() => {
    const started = performance.now();
    const raw = generate_json(request);
    const ms = performance.now() - started;
    self.postMessage({ id, raw, ms });
  });
};

export interface GenReply {
  raw: string;
  ms: number;
}

export class CoreBridge {
  private readonly worker: Worker;
  private next = 1;
  private readonly pending = new Map<number, (value: GenReply) => void>();

  constructor() {
    this.worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
    this.worker.onmessage = (event: MessageEvent<GenReply & { id: number }>) => {
      const resolve = this.pending.get(event.data.id);
      this.pending.delete(event.data.id);
      resolve?.(event.data);
    };
  }

  generate(request: string): Promise<GenReply> {
    const id = this.next;
    this.next += 1;
    return new Promise((resolve) => {
      this.pending.set(id, resolve);
      this.worker.postMessage({ id, request });
    });
  }
}

import { bytesToBase64 } from "./model";

export const EXTRACT_EDGE = 128;
export const WALLPAPER_EDGE = 2560;
export const MAX_IMAGES = 8;
export const MAX_BYTES = 10 * 1024 * 1024;

export interface DecodedImage {
  width: number;
  height: number;
  rgba: string;
  raw: Uint8Array;
}

export async function decodeImage(file: Blob): Promise<DecodedImage> {
  const bitmap = await createImageBitmap(file);
  try {
    const scale = Math.min(1, EXTRACT_EDGE / Math.max(bitmap.width, bitmap.height));
    const width = Math.max(1, Math.round(bitmap.width * scale));
    const height = Math.max(1, Math.round(bitmap.height * scale));
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) throw new Error("Could not read that image");
    context.drawImage(bitmap, 0, 0, width, height);
    const raw = new Uint8Array(context.getImageData(0, 0, width, height).data);
    return { width, height, rgba: bytesToBase64(raw), raw };
  } finally {
    bitmap.close();
  }
}

function blobOf(canvas: HTMLCanvasElement, type: string, quality: number): Promise<Blob | null> {
  return new Promise((resolve) => {
    canvas.toBlob((blob) => resolve(blob), type, quality);
  });
}

export async function encodeWallpaper(file: Blob): Promise<{ bytes: Uint8Array; ext: "webp" | "jpg" }> {
  const bitmap = await createImageBitmap(file);
  try {
    const scale = Math.min(1, WALLPAPER_EDGE / Math.max(bitmap.width, bitmap.height));
    const width = Math.max(1, Math.round(bitmap.width * scale));
    const height = Math.max(1, Math.round(bitmap.height * scale));
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("Could not encode the wallpaper");
    context.drawImage(bitmap, 0, 0, width, height);
    const webp = await blobOf(canvas, "image/webp", 0.9);
    if (webp && webp.size > 0 && webp.type === "image/webp") {
      return { bytes: new Uint8Array(await webp.arrayBuffer()), ext: "webp" };
    }
    const jpeg = await blobOf(canvas, "image/jpeg", 0.8);
    if (!jpeg || jpeg.size === 0) throw new Error("Could not encode the wallpaper");
    return { bytes: new Uint8Array(await jpeg.arrayBuffer()), ext: "jpg" };
  } finally {
    bitmap.close();
  }
}

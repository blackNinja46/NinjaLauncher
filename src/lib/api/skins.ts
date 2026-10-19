import { invoke, isTauri } from "@tauri-apps/api/core";

export type SkinModel = "classic" | "slim";
export interface SavedSkin {
  id: string;
  name: string;
  texture: string;
  model: SkinModel;
  capeId: string | null;
  capeAccountUuid: string | null;
}
export interface OwnedCape {
  id: string;
  url: string;
  alias: string;
  state: string;
}
export interface SkinProfile {
  id: string;
  skins: { url: string; variant: string; state: string }[];
  capes: OwnedCape[];
}
export interface ImportedSkin {
  texture: string;
  name: string;
  model: SkinModel;
}
const previewKey = "ninja-skin-library-preview";
export async function listSkins(): Promise<SavedSkin[]> {
  return isTauri() ? invoke("list_skins") : JSON.parse(localStorage.getItem(previewKey) ?? "[]");
}
export async function saveSkin(skin: SavedSkin): Promise<SavedSkin> {
  if (isTauri()) return invoke("save_skin", { skin });
  const skins = await listSkins();
  localStorage.setItem(
    previewKey,
    JSON.stringify([...skins.filter((item) => item.id !== skin.id), skin]),
  );
  return skin;
}
export async function deleteSkin(id: string): Promise<void> {
  if (isTauri()) return invoke("delete_skin", { id });
  localStorage.setItem(
    previewKey,
    JSON.stringify((await listSkins()).filter((skin) => skin.id !== id)),
  );
}
export async function importSkin(
  source: "url" | "uuid" | "name",
  value: string,
): Promise<ImportedSkin> {
  if (!isTauri())
    throw new Error(
      "Online-Import ist in der Desktop-App verfügbar. Im Browser kannst du PNG-Dateien auswählen.",
    );
  return invoke("import_skin", { source, value });
}
export async function getSkinProfile(uuid: string): Promise<SkinProfile> {
  if (!isTauri()) throw new Error("Minecraft-Konten und Capes sind in der Desktop-App verfügbar.");
  const profile = await invoke<SkinProfile>("get_skin_profile", { uuid });
  const secure = (url: string) =>
    url.replace(/^http:\/\/textures\.minecraft\.net\//, "https://textures.minecraft.net/");
  return {
    ...profile,
    skins: profile.skins.map((skin) => ({ ...skin, url: secure(skin.url) })),
    capes: profile.capes.map((cape) => ({ ...cape, url: secure(cape.url) })),
  };
}
export async function applySkin(id: string, uuid: string): Promise<{ warning: string | null }> {
  if (!isTauri())
    throw new Error(
      "Skins können nur in der Desktop-App auf dein Minecraft-Konto übertragen werden.",
    );
  return invoke("apply_skin", { id, uuid });
}

export async function readSkinFile(file: File): Promise<ImportedSkin> {
  if (file.size > 1024 * 1024) throw new Error("Die PNG-Datei darf höchstens 1 MB groß sein.");
  const bytes = new Uint8Array(await file.arrayBuffer());
  if (![137, 80, 78, 71, 13, 10, 26, 10].every((byte, index) => bytes[index] === byte)) {
    throw new Error("Bitte eine PNG-Datei auswählen.");
  }
  const texture = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(new Error("Die Datei konnte nicht gelesen werden."));
    reader.readAsDataURL(new Blob([bytes], { type: "image/png" }));
  });
  const image = new Image();
  image.src = texture;
  try {
    await image.decode();
  } catch {
    throw new Error("Die PNG-Datei ist beschädigt.");
  }
  if (image.width !== 64 || ![32, 64].includes(image.height)) {
    throw new Error("Skins müssen 64 × 64 oder 64 × 32 Pixel groß sein.");
  }
  return { texture, name: file.name.replace(/\.png$/i, "").slice(0, 48), model: "classic" };
}

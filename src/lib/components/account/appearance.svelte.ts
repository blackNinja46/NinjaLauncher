import type { SkinModel } from "$lib/api/skins";

// Keep the launcher preview current immediately after a successful account change.
export const appearances = $state<
  Record<
    string,
    {
      texture: string;
      model: SkinModel;
      cape: string | null;
    }
  >
>({});

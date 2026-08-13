import { writable } from "svelte/store";
import type { LauncherInstance, NavigationItem } from "$lib/types";

export const activePage = writable<NavigationItem>("home");

export const selectedInstance = writable<LauncherInstance>({
  id: "default-fabric",
  name: "My Fabric World",
  minecraftVersion: "26.2",
  loader: "fabric",
  loaderVersion: "0.19.3",
  mods: [],
});

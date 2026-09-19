<script lang="ts">
  import {
    faBoxArchive,
    faBell,
    faShirt,
    faPen,
    faCheck,
    faChevronDown,
    faChevronRight,
    faCircleInfo,
    faCircleQuestion,
    faDownload,
    faEllipsis,
    faFolderOpen,
    faGear,
    faHouse,
    faLayerGroup,
    faLeftRight,
    faMagnifyingGlass,
    faMinus,
    faPlay,
    faPalette,
    faPlus,
    faPuzzlePiece,
    faServer,
    faTerminal,
    faTrashCan,
    faUpload,
    faWindowMaximize,
    faWindowRestore,
    faWandMagicSparkles,
    faXmark, faUpRightAndDownLeftFromCenter,
  } from "@fortawesome/free-solid-svg-icons";
  import {faWindowMinimize} from "@fortawesome/free-solid-svg-icons/faWindowMinimize";
  import {faDownLeftAndUpRightToCenter} from "@fortawesome/free-solid-svg-icons/faDownLeftAndUpRightToCenter";

  let { name, size = 20 }: { name: string; size?: number } = $props();

  const icons = {
    home: faHouse,
    bell: faBell,
    skins: faShirt,
    edit: faPen,
    mods: faPuzzlePiece,
    server: faServer,
    modpacks: faBoxArchive,
    instances: faLayerGroup,
    settings: faGear,
    play: faPlay,
    palette: faPalette,
    shader: faWandMagicSparkles,
    search: faMagnifyingGlass,
    chevron: faChevronRight,
    "chevron-down": faChevronDown,
    plus: faPlus,
    download: faDownload,
    upload: faUpload,
    folder: faFolderOpen,
    logs: faTerminal,
    trash: faTrashCan,
    check: faCheck,
    info: faCircleInfo,
    more: faEllipsis,
    "left-right": faLeftRight,
    minus: faMinus,
    minimize: faWindowMinimize,
    maximize: faWindowMaximize,
    restore: faWindowRestore,
    close: faXmark,
    upRightAndDownLeftFromCenter: faUpRightAndDownLeftFromCenter,
    downLeftAndUpRightToCenter: faDownLeftAndUpRightToCenter,
  } as const;

  let definition = $derived(icons[name as keyof typeof icons] ?? faCircleQuestion);
  let iconWidth = $derived(definition.icon[0]);
  let iconHeight = $derived(definition.icon[1]);
  let pathData = $derived(
    Array.isArray(definition.icon[4]) ? definition.icon[4] : [definition.icon[4]],
  );
</script>

<svg
  class="inline-block shrink-0 overflow-visible align-[-.125em]"
  width={size}
  height={size}
  viewBox={`0 0 ${iconWidth} ${iconHeight}`}
  fill="currentColor"
  aria-hidden="true"
  focusable="false"
>
  {#each pathData as path}
    <path d={path}></path>
  {/each}
</svg>

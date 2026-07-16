<script>
  import sprite from "../assets/devices/mic-mini-2-cover-cutouts.png";
  import { txCover, txCoverPosition } from "./txCovers.js";

  let { value = "obsidian-black", size = 124 } = $props();
  const selected = $derived(txCover(value));
</script>

<span
  class="artwork"
  style:width={`${size}px`}
  style:height={`${size}px`}
  style:background-image={`url(${sprite})`}
  style:background-position={txCoverPosition(selected)}
  role="img"
  aria-label={`DJI Mic Mini 2，${selected.name}前盖`}
></span>

<style>
  .artwork {
    display: block;
    background-repeat: no-repeat;
    background-size: 500% 200%;
  }
</style>

<script>
  import sprite from "../assets/devices/mic-mini-2-cover-cutouts.png";
  import { txCovers, txCover, txCoverPosition } from "./txCovers.js";

  let { value = "obsidian-black", size = 42, onchange } = $props();
  const selected = $derived(txCover(value));
</script>

<details class="picker">
  <summary aria-label={`更换磁吸前盖，当前${selected.name}`} title={`磁吸前盖：${selected.name}`}>
    <span
      class="product"
      style:width={`${size}px`}
      style:height={`${size}px`}
      style:background-image={`url(${sprite})`}
      style:background-position={txCoverPosition(selected)}
    ></span>
    <span class="edit-mark" aria-hidden="true"></span>
  </summary>
  <div class="palette">
    <div class="palette-title">磁吸前盖</div>
    <div class="swatches">
      {#each txCovers as cover (cover.id)}
        <button
          class:active={cover.id === selected.id}
          aria-label={cover.name}
          title={cover.name}
          onclick={(event) => {
            onchange?.(cover.id);
            event.currentTarget.closest("details").open = false;
          }}
        >
          <span class="swatch" style:background={cover.swatch}></span>
          <span>{cover.name}</span>
        </button>
      {/each}
    </div>
  </div>
</details>

<style>
  .picker {
    position: relative;
    flex: 0 0 auto;
  }
  summary {
    position: relative;
    display: block;
    list-style: none;
    border-radius: 8px;
    cursor: pointer;
    outline: none;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  summary:focus-visible {
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .product {
    display: block;
    background-repeat: no-repeat;
    background-size: 500% 200%;
  }
  .edit-mark {
    position: absolute;
    right: -2px;
    bottom: -2px;
    width: 12px;
    height: 12px;
    border: 2px solid var(--bg-panel);
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
  }
  .palette {
    position: absolute;
    z-index: 120;
    top: calc(100% + 8px);
    right: 0;
    width: 274px;
    padding: 12px;
    border: 1px solid color-mix(in srgb, var(--border-strong) 72%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-panel) 88%, transparent);
    box-shadow: 0 18px 48px rgba(0, 0, 0, 0.2);
    backdrop-filter: blur(24px) saturate(180%);
  }
  .palette-title {
    margin: 0 2px 9px;
    font-size: 12px;
    font-weight: 650;
  }
  .swatches {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 7px;
  }
  button {
    display: grid;
    justify-items: center;
    gap: 4px;
    min-width: 0;
    padding: 5px 2px 4px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--text-dim);
    font-size: 9px;
  }
  button:hover,
  button.active {
    border-color: var(--border);
    background: color-mix(in srgb, var(--bg-elev) 76%, transparent);
    color: var(--text);
  }
  button.active {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .swatch {
    width: 25px;
    height: 25px;
    border: 1px solid rgba(0, 0, 0, 0.16);
    border-radius: 50%;
    box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.4);
  }
</style>

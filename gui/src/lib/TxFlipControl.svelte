<script>
  import SettingControl from "./SettingControl.svelte";
  import TxArtwork from "./TxArtwork.svelte";

  let { tx, index, cover, settingsById = {}, pending = false, onchange } = $props();

  const orUnknown = (value) => value ?? "未知";
  const ncPower = $derived(tx.nc_enabled == null ? null : tx.nc_enabled ? "on" : "off");
  const level = $derived(tx.level == null ? 0 : Math.max(0, Math.min(100, Math.round((tx.level / 80) * 100))));
</script>

<div class="tx-direct">
  <div class="content">
    <div class="product">
      <TxArtwork value={cover} size={126} />
      <div class="identity">
        <span><small>序列号</small><b>{orUnknown(tx.serial)}</b></span>
        <span><small>固件</small><b>{orUnknown(tx.firmware)}</b></span>
      </div>
    </div>

    <div class="controls" aria-label={`发射器 ${index + 1} 单独设置`}>
      <div class="controls-title">TX {index + 1} 单独设置</div>
      {#if settingsById["noise-cancel-power"]}
        <SettingControl
          setting={settingsById["noise-cancel-power"]}
          value={ncPower}
          {pending}
          compact
          onchange={(id, value) => onchange?.(id, value)}
        />
      {/if}
      {#if settingsById["noise-cancel"]}
        <SettingControl
          setting={settingsById["noise-cancel"]}
          value={tx.nc_mode ?? null}
          {pending}
          compact
          locked={ncPower === "off"}
          lockReason="请先开启该发射器的降噪"
          onchange={(id, value) => onchange?.(id, value)}
        />
      {/if}
      {#if settingsById["voice-tone"]}
        <SettingControl
          setting={settingsById["voice-tone"]}
          value={tx.voice_tone ?? null}
          {pending}
          compact
          onchange={(id, value) => onchange?.(id, value)}
        />
      {/if}
    </div>
  </div>

  <div class="level-row" aria-label={`发射器 ${index + 1} 实时电平`}>
    <span>实时电平</span>
    <div class="meter"><i style:width={`${level}%`}></i></div>
    <output>{level}%</output>
  </div>
</div>

<style>
  .tx-direct {
    display: grid;
    gap: 14px;
  }
  .content {
    display: grid;
    grid-template-columns: 142px minmax(0, 1fr);
    align-items: start;
    gap: 17px;
  }
  .product {
    display: grid;
    justify-items: center;
    gap: 10px;
    min-width: 0;
    padding-top: 4px;
  }
  .identity {
    display: grid;
    width: 100%;
    gap: 4px;
  }
  .identity span {
    display: grid;
    grid-template-columns: 38px minmax(0, 1fr);
    align-items: baseline;
    gap: 6px;
    font-size: 10px;
  }
  .identity small {
    color: var(--text-dim);
  }
  .identity b {
    overflow: hidden;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .controls {
    display: grid;
    align-content: start;
    gap: 6px;
    min-width: 0;
  }
  .controls-title {
    min-height: 24px;
    padding: 3px 7px 0;
    color: var(--text-dim);
    font-size: 11px;
    font-weight: 650;
  }
  .level-row {
    display: grid;
    grid-template-columns: 54px minmax(0, 1fr) 36px;
    align-items: center;
    gap: 9px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 10px;
  }
  .level-row output {
    color: var(--text);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .meter {
    height: 7px;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-elev);
  }
  .meter i {
    display: block;
    height: 100%;
    background: linear-gradient(90deg, #2ea043 0%, #57b846 48%, #e0b400 76%, #e5484d 100%);
    transition: width 0.09s linear;
  }
  @media (max-width: 980px) {
    .content {
      grid-template-columns: 112px minmax(0, 1fr);
      gap: 12px;
    }
    .product :global(.artwork) {
      width: 104px !important;
      height: 104px !important;
    }
  }
  @media (max-width: 760px) {
    .content {
      grid-template-columns: 1fr;
    }
    .product {
      grid-template-columns: 112px minmax(0, 1fr);
      align-items: center;
      justify-items: stretch;
    }
  }
</style>

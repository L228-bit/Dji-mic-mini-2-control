<script>
  import DevicePicto from "./DevicePicto.svelte";
  import SettingControl from "./SettingControl.svelte";
  import NoiseCancelControl from "./NoiseCancelControl.svelte";
  import BatteryIcon from "./BatteryIcon.svelte";
  import TxCoverPicker from "./TxCoverPicker.svelte";
  import TxFlipControl from "./TxFlipControl.svelte";

  let {
    device,
    status,
    settings = [],
    values = {},
    pending = {},
    pendingTx = {},
    view = "compact",
    onchange,
    onchangeTx,
    onview,
  } = $props();

  // Audio levels sit roughly in the 0x14–0x4f range; scale to a full bar.
  // `v` is null when the connected firmware hasn't reported a level yet.
  const levelPct = (v) => (v == null ? 0 : Math.max(0, Math.min(100, Math.round((v / 80) * 100))));

  // Identity/level fields the model hasn't decoded yet read as null.
  const orUnknown = (v) => v ?? "未知";

  // A transmitter card's title: its own reported product name once known
  // (e.g. "DJI Mic Mini 2 (TX1)"), else the bare slot number.
  const txLabel = (tx, i) => (tx?.product_name ? `${tx.product_name}（发射器 ${i + 1}）` : `发射器 ${i + 1}`);

  let savedCoverColors = {};
  try {
    savedCoverColors = JSON.parse(globalThis.localStorage?.getItem("dji-mic-mini-2-covers") ?? "{}");
  } catch {
    savedCoverColors = {};
  }
  let coverColors = $state(savedCoverColors);
  const defaultCover = (i) => (i === 0 ? "obsidian-black" : "glaze-white");
  const coverColor = (tx, i) => coverColors[tx?.serial] ?? coverColors[`slot-${i}`] ?? defaultCover(i);
  function setCoverColor(tx, i, color) {
    coverColors = { ...coverColors, [`slot-${i}`]: color, ...(tx?.serial ? { [tx.serial]: color } : {}) };
    globalThis.localStorage?.setItem("dji-mic-mini-2-covers", JSON.stringify(coverColors));
  }

  // Artwork varies by the transmitter's own reported product name — a Mic
  // Mini 2 looks different from the original. Returns null (no picture)
  // while that's still a transient unknown rather than a guess: v2 firmware
  // reports it a moment after connecting, so briefly null there means "not
  // arrived yet," not "never coming" — only v1 firmware (which has no such
  // field at all) or a disconnected slot falls back to the base picture.
  const TX_PICTOGRAMS = { "DJI Mic Mini 2": "mic-mini-2" };
  function txPictogram(tx, protocolVersion) {
    if (!tx) return null;
    if (tx.product_name) return `${TX_PICTOGRAMS[tx.product_name] ?? device.pictogram_key}-tx`;
    if (protocolVersion === 2) return null;
    return `${device.pictogram_key}-tx`;
  }

  // Noise Cancel's three settings render as one combined element (see
  // NoiseCancelControl) rather than three separate rows. Collapse them into
  // a single placeholder, at the position the first of them would have
  // occupied, before grouping/columning.
  const NC_IDS = new Set(["noise-cancel", "noise-cancel-power", "noise-cancel-button"]);
  // Settings addressed to one specific TX (rather than the receiver or a
  // broadcast to both) don't fit this shared list at all — they're rendered
  // on their own TX card instead (see the Voice Tone control below).
  const TX_TARGETED_IDS = new Set(["voice-tone"]);
  const renderSettings = $derived.by(() => {
    const out = [];
    let ncInserted = false;
    for (const s of settings) {
      if (TX_TARGETED_IDS.has(s.id)) continue;
      if (NC_IDS.has(s.id)) {
        if (!ncInserted) {
          out.push({ id: "__nc-group", group: s.group, __ncComposite: true });
          ncInserted = true;
        }
        continue;
      }
      out.push(s);
    }
    return out;
  });

  const settingsById = $derived(Object.fromEntries(settings.map((s) => [s.id, s])));

  // Cluster settings by their declared group, preserving first-seen order.
  const groups = $derived.by(() => {
    const map = new Map();
    for (const s of renderSettings) {
      if (!map.has(s.group)) map.set(s.group, []);
      map.get(s.group).push(s);
    }
    return [...map.entries()];
  });

  // JS masonry: distribute group cards into balanced columns ourselves rather
  // than using CSS multi-column, which paints card shadows across column
  // boundaries. Each card lands in the currently-shortest column.
  let groupsWidth = $state(0);
  const MIN_COL = 260;
  const COL_GAP = 18;
  const colCount = $derived(
    Math.max(1, Math.min(groups.length, Math.floor((groupsWidth + COL_GAP) / (MIN_COL + COL_GAP)))),
  );
  const columns = $derived.by(() => {
    const cols = Array.from({ length: colCount }, () => []);
    const heights = new Array(colCount).fill(0);
    for (const [name, items] of groups) {
      let min = 0;
      for (let i = 1; i < colCount; i++) if (heights[i] < heights[min]) min = i;
      cols[min].push([name, items]);
      heights[min] += items.length + 1.5; // ~header + rows, as a height estimate
    }
    return cols;
  });

  const labelById = $derived(Object.fromEntries(settings.map((s) => [s.id, s.label])));

  // Why a setting is locked, or null if it is freely settable.
  function lockReason(s) {
    // Settings v2 firmware introduced don't exist on a v1 device at all.
    // "noise-cancel-power" still shows its real value there (see the
    // decoder) since v1 firmware reports it, just via the TX's button
    // rather than a settable command.
    if (status.protocol_version === 1 && s.v1_command == null) {
      return "需要 v2 协议，请更新固件";
    }
    // NC mode is meaningless unless NC is enabled — on v2 firmware that's a
    // toggle in its own right; on v1 firmware it's only the TX's button.
    if (s.id === "noise-cancel" && values["noise-cancel-power"] === "off") {
      const toggle = labelById["noise-cancel-power"];
      return toggle
        ? `请先开启“${toggle}”`
        : "请先用发射器按键开启降噪";
    }
    // Mutually exclusive settings lock each other while active. Audio
    // Channels is an enum ("mono"/"stereo"), not a switch, so its "active"
    // value isn't the usual "on".
    for (const other of s.exclusive_with ?? []) {
      if (other === "stereo") {
        if (values.stereo === "stereo") {
          return `请先将“${labelById.stereo ?? "音频声道"}”切换为单声道`;
        }
      } else if (values[other] === "on") {
        return `请先关闭“${labelById[other] ?? other}”`;
      }
    }
    return null;
  }
</script>

<div class="panel">
  <header class="head">
    <span class="picto"><DevicePicto pictogram={`${device.pictogram_key}-rx`} size={54} /></span>
    <div class="titles">
      <h1>{device.model_name}</h1>
      <span class="sub">{status.rx?.serial ?? device.id}</span>
    </div>
    <span class="pill" class:on={status.connected}>
      {status.connected ? "已连接" : "无信号"}
    </span>
  </header>

  <section class="cards">
    {#each status.tx as tx, i}
      {@const picto = txPictogram(tx, status.protocol_version)}
      <div class="card">
        <div class="card-head">
          <span class="card-title">
            {txLabel(tx, i)}
            {#if tx}
              <BatteryIcon value={tx.battery} charging={!!tx.charging} />
            {/if}
          </span>
          {#if tx?.product_name === "DJI Mic Mini 2"}
            <TxCoverPicker value={coverColor(tx, i)} onchange={(color) => setCoverColor(tx, i, color)} />
          {:else if picto}
            <DevicePicto pictogram={picto} size={38} />
          {/if}
        </div>
        {#if tx}
          {#if tx.product_name === "DJI Mic Mini 2" && status.protocol_version === 2}
            <TxFlipControl
              {tx}
              index={i}
              cover={coverColor(tx, i)}
              {settingsById}
              pending={!!pendingTx[i]}
              onchange={(settingId, value) => onchangeTx?.(i, settingId, value)}
            />
          {:else}
            <div class="kv"><span>序列号</span><b>{orUnknown(tx.serial)}</b></div>
            <div class="kv"><span>固件</span><b>{orUnknown(tx.firmware)}</b></div>
            <div class="level">
              <span class="lvl-label">电平</span>
              <div class="meter">
                <div class="unfilled" style="left:{levelPct(tx.level)}%"></div>
              </div>
            </div>
            {#if settingsById["voice-tone"]}
              <SettingControl
                setting={settingsById["voice-tone"]}
                value={tx.voice_tone ?? null}
                pending={!!pendingTx[i]}
                compact
                onchange={(settingId, value) => onchangeTx?.(i, settingId, value)}
              />
            {/if}
          {/if}
        {:else}
          <div class="muted">未连接</div>
        {/if}
      </div>
    {/each}
  </section>

  <section class="settings-head">
    <span class="settings-title">设置</span>
    <div class="view-switch" role="group" aria-label="设置布局">
      <button class:active={view === "compact"} onclick={() => onview?.("compact")}>紧凑</button>
      <button class:active={view === "list"} onclick={() => onview?.("list")}>列表</button>
    </div>
  </section>

  {#if view === "list"}
    <section class="settings">
      {#each renderSettings as s (s.id)}
        {#if s.__ncComposite}
          <NoiseCancelControl
            power={settingsById["noise-cancel-power"]}
            mode={settingsById["noise-cancel"]}
            button={settingsById["noise-cancel-button"]}
            {values}
            {pending}
            {lockReason}
            {onchange}
          />
        {:else}
          <SettingControl
            setting={s}
            value={values[s.id] ?? null}
            pending={!!pending[s.id]}
            locked={!!lockReason(s)}
            lockReason={lockReason(s) ?? ""}
            {onchange}
          />
        {/if}
      {/each}
    </section>
  {:else}
    <section class="groups" bind:clientWidth={groupsWidth}>
      {#each columns as col, ci (ci)}
        <div class="col">
          {#each col as [name, items] (name)}
            <div class="group">
              <div class="group-title">{name}</div>
              <div class="group-grid">
                {#each items as s (s.id)}
                  {#if s.__ncComposite}
                    <NoiseCancelControl
                      power={settingsById["noise-cancel-power"]}
                      mode={settingsById["noise-cancel"]}
                      button={settingsById["noise-cancel-button"]}
                      {values}
                      {pending}
                      {lockReason}
                      compact
                      {onchange}
                    />
                  {:else}
                    <SettingControl
                      setting={s}
                      value={values[s.id] ?? null}
                      pending={!!pending[s.id]}
                      locked={!!lockReason(s)}
                      lockReason={lockReason(s) ?? ""}
                      compact
                      {onchange}
                    />
                  {/if}
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .panel {
    flex: 1 1 auto;
    height: 100%;
    overflow-y: auto;
    padding: 22px 26px 40px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 22px;
  }
  .picto {
    width: 52px;
    height: 52px;
    display: grid;
    place-items: center;
    color: var(--accent);
  }
  .titles {
    flex: 1 1 auto;
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 650;
  }
  .sub {
    color: var(--text-dim);
    font-size: 13px;
  }
  .pill {
    font-size: 12px;
    font-weight: 600;
    padding: 5px 11px;
    border-radius: 999px;
    background: var(--bg-elev);
    color: var(--text-dim);
    border: 1px solid var(--border);
  }
  .pill.on {
    color: var(--good);
    border-color: color-mix(in srgb, var(--good) 40%, transparent);
    background: color-mix(in srgb, var(--good) 12%, transparent);
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 14px;
    margin-bottom: 22px;
  }
  .card {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 15px 16px;
    box-shadow: var(--shadow);
  }
  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 10px;
    min-height: 40px;
  }
  .card-title {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
  .kv {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 3px 0;
    font-size: 13px;
  }
  .kv span {
    color: var(--text-dim);
  }
  .kv b {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .muted {
    color: var(--text-faint);
  }
  .level {
    margin-top: 8px;
  }
  .lvl-label {
    font-size: 12px;
    color: var(--text-dim);
  }
  /* VU meter: the track holds a fixed green→yellow→red gradient across its full
     width; the .unfilled overlay hides everything past the current level, so low
     levels show green and high levels reveal yellow then red. */
  .meter {
    position: relative;
    margin-top: 5px;
    height: 8px;
    border-radius: 999px;
    overflow: hidden;
    border: 1px solid var(--border);
    background: linear-gradient(
      90deg,
      #2ea043 0%,
      #57b846 35%,
      #e0b400 68%,
      #e8863a 85%,
      #e5484d 100%
    );
  }
  .unfilled {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    background: var(--bg);
    transition: left 0.09s linear;
  }

  .settings-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .settings-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
  .view-switch {
    display: inline-flex;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px;
  }
  .view-switch button {
    border: none;
    background: transparent;
    color: var(--text-dim);
    padding: 4px 12px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
  }
  .view-switch button.active {
    background: var(--accent);
    color: var(--accent-contrast);
  }

  .settings {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 4px 18px 8px;
    box-shadow: var(--shadow);
  }

  /* Masonry via balanced flex columns (see the script). Normal block flow means
     shadows render cleanly, unlike CSS multi-column. */
  .groups {
    display: flex;
    align-items: flex-start;
    gap: 18px;
  }
  .col {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .group {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 14px 14px;
    box-shadow: var(--shadow-card);
  }
  .group-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-faint);
    margin-bottom: 10px;
  }
  .group-grid {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>

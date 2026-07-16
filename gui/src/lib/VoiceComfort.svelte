<script>
  let {
    status = null,
    busy = false,
    error = null,
    ontoggle,
    onchange,
    onshortcuttoggle,
  } = $props();

  const presets = [
    { id: "natural", name: "温和自然", soften: 0.42, fullness: 0.42, deEss: 0.35, compression: 0.38, gainDb: 0 },
    { id: "soft", name: "重点去刺耳", soften: 0.78, fullness: 0.53, deEss: 0.68, compression: 0.58, gainDb: -0.5 },
    { id: "full", name: "饱满人声", soften: 0.58, fullness: 0.82, deEss: 0.45, compression: 0.66, gainDb: -1 },
    { id: "night", name: "深夜网课", soften: 0.84, fullness: 0.64, deEss: 0.76, compression: 0.48, gainDb: -3 },
  ];

  const controls = [
    { id: "soften", label: "柔化刺耳", min: 0, max: 1, step: 0.01, format: (v) => `${Math.round(v * 100)}%` },
    { id: "fullness", label: "人声饱满", min: 0, max: 1, step: 0.01, format: (v) => `${Math.round(v * 100)}%` },
    { id: "deEss", label: "齿音控制", min: 0, max: 1, step: 0.01, format: (v) => `${Math.round(v * 100)}%` },
    { id: "compression", label: "动态压缩", min: 0, max: 1, step: 0.01, format: (v) => `${Math.round(v * 100)}%` },
    { id: "gainDb", label: "输出音量", min: -12, max: 6, step: 0.1, format: (v) => `${v > 0 ? "+" : ""}${Number(v).toFixed(1)} dB` },
  ];

  const meterWidth = (db) => `${Math.max(0, Math.min(100, ((db ?? -80) + 60) / 60 * 100))}%`;
  function applyPreset(preset) {
    onchange?.({
      soften: preset.soften,
      fullness: preset.fullness,
      deEss: preset.deEss,
      compression: preset.compression,
      gainDb: preset.gainDb,
      bypass: false,
    });
  }

  function changeOne(id, value) {
    if (!status) return;
    onchange?.({
      soften: status.soften,
      fullness: status.fullness,
      deEss: status.deEss,
      compression: status.compression,
      gainDb: status.gainDb,
      bypass: status.bypass,
      [id]: id === "bypass" ? value : Number(value),
    });
  }
</script>

<div class="comfort-panel">
  <header class="comfort-head">
    <div class="title-mark" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="25" height="25" fill="none">
        <path d="M5 10v4M8.5 7v10M12 4v16M15.5 7v10M19 10v4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
      </svg>
    </div>
    <div>
      <h1>Voice Comfort</h1>
      <span class="engine-state" class:on={status?.running}>{status?.running ? "实时处理中" : "已停止"}</span>
    </div>
    <div class="head-actions">
      <label class="bypass">
        <input type="checkbox" checked={status?.bypass ?? false} disabled={!status?.running || busy} onchange={(e) => changeOne("bypass", e.currentTarget.checked)} />
        <span>原声</span>
      </label>
      <button class="power" class:on={status?.running} disabled={busy || !status?.available} onclick={() => ontoggle?.(!status?.running)}>
        <svg viewBox="0 0 24 24" width="17" height="17" fill="none" aria-hidden="true">
          <path d="M12 3v8M7.2 5.8a8 8 0 1 0 9.6 0" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" />
        </svg>
        {status?.running ? "停止处理" : "开始处理"}
      </button>
    </div>
  </header>

  {#if error || status?.error}
    <div class="error" role="alert">{error || status?.error}</div>
  {/if}

  <section class="preset-band" aria-label="预设">
    <span class="section-label">预设</span>
    <div class="preset-switch">
      {#each presets as preset (preset.id)}
        <button disabled={busy} onclick={() => applyPreset(preset)}>{preset.name}</button>
      {/each}
    </div>
  </section>

  <section class="controls" aria-label="人声参数">
    {#each controls as control (control.id)}
      <label class="control">
        <span class="control-head"><b>{control.label}</b><output>{control.format(status?.[control.id] ?? 0)}</output></span>
        <input
          type="range"
          min={control.min}
          max={control.max}
          step={control.step}
          value={status?.[control.id] ?? 0}
          disabled={busy}
          oninput={(event) => changeOne(control.id, event.currentTarget.value)}
        />
      </label>
    {/each}
  </section>

  <section class="meters" aria-label="实时电平">
    <div class="meter-row">
      <span>输入</span>
      <div class="meter"><i class="input" style:width={meterWidth(status?.inputDb)}></i></div>
      <output>{Math.round(status?.inputDb ?? -80)} dB</output>
    </div>
    <div class="meter-row">
      <span>输出</span>
      <div class="meter"><i class="output" style:width={meterWidth(status?.outputDb)}></i></div>
      <output>{Math.round(status?.outputDb ?? -80)} dB</output>
    </div>
    <div class="reduction">齿音衰减 <b>{(status?.deEssReductionDb ?? 0).toFixed(1)} dB</b></div>
  </section>

  <section class="shortcut-control" aria-label="接收器快捷键">
    <div class="shortcut-head">
      <div>
        <span class="section-label">接收器快捷键</span>
        <strong class:active={status?.shortcutRunning}>{status?.shortcutRunning ? "已接管" : "已停止"}</strong>
      </div>
      <button
        class="power"
        class:on={status?.shortcutRunning}
        disabled={busy || !status?.shortcutAvailable}
        onclick={() => onshortcuttoggle?.(!status?.shortcutRunning)}
      >
        {status?.shortcutRunning ? "停用" : "启用"}
      </button>
    </div>

    {#if status?.shortcutError}
      <div class="error shortcut-error" role="alert">{status.shortcutError}</div>
    {/if}

    <div class="shortcut-action">
      <span>接收器短按</span>
      <strong>Fn + Control</strong>
    </div>
  </section>
</div>

<style>
  .comfort-panel {
    flex: 1 1 auto;
    min-width: 0;
    overflow-y: auto;
    padding: 30px 34px 42px;
  }
  .comfort-head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 24px;
  }
  .title-mark {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--good) 15%, var(--bg-panel));
    color: var(--good);
  }
  h1 {
    margin: 0;
    font-size: 21px;
    letter-spacing: 0;
  }
  .engine-state {
    color: var(--text-dim);
    font-size: 12px;
  }
  .engine-state.on { color: var(--good); }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-left: auto;
  }
  .bypass {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--text-dim);
    font-size: 12px;
  }
  .power {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: 36px;
    padding: 0 13px;
    border: 1px solid var(--border-strong);
    border-radius: 7px;
    background: var(--bg-panel);
    color: var(--text);
  }
  .power.on {
    border-color: color-mix(in srgb, var(--good) 45%, var(--border));
    background: color-mix(in srgb, var(--good) 10%, var(--bg-panel));
    color: var(--good);
  }
  .error {
    margin-bottom: 18px;
    padding: 10px 12px;
    border-left: 3px solid var(--danger);
    background: color-mix(in srgb, var(--danger) 8%, transparent);
    color: var(--danger);
    font-size: 12px;
  }
  .preset-band {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 0 22px;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }
  .section-label {
    color: var(--text-dim);
    font-size: 12px;
    font-weight: 650;
  }
  .preset-switch {
    display: flex;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 7px;
  }
  .preset-switch button {
    min-height: 32px;
    padding: 0 13px;
    border: 0;
    border-right: 1px solid var(--border);
    background: var(--bg-panel);
    color: var(--text-dim);
    font-size: 12px;
  }
  .preset-switch button:last-child { border-right: 0; }
  .preset-switch button:hover { background: var(--bg-elev); color: var(--text); }
  .controls {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0 28px;
    padding: 20px 0;
    border-bottom: 1px solid var(--border);
  }
  .control {
    display: grid;
    gap: 10px;
    padding: 14px 0;
  }
  .control-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
  }
  .control-head b { font-weight: 620; }
  .control-head output { color: var(--text-dim); font-variant-numeric: tabular-nums; }
  input[type="range"] { width: 100%; accent-color: var(--good); }
  .meters {
    display: grid;
    gap: 11px;
    padding-top: 24px;
  }
  .meter-row {
    display: grid;
    grid-template-columns: 42px minmax(0, 1fr) 58px;
    align-items: center;
    gap: 10px;
    color: var(--text-dim);
    font-size: 11px;
  }
  .meter-row output { text-align: right; font-variant-numeric: tabular-nums; }
  .meter {
    height: 8px;
    overflow: hidden;
    border-radius: 4px;
    background: var(--bg-elev);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .meter i { display: block; height: 100%; transition: width .08s linear; }
  .meter .input { background: var(--warn); }
  .meter .output { background: var(--good); }
  .reduction {
    margin-top: 5px;
    color: var(--text-dim);
    font-size: 11px;
    text-align: right;
  }
  .reduction b { color: var(--text); font-variant-numeric: tabular-nums; }
  .shortcut-control {
    display: grid;
    gap: 14px;
    margin-top: 26px;
    padding-top: 22px;
    border-top: 1px solid var(--border);
  }
  .shortcut-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .shortcut-head > div { display: grid; gap: 4px; }
  .shortcut-head strong { font-size: 13px; font-weight: 650; color: var(--text-dim); }
  .shortcut-head strong.active { color: var(--good); }
  .shortcut-error { margin: 0; }
  .shortcut-action {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 11px 13px;
    background: var(--bg-panel);
    color: var(--text-dim);
    font-size: 12px;
  }
  .shortcut-action strong { color: var(--text); font-weight: 650; }
  @media (max-width: 760px) {
    .comfort-panel { padding: 22px 18px 34px; }
    .comfort-head { align-items: flex-start; flex-wrap: wrap; }
    .head-actions { width: 100%; margin-left: 0; justify-content: space-between; }
    .preset-band { align-items: flex-start; flex-direction: column; }
    .preset-switch { width: 100%; display: grid; grid-template-columns: 1fr 1fr; }
    .preset-switch button { border-bottom: 1px solid var(--border); }
  }
</style>

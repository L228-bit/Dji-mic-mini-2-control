<script>
  let { devices = null, busy = false, error = null, onchange, onrefresh } = $props();

  const isCurrent = (item) => item.is_default;
</script>

<details class="routing" ontoggle={(event) => event.currentTarget.open && onrefresh?.()}>
  <summary class="audio-orb" aria-label="音频输入与输出" title="音频输入与输出">
    <span class="orb-glow"></span>
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path d="M5 10v4M8.5 7.5v9M12 4.5v15M15.5 7.5v9M19 10v4" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" />
    </svg>
  </summary>

  <div class="popover">
    <header>
      <span class="hero-icon">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M5 10v4M8.5 7.5v9M12 4.5v15M15.5 7.5v9M19 10v4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
      </span>
      <div>
        <h2>音频路由</h2>
        <span>macOS 系统音频</span>
      </div>
      {#if busy}<span class="spinner" aria-label="正在切换"></span>{/if}
    </header>

    {#if !devices}
      <div class="empty">正在读取音频设备…</div>
    {:else if !devices.supported}
      <div class="empty">当前系统暂不支持。</div>
    {:else}
      <section>
        <div class="section-title">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <rect x="8" y="3" width="8" height="12" rx="4" stroke="currentColor" stroke-width="1.8" />
            <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
          </svg>
          输入
        </div>
        <div class="device-list">
          {#each devices.inputs as item (item.id)}
            <button class:active={isCurrent(item)} disabled={busy} onclick={() => onchange?.("input", item.id)}>
              <span class="device-dot input-dot"></span>
              <span class="device-name">{item.name}</span>
              {#if isCurrent(item)}
                <svg class="check" width="17" height="17" viewBox="0 0 20 20" fill="none" aria-hidden="true">
                  <circle cx="10" cy="10" r="9" fill="currentColor" /><path d="m6 10 2.5 2.5L14.5 7" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
              {/if}
            </button>
          {/each}
        </div>
      </section>

      <section>
        <div class="section-title">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M5 9v6h4l5 4V5L9 9H5Z" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" />
            <path d="M17 9.2a4 4 0 0 1 0 5.6M19.5 7a7 7 0 0 1 0 10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
          </svg>
          输出
        </div>
        <div class="device-list">
          {#each devices.outputs as item (item.id)}
            <button class:active={isCurrent(item)} disabled={busy} onclick={() => onchange?.("output", item.id)}>
              <span class="device-dot output-dot"></span>
              <span class="device-name">{item.name}</span>
              {#if isCurrent(item)}
                <svg class="check" width="17" height="17" viewBox="0 0 20 20" fill="none" aria-hidden="true">
                  <circle cx="10" cy="10" r="9" fill="currentColor" /><path d="m6 10 2.5 2.5L14.5 7" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
              {/if}
            </button>
          {/each}
        </div>
      </section>
    {/if}
    {#if error}<div class="error" role="alert">{error}</div>{/if}
  </div>
</details>

<style>
  .routing {
    position: relative;
    z-index: 150;
  }
  summary {
    list-style: none;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  .audio-orb {
    position: relative;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.52);
    border-radius: 50%;
    background: linear-gradient(145deg, rgba(255, 255, 255, 0.82), rgba(218, 226, 240, 0.62));
    box-shadow: 0 1px 2px rgba(19, 35, 58, 0.12), inset 0 1px 0 rgba(255, 255, 255, 0.84);
    color: #345f9e;
    cursor: pointer;
    transition: transform 0.18s ease, box-shadow 0.18s ease;
  }
  .audio-orb:hover,
  .routing[open] .audio-orb {
    transform: scale(1.08);
    box-shadow: 0 5px 16px rgba(42, 96, 180, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.9);
  }
  .orb-glow {
    position: absolute;
    inset: auto -4px -8px;
    height: 16px;
    background: #67a8ff;
    filter: blur(7px);
    opacity: 0.45;
  }
  .audio-orb svg {
    position: relative;
  }
  .popover {
    position: absolute;
    top: 35px;
    right: -2px;
    width: min(372px, calc(100vw - 24px));
    max-height: min(620px, calc(100vh - 62px));
    overflow: auto;
    padding: 15px;
    border: 1px solid rgba(255, 255, 255, 0.54);
    border-radius: 18px;
    background: color-mix(in srgb, var(--bg-panel) 78%, transparent);
    box-shadow: 0 28px 70px rgba(12, 22, 38, 0.28), inset 0 1px 0 rgba(255, 255, 255, 0.55);
    backdrop-filter: blur(30px) saturate(190%);
    user-select: text;
  }
  header {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 1px 2px 13px;
  }
  .hero-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 12px;
    background: linear-gradient(145deg, #6eabff, #2566c5);
    box-shadow: 0 7px 20px rgba(37, 102, 197, 0.28), inset 0 1px 0 rgba(255, 255, 255, 0.48);
    color: white;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 680;
    letter-spacing: 0;
  }
  header div > span {
    color: var(--text-dim);
    font-size: 11px;
  }
  .spinner {
    width: 14px;
    height: 14px;
    margin-left: auto;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  section + section {
    margin-top: 14px;
  }
  .section-title {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 4px 6px;
    color: var(--text-dim);
    font-size: 11px;
    font-weight: 650;
  }
  .device-list {
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-elev) 66%, transparent);
  }
  .device-list button {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) 18px;
    align-items: center;
    gap: 9px;
    width: 100%;
    min-height: 39px;
    padding: 7px 10px;
    border: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
    background: transparent;
    color: var(--text);
    text-align: left;
    transition: background 0.14s ease;
  }
  .device-list button:last-child {
    border-bottom: 0;
  }
  .device-list button:hover {
    background: color-mix(in srgb, var(--accent-soft) 55%, transparent);
  }
  .device-list button.active {
    background: color-mix(in srgb, var(--accent-soft) 80%, transparent);
  }
  .device-dot {
    width: 22px;
    height: 22px;
    border-radius: 7px;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.52);
  }
  .input-dot {
    background: linear-gradient(145deg, #5dd29a, #18865b);
  }
  .output-dot {
    background: linear-gradient(145deg, #8caeff, #5367c9);
  }
  .device-name {
    overflow: hidden;
    font-size: 12px;
    font-weight: 520;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .check {
    color: var(--accent);
  }
  .empty {
    padding: 22px 8px;
    color: var(--text-dim);
    font-size: 12px;
    text-align: center;
  }
  .error {
    margin-top: 10px;
    padding: 8px 10px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--danger);
    font-size: 11px;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  @media (prefers-color-scheme: dark) {
    .audio-orb {
      border-color: rgba(255, 255, 255, 0.16);
      background: linear-gradient(145deg, rgba(86, 96, 116, 0.82), rgba(35, 41, 52, 0.72));
      color: #a9c9ff;
    }
    .popover {
      border-color: rgba(255, 255, 255, 0.14);
      background: rgba(30, 33, 40, 0.82);
      box-shadow: 0 28px 70px rgba(0, 0, 0, 0.52), inset 0 1px 0 rgba(255, 255, 255, 0.1);
    }
  }
</style>

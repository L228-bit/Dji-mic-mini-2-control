//! DJI Mic Mini model definition and heartbeat decoder.
//!
//! The v2 firmware update speaks a different dialect than v1 firmware: a
//! target-addressed 22-byte command shape (see [`packet::Dialect`]), a
//! different CRC-16 seed, and a restructured heartbeat.
//! [`decode`](MicMini::decode) recognises either version from the frame
//! itself; [`DeviceModel::build_command`]'s default implementation needs to
//! be told which one to speak since a command can't carry that information
//! implicitly the way a heartbeat can.

use super::DeviceModel;
use crate::packet;
use crate::settings::{Setting, SettingKind, SettingOption, V2Target};
use crate::state::{DeviceStatus, Firmware, RxInfo, TxInfo};

/// The DJI Mic Mini (USB-C receiver + up to two transmitters).
pub struct MicMini;

const MODEL_ID: &str = "dji-mic-mini";

// v2 target addresses (see `PROTOCOL.md`).
const TARGET_RX: V2Target = V2Target::Fixed(0x0000);
const TARGET_TX: V2Target = V2Target::Fixed(0xffff);
/// One specific transmitter, chosen by the caller — see Voice Tone.
const TARGET_TX_ONE: V2Target = V2Target::Tx;
/// Broadcast by default, but optionally address one physical transmitter.
const TARGET_TX_FLEX: V2Target = V2Target::TxOrBroadcast;

// Standard Off=0x00 / On=0x01 pair shared by most toggles.
const fn off_on() -> [SettingOption; 2] {
    [
        SettingOption {
            value: "off",
            label: "关闭",
            wire: 0x00,
        },
        SettingOption {
            value: "on",
            label: "开启",
            wire: 0x01,
        },
    ]
}

static NC_OPTS: [SettingOption; 2] = [
    SettingOption {
        value: "basic",
        label: "基础",
        wire: 0x00,
    },
    SettingOption {
        value: "strong",
        label: "强力",
        wire: 0x01,
    },
];
static NC_POWER_OPTS: [SettingOption; 2] = off_on();
static NC_BUTTON_OPTS: [SettingOption; 2] = off_on();
static LOW_CUT_OPTS: [SettingOption; 2] = off_on();
// Segmented like Noise Cancel rather than a switch, since "off/on" reads
// oddly for a channel-mode choice. Wire values unchanged (stereo is 0x02,
// not 0x01).
static AUDIO_CHANNEL_OPTS: [SettingOption; 2] = [
    SettingOption {
        value: "mono",
        label: "单声道",
        wire: 0x00,
    },
    SettingOption {
        value: "stereo",
        label: "立体声",
        wire: 0x02,
    },
];
static SAFETY_OPTS: [SettingOption; 2] = off_on();
static CLIP_OPTS: [SettingOption; 2] = off_on();
static AUTO_OFF_OPTS: [SettingOption; 2] = off_on();
static CAMERA_OPTS: [SettingOption; 2] = off_on();
static PLUG_FREE_OPTS: [SettingOption; 2] = off_on();
// Off first / On second like every other toggle, so the UI switch reads
// naturally; the wire values remain inverted (On=0x00, Off=0x02).
static LED_OPTS: [SettingOption; 2] = [
    SettingOption {
        value: "off",
        label: "关闭",
        wire: 0x02,
    },
    SettingOption {
        value: "on",
        label: "开启",
        wire: 0x00,
    },
];
// DJI Mic Mini 2 only; addressed per-transmitter rather than broadcast (see
// `TARGET_TX_ONE`), so this is rendered on the TX card, not the shared
// settings list — front-ends key off this exact id.
static VOICE_TONE_OPTS: [SettingOption; 3] = [
    SettingOption {
        value: "standard",
        label: "标准",
        wire: 0x00,
    },
    SettingOption {
        value: "rich",
        label: "醇厚",
        wire: 0x01,
    },
    SettingOption {
        value: "bright",
        label: "明亮",
        wire: 0x02,
    },
];

const G_AUDIO: &str = "音频";
const G_POWER: &str = "电源与启动";
const G_DEVICE: &str = "设备";

static SETTINGS: [Setting; 13] = [
    Setting {
        id: "noise-cancel",
        label: "降噪模式",
        group: G_AUDIO,
        kind: SettingKind::Enum,
        v1_command: Some(0x031d),
        v2_command: 0x0037,
        v2_target: TARGET_TX_FLEX,
        options: &NC_OPTS,
        exclusive_with: &[],
        note: None,
    },
    Setting {
        id: "noise-cancel-power",
        label: "降噪",
        group: G_AUDIO,
        kind: SettingKind::Toggle,
        // New in v2 firmware: previously this could only be toggled by the
        // physical button on the TX (see `noise-cancel-button` below).
        v1_command: None,
        v2_command: 0x0038,
        v2_target: TARGET_TX_FLEX,
        options: &NC_POWER_OPTS,
        exclusive_with: &[],
        note: None,
    },
    Setting {
        id: "noise-cancel-button",
        label: "发射器电源键切换降噪",
        group: G_AUDIO,
        kind: SettingKind::Toggle,
        // New in v2 firmware. Controls whether a quick single press of the
        // transmitter's power button toggles noise cancellation — it does
        // not itself enable/disable NC (see `noise-cancel-power`).
        v1_command: None,
        v2_command: 0x000f,
        v2_target: TARGET_TX,
        options: &NC_BUTTON_OPTS,
        exclusive_with: &[],
        note: None,
    },
    Setting {
        id: "low-cut",
        label: "低切",
        group: G_AUDIO,
        kind: SettingKind::Toggle,
        v1_command: Some(0x0303),
        v2_command: 0x0003,
        v2_target: TARGET_TX,
        options: &LOW_CUT_OPTS,
        exclusive_with: &[],
        note: None,
    },
    Setting {
        id: "stereo",
        label: "音频声道",
        group: G_AUDIO,
        kind: SettingKind::Enum,
        v1_command: Some(0x0008),
        v2_command: 0x0008,
        v2_target: TARGET_RX,
        options: &AUDIO_CHANNEL_OPTS,
        // Stereo and Safety Track share the second channel; enabling one
        // disables the other on the device.
        exclusive_with: &["safety-track"],
        note: None,
    },
    Setting {
        id: "safety-track",
        label: "安全音轨",
        group: G_AUDIO,
        kind: SettingKind::Toggle,
        v1_command: Some(0x0021),
        v2_command: 0x0021,
        v2_target: TARGET_RX,
        options: &SAFETY_OPTS,
        exclusive_with: &["stereo"],
        note: None,
    },
    Setting {
        id: "clip-limiter",
        label: "削波控制",
        group: G_AUDIO,
        kind: SettingKind::Toggle,
        v1_command: Some(0x001e),
        v2_command: 0x001e,
        v2_target: TARGET_RX,
        options: &CLIP_OPTS,
        exclusive_with: &[],
        note: None,
    },
    Setting {
        id: "auto-off-15m",
        label: "接收器 15 分钟自动关机",
        group: G_POWER,
        kind: SettingKind::Toggle,
        v1_command: Some(0x0010),
        v2_command: 0x0010,
        v2_target: TARGET_RX,
        options: &AUTO_OFF_OPTS,
        exclusive_with: &[],
        note: None,
    },
    Setting {
        id: "tx-auto-off-15m",
        label: "发射器 15 分钟自动关机",
        group: G_POWER,
        kind: SettingKind::Toggle,
        // New in v2 firmware: the receiver and transmitters' auto-off timers
        // are now configured independently, sharing one command id
        // distinguished only by target address.
        v1_command: None,
        v2_command: 0x0010,
        v2_target: TARGET_TX,
        options: &AUTO_OFF_OPTS,
        exclusive_with: &[],
        note: None,
    },
    Setting {
        id: "camera-power",
        label: "接收器跟随相机开关机",
        group: G_POWER,
        kind: SettingKind::Toggle,
        v1_command: Some(0x0020),
        v2_command: 0x0020,
        v2_target: TARGET_RX,
        options: &CAMERA_OPTS,
        exclusive_with: &[],
        note: None,
    },
    Setting {
        id: "plug-free",
        label: "免插拔外放",
        group: G_DEVICE,
        kind: SettingKind::Toggle,
        v1_command: Some(0x0023),
        v2_command: 0x0023,
        v2_target: TARGET_RX,
        options: &PLUG_FREE_OPTS,
        exclusive_with: &[],
        note: Some("修改后接收器会重启。"),
    },
    Setting {
        id: "mic-leds",
        label: "麦克风指示灯",
        group: G_DEVICE,
        kind: SettingKind::Toggle,
        v1_command: Some(0x030a),
        // v2 firmware issued this setting a new command id even though
        // nothing else about it changed.
        v2_command: 0x000a,
        v2_target: TARGET_TX,
        options: &LED_OPTS,
        exclusive_with: &[],
        note: None,
    },
    Setting {
        id: "voice-tone",
        label: "人声音色",
        group: G_AUDIO,
        kind: SettingKind::Enum,
        // DJI Mic Mini 2 only; no v1 equivalent.
        v1_command: None,
        v2_command: 0x0029,
        v2_target: TARGET_TX_ONE,
        options: &VOICE_TONE_OPTS,
        exclusive_with: &[],
        note: None,
    },
];

const HEADER_LEN: usize = 14;
const PRESENT_TX_LEN: usize = 23;
const ABSENT_TX_LEN: usize = 9;
const RX_LEN: usize = 22;

/// A decoded v1 TX slot: its info if present, plus fields valid even in a stub.
struct V1Slot {
    tx: Option<TxInfo>,
    state: u8,
    flags: u8,
    led_off: Option<bool>,
    len: usize,
}

fn read_firmware(b: &[u8]) -> Firmware {
    Firmware([b[0], b[1], b[2], b[3]])
}

fn read_ascii_field(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}

fn parse_v1_slot(f: &[u8], off: usize) -> Option<V1Slot> {
    let flags = *f.get(off + 1)?;
    if flags & 0x20 != 0 {
        // Present entry.
        let end = off + PRESENT_TX_LEN;
        if f.len() < end {
            return None;
        }
        let state = f[off];
        let level = f[off + 2];
        let led_off = Some(f[off + 3] & 0x80 != 0);
        let firmware = read_firmware(&f[off + 4..off + 8]);
        let serial = read_ascii_field(&f[off + 9..off + 23]);
        Some(V1Slot {
            tx: Some(TxInfo {
                level: Some(level),
                serial: Some(serial),
                firmware: Some(firmware),
                product_name: None,
                voice_tone: None,
                charging: None,
                battery: None,
                nc_enabled: Some(flags & 0x01 != 0),
                nc_mode: Some(bit(state & 0x20 != 0, "strong", "basic")),
                low_cut: Some(state & 0x04 != 0),
                mic_leds: led_off.map(|off| !off),
                auto_off: None,
                nc_button: None,
            }),
            state,
            flags,
            led_off,
            len: PRESENT_TX_LEN,
        })
    } else {
        // Absent stub.
        let end = off + ABSENT_TX_LEN;
        if f.len() < end {
            return None;
        }
        Some(V1Slot {
            tx: None,
            state: f[off],
            flags,
            led_off: None,
            len: ABSENT_TX_LEN,
        })
    }
}

/// Decode a v1 heartbeat.
fn decode_v1(f: &[u8]) -> Option<DeviceStatus> {
    if f.len() < HEADER_LEN + RX_LEN {
        return None;
    }

    let slot_a = parse_v1_slot(f, HEADER_LEN)?;
    let slot_b = parse_v1_slot(f, HEADER_LEN + slot_a.len)?;
    let rx_off = HEADER_LEN + slot_a.len + slot_b.len;
    if f.len() < rx_off + RX_LEN {
        return None;
    }

    // State/flags are identical across slots and valid even in stubs; prefer
    // a present slot so we read live data rather than a stale stub.
    let (state, flags) = if slot_a.tx.is_some() {
        (slot_a.state, slot_a.flags)
    } else {
        (slot_b.state, slot_b.flags)
    };
    let led_off = slot_a.led_off.or(slot_b.led_off);

    let rx_flags0 = f[rx_off];
    let rx_flags1 = f[rx_off + 1];
    let rx = RxInfo {
        firmware: Some(read_firmware(&f[rx_off + 2..rx_off + 6])),
        serial: Some(read_ascii_field(&f[rx_off + 7..rx_off + 21])),
    };
    let plug_free_on = f[rx_off + 21] == 0x01;

    let mut status = DeviceStatus::disconnected(MODEL_ID);
    status.connected = true;
    status.nc_enabled = flags & 0x01 != 0;
    status.tx = [slot_a.tx, slot_b.tx];
    status.rx = Some(rx);

    let s = &mut status.settings;
    s.insert(
        "noise-cancel".into(),
        bit(state & 0x20 != 0, "strong", "basic"),
    );
    // V1 firmware can't set this independently of the mode above (only the
    // TX's physical button toggles it), but it does report it — reflect the
    // real state so the GUI can show it read-only rather than blank.
    s.insert("noise-cancel-power".into(), on_off(status.nc_enabled));
    s.insert("low-cut".into(), on_off(state & 0x04 != 0));
    s.insert(
        "stereo".into(),
        bit(rx_flags0 & 0x08 != 0, "stereo", "mono"),
    );
    s.insert("camera-power".into(), on_off(rx_flags0 & 0x01 != 0));
    s.insert("auto-off-15m".into(), on_off(rx_flags0 & 0x02 != 0));
    s.insert("safety-track".into(), on_off(rx_flags1 & 0x80 != 0));
    s.insert("clip-limiter".into(), on_off(rx_flags1 & 0x20 != 0));
    s.insert("plug-free".into(), on_off(plug_free_on));
    if let Some(off) = led_off {
        // LED state is only meaningful from a present TX entry.
        s.insert("mic-leds".into(), bit(off, "off", "on"));
    }

    Some(status)
}

// v2 status push: a fixed 52-byte flags header, then one 32-byte slot per
// connected transmitter (none for an absent one — unlike v1 there is no
// stub), then a 2-byte CRC. Identity (serial) and live audio level arrive in
// separate periodic pushes on this dialect (see [`decode_v2_records`]) that
// this frame carries forward from `previous` rather than knowing itself.
const V2_HEADER_LEN: usize = 52;
const V2_TX_SLOT_LEN: usize = 32;
/// Per-TX connected bitmask: `0x01` = TX1, `0x02` = TX2 (see `PROTOCOL.md`).
/// This is the authoritative signal for which physical transmitter(s) are
/// connected — it's set the instant a TX connects, up to one status-push
/// tick *before* that TX's 32-byte slot appears in the slot list, so slot
/// presence alone would lag it in that narrow window.
const V2_TX_CONNECTED_OFFSET: usize = 44;

/// Decode a v2 status push.
///
/// Returns `None` for anything of this dialect that isn't a status push (the
/// identity and audio-level pushes share the same heartbeat marker but don't
/// fit this frame's fixed header-plus-32-byte-slots shape).
fn decode_v2_status(previous: &DeviceStatus, f: &[u8]) -> Option<DeviceStatus> {
    if f.len() < V2_HEADER_LEN + 2 {
        return None;
    }
    // Status pushes use the 0x03 record family at the payload start. Mini 2
    // identity pushes with two 14-byte product names are 182 bytes long,
    // which also happens to satisfy the header + N*32 length formula below;
    // without this discriminator they are misread as status and make RX
    // settings flicker while dropping TX identity fields.
    if f[RECORDS_START] != 0x03 {
        return None;
    }
    let remainder = f.len() - (V2_HEADER_LEN + 2);
    if remainder % V2_TX_SLOT_LEN != 0 {
        return None;
    }
    let tx_count = (remainder / V2_TX_SLOT_LEN).min(2);

    let flags1 = f[20]; // strong-NC 0x08, NC power 0x20, camera-power 0x80
    let flags2 = f[21]; // auto-off (RX) 0x01, stereo 0x04
    let gain_dial = f[22] as i8; // signed dB, see PROTOCOL.md
    let flags3 = f[48]; // plug-free 0x02, clip limiter 0x10, safety track 0x40

    let mut status = DeviceStatus::disconnected(MODEL_ID);
    status.connected = true;
    status.nc_enabled = flags1 & 0x20 != 0;
    status.gain_dial = Some(gain_dial);
    // `V2_TX_CONNECTED_OFFSET`'s bits are the connected/disconnected source
    // of truth per physical TX — identity/level (from `decode_v2_records`)
    // is carried forward under whichever array index the bit for that unit
    // occupies.
    let connected = f[V2_TX_CONNECTED_OFFSET];
    status.tx = std::array::from_fn(|i| {
        (connected & (1 << i) != 0).then(|| {
            previous.tx[i].clone().unwrap_or(TxInfo {
                level: None,
                serial: None,
                firmware: None,
                product_name: None,
                voice_tone: None,
                charging: None,
                battery: None,
                nc_enabled: None,
                nc_mode: None,
                low_cut: None,
                mic_leds: None,
                auto_off: None,
                nc_button: None,
            })
        })
    });
    // Voice Tone, charging, and battery level don't mirror across TX the way
    // every other per-transmitter setting does (see `PROTOCOL.md`), so —
    // unlike `tx_flags`/`low_cut_flags` above — each present slot's own bits
    // are read individually, keyed by that slot's own physical unit byte at
    // `+1`.
    for slot_pos in 0..tx_count {
        let slot_off = V2_HEADER_LEN + V2_TX_SLOT_LEN * slot_pos;
        let unit = f[slot_off + 1];
        let Some(i) = (unit as usize).checked_sub(1).filter(|&i| i < 2) else {
            continue;
        };
        if let Some(tx) = status.tx[i].as_mut() {
            let tx_flags = f[slot_off + 6];
            let tone = f[slot_off + 9];
            tx.voice_tone = Some(
                if tone & 0x80 != 0 {
                    "bright"
                } else if tone & 0x40 != 0 {
                    "rich"
                } else {
                    "standard"
                }
                .into(),
            );
            let battery_flags = f[slot_off + 7];
            tx.charging = Some(battery_flags & 0x02 != 0);
            tx.battery = Some((battery_flags >> 2) & 0x07);
            tx.nc_enabled = Some(battery_flags & 0x01 != 0);
            tx.nc_mode = Some(bit(tx_flags & 0x20 != 0, "strong", "basic"));
            tx.mic_leds = Some(tx_flags & 0x02 == 0);
            tx.auto_off = Some(tx_flags & 0x10 != 0);
            tx.nc_button = Some(tx_flags & 0x80 != 0);
            tx.low_cut = Some(tone & 0x20 != 0);
        }
    }
    status.rx = Some(previous.rx.clone().unwrap_or(RxInfo {
        serial: None,
        firmware: None,
    }));

    let fallback_mode = previous
        .settings
        .get("noise-cancel")
        .cloned()
        .unwrap_or_else(|| bit(flags1 & 0x08 != 0, "strong", "basic"));
    let fallback_power = previous
        .settings
        .get("noise-cancel-power")
        .cloned()
        .unwrap_or_else(|| on_off(flags1 & 0x20 != 0));
    let fallback_low_cut = previous
        .settings
        .get("low-cut")
        .map(String::as_str)
        .unwrap_or("off");
    let fallback_leds = previous
        .settings
        .get("mic-leds")
        .map(String::as_str)
        .unwrap_or("on");
    let fallback_auto_off = previous
        .settings
        .get("tx-auto-off-15m")
        .map(String::as_str)
        .unwrap_or("off");
    let fallback_nc_button = previous
        .settings
        .get("noise-cancel-button")
        .map(String::as_str)
        .unwrap_or("off");

    let nc_mode = aggregate_values(
        status
            .tx
            .iter()
            .flatten()
            .filter_map(|tx| tx.nc_mode.clone()),
        &fallback_mode,
    );
    let nc_power = aggregate_values(
        status
            .tx
            .iter()
            .flatten()
            .filter_map(|tx| tx.nc_enabled.map(on_off)),
        &fallback_power,
    );
    let low_cut = aggregate_values(
        status
            .tx
            .iter()
            .flatten()
            .filter_map(|tx| tx.low_cut.map(on_off)),
        fallback_low_cut,
    );
    let mic_leds = aggregate_values(
        status
            .tx
            .iter()
            .flatten()
            .filter_map(|tx| tx.mic_leds.map(on_off)),
        fallback_leds,
    );
    let tx_auto_off = aggregate_values(
        status
            .tx
            .iter()
            .flatten()
            .filter_map(|tx| tx.auto_off.map(on_off)),
        fallback_auto_off,
    );
    let nc_button = aggregate_values(
        status
            .tx
            .iter()
            .flatten()
            .filter_map(|tx| tx.nc_button.map(on_off)),
        fallback_nc_button,
    );
    status.nc_enabled = nc_power == "on";

    let s = &mut status.settings;
    s.insert("noise-cancel".into(), nc_mode);
    s.insert("noise-cancel-power".into(), nc_power);
    s.insert("camera-power".into(), on_off(flags1 & 0x80 != 0));
    s.insert("auto-off-15m".into(), on_off(flags2 & 0x01 != 0));
    s.insert("stereo".into(), bit(flags2 & 0x04 != 0, "stereo", "mono"));
    s.insert("plug-free".into(), on_off(flags3 & 0x02 != 0));
    s.insert("clip-limiter".into(), on_off(flags3 & 0x10 != 0));
    s.insert("safety-track".into(), on_off(flags3 & 0x40 != 0));
    s.insert("mic-leds".into(), mic_leds);
    s.insert("tx-auto-off-15m".into(), tx_auto_off);
    s.insert("noise-cancel-button".into(), nc_button);
    s.insert("low-cut".into(), low_cut);

    Some(status)
}

// v2 identity and audio-level pushes share one record format starting at
// offset 14 (offsets 12–13 are a header this decoder doesn't need to
// interpret): repeated `[tag:1][unit index:1][reserved:3][length:1][data:
// length]` records running to the frame's trailing CRC.
//
// Known tags:
// - `0x01` (identity push): data is a 4-byte firmware version — most
//   significant component last, i.e. reversed relative to how it reads —
//   followed by a 14-byte ASCII serial. Unit index `0` = receiver, `1`/`2` =
//   transmitter slot 0/1.
// - `0x05` (audio-level push): one-byte data is the live input level. Unit
//   index is `1`/`2` for transmitter slot 0/1 (1-based, unlike the identity
//   push's 0-based index).
// - `0x06` (product name, e.g. `DJI Mic Mini` or `DJI Mic Mini 2`):
//   variable-length ASCII, length matching the record's own length byte.
//   Same unit index convention as `0x01`; only TX (`1`/`2`) is modelled.
// - `0x04` (a 6-character hex-like id) also appears in the identity push but
//   isn't modelled by `DeviceStatus` and is skipped.
const RECORDS_START: usize = 14;
const RECORD_HEADER_LEN: usize = 6;

fn decode_v2_records(previous: &DeviceStatus, f: &[u8]) -> Option<DeviceStatus> {
    if f.len() < RECORDS_START + 2 {
        return None;
    }
    let end = f.len() - 2; // exclude CRC
    let mut status = previous.clone();
    let mut matched = false;
    let mut i = RECORDS_START;

    while i + RECORD_HEADER_LEN <= end {
        let tag = f[i];
        let index = f[i + 1];
        let length = f[i + 5] as usize;
        let data_start = i + RECORD_HEADER_LEN;
        let data_end = data_start + length;
        if data_end > end {
            break; // not a frame built from these records after all
        }
        let data = &f[data_start..data_end];

        match tag {
            0x01 if data.len() > 4 => {
                // The first 4 bytes are the firmware version, most-significant
                // component last (i.e. reversed relative to how it reads);
                // the serial is whatever follows.
                let firmware = Firmware([data[3], data[2], data[1], data[0]]);
                let serial = read_ascii_field(&data[4..]);
                match index {
                    0 => {
                        let rx = status.rx.get_or_insert(RxInfo {
                            serial: None,
                            firmware: None,
                        });
                        rx.serial = Some(serial);
                        rx.firmware = Some(firmware);
                    }
                    1 | 2 => {
                        let i = (index - 1) as usize;
                        if let Some(slot) = status.tx.get_mut(i) {
                            let tx = slot.get_or_insert_with(|| TxInfo {
                                level: None,
                                serial: None,
                                firmware: None,
                                product_name: None,
                                voice_tone: None,
                                charging: None,
                                battery: None,
                                nc_enabled: None,
                                nc_mode: None,
                                low_cut: None,
                                mic_leds: None,
                                auto_off: None,
                                nc_button: None,
                            });
                            tx.serial = Some(serial);
                            tx.firmware = Some(firmware);
                        }
                    }
                    _ => {}
                }
                matched = true;
            }
            0x05 if data.len() == 1 && (index == 1 || index == 2) => {
                let i = (index - 1) as usize;
                if let Some(slot) = status.tx.get_mut(i) {
                    slot.get_or_insert_with(|| TxInfo {
                        level: None,
                        serial: None,
                        firmware: None,
                        product_name: None,
                        voice_tone: None,
                        charging: None,
                        battery: None,
                        nc_enabled: None,
                        nc_mode: None,
                        low_cut: None,
                        mic_leds: None,
                        auto_off: None,
                        nc_button: None,
                    })
                    .level = Some(data[0]);
                }
                matched = true;
            }
            0x06 if index == 1 || index == 2 => {
                let i = (index - 1) as usize;
                if let Some(slot) = status.tx.get_mut(i) {
                    slot.get_or_insert_with(|| TxInfo {
                        level: None,
                        serial: None,
                        firmware: None,
                        product_name: None,
                        voice_tone: None,
                        charging: None,
                        battery: None,
                        nc_enabled: None,
                        nc_mode: None,
                        low_cut: None,
                        mic_leds: None,
                        auto_off: None,
                        nc_button: None,
                    })
                    .product_name = Some(read_ascii_field(data));
                }
                matched = true;
            }
            _ => {}
        }

        i = data_end;
    }

    matched.then_some(status)
}

fn decode_v2(previous: &DeviceStatus, f: &[u8]) -> Option<DeviceStatus> {
    decode_v2_status(previous, f).or_else(|| decode_v2_records(previous, f))
}

impl DeviceModel for MicMini {
    fn id(&self) -> &'static str {
        MODEL_ID
    }
    fn name(&self) -> &'static str {
        "DJI Mic Mini / Mini 2"
    }
    fn pictogram_key(&self) -> &'static str {
        "mic-mini"
    }
    fn usb_ids(&self) -> &'static [(u16, u16)] {
        &[(0x2ca3, 0x4011)]
    }
    fn interface(&self) -> u8 {
        6
    }
    fn bulk_in(&self) -> u8 {
        0x86
    }
    fn bulk_out(&self) -> u8 {
        0x06
    }
    fn settings(&self) -> &'static [Setting] {
        &SETTINGS
    }

    fn decode(&self, previous: &DeviceStatus, f: &[u8]) -> Option<DeviceStatus> {
        let dialect = packet::heartbeat_dialect(f)?;
        let mut status = match dialect {
            packet::Dialect::V1 => decode_v1(f),
            packet::Dialect::V2 => decode_v2(previous, f),
        }?;
        status.protocol_version = Some(match dialect {
            packet::Dialect::V1 => 1,
            packet::Dialect::V2 => 2,
        });
        Some(status)
    }
}

fn on_off(b: bool) -> String {
    bit(b, "on", "off")
}

fn aggregate_values(values: impl Iterator<Item = String>, fallback: &str) -> String {
    let values: Vec<String> = values.collect();
    let Some(first) = values.first() else {
        return fallback.to_string();
    };
    if values.iter().all(|value| value == first) {
        first.clone()
    } else {
        "mixed".into()
    }
}

fn bit(b: bool, yes: &str, no: &str) -> String {
    if b { yes } else { no }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::Dialect;

    fn fresh_status() -> DeviceStatus {
        DeviceStatus::disconnected(MODEL_ID)
    }

    /// A synthetic 84-byte 2-TX v1 heartbeat matching the protocol spec's
    /// documented layout, with placeholder serials and firmware.
    fn sample_2tx_v1() -> Vec<u8> {
        let body: Vec<u8> = vec![
            0x55, 0x54, 0x04, 0xf6, 0x5a, 0x02, 0x0e, 0x79, 0x00, 0x5b, 0x03, 0x00, 0x47,
            0x00, // header
            // TX1, serial "BBBBBBBBBBBBBB"
            0xb0, 0x25, 0x38, 0x00, 0x01, 0x01, 0x00, 0x38, 0x0e, 0x42, 0x42, 0x42, 0x42, 0x42,
            0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
            0x42, // TX2, serial "CCCCCCCCCCCCCC"
            0xb0, 0x25, 0x38, 0x00, 0x01, 0x01, 0x00, 0x38, 0x0e, 0x43, 0x43, 0x43, 0x43, 0x43,
            0x43, 0x43, 0x43, 0x43, 0x43, 0x43, 0x43, 0x43,
            0x43, // RX, serial "AAAAAAAAAAAAAA"
            0x32, 0x20, 0x01, 0x01, 0x00, 0x38, 0x0e, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
            0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x00,
        ];
        crate::crc::append(body, crate::crc::V1_INIT, crate::crc::V1_RESIDUE)
    }

    #[test]
    fn decodes_two_tx_v1_heartbeat() {
        let hb = sample_2tx_v1();
        let st = MicMini.decode(&fresh_status(), &hb).expect("decode");
        assert!(st.connected);
        assert_eq!(st.protocol_version, Some(1));
        assert_eq!(st.tx_count(), 2);
        let tx1 = st.tx[0].as_ref().unwrap();
        assert_eq!(tx1.serial.as_deref(), Some("BBBBBBBBBBBBBB"));
        assert_eq!(tx1.firmware.unwrap().to_string(), "01.01.00.56");
        assert_eq!(
            st.tx[1].as_ref().unwrap().serial.as_deref(),
            Some("CCCCCCCCCCCCCC")
        );
        assert_eq!(
            st.rx.as_ref().unwrap().serial.as_deref(),
            Some("AAAAAAAAAAAAAA")
        );
        // state 0xb0 -> Strong NC (0x20 set), low cut clear.
        assert_eq!(st.settings["noise-cancel"], "strong");
        assert_eq!(st.settings["low-cut"], "off");
        // flags 0x25 -> NC enabled; v1 firmware still reports this even
        // though only the TX's button can change it.
        assert!(st.nc_enabled);
        assert_eq!(st.settings["noise-cancel-power"], "on");
        // rx flags 0x32/0x20 -> auto-off on, clip on, others off.
        assert_eq!(st.settings["auto-off-15m"], "on");
        assert_eq!(st.settings["stereo"], "mono");
        assert_eq!(st.settings["clip-limiter"], "on");
        assert_eq!(st.settings["safety-track"], "off");
        assert_eq!(st.settings["plug-free"], "off");
    }

    /// A synthetic 118-byte 2-TX v2 status push, CRC stripped (the test
    /// re-appends it) so this is exactly the 116-byte body.
    fn sample_2tx_v2() -> Vec<u8> {
        let hex = concat!(
            "557604a65a020000005b0303660003000000002028310c00000000000000",
            "0000000000000000000000000000030000001c00000002010000001ab825",
            "04010078000000000000000000000000000000000000000002020000001a",
            "b825040100780000000000000000000000000000000000000000",
        );
        let body: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        crate::crc::append(body, crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    #[test]
    fn decodes_two_tx_v2_heartbeat() {
        let hb = sample_2tx_v2();
        let st = MicMini.decode(&fresh_status(), &hb).expect("decode");
        assert!(st.connected);
        assert_eq!(st.protocol_version, Some(2));
        assert_eq!(st.tx_count(), 2);
        // Identity isn't carried by this push on v2 firmware.
        assert!(st.tx[0].as_ref().unwrap().serial.is_none());
        assert!(st.rx.as_ref().unwrap().serial.is_none());
        // offset 20 = 0x28 -> strong NC (0x08) + NC power on (0x20).
        assert_eq!(st.settings["noise-cancel"], "strong");
        assert_eq!(st.settings["noise-cancel-power"], "on");
        assert!(st.nc_enabled);
        assert_eq!(st.settings["camera-power"], "off");
        // offset 21 = 0x31 -> auto-off on (0x01), stereo off (0x04 clear).
        assert_eq!(st.settings["auto-off-15m"], "on");
        assert_eq!(st.settings["stereo"], "mono");
        // offset 22 = 0x0c -> gain dial at +12 dB.
        assert_eq!(st.gain_dial, Some(12));
        // offset 48 = 0x1c -> plug-free off, clip on (0x10), safety off.
        assert_eq!(st.settings["plug-free"], "off");
        assert_eq!(st.settings["clip-limiter"], "on");
        assert_eq!(st.settings["safety-track"], "off");
        // TX slot flags byte (offset 58) = 0xb8 -> LED on (0x02 clear),
        // TX auto-off on (0x10 set), NC-button permission on (0x80 set).
        assert_eq!(st.settings["mic-leds"], "on");
        assert_eq!(st.settings["tx-auto-off-15m"], "on");
        assert_eq!(st.settings["noise-cancel-button"], "on");
        // Low-cut byte (offset 61) = 0x01 -> off (0x20 clear).
        assert_eq!(st.settings["low-cut"], "off");
        // Neither tone bit (+9 = 0x01, just the constant) set on either slot.
        assert_eq!(
            st.tx[0].as_ref().unwrap().voice_tone.as_deref(),
            Some("standard")
        );
        assert_eq!(
            st.tx[1].as_ref().unwrap().voice_tone.as_deref(),
            Some("standard")
        );
    }

    #[test]
    fn keeps_noise_cancel_power_independent_per_tx() {
        let mut body = sample_2tx_v2();
        body.truncate(body.len() - 2);
        // TX1 +7 keeps bit 0 set; TX2 +7 clears it while retaining the
        // constant and battery bits. The header still says NC is on.
        body[V2_HEADER_LEN + V2_TX_SLOT_LEN + 7] &= !0x01;
        let frame = crate::crc::append(body, crate::crc::V2_INIT, crate::crc::V2_RESIDUE);
        let st = MicMini.decode(&fresh_status(), &frame).expect("decode");
        assert_eq!(st.tx[0].as_ref().unwrap().nc_enabled, Some(true));
        assert_eq!(st.tx[1].as_ref().unwrap().nc_enabled, Some(false));
        assert_eq!(st.settings["noise-cancel-power"], "mixed");
        assert!(!st.nc_enabled, "mixed is not reported as globally enabled");
    }

    fn hex_body(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    /// The same shape as [`sample_2tx_v2`], but header offset 22 is `0xf4`
    /// (-12 as a signed byte) rather than `0x0c` (+12) — confirming the
    /// gain dial decodes as signed, not just happening to work for a
    /// positive value.
    fn sample_2tx_v2_negative_gain() -> Vec<u8> {
        let hex = "557604a65a020000005b030366000300000000202831f4000000000000000000000000000000000000000000030000001c00000002010000001ab82504010078000000000000000000000000000000000000000002020000001ab825040100780000000000000000000000000000000000000000";
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    #[test]
    fn decodes_negative_gain_dial() {
        let st = MicMini
            .decode(&fresh_status(), &sample_2tx_v2_negative_gain())
            .expect("decode");
        assert_eq!(st.gain_dial, Some(-12));
    }

    /// The same shape as [`sample_2tx_v2`], but TX1's tone byte (+9) is
    /// `0x41` (Rich) and TX2's is `0x81` (Bright) — confirming Voice Tone is
    /// read per-slot rather than mirrored like every other TX setting.
    fn sample_2tx_v2_distinct_tones() -> Vec<u8> {
        let hex = "557604a65a020000005b0303660003000000002028310c000000000000000000000000000000000000000000030000001c00000002010000001ab82504410078000000000000000000000000000000000000000002020000001ab825048100780000000000000000000000000000000000000000";
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    #[test]
    fn decodes_voice_tone_independently_per_tx() {
        let st = MicMini
            .decode(&fresh_status(), &sample_2tx_v2_distinct_tones())
            .expect("decode");
        assert_eq!(
            st.tx[0].as_ref().unwrap().voice_tone.as_deref(),
            Some("rich")
        );
        assert_eq!(
            st.tx[1].as_ref().unwrap().voice_tone.as_deref(),
            Some("bright")
        );
    }

    /// The same shape as [`sample_2tx_v2`], but TX2's `+7` bit `0x02` is
    /// set — the docked/charging state.
    fn sample_2tx_v2_tx2_charging() -> Vec<u8> {
        let hex = "557604a65a020000005b0303660003000000002028310c000000000000000000000000000000000000000000030000001c00000002010000001ab82504010078000000000000000000000000000000000000000002020000001ab827040100780000000000000000000000000000000000000000";
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    #[test]
    fn decodes_charging_independently_per_tx() {
        let st = MicMini
            .decode(&fresh_status(), &sample_2tx_v2_tx2_charging())
            .expect("decode");
        assert_eq!(st.tx[0].as_ref().unwrap().charging, Some(false));
        assert_eq!(st.tx[1].as_ref().unwrap().charging, Some(true));
    }

    /// The same shape as [`sample_2tx_v2`] (TX1's `+7` stays `0x25`, battery
    /// gauge value 1), but TX2's `+7` is overridden per case to cover the
    /// battery gauge's range.
    fn sample_2tx_v2_tx2_battery(tx2_plus7: u8) -> Vec<u8> {
        let hex = format!(
            "557604a65a020000005b0303660003000000002028310c000000000000000000000000000000000000000000030000001c\
             00000002010000001ab82504010078000000000000000000000000000000000000000002020000001ab8{tx2_plus7:02x}\
             040100780000000000000000000000000000000000000000"
        );
        crate::crc::append(hex_body(&hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    #[test]
    fn decodes_battery_gauge_independently_per_tx() {
        // The `+7` battery bits (`0x1c`) are a 3-bit gauge read as
        // `(byte >> 2) & 0x07`, not a mirrored flag — TX1 stays at gauge
        // value 1 (`0x04` on top of the base `0x21`) while TX2 varies.
        let st = MicMini
            .decode(&fresh_status(), &sample_2tx_v2_tx2_battery(0x39))
            .expect("decode");
        assert_eq!(st.tx[0].as_ref().unwrap().battery, Some(1));
        assert_eq!(st.tx[1].as_ref().unwrap().battery, Some(6)); // 0x08|0x10

        let st = MicMini
            .decode(&fresh_status(), &sample_2tx_v2_tx2_battery(0x31))
            .expect("decode");
        assert_eq!(st.tx[1].as_ref().unwrap().battery, Some(4)); // 0x10 alone

        // All three bits set: the terminal reading before auto-shutoff.
        let st = MicMini
            .decode(&fresh_status(), &sample_2tx_v2_tx2_battery(0x3d))
            .expect("decode");
        assert_eq!(st.tx[1].as_ref().unwrap().battery, Some(7));

        // No battery bits set: gauge value 0 (believed unreachable in
        // practice, but the decoder must still represent it correctly).
        let st = MicMini
            .decode(&fresh_status(), &sample_2tx_v2_tx2_battery(0x21))
            .expect("decode");
        assert_eq!(st.tx[1].as_ref().unwrap().battery, Some(0));
    }

    /// A synthetic receiver-only identity push (70 bytes), CRC stripped.
    fn sample_identity_rx_only() -> Vec<u8> {
        let hex = concat!(
            "5546048b5a020000005b0303360001000000001200110302444444444444444444",
            "444444444404000000000630303030303006000000000c444a49204d6963204d69",
            "6e69",
        );
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    /// A synthetic receiver + one transmitter identity push (124 bytes, as
    /// sent after a reboot caused by toggling the plug-free speaker), CRC
    /// stripped.
    fn sample_identity_rx_and_tx() -> Vec<u8> {
        let hex = concat!(
            "557c04415a020000005b03036c0001000000001200110302444444444444444444",
            "444444444404000000000630303030303006000000000c444a49204d6963204d69",
            "6e6901010000001200110302454545454545454545454545454504010000000630",
            "303030303006010000000c444a49204d6963204d696e69",
        );
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    /// A synthetic receiver + both transmitters identity push (178 bytes),
    /// CRC stripped. This is the largest v2 frame the protocol produces —
    /// `packet::take_frame` used to silently drop it as an "implausible
    /// length" until `MAX_FRAME` was raised to account for it, which is why
    /// this fixture exists.
    fn sample_identity_rx_and_two_tx() -> Vec<u8> {
        let hex = concat!(
            "55b204295a020000005b0303a20001000000001200110302444444444444444444",
            "444444444404000000000630303030303006000000000c444a49204d6963204d69",
            "6e6901010000001200110302454545454545454545454545454504010000000630",
            "303030303006010000000c444a49204d6963204d696e6901020000001200110302",
            "464646464646464646464646464604020000000630303030303006020000000c44",
            "4a49204d6963204d696e69",
        );
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    /// A synthetic receiver + one transmitter identity push (124 bytes), CRC
    /// stripped, where the transmitter's product-name record is the
    /// variable-length `"DJI Mic Mini 2"` (14 bytes) rather than the usual
    /// `"DJI Mic Mini"` (12 bytes) — the record's own length byte differs
    /// accordingly.
    fn sample_identity_rx_and_tx_v2_model() -> Vec<u8> {
        let hex = "557e040000000000005b0303000001000000001200110302444444444444444444444444444404000000000637626165386606000000000c444a49204d6963204d696e69010100000012000b0302454545454545454545454545454504010000000661376330313206010000000e444a49204d6963204d696e692032";
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    /// A synthetic 2-TX audio-level push (30 bytes), CRC stripped.
    fn sample_level_2tx() -> Vec<u8> {
        let hex = "551e048a5a020000005b03030e000501000000013005020000000130";
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    #[test]
    fn decodes_v2_identity_push_for_receiver_only() {
        let st = MicMini
            .decode(&fresh_status(), &sample_identity_rx_only())
            .expect("decode");
        let rx = st.rx.as_ref().unwrap();
        assert_eq!(rx.serial.as_deref(), Some("DDDDDDDDDDDDDD"));
        assert_eq!(rx.firmware.unwrap().to_string(), "02.03.17.00");
    }

    #[test]
    fn decodes_v2_identity_push_for_receiver_and_transmitter() {
        let st = MicMini
            .decode(&fresh_status(), &sample_identity_rx_and_tx())
            .expect("decode");
        assert_eq!(
            st.rx.as_ref().unwrap().serial.as_deref(),
            Some("DDDDDDDDDDDDDD")
        );
        let tx1 = st.tx[0].as_ref().unwrap();
        assert_eq!(tx1.serial.as_deref(), Some("EEEEEEEEEEEEEE"));
        assert_eq!(tx1.firmware.unwrap().to_string(), "02.03.17.00");
        assert_eq!(tx1.product_name.as_deref(), Some("DJI Mic Mini"));
    }

    #[test]
    fn decodes_v2_identity_push_for_receiver_and_both_transmitters() {
        let st = MicMini
            .decode(&fresh_status(), &sample_identity_rx_and_two_tx())
            .expect("decode");
        assert_eq!(
            st.rx.as_ref().unwrap().serial.as_deref(),
            Some("DDDDDDDDDDDDDD")
        );
        assert_eq!(
            st.tx[0].as_ref().unwrap().serial.as_deref(),
            Some("EEEEEEEEEEEEEE")
        );
        assert_eq!(
            st.tx[1].as_ref().unwrap().serial.as_deref(),
            Some("FFFFFFFFFFFFFF")
        );
        // Firmware and product name are identical across every unit in this fixture.
        for tx in st.tx.iter().flatten() {
            assert_eq!(tx.firmware.unwrap().to_string(), "02.03.17.00");
            assert_eq!(tx.product_name.as_deref(), Some("DJI Mic Mini"));
        }
    }

    #[test]
    fn live_mini_2_sequence_keeps_both_identities_and_receiver_settings_stable() {
        // Captured from a receiver with two Mic Mini 2 transmitters connected.
        let status = hex_body("557604a65a020000005b03036600030000000020a03500000000000000000000000000000000000000000000030000001e00000002010000001a982504210078000000000000000000000000000000000000000002020000001a9825042100780000000000000000000000000000000000000000dda0");
        let identity = hex_body(concat!(
            "55b604125a020000005b0303a600010000000012001103023850435450323630313535573633040000000006373265363563",
            "06000000000c444a49204d6963204d696e690101000000120011030242334d5450354c30313538365a500401000000063835",
            "3431373806010000000e444a49204d6963204d696e6920320102000000120011030242334d5450354c303135525432310402",
            "0000000638323938323406020000000e444a49204d6963204d696e692032e103"
        ));
        let levels = hex_body("551e048a5a020000005b03030e000501000000010f050200000001156760");

        let after_status = MicMini.decode(&fresh_status(), &status).expect("status");
        let after_identity = MicMini.decode(&after_status, &identity).expect("identity");
        let after_levels = MicMini.decode(&after_identity, &levels).expect("levels");
        let repeated = MicMini
            .decode(&after_levels, &status)
            .expect("repeated status");

        assert_eq!(
            repeated.tx[0].as_ref().unwrap().serial.as_deref(),
            Some("B3MTP5L01586ZP")
        );
        assert_eq!(
            repeated.tx[1].as_ref().unwrap().serial.as_deref(),
            Some("B3MTP5L015RT21")
        );
        assert_eq!(repeated.settings["plug-free"], "on");
        assert_eq!(repeated.settings["camera-power"], "on");
        assert_eq!(repeated.settings["stereo"], "stereo");
    }

    #[test]
    fn decodes_variable_length_product_name_for_a_different_tx_model() {
        // A transmitter can report a product name of a different length than
        // its peer (e.g. a newer-generation unit's name is longer) — the
        // record's own length byte, not a fixed width, governs how much to
        // read.
        let st = MicMini
            .decode(&fresh_status(), &sample_identity_rx_and_tx_v2_model())
            .expect("decode");
        assert_eq!(
            st.tx[0].as_ref().unwrap().product_name.as_deref(),
            Some("DJI Mic Mini 2")
        );
    }

    #[test]
    fn take_frame_does_not_drop_the_178_byte_identity_push() {
        // Regression test for the bug this fixture exists to catch:
        // `MAX_FRAME` used to reject this frame outright as an implausible
        // length, silently discarding every identity push with two TX
        // connected before it ever reached `decode`.
        let mut buf = sample_identity_rx_and_two_tx();
        let frame = packet::take_frame(&mut buf).expect("frame");
        assert_eq!(frame.len(), 178);
        assert!(buf.is_empty());
    }

    #[test]
    fn decodes_v2_level_push() {
        // The level push alone doesn't know a TX is connected, so seed
        // `previous` as the status push already would have.
        let mut previous = fresh_status();
        previous.tx = [
            Some(TxInfo {
                level: None,
                serial: None,
                firmware: None,
                product_name: None,
                voice_tone: None,
                charging: None,
                battery: None,
                nc_enabled: None,
                nc_mode: None,
                low_cut: None,
                mic_leds: None,
                auto_off: None,
                nc_button: None,
            }),
            Some(TxInfo {
                level: None,
                serial: None,
                firmware: None,
                product_name: None,
                voice_tone: None,
                charging: None,
                battery: None,
                nc_enabled: None,
                nc_mode: None,
                low_cut: None,
                mic_leds: None,
                auto_off: None,
                nc_button: None,
            }),
        ];
        let st = MicMini
            .decode(&previous, &sample_level_2tx())
            .expect("decode");
        assert_eq!(st.tx[0].as_ref().unwrap().level, Some(0x30));
        assert_eq!(st.tx[1].as_ref().unwrap().level, Some(0x30));
    }

    #[test]
    fn v2_status_push_carries_identity_and_level_forward() {
        // Identity/level arrive on separate pushes from the flags; a later
        // status push for the same connected TX must not erase them.
        let after_identity = MicMini
            .decode(&fresh_status(), &sample_identity_rx_and_tx())
            .unwrap();
        let with_level = MicMini
            .decode(&after_identity, &sample_level_2tx())
            .unwrap();
        let status_push = MicMini.decode(&with_level, &sample_2tx_v2()).unwrap();

        assert_eq!(
            status_push.rx.as_ref().unwrap().serial.as_deref(),
            Some("DDDDDDDDDDDDDD")
        );
        assert_eq!(
            status_push
                .rx
                .as_ref()
                .unwrap()
                .firmware
                .unwrap()
                .to_string(),
            "02.03.17.00"
        );
        assert_eq!(
            status_push.tx[0].as_ref().unwrap().serial.as_deref(),
            Some("EEEEEEEEEEEEEE")
        );
        assert_eq!(status_push.tx[0].as_ref().unwrap().level, Some(0x30));
        assert_eq!(status_push.tx[1].as_ref().unwrap().level, Some(0x30));
        // The flags themselves still decode normally alongside the carried-
        // forward identity/level.
        assert_eq!(status_push.settings["noise-cancel"], "strong");
    }

    /// A synthetic 1-TX v2 status push (86 bytes), CRC stripped: capability
    /// byte, connected-bitmask (`+44` = `0x02`), and sole slot (unit byte
    /// `2` at slot offset +1) all agree it's TX2 alone connected.
    fn sample_tx2_only_v2() -> Vec<u8> {
        let hex = "555604a65a020000005b0303460003000000002028310c000000000000000000000000000000000000000000020000001c00000002020000001ab825040100780000000000000000000000000000000000000000";
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    /// The same shape as [`sample_tx2_only_v2`] but for TX1 alone (connected
    /// bitmask `0x01`, slot unit byte `1`).
    fn sample_tx1_only_v2() -> Vec<u8> {
        let hex = "555604a65a020000005b0303460003000000002028310c000000000000000000000000000000000000000000010000001c00000002010000001ab825040100780000000000000000000000000000000000000000";
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    fn one_slot_with_both_connected(
        mut frame: Vec<u8>,
        tx_flags: u8,
        battery_flags: u8,
        tone_flags: u8,
    ) -> Vec<u8> {
        frame.truncate(frame.len() - 2);
        frame[V2_TX_CONNECTED_OFFSET] = 0x03;
        frame[V2_HEADER_LEN + 6] = tx_flags;
        frame[V2_HEADER_LEN + 7] = battery_flags;
        frame[V2_HEADER_LEN + 9] = tone_flags;
        crate::crc::append(frame, crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    #[test]
    fn alternating_single_tx_pushes_keep_a_stable_mixed_state() {
        // This is the real two-transmitter shape seen in the screen recording:
        // the connected mask says both units are online, but each push carries
        // only one TX slot. TX1 has every tested option on; TX2 differs.
        let tx1 = one_slot_with_both_connected(sample_tx1_only_v2(), 0xb8, 0x25, 0x01);
        let tx2 = one_slot_with_both_connected(sample_tx2_only_v2(), 0x02, 0x24, 0x21);

        let after_tx1 = MicMini.decode(&fresh_status(), &tx1).expect("TX1 push");
        let after_tx2 = MicMini.decode(&after_tx1, &tx2).expect("TX2 push");
        let repeated = MicMini.decode(&after_tx2, &tx1).expect("TX1 repeat");

        for setting in [
            "noise-cancel",
            "noise-cancel-power",
            "low-cut",
            "mic-leds",
            "tx-auto-off-15m",
            "noise-cancel-button",
        ] {
            assert_eq!(after_tx2.settings[setting], "mixed", "{setting}");
            assert_eq!(
                repeated.settings[setting], "mixed",
                "{setting} must not flicker"
            );
        }
    }

    #[test]
    fn decodes_solo_tx_status_pushes_by_physical_unit_byte() {
        // Each slot names its own physical TX (1 or 2) at offset +1 of its
        // 32-byte record — a lone connected transmitter lands in the array
        // index matching that number, not always index 0.
        let tx1 = MicMini
            .decode(&fresh_status(), &sample_tx1_only_v2())
            .expect("decode");
        assert!(tx1.tx[0].is_some());
        assert!(tx1.tx[1].is_none());

        let tx2 = MicMini
            .decode(&fresh_status(), &sample_tx2_only_v2())
            .expect("decode");
        assert!(tx2.tx[0].is_none());
        assert!(tx2.tx[1].is_some());
    }

    #[test]
    fn tx1_disconnecting_from_both_resolves_on_the_very_next_frame() {
        // Both connected and fully identified (mirrors
        // `v2_status_push_carries_identity_and_level_forward`'s end state).
        let after_identity = MicMini
            .decode(&fresh_status(), &sample_identity_rx_and_two_tx())
            .unwrap();
        let both = MicMini.decode(&after_identity, &sample_2tx_v2()).unwrap();
        assert!(both.tx[0].is_some());
        assert!(both.tx[1].is_some());

        // TX1 physically disconnects, leaving TX2. TX2's own claimed unit
        // number places its data correctly on this single frame.
        let after = MicMini.decode(&both, &sample_tx2_only_v2()).unwrap();
        assert!(after.tx[0].is_none());
        assert_eq!(
            after.tx[1].as_ref().unwrap().serial.as_deref(),
            Some("FFFFFFFFFFFFFF")
        );
    }

    /// A synthetic 0-TX v2 status push (54 bytes), CRC stripped, but with
    /// the connected bitmask (`+44`) already flagging TX2 — the shape a real
    /// device sends for up to one tick right as a transmitter connects,
    /// before its slot appears in the slot list.
    fn sample_tx2_flagged_but_no_slot_yet_v2() -> Vec<u8> {
        let hex = "553604a65a020000005b0303260003000000002028310c000000000000000000000000000000000000000000020000001c000000";
        crate::crc::append(hex_body(hex), crate::crc::V2_INIT, crate::crc::V2_RESIDUE)
    }

    #[test]
    fn tx_connecting_shows_as_connected_before_its_slot_appears() {
        // `+44` is flagged for TX2 a tick before the slot list catches up —
        // the decoder should show TX2 as connected-but-unidentified
        // immediately rather than waiting for its slot.
        let st = MicMini
            .decode(&fresh_status(), &sample_tx2_flagged_but_no_slot_yet_v2())
            .expect("decode");
        assert!(st.tx[0].is_none());
        let tx2 = st.tx[1]
            .as_ref()
            .expect("TX2 shown connected from the bitmask alone");
        assert_eq!(tx2.serial, None);
        assert_eq!(tx2.level, None);

        // Its slot appears on the next tick; identity/level fill in later.
        let after = MicMini.decode(&st, &sample_tx2_only_v2()).expect("decode");
        assert!(after.tx[1].is_some());
    }

    #[test]
    fn builds_v1_command_from_setting_slug() {
        let pkt = MicMini
            .build_command(1, Dialect::V1, "noise-cancel", "strong", None)
            .unwrap();
        assert_eq!(pkt.len(), packet::V1_COMMAND_LEN);
        assert_eq!(pkt[16], 0x01); // wire value
        assert!(MicMini
            .build_command(1, Dialect::V1, "bogus", "x", None)
            .is_none());
        assert!(MicMini
            .build_command(1, Dialect::V1, "stereo", "maybe", None)
            .is_none());
    }

    #[test]
    fn builds_v2_command_from_setting_slug() {
        let pkt = MicMini
            .build_command(1, Dialect::V2, "noise-cancel", "strong", None)
            .unwrap();
        assert_eq!(pkt.len(), packet::V2_COMMAND_LEN);
        assert_eq!(&pkt[12..14], &[0xff, 0xff]); // broadcast to both TX
        assert_eq!(pkt[19], 0x01); // wire value
    }

    #[test]
    fn builds_noise_cancel_commands_for_one_transmitter() {
        let tx1 = MicMini
            .build_command(1, Dialect::V2, "noise-cancel-power", "on", Some(0))
            .unwrap();
        assert_eq!(&tx1[12..14], &[0x01, 0x00]);
        assert_eq!(tx1[19], 0x01);

        let tx2 = MicMini
            .build_command(2, Dialect::V2, "noise-cancel", "strong", Some(1))
            .unwrap();
        assert_eq!(&tx2[12..14], &[0x02, 0x00]);
        assert_eq!(tx2[19], 0x01);
    }

    #[test]
    fn v2_only_setting_has_no_v1_command() {
        // "noise-cancel-power" was introduced by v2 firmware, so building it
        // for v1 devices must fail cleanly rather than send a command that
        // doesn't exist there.
        assert!(MicMini
            .build_command(1, Dialect::V1, "noise-cancel-power", "on", None)
            .is_none());
        assert!(MicMini
            .build_command(1, Dialect::V2, "noise-cancel-power", "on", None)
            .is_some());
    }

    #[test]
    fn builds_voice_tone_command_targeted_at_a_specific_tx() {
        // Voice Tone addresses one physical transmitter (unit 1 or 2), not a
        // broadcast — this must fail without a `tx`, and the built packet's
        // target must match whichever slot was requested.
        assert!(MicMini
            .build_command(1, Dialect::V2, "voice-tone", "rich", None)
            .is_none());

        let pkt_tx1 = MicMini
            .build_command(1, Dialect::V2, "voice-tone", "rich", Some(0))
            .unwrap();
        assert_eq!(pkt_tx1[12], 0x01); // target unit 1, little-endian
        assert_eq!(pkt_tx1[13], 0x00);
        assert_eq!(pkt_tx1[19], 0x01); // wire value for "rich"

        let pkt_tx2 = MicMini
            .build_command(1, Dialect::V2, "voice-tone", "bright", Some(1))
            .unwrap();
        assert_eq!(pkt_tx2[12], 0x02); // target unit 2
        assert_eq!(pkt_tx2[19], 0x02); // wire value for "bright"

        // No v1 equivalent.
        assert!(MicMini
            .build_command(1, Dialect::V1, "voice-tone", "rich", Some(0))
            .is_none());
    }
}

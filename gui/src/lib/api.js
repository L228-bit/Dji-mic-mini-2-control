import { invoke } from "@tauri-apps/api/core";

/** Fetch a full UI snapshot for the given device id (or the sole device). */
export function snapshot(device) {
  return invoke("snapshot", { device: device ?? null });
}

/** Change one setting on a device. Resolves on success, rejects with a message. */
export function setSetting(device, setting, value) {
  return invoke("set_setting", { device, setting, value });
}

/**
 * Change one setting on a specific transmitter slot (0-based) of a device —
 * for a setting that targets one TX individually rather than mirroring
 * across both (currently just Voice Tone).
 */
export function setTxSetting(device, tx, setting, value) {
  return invoke("set_tx_setting", { device, tx, setting, value });
}

/** Fetch the Linux udev-rules helper text. */
export function udevHelp() {
  return invoke("udev_help");
}

/** List macOS audio devices and the current system defaults. */
export function audioDevices() {
  return invoke("audio_devices");
}

/** Set the system default audio input or output device. */
export function setAudioDevice(kind, device) {
  return invoke("set_audio_device", { kind, device });
}

export function comfortStatus() {
  return invoke("comfort_status");
}

export function comfortStart() {
  return invoke("comfort_start");
}

export function comfortStop() {
  return invoke("comfort_stop");
}

export function comfortSet(parameters) {
  return invoke("comfort_set", { parameters });
}

export function receiverShortcutStart() {
  return invoke("receiver_shortcut_start");
}

export function receiverShortcutStop() {
  return invoke("receiver_shortcut_stop");
}

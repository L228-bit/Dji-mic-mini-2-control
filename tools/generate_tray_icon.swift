import AppKit

let output = CommandLine.arguments.dropFirst().first ?? "tray-microphone.png"
let canvasSize = NSSize(width: 64, height: 64)
let configuration = NSImage.SymbolConfiguration(pointSize: 46, weight: .regular)

guard let symbol = NSImage(
    systemSymbolName: "microphone.and.signal.meter",
    accessibilityDescription: "Microphone"
)?.withSymbolConfiguration(configuration) else {
    fatalError("microphone.and.signal.meter is unavailable on this macOS version")
}

let canvas = NSImage(size: canvasSize)
canvas.lockFocus()
NSColor.clear.setFill()
NSRect(origin: .zero, size: canvasSize).fill()

let symbolSize = symbol.size
let scale = min(54 / symbolSize.width, 54 / symbolSize.height)
let targetSize = NSSize(width: symbolSize.width * scale, height: symbolSize.height * scale)
let target = NSRect(
    x: (canvasSize.width - targetSize.width) / 2,
    y: (canvasSize.height - targetSize.height) / 2,
    width: targetSize.width,
    height: targetSize.height
)
symbol.draw(in: target)
canvas.unlockFocus()

guard let tiff = canvas.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: tiff),
      let png = bitmap.representation(using: .png, properties: [:]) else {
    fatalError("Unable to render tray icon")
}

try png.write(to: URL(fileURLWithPath: output))

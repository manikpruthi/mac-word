import AppKit
import Foundation

let sizes = [16, 32, 64, 128, 256, 512, 1024]
let outDir = CommandLine.arguments[1]
let iconset = outDir + "/mac-word.iconset"
try? FileManager.default.createDirectory(atPath: iconset, withIntermediateDirectories: true)

func drawIcon(_ size: Int) -> Data? {
    guard let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
                                     bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
                                     isPlanar: false, colorSpaceName: .deviceRGB,
                                     bytesPerRow: 0, bitsPerPixel: 0) else { return nil }
    rep.size = NSSize(width: size, height: size)
    NSGraphicsContext.saveGraphicsState()
    guard let ctx = NSGraphicsContext(bitmapImageRep: rep) else { return nil }
    NSGraphicsContext.current = ctx
    let s = CGFloat(size)
    let full = NSRect(x: 0, y: 0, width: s, height: s)

    // Rounded app tile with a deep blue gradient.
    let tile = full.insetBy(dx: s * 0.055, dy: s * 0.055)
    let radius = s * 0.215
    let tilePath = NSBezierPath(roundedRect: tile, xRadius: radius, yRadius: radius)
    let gradient = NSGradient(colors: [
        NSColor(calibratedRed: 0.10, green: 0.26, blue: 0.72, alpha: 1),
        NSColor(calibratedRed: 0.04, green: 0.10, blue: 0.38, alpha: 1),
    ])!
    gradient.draw(in: tilePath, angle: -90)

    // A soft highlight across the top of the tile.
    NSGraphicsContext.saveGraphicsState()
    tilePath.addClip()
    NSGradient(colors: [
        NSColor(calibratedWhite: 1, alpha: 0.24),
        NSColor(calibratedWhite: 1, alpha: 0.0),
    ])!.draw(in: NSRect(x: tile.minX, y: tile.midY, width: tile.width, height: tile.height / 2), angle: -90)
    NSGraphicsContext.restoreGraphicsState()

    // The page.
    let page = NSRect(x: s * 0.255, y: s * 0.165, width: s * 0.49, height: s * 0.66)
    let pagePath = NSBezierPath(roundedRect: page, xRadius: s * 0.028, yRadius: s * 0.028)
    NSGraphicsContext.saveGraphicsState()
    let shadow = NSShadow()
    shadow.shadowColor = NSColor(calibratedWhite: 0, alpha: 0.35)
    shadow.shadowBlurRadius = s * 0.045
    shadow.shadowOffset = NSSize(width: 0, height: -s * 0.012)
    shadow.set()
    NSColor(calibratedRed: 0.985, green: 0.99, blue: 1.0, alpha: 1).setFill()
    pagePath.fill()
    NSGraphicsContext.restoreGraphicsState()

    // A blue header band and a folded corner.
    NSGraphicsContext.saveGraphicsState()
    pagePath.addClip()
    NSColor(calibratedRed: 0.14, green: 0.42, blue: 0.94, alpha: 1).setFill()
    NSRect(x: page.minX, y: page.maxY - s * 0.075, width: page.width, height: s * 0.075).fill()
    NSGraphicsContext.restoreGraphicsState()

    let fold = s * 0.16
    let foldPath = NSBezierPath()
    foldPath.move(to: NSPoint(x: page.maxX - fold, y: page.maxY))
    foldPath.line(to: NSPoint(x: page.maxX, y: page.maxY))
    foldPath.line(to: NSPoint(x: page.maxX, y: page.maxY - fold))
    foldPath.close()
    NSColor(calibratedRed: 0.06, green: 0.18, blue: 0.50, alpha: 1).setFill()
    foldPath.fill()

    // The letter W, drawn as three clean strokes.
    let ink = NSColor(calibratedRed: 0.06, green: 0.14, blue: 0.38, alpha: 1)
    ink.setStroke()
    let w = NSBezierPath()
    w.lineWidth = max(1, s * 0.055)
    w.lineCapStyle = .round
    w.lineJoinStyle = .round
    let left = page.minX + page.width * 0.19
    let right = page.maxX - page.width * 0.19
    let top = page.minY + page.height * 0.60
    let bottom = page.minY + page.height * 0.25
    let mid = (left + right) / 2
    w.move(to: NSPoint(x: left, y: top))
    w.line(to: NSPoint(x: left + (right - left) * 0.18, y: bottom))
    w.line(to: NSPoint(x: mid, y: bottom + (top - bottom) * 0.36))
    w.line(to: NSPoint(x: right - (right - left) * 0.18, y: bottom))
    w.line(to: NSPoint(x: right, y: top))
    w.stroke()

    // Two short text rules under the W.
    let line = NSBezierPath()
    line.lineWidth = max(1, s * 0.018)
    line.lineCapStyle = .round
    NSColor(calibratedRed: 0.45, green: 0.52, blue: 0.68, alpha: 1).setStroke()
    for (i, widthFactor) in [0.62, 0.44].enumerated() {
        let y = page.minY + page.height * (0.15 - CGFloat(i) * 0.07)
        line.move(to: NSPoint(x: left, y: y))
        line.line(to: NSPoint(x: left + (right - left) * widthFactor, y: y))
    }
    line.stroke()

    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])
}

func write(_ data: Data, _ name: String) {
    try? data.write(to: URL(fileURLWithPath: iconset + "/" + name))
}

for size in sizes {
    guard let data = drawIcon(size) else { continue }
    if size == 1024 { write(data, "icon_512x512@2x.png") }
    if size == 512 {
        write(data, "icon_512x512.png")
        try? data.write(to: URL(fileURLWithPath: outDir + "/mac-word-macos-512.png"))
    }
    if size == 256 { write(data, "icon_128x128@2x.png") }
    if size == 128 { write(data, "icon_128x128.png") }
    if size == 64 { write(data, "icon_32x32@2x.png") }
    if size == 32 { write(data, "icon_32x32.png") }
    if size == 16 { write(data, "icon_16x16.png") }
    if size == 256 { try? data.write(to: URL(fileURLWithPath: outDir + "/mac-word-256.png")) }
}

import AppKit
import Foundation

let projectRoot = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let markURL = projectRoot.appendingPathComponent("static/brand/ninja-launcher-mark-source.svg")
let outputDirectory = projectRoot.appendingPathComponent("src-tauri/icons")
let macOSDefaultIconURL = outputDirectory
    .appendingPathComponent("macos")
    .appendingPathComponent("appearances")
    .appendingPathComponent("default.png")
let windowsOriginalIconURL = outputDirectory
    .appendingPathComponent("windows")
    .appendingPathComponent("source.png")

guard let mark = NSImage(contentsOf: markURL) else {
    fatalError("Could not load Ninja Launcher mark at \(markURL.path)")
}

func bitmap(size: Int) -> NSBitmapImageRep {
    guard let result = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: size,
        pixelsHigh: size,
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    ) else {
        fatalError("Could not create icon bitmap")
    }
    return result
}

func drawIcon(size: Int = 1024, drawing: () -> Void) -> NSBitmapImageRep {
    let result = bitmap(size: size)
    guard let context = NSGraphicsContext(bitmapImageRep: result) else {
        fatalError("Could not create icon graphics context")
    }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    context.imageInterpolation = .high
    context.cgContext.clear(CGRect(x: 0, y: 0, width: size, height: size))
    drawing()
    context.flushGraphics()
    NSGraphicsContext.restoreGraphicsState()
    return result
}

func drawMark(in rect: NSRect, colors: [NSColor]? = nil) {
    guard let colors else {
        mark.draw(
            in: rect,
            from: .zero,
            operation: .sourceOver,
            fraction: 1,
            respectFlipped: true,
            hints: [.interpolation: NSImageInterpolation.high]
        )
        return
    }

    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current?.cgContext.beginTransparencyLayer(auxiliaryInfo: nil)
    mark.draw(
        in: rect,
        from: .zero,
        operation: .sourceOver,
        fraction: 1,
        respectFlipped: true,
        hints: [.interpolation: NSImageInterpolation.high]
    )
    NSGraphicsContext.current?.cgContext.setBlendMode(.sourceIn)
    NSGradient(colors: colors)!.draw(in: rect, angle: -90)
    NSGraphicsContext.current?.cgContext.endTransparencyLayer()
    NSGraphicsContext.restoreGraphicsState()
}

func write(_ image: NSBitmapImageRep, to url: URL) {
    guard let data = image.representation(using: .png, properties: [:]) else {
        fatalError("Could not encode \(url.lastPathComponent)")
    }
    try! data.write(to: url, options: .atomic)
}

func write(_ image: NSBitmapImageRep, named filename: String) {
    write(image, to: outputDirectory.appendingPathComponent(filename))
}

func drawPlatformTile(
    tile: NSRect,
    radius: CGFloat,
    markRect: NSRect,
    background: [NSColor] = [
        NSColor(red: 0.025, green: 0.12, blue: 0.29, alpha: 1),
        NSColor(red: 0.006, green: 0.025, blue: 0.075, alpha: 1),
    ],
    markColors: [NSColor] = [
        NSColor(calibratedWhite: 1, alpha: 1),
        NSColor(calibratedWhite: 0.88, alpha: 1),
    ],
    border: NSColor = NSColor(red: 0.11, green: 0.73, blue: 0.94, alpha: 0.28)
) {
    let shape = NSBezierPath(roundedRect: tile, xRadius: radius, yRadius: radius)

    NSGraphicsContext.saveGraphicsState()
    let shadow = NSShadow()
    shadow.shadowColor = NSColor(calibratedWhite: 0, alpha: 0.52)
    shadow.shadowBlurRadius = 34
    shadow.shadowOffset = NSSize(width: 0, height: -16)
    shadow.set()
    NSColor(red: 0.012, green: 0.045, blue: 0.12, alpha: 1).setFill()
    shape.fill()
    NSGraphicsContext.restoreGraphicsState()

    NSGraphicsContext.saveGraphicsState()
    shape.addClip()
    let gradient = NSGradient(colors: background)!
    gradient.draw(in: tile, angle: -90)
    drawMark(in: markRect, colors: markColors)
    NSGraphicsContext.restoreGraphicsState()

    border.setStroke()
    shape.lineWidth = 4
    shape.stroke()
}

let requestedPlatform = CommandLine.arguments.dropFirst().first
if requestedPlatform != "--windows-only" {
    guard FileManager.default.fileExists(atPath: macOSDefaultIconURL.path) else {
        fatalError("Missing exported macOS default icon at \(macOSDefaultIconURL.path)")
    }
    let originalDefaultIcon = try Data(contentsOf: macOSDefaultIconURL)
    try originalDefaultIcon.write(
        to: outputDirectory.appendingPathComponent("macos-icon-source.png"),
        options: .atomic
    )
}
if requestedPlatform != "--macos-only" {
    guard FileManager.default.fileExists(atPath: windowsOriginalIconURL.path) else {
        fatalError("Missing original Windows icon at \(windowsOriginalIconURL.path)")
    }
    guard let originalWindowsIcon = NSImage(contentsOf: windowsOriginalIconURL) else {
        fatalError("Could not load original Windows icon at \(windowsOriginalIconURL.path)")
    }
    let windowsIcon = drawIcon {
        let canvas = NSRect(x: 0, y: 0, width: 1024, height: 1024)
        NSGraphicsContext.saveGraphicsState()
        NSBezierPath(roundedRect: canvas, xRadius: 286, yRadius: 286).addClip()
        originalWindowsIcon.draw(
            in: canvas,
            from: .zero,
            operation: .sourceOver,
            fraction: 1,
            respectFlipped: true,
            hints: [.interpolation: NSImageInterpolation.high]
        )
        NSGraphicsContext.restoreGraphicsState()
    }
    write(windowsIcon, named: "windows-icon-source.png")
}

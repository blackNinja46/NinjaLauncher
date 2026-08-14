import AppKit
import Foundation

let projectRoot = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let markURL = projectRoot.appendingPathComponent("static/brand/ninja-launcher-mark.png")
let outputDirectory = projectRoot.appendingPathComponent("src-tauri/icons")

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

func drawMark(in rect: NSRect) {
    mark.draw(
        in: rect,
        from: .zero,
        operation: .sourceOver,
        fraction: 1,
        respectFlipped: true,
        hints: [.interpolation: NSImageInterpolation.high]
    )
}

func write(_ image: NSBitmapImageRep, named filename: String) {
    guard let data = image.representation(using: .png, properties: [:]) else {
        fatalError("Could not encode \(filename)")
    }
    try! data.write(to: outputDirectory.appendingPathComponent(filename), options: .atomic)
}

let macIcon = drawIcon {
    let tile = NSRect(x: 78, y: 78, width: 868, height: 868)
    let shape = NSBezierPath(roundedRect: tile, xRadius: 190, yRadius: 190)

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
    let gradient = NSGradient(colors: [
        NSColor(red: 0.025, green: 0.12, blue: 0.29, alpha: 1),
        NSColor(red: 0.006, green: 0.025, blue: 0.075, alpha: 1),
    ])!
    gradient.draw(in: tile, angle: -90)
    drawMark(in: NSRect(x: 168, y: 246, width: 688, height: 518))
    NSGraphicsContext.restoreGraphicsState()

    NSColor(red: 0.11, green: 0.73, blue: 0.94, alpha: 0.28).setStroke()
    shape.lineWidth = 4
    shape.stroke()
}

let windowsIcon = drawIcon {
    drawMark(in: NSRect(x: 62, y: 173, width: 900, height: 678))
}

write(macIcon, named: "macos-icon-source.png")
write(windowsIcon, named: "windows-icon-source.png")

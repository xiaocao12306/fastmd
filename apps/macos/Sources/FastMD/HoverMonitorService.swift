import AppKit

@MainActor
final class HoverMonitorService {
    var onHoverPause: ((NSPoint) -> Void)?
    var onHoverWarmup: ((NSPoint) -> Void)?
    var onMouseActivity: (() -> Void)?

    private var globalMonitor: Any?
    private var localMonitor: Any?
    private var hoverWarmupWorkItem: DispatchWorkItem?
    private var hoverWorkItem: DispatchWorkItem?
    private let hoverDelay: TimeInterval
    private let hoverWarmupDelay: TimeInterval

    init(hoverDelay: TimeInterval = 1.0, hoverWarmupDelay: TimeInterval = 0.7) {
        self.hoverDelay = hoverDelay
        self.hoverWarmupDelay = min(max(0.0, hoverWarmupDelay), hoverDelay)
    }

    func start() {
        stop()

        let mask: NSEvent.EventTypeMask = [.mouseMoved, .leftMouseDragged, .rightMouseDragged, .scrollWheel]
        globalMonitor = NSEvent.addGlobalMonitorForEvents(matching: mask) { [weak self] _ in
            Task { @MainActor in
                self?.handleMouseActivity()
            }
        }
        localMonitor = NSEvent.addLocalMonitorForEvents(matching: mask) { [weak self] event in
            Task { @MainActor in
                self?.handleMouseActivity()
            }
            return event
        }

        handleMouseActivity()
    }

    func stop() {
        if let globalMonitor {
            NSEvent.removeMonitor(globalMonitor)
            self.globalMonitor = nil
        }
        if let localMonitor {
            NSEvent.removeMonitor(localMonitor)
            self.localMonitor = nil
        }
        hoverWarmupWorkItem?.cancel()
        hoverWarmupWorkItem = nil
        hoverWorkItem?.cancel()
        hoverWorkItem = nil
    }

    private func handleMouseActivity() {
        onMouseActivity?()
        hoverWarmupWorkItem?.cancel()
        hoverWorkItem?.cancel()

        let warmupWork = DispatchWorkItem { [weak self] in
            guard let self else { return }
            self.onHoverWarmup?(NSEvent.mouseLocation)
        }
        hoverWarmupWorkItem = warmupWork
        DispatchQueue.main.asyncAfter(deadline: .now() + hoverWarmupDelay, execute: warmupWork)

        let work = DispatchWorkItem { [weak self] in
            guard let self else { return }
            self.onHoverPause?(NSEvent.mouseLocation)
        }
        hoverWorkItem = work
        DispatchQueue.main.asyncAfter(deadline: .now() + hoverDelay, execute: work)
    }
}

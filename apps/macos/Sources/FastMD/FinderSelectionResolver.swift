import AppKit
@preconcurrency import ApplicationServices
import Foundation

/// Outcome of resolving Finder's current selection for Space-trigger preview.
///
/// The CGEventTap callback reads this via `FinderSelectionSnapshot` from a
/// non-main thread, so the enum (and any value it carries) must be Sendable.
/// The three states map directly to tap policy:
///
/// - `.unknown` — fail-open. Pass Space through to Quick Look. We have not
///   resolved selection yet, or we have reason to believe the cache is stale.
/// - `.nonMarkdown` — fail-open. Pass Space through to Quick Look.
/// - `.markdown(url:)` — fail-closed. Swallow Space and dispatch a toggle to
///   the coordinator on the main actor.
enum FinderSelectionState: Sendable, Equatable {
    case unknown
    case nonMarkdown
    case markdown(url: URL)
}

/// Lock-free-to-read snapshot of what the resolver currently knows about
/// Finder and user intent. Held in `SelectionSnapshotHolder` and refreshed
/// only on the main actor; readable from any thread via the holder.
///
/// Every store bumps `generation`, so consumers can distinguish "same
/// selection seen twice" from "a newer decision landed".
struct FinderSelectionSnapshot: Sendable {
    let state: FinderSelectionState
    let finderPid: pid_t
    /// True when Finder's focused AX element is a text input — rename field,
    /// search field, path bar editor, etc. Even with a markdown file selected,
    /// Space must pass through when this is true so the user can type in the
    /// text field. Always false until AX observer integration lands.
    let isFinderEditingText: Bool
    /// Mirrors `PreferencesStore.spaceTriggerEnabled`. The tap checks this
    /// synchronously instead of reaching across threads into the store.
    let spaceTriggerEnabled: Bool
    /// Monotonic counter across all store operations.
    let generation: UInt64

    static let empty = FinderSelectionSnapshot(
        state: .unknown,
        finderPid: 0,
        isFinderEditingText: false,
        spaceTriggerEnabled: false,
        generation: 0
    )
}

/// Thread-safe holder for the latest `FinderSelectionSnapshot`.
///
/// Read-mostly usage from the CGEventTap thread, written from main. Uses
/// `NSLock` rather than `OSAllocatedUnfairLock` to avoid pulling in the
/// `os` allocated-lock API surface here; the struct is tiny and the critical
/// sections copy a 48-byte value, so contention is a non-issue.
final class SelectionSnapshotHolder: @unchecked Sendable {
    private let lock = NSLock()
    private var value: FinderSelectionSnapshot = .empty

    var current: FinderSelectionSnapshot {
        lock.lock()
        defer { lock.unlock() }
        return value
    }

    func store(_ new: FinderSelectionSnapshot) {
        lock.lock()
        defer { lock.unlock() }
        value = new
    }
}

/// Owns the Finder selection cache that the Space tap reads synchronously.
///
/// First increment: AppleScript-based selection query, debounced refreshes
/// on NSWorkspace activation / launch / termination notifications. No AX
/// observer, no focused-element text-entry detection, no anchor geometry —
/// those land in follow-up increments.
@MainActor
final class FinderSelectionResolver {
    let snapshotHolder = SelectionSnapshotHolder()

    private var generationCounter: UInt64 = 0
    private var finderPid: pid_t = 0
    private var spaceTriggerEnabled: Bool = true
    private var pendingRefreshWorkItem: DispatchWorkItem?
    private let refreshDebounce: TimeInterval = 0.05

    /// First .md in the selection, or the first item overall if none of the
    /// selection is Markdown. Returning a non-Markdown path lets the resolver
    /// still mark the state `.nonMarkdown` (so the tap passes Space through)
    /// without a second AppleScript round-trip.
    private let selectionScript: NSAppleScript? = NSAppleScript(source: """
    tell application "Finder"
        if (count of Finder windows) is 0 then return ""
        try
            set theSelection to selection
        on error
            return ""
        end try
        if (count of theSelection) is 0 then return ""
        repeat with theItem in theSelection
            try
                set itemPath to POSIX path of (theItem as alias)
                if itemPath ends with ".md" or itemPath ends with ".MD" then
                    return itemPath
                end if
            end try
        end repeat
        try
            return POSIX path of ((item 1 of theSelection) as alias)
        on error
            return ""
        end try
    end tell
    """)

    init() {
        installWorkspaceObservers()
        refreshFinderPid()
        scheduleRefresh(reason: "initial")
    }

    deinit {
        NSWorkspace.shared.notificationCenter.removeObserver(self)
    }

    func setSpaceTriggerEnabled(_ enabled: Bool) {
        guard spaceTriggerEnabled != enabled else { return }
        spaceTriggerEnabled = enabled
        scheduleRefresh(reason: "spaceTriggerEnabled=\(enabled)")
    }

    /// Force a refresh on demand. Used by the coordinator right before it
    /// asks the resolver anything at startup, and by tests.
    func refreshNow(reason: String) {
        performRefresh(reason: reason)
    }

    // MARK: - Workspace observers

    private func installWorkspaceObservers() {
        let nc = NSWorkspace.shared.notificationCenter
        nc.addObserver(
            self,
            selector: #selector(frontAppChanged),
            name: NSWorkspace.didActivateApplicationNotification,
            object: nil
        )
        nc.addObserver(
            self,
            selector: #selector(appLaunched(_:)),
            name: NSWorkspace.didLaunchApplicationNotification,
            object: nil
        )
        nc.addObserver(
            self,
            selector: #selector(appTerminated(_:)),
            name: NSWorkspace.didTerminateApplicationNotification,
            object: nil
        )
    }

    @objc
    private func frontAppChanged() {
        scheduleRefresh(reason: "frontmost app changed")
    }

    @objc
    private func appLaunched(_ note: Notification) {
        guard let app = note.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
              app.bundleIdentifier == "com.apple.finder"
        else { return }
        finderPid = app.processIdentifier
        RuntimeLogger.log("FinderSelectionResolver: Finder launched pid=\(finderPid)")
        scheduleRefresh(reason: "Finder launched")
    }

    @objc
    private func appTerminated(_ note: Notification) {
        guard let app = note.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
              app.bundleIdentifier == "com.apple.finder"
        else { return }
        RuntimeLogger.log("FinderSelectionResolver: Finder terminated pid=\(finderPid)")
        finderPid = 0
        storeSnapshot(state: .unknown, isEditingText: false, reason: "Finder terminated")
    }

    private func refreshFinderPid() {
        let runningFinder = NSRunningApplication
            .runningApplications(withBundleIdentifier: "com.apple.finder")
            .first
        finderPid = runningFinder?.processIdentifier ?? 0
    }

    // MARK: - Refresh pipeline

    private func scheduleRefresh(reason: String) {
        pendingRefreshWorkItem?.cancel()
        let work = DispatchWorkItem { [weak self] in
            guard let self else { return }
            self.performRefresh(reason: reason)
        }
        pendingRefreshWorkItem = work
        DispatchQueue.main.asyncAfter(deadline: .now() + refreshDebounce, execute: work)
    }

    private func performRefresh(reason: String) {
        if finderPid == 0 {
            refreshFinderPid()
        }

        guard finderPid != 0 else {
            storeSnapshot(state: .unknown, isEditingText: false, reason: "\(reason) (Finder not running)")
            return
        }

        let path = queryFinderSelectionPath()
        let state = classify(rawPath: path)
        storeSnapshot(state: state, isEditingText: false, reason: reason)
    }

    private func queryFinderSelectionPath() -> String? {
        guard let script = selectionScript else {
            RuntimeLogger.log("FinderSelectionResolver: selection AppleScript failed to compile.")
            return nil
        }
        var error: NSDictionary?
        let value = script.executeAndReturnError(&error)
        if let error {
            RuntimeLogger.log("FinderSelectionResolver: selection AppleScript error: \(error)")
            return nil
        }
        let path = value.stringValue?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return path.isEmpty ? nil : path
    }

    private func classify(rawPath: String?) -> FinderSelectionState {
        guard let rawPath else { return .nonMarkdown }
        let url = URL(fileURLWithPath: rawPath).standardizedFileURL
        guard url.pathExtension.lowercased() == "md" else {
            return .nonMarkdown
        }
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory),
              !isDirectory.boolValue
        else {
            return .nonMarkdown
        }
        return .markdown(url: url)
    }

    private func storeSnapshot(state: FinderSelectionState, isEditingText: Bool, reason: String) {
        generationCounter &+= 1
        let snapshot = FinderSelectionSnapshot(
            state: state,
            finderPid: finderPid,
            isFinderEditingText: isEditingText,
            spaceTriggerEnabled: spaceTriggerEnabled,
            generation: generationCounter
        )
        snapshotHolder.store(snapshot)

        let stateText: String
        switch state {
        case .unknown: stateText = "unknown"
        case .nonMarkdown: stateText = "nonMarkdown"
        case .markdown(let url): stateText = "markdown(\(url.lastPathComponent))"
        }
        RuntimeLogger.log(
            "FinderSelectionResolver: gen=\(generationCounter) state=\(stateText) editingText=\(isEditingText) pid=\(finderPid) reason=\"\(reason)\""
        )
    }
}

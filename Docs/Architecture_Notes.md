# FastMD Architecture Notes

These notes describe the actual prototype architecture currently implemented in the repository. They are intentionally narrow and should be updated as the Finder resolver and release workflow mature.

## Layer 1 Runtime Shape

FastMD is a macOS accessory app with a menu bar status item. The runtime is built around a single coordinator that listens for hover pauses, resolves the hovered Finder item into a Markdown file path, and displays a floating preview panel.

## Current Execution Flow

1. `FastMDApp` boots the app and hands control to `AppDelegate`.
2. `AppDelegate` sets the activation policy to `.accessory`, creates the menu bar item, and starts the `FinderHoverCoordinator`.
3. `FinderHoverCoordinator.start()` requests Accessibility trust before starting mouse monitoring.
4. `HoverMonitorService` installs global and local monitors for:
   - `mouseMoved`
   - `leftMouseDragged`
   - `rightMouseDragged`
   - `scrollWheel`
5. Every mouse activity event hides the current preview and resets a 1-second debounce timer.
6. When the timer fires, the coordinator asks `FinderItemResolver` to resolve the hovered item at the current screen coordinate.
7. If resolution succeeds and the item differs from the current one, `PreviewPanelController` reads the file, renders HTML with `MarkdownRenderer`, and shows a floating `NSPanel` near the cursor.
8. If resolution fails, file loading fails, Finder loses focus, or monitoring stops, the preview is hidden.

## Finder Resolution Strategy

The current Finder list-view path resolution is intentionally simple:

1. Confirm Finder is the frontmost app.
2. Perform an accessibility hit-test using `AXUIElementCopyElementAtPosition`.
3. Attempt direct path extraction from `AXFilename`, `AXPath`, or `AXDocument`.
4. If no direct path is available, walk up the parent chain for up to 10 elements.
5. Take the first non-empty value from `AXTitle`, `AXValue`, or `AXDescription`.
6. Ask Finder, via AppleScript, for the target directory of the front Finder window.
7. Join `front window target directory + hovered item title`.
8. For the fallback branch only, require `FileManager.default.fileExists(atPath:)`.
9. Return the result only when the resolved extension lowercases to `md`.

This strategy is enough for a first prototype, but it is not yet a dedicated Finder list-row implementation. It is still a heuristic parent-walk that assumes Finder exposes a usable row title and that the front Finder window target directory matches the hovered list item.

## Preview and Rendering Path

- `PreviewPanelController` owns a floating `NSPanel` and a `WKWebView`.
- File loading currently happens inline in the preview layer rather than behind a dedicated loader abstraction.
- File reads currently assume UTF-8.
- `MarkdownRenderer` is a lightweight line-oriented renderer that supports:
  - headings
  - paragraphs
  - inline code
  - fenced code blocks
  - blockquotes
  - unordered lists
  - emphasis / strong
  - links

The renderer is deliberately small and readable, but it is not yet a full Markdown implementation.

## Current Safety Boundaries

- Monitoring does not start without Accessibility trust.
- Preview display is suppressed when Finder is not frontmost.
- Preview display fails closed when path resolution or UTF-8 file loading fails.
- Re-hovering the same resolved item does not repeatedly reopen the panel while the cursor is stationary.

## Known Gaps

- No Escape-key dismissal exists yet.
- No explicit Finder list-row role map or AX fixture capture exists yet.
- The direct-path resolution branch does not currently verify path existence before returning.
- The fallback branch checks only path existence, not whether the target is a readable regular file.
- No support exists yet for Finder icon view, column view, gallery view, or Desktop hover.
- No non-UTF-8 decoding fallback or file-size guardrail exists yet.
- The repository has only minimal automated test coverage for the renderer and no committed Finder integration evidence.

## Practical Implication

The current architecture is suitable for proving the closed-loop concept on one machine, but it is not yet release-hard. Future Layer 1 work should focus on turning the Finder resolver from a permissive heuristic into a role-aware, fixture-backed implementation with manual test evidence and explicit failure logging.

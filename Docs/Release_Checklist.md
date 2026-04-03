# FastMD Release Checklist

This checklist records the release bar for the current Layer 1 prototype. It is intentionally conservative and should only be checked off with real validation evidence.

## Current Release Decision

Layer 1 is not release-ready yet. The repository builds and tests, but Finder validation evidence, fixture capture, and several blueprint-required behaviors are still incomplete.

## Build and Test Gate

- [x] Run `swift build` successfully from the repository root on 2026-04-03
- [x] Run `swift test` successfully from the repository root on 2026-04-03
- [ ] Re-run both commands after the final Layer 1 code change

## Environment Gate

- [ ] Confirm the app launches as a menu bar accessory app
- [ ] Confirm Accessibility permission is granted for the built app
- [ ] Confirm Finder is frontmost and using the intended list-view path during manual smoke tests

## Manual Smoke Gate

- [ ] Hover a visible local UTF-8 `.md` file in Finder list view for 1 second and confirm a preview appears
- [ ] Hover a non-Markdown file and confirm no preview appears
- [ ] Move the cursor away and confirm the preview closes
- [ ] Switch to another app and confirm the preview closes
- [ ] Toggle monitoring from the menu and confirm the menu title updates correctly
- [ ] Record the result of each smoke test in project documentation

## Finder Resolver Confidence Gate

- [ ] Capture real Finder AX snapshots for the supported list-view case
- [ ] Verify resolution on file names with spaces
- [ ] Verify resolution on file names with Chinese characters
- [ ] Verify resolution after switching Finder windows or directories
- [ ] Update `Docs/Support_Matrix.md` with confirmed supported and unsupported cases
- [ ] Update `Docs/Architecture_Notes.md` if the implemented resolver strategy changes

## Packaging and Product Gate

- [ ] Add the remaining Layer 1 menu behavior required by the blueprint
- [ ] Add Escape-key dismissal
- [ ] Add a dedicated manual test plan and committed test logs
- [ ] Produce the intended installable `.app` packaging flow
- [ ] Update `README.md` to match the actual shipped behavior and exact local verification command

## Release Sign-off

- [ ] Final Layer 1 manual smoke run completed on the target machine
- [ ] Final Layer 1 build and test run completed without local workarounds
- [ ] Blueprint completion items updated only for work that is actually implemented and validated

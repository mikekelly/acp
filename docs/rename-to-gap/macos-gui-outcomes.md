# macOS GUI Rename Outcomes

## Objective
Rename the macOS GUI Xcode project from ACP to GAP.

## Success Criteria
1. All Swift source files with "ACP" in the name are renamed to "GAP" (using git mv)
2. All Swift code content updated (class names, struct names, string references)
3. Directory structure updated: `ACP/` → `GAP/`, `ACPTests/` → `GAPTests/`
4. Xcode project renamed: `ACP.xcodeproj` → `GAP.xcodeproj`
5. project.pbxproj file properly updated with all path and reference changes
6. Bundle identifier updated (e.g., com.*.acp → com.*.gap)
7. Info.plist files updated (CFBundleName, etc.)
8. No remaining "ACP" references in any macos-gui files (excluding .build artifacts)
9. All changes committed with appropriate message

## Out of Scope
- Verifying the Xcode build works (no Xcode available in this environment)
- Updating .build directory artifacts (these are generated)

## Why This Matters
Part of the broader ACP → GAP rename. The macOS GUI is user-facing and needs consistent naming.

## Files in Scope
Source directory: `/Users/mike/code/agent-credential-proxy/macos-gui/`

Key files identified:
- `ACP/` directory with Swift sources
- `ACPTests/` directory with test files
- `ACP.xcodeproj/` project bundle
- `ACP.xcodeproj/project.pbxproj` (critical - very sensitive to formatting)
- `ACP/Info.plist`

## Critical Ordering
Xcode projects are sensitive to the order of operations:
1. Rename Swift source files first (git mv)
2. Update Swift file contents
3. Rename source directories (git mv)
4. Rename Xcode project directory (git mv)
5. Update project.pbxproj (CAREFUL - sensitive to formatting)
6. Update Info.plist files
7. Final verification scan

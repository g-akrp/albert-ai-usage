# Windows tray icon

The executable application icon uses the exact PNG artwork from `Resources/AppIcon.icns`, repackaged into an ICO at build time. macOS and Windows share the dark rounded square, white pixel AI label, and colored usage bar. The dynamic tray icon remains separate.

The app renders a native pixel icon with a provider or pinned group label and headline percentage. Thresholds are green below 70%, orange from 70%, and red from 90%. Pin state is persisted; a legacy Copilot provider pin migrates to its first account. Successful grouped reports resolve a whole-provider pin to the first group. Loading and errors retain an existing valid pin.

Explorer restart restoration and mixed-DPI rendering are implemented but still need manual verification.

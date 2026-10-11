# Windows tray icon

The app renders a native pixel icon with a provider or pinned group label and headline percentage. Thresholds are green below 70%, orange from 70%, and red from 90%. Pin state is persisted; a legacy Copilot provider pin migrates to its first account. Successful grouped reports resolve a whole-provider pin to the first group. Loading and errors retain an existing valid pin.

Explorer restart restoration and mixed-DPI rendering are implemented but still need manual verification.

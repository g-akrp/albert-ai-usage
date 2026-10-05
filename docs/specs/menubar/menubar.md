# Menu bar

AI Usage is a menu bar app with no Dock icon. It has two parts:

- [Menu bar icon](menubar-icon.md): a two-row pixel label that shows one provider's usage at a glance.
- [Menu bar panel](menubar-panel.md): the menu that opens on click, with every provider and its limits.

Data comes from the providers described in [data source](../data-source/data-source.md).

The app is AppKit only, with no third-party packages. Its physical memory footprint is about 11 MB and must stay under 30 MB. The only timers are the 30 s schedule timer and the 5 s icon cycle.

Requires macOS 13 or later.

# VK-Swarm App — UI Kit

Interactive recreation of the **VK-Swarm** orchestration surface — a swarm-based
fork of vibe-kanban. Built entirely on the design-system bundle
(`window.VKSwarmDesignSystem_067861`) plus the shared component classes in
`styles.css`.

## Files
- `index.html` — entry point. Wires the navbar, board, nodes, processes and the
  task drawer into one click-through app.
- `chrome.jsx` — top navbar (wordmark, project switcher, search, new-task,
  view tabs) and the inline icon set.
- `board.jsx` — the five-column kanban board (To Do → In Progress → In Review →
  Done → Cancelled) rendering `TaskCard` from the bundle.
- `panels.jsx` — the **Hive / Nodes** grid, **Processes** list, and the
  **Task drawer** (Diff / Logs / Attempts tabs, merge/rebase actions).

## Interactions
- Click a card → opens the task drawer with diff, logs and attempt history.
- **+ Task** (navbar or column header) → inserts a card and opens its drawer.
- Tabs switch between Board, Nodes and Processes views.

## Fidelity notes
This recreates the *visual + interaction language* of the product, not its real
data layer. Drag-and-drop, live log streaming, and the hive WebSocket are
represented statically. The column model, status colors, card anatomy (left
status strip, node tag, attempt indicator, days-in-column badge) and the
Midnight Terminal palette all mirror the source repo.

Source: `frontend/src/components/{tasks,swarm,layout}` in
[davidrudduck/vk-swarm](https://github.com/davidrudduck/vk-swarm).

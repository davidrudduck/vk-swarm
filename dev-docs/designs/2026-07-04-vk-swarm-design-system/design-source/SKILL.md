---
name: vk-swarm-design
description: Use this skill to generate well-branded interfaces and assets for VK-Swarm (a kanban-based code executor orchestration system, "Midnight Terminal" theme), either for production or throwaway prototypes/mocks/etc. Contains essential design guidelines, colors, type, fonts, assets, and UI kit components for prototyping.
user-invocable: true
---

Read the README.md file within this skill, and explore the other available files.
If creating visual artifacts (slides, mocks, throwaway prototypes, etc), copy assets out and create static HTML files for the user to view. If working on production code, you can copy assets and read the rules here to become an expert in designing with this brand.
If the user invokes this skill without any other guidance, ask them what they want to build or design, ask some questions, and act as an expert designer who outputs HTML artifacts _or_ production code, depending on the need.

Quick orientation:
- `styles.css` is the single stylesheet to link — it pulls in all tokens, base styles and component classes. Theme is dark-first ("Midnight Terminal"): near-black void background, cyan `#00d4ff` primary accent, dense terminal-flavored UI.
- `tokens/` holds the CSS custom properties (colors, typography downshifted to 14px base, spacing on a 4px grid, radius/borders, component classes).
- `components/` holds React primitives (`Button`, `Badge`, `Card`, `Input`, `Switch`, `Checkbox`, `Tabs`, `Select`, `Loader`, `StatusBadge`, `TaskCard`, `NodeCard`) — each with a `.d.ts` contract and `.prompt.md` usage note. They are bundled to `window.VKSwarmDesignSystem_067861` (load `_ds_bundle.js` after React).
- `ui_kits/vk-swarm-app/` is a full interactive recreation of the product (kanban board, hive/nodes, processes, task drawer) — the best reference for composing screens.
- `guidelines/*.html` are visual specimen cards for colors, type, spacing and brand.
- `assets/` holds the wordmark favicons and IDE brand icons. Icons in the product are lucide-react (stroke ~1.6px) — link lucide from CDN for coverage.

Fonts: Inter (UI/prose), JetBrains Mono (code/terminal), Source Serif 4 (display headings), Chivo Mono (wordmark — "VK" cyan + "-SWARM" foreground). Loaded via Google Fonts.

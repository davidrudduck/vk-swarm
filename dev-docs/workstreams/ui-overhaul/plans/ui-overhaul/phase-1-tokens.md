# Phase 1 — Token foundation

Lands all design tokens in `frontend/src/styles/index.css` + `frontend/tailwind.config.js`, and
(scope C / D10) makes the brand palette render by promoting `--vks-*` to `:root` and merging the
remap into `.dark`. Everything downstream references these tokens.

Tasks (sequential where same-file):
- 001 — brand cascade live (`:root` primitives, `.dark` remap, `.light` block) → SC22
- 002 — `--status-*` + semantic aliases + `--strip-width` → SC6, SC7 (dep 001)
- 003 — shadow/glow + `vks-pulse` keyframe + ANSI texture classes → SC15 (dep 002)
- 004 — `wordmark` fontFamily key (tailwind.config.js)

Shippable boundary: after Phase 1, the dark theme renders cyan/void and all tokens resolve, but
components still reference old hardcoded colours (corrected in Phase 2).

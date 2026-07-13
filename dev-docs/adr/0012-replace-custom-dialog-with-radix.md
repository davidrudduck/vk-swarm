# ADR-0012 — Replace custom dialog.tsx with @radix-ui/react-dialog

- **Status:** accepted
- **Date:** 2026-07-10
- **Workstream:** error-handling-and-dialog-a11y
- **Supersedes behaviour of:** custom `dialog.tsx` plain-HTML implementation

## Context

`remote-frontend/src/components/ui/dialog.tsx` is a 116-line custom implementation using plain
`<div>` elements for the overlay, content, and close button. It has no ARIA attributes, no focus
trapping, no Escape-to-close handling, and no portal rendering. The `uncloseable` prop is
implemented by conditionally hiding the close button and preventing overlay click — but provides
no accessible signal that the dialog is intentionally uncloseable.

`@radix-ui/react-dialog` is already installed (`^1.1.18` in `package.json`) but is not imported
anywhere in the codebase. The component is used by 9 files across the swarm dialog tree.

Every tournament review round (22 rounds for hive-node-api-key-ui) and code review flagged the
missing a11y. Screen reader users cannot detect the dialog. Keyboard users cannot Escape out of
it. Focus is not trapped within the dialog.

## Decision

Replace the custom `dialog.tsx` implementation entirely with `@radix-ui/react-dialog` primitives.
The custom implementation is deleted. All 9 callers are updated to the same API surface.

The `uncloseable` prop is preserved as a first-class variant that uses Radix's
`onEscapeKeyDown` and `onPointerDownOutside` event prevention (`e.preventDefault()`) to suppress
close behavior while maintaining focus trap and `aria-modal`.

## Consequences

**Positive:**
- `role="dialog"` and `aria-modal="true"` added for free
- Focus trapping (Tab cycles within dialog)
- Escape-to-close (blocked when `uncloseable`)
- Proper portal rendering
- Animation support via `data-state` attributes
- Industry-standard a11y behavior maintained by the Radix team

**Negative:**
- The `Dialog` component API changes from `React.forwardRef<HTMLDivElement>` to Radix's
  `DialogPrimitive.Root`. Callers using `ref` on `Dialog` may need adjustment (none currently do).
- `DialogContent`'s `onOpenChange` behavior is now controlled by Radix rather than custom logic.
  The `uncloseable` variant must be tested to ensure it doesn't regress the secret-reveal flow.

**Neutral:**
- `DialogHeader`, `DialogTitle`, `DialogDescription`, `DialogFooter` remain plain `<div>` / `<h3>`
  / `<p>` elements — unchanged.
- The 9 caller files need import path changes only (same exports, same names).

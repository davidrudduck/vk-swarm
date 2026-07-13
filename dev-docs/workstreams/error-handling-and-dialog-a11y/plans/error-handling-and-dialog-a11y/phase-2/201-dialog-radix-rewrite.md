---
id: "201"
phase: 2
title: "Rewrite dialog.tsx to use @radix-ui/react-dialog"
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/ui/dialog.tsx
irreversible: true
scope_test: "remote-frontend/src/components/ui/dialog.test.tsx"
allowed_change: edit
covers_criteria: [SC3]
---
## Failing test (write first)
Covered by: `remote-frontend/src/components/ui/dialog.test.tsx` (task 203). The 7 tests in that
file verify role="dialog", aria-modal, Escape handling, and uncloseable behavior. Task 201
rewrites the component; task 203 creates the tests.

## Change
- **File:** remote-frontend/src/components/ui/dialog.tsx
- **Anchor:** entire file (116 lines of custom HTML → Radix wrapper)
- **Before:** Custom implementation using plain `<div>` elements with no role, aria-modal, focus trap, or Escape handling
- **After:**
```tsx
"use client";

import * as React from "react";
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { X } from "lucide-react";

import { cn } from "@/lib/utils";

const Dialog = DialogPrimitive.Root;

const DialogTrigger = DialogPrimitive.Trigger;

const DialogPortal = DialogPrimitive.Portal;

const DialogClose = DialogPrimitive.Close;

const DialogOverlay = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Overlay>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Overlay>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Overlay
    ref={ref}
    className={cn(
      "fixed inset-0 z-50 bg-black/80 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
      className
    )}
    {...props}
  />
));
DialogOverlay.displayName = DialogPrimitive.Overlay.displayName;

const DialogContent = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Content> & {
    uncloseable?: boolean;
  }
>(({ className, children, uncloseable, ...props }, ref) => (
  <DialogPortal>
    <DialogOverlay />
    <DialogPrimitive.Content
      ref={ref}
      onEscapeKeyDown={uncloseable ? (e) => e.preventDefault() : undefined}
      onPointerDownOutside={uncloseable ? (e) => e.preventDefault() : undefined}
      className={cn(
        "fixed left-[50%] top-[50%] z-50 grid w-full max-w-lg translate-x-[-50%] translate-y-[-50%] gap-4 border bg-background p-6 shadow-lg duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-lg",
        className
      )}
      {...props}
    >
      {children}
      {!uncloseable && (
        <DialogPrimitive.Close className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-accent data-[state=open]:text-muted-foreground">
          <X className="h-4 w-4" />
          <span className="sr-only">Close</span>
        </DialogPrimitive.Close>
      )}
    </DialogPrimitive.Content>
  </DialogPortal>
));
DialogContent.displayName = DialogPrimitive.Content.displayName;

const DialogHeader = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn(
      "flex flex-col space-y-1.5 text-center sm:text-left",
      className
    )}
    {...props}
  />
);
DialogHeader.displayName = "DialogHeader";

const DialogFooter = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn(
      "flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2",
      className
    )}
    {...props}
  />
);
DialogFooter.displayName = "DialogFooter";

const DialogTitle = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Title>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Title>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Title
    ref={ref}
    className={cn(
      "text-lg font-semibold leading-none tracking-tight",
      className
    )}
    {...props}
  />
));
DialogTitle.displayName = DialogPrimitive.Title.displayName;

const DialogDescription = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Description>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Description>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Description
    ref={ref}
    className={cn("text-sm text-muted-foreground", className)}
    {...props}
  />
));
DialogDescription.displayName = DialogPrimitive.Description.displayName;

export {
  Dialog,
  DialogPortal,
  DialogOverlay,
  DialogClose,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogFooter,
  DialogTitle,
  DialogDescription,
};
```

## Allowed moves
Replace entire content of `remote-frontend/src/components/ui/dialog.tsx` with the Radix-based implementation above.

## STOP triggers
- If `@radix-ui/react-dialog` is not in package.json (it IS at `^1.1.18`)
- If any caller passes props not preserved by the new API (open, onOpenChange, uncloseable, className, children)

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx tsc --noEmit
# Expected: no type errors (all callers use the preserved API surface)
cd remote-frontend && npx vitest run
# Expected: existing tests pass (some may need import updates for new exports)
```

## Done when
- `dialog.tsx` uses `@radix-ui/react-dialog`
- `role="dialog"` and `aria-modal="true"` are present (via Radix)
- `uncloseable` prop suppresses Escape and overlay click via `e.preventDefault()`
- All existing exports preserved: Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter

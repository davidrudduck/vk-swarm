import { Alert, AlertDescription } from './alert';

export interface ErrorBannerProps {
  /** Human-readable failure message shown to the user. */
  message: string;
}

/**
 * Distinct, authoritative error state for a page whose data fetch FAILED.
 *
 * Renders a `role="alert"` destructive banner so a failure is visually and
 * semantically distinguishable from a successful-but-empty result (Codex review
 * finding: failure must not read as "no data"). Reuses the shared `Alert`
 * primitive (`components/ui/alert.tsx`) rather than introducing a new visual.
 */
export function ErrorBanner({ message }: ErrorBannerProps) {
  return (
    <div style={{ padding: '12px 18px' }}>
      <Alert variant="destructive" data-testid="page-error-banner">
        <AlertDescription>{message}</AlertDescription>
      </Alert>
    </div>
  );
}

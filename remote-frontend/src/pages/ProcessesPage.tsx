import { ProcessesView } from '@/ui/panels';

/**
 * Placeholder wiring for `ProcessesView` (SC8).
 *
 * Known gap (ledgered, task 309): the hive has no `/processes` REST route
 * (`crates/remote/src/routes/` has no "process" handler), so this renders
 * an empty list until a processes endpoint is added.
 */
export function ProcessesPage() {
  return <ProcessesView processes={[]} />;
}

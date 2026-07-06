import { execSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const FRONTEND_ROOT = path.resolve(__dirname, '../../frontend');

export default async function sc4Guard() {
  console.log('[SC4] Running SC4 guard (frontend/ must still build)...');

  try {
    execSync('npx tsc --noEmit', { cwd: FRONTEND_ROOT, stdio: 'inherit' });
    console.log('[SC4] frontend typecheck: PASS');
  } catch {
    console.error('[SC4] frontend typecheck: FAIL');
    process.exit(1);
  }

  try {
    execSync('npm run lint', { cwd: FRONTEND_ROOT, stdio: 'inherit' });
    console.log('[SC4] frontend lint: PASS');
  } catch {
    console.error('[SC4] frontend lint: FAIL');
    process.exit(1);
  }

  try {
    execSync('npx vitest run', { cwd: FRONTEND_ROOT, stdio: 'inherit' });
    console.log('[SC4] frontend tests: PASS');
  } catch {
    console.error('[SC4] frontend tests: FAIL');
    process.exit(1);
  }

  console.log('[SC4] SC4 guard: ALL PASS');
}

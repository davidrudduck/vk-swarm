// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const indexCss = readFileSync(join(__dirname, '..', '..', 'index.css'), 'utf-8');

describe('index.css wires the token files (SC2/SC3)', () => {
  it('@imports fonts.css before colors/typography/spacing/base', () => {
    expect(indexCss).toContain("@import './styles/tokens/fonts.css';");
    expect(indexCss).toContain("@import './styles/tokens/colors.css';");
    expect(indexCss).toContain("@import './styles/tokens/typography.css';");
    expect(indexCss).toContain("@import './styles/tokens/spacing.css';");
    expect(indexCss).toContain("@import './styles/tokens/base.css';");
  });

  it('keeps the existing tailwind directives', () => {
    expect(indexCss).toContain('@tailwind base');
    expect(indexCss).toContain('@tailwind components');
    expect(indexCss).toContain('@tailwind utilities');
  });

  it('places @import statements before @tailwind directives (CSS @import ordering rule)', () => {
    const firstImport = indexCss.indexOf('@import');
    const firstTailwind = indexCss.indexOf('@tailwind');
    expect(firstImport).toBeLessThan(firstTailwind);
  });
});

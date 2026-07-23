// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const indexCss = readFileSync(join(__dirname, '..', '..', 'index.css'), 'utf-8');

describe('index.css wires the token files (SC2/SC3)', () => {
  it('@imports all five token files', () => {
    expect(indexCss).toContain("@import './styles/tokens/fonts.css';");
    expect(indexCss).toContain("@import './styles/tokens/colors.css';");
    expect(indexCss).toContain("@import './styles/tokens/typography.css';");
    expect(indexCss).toContain("@import './styles/tokens/spacing.css';");
    expect(indexCss).toContain("@import './styles/tokens/base.css';");
  });

  it('uses the Tailwind v3 @import form (not @tailwind directives)', () => {
    expect(indexCss).toContain("@import 'tailwindcss/base';");
    expect(indexCss).toContain("@import 'tailwindcss/components';");
    expect(indexCss).toContain("@import 'tailwindcss/utilities';");
  });

  it('keeps the external Google Fonts @import first (browsers ignore late @import)', () => {
    const fontsIdx = indexCss.indexOf("@import './styles/tokens/fonts.css';");
    const tailwindBaseIdx = indexCss.indexOf("@import 'tailwindcss/base';");
    expect(fontsIdx).toBeGreaterThanOrEqual(0);
    // fonts.css is the very first @import statement in the file
    expect(fontsIdx).toBe(indexCss.indexOf("@import '"));
    expect(fontsIdx).toBeLessThan(tailwindBaseIdx);
  });

  it('imports base.css AFTER Preflight so base element rules win the cascade (F1/F2/F3)', () => {
    const tailwindBaseIdx = indexCss.indexOf("@import 'tailwindcss/base';");
    const baseCssIdx = indexCss.indexOf("@import './styles/tokens/base.css';");
    const componentsIdx = indexCss.indexOf("@import 'tailwindcss/components';");
    // token base.css lands between Preflight (tailwindcss/base) and components
    expect(tailwindBaseIdx).toBeLessThan(baseCssIdx);
    expect(baseCssIdx).toBeLessThan(componentsIdx);
  });

  it('imports styles/components.css AFTER tailwindcss/components and BEFORE tailwindcss/utilities (design-system rules win over Tailwind components, utilities can still override)', () => {
    const tailwindComponentsIdx = indexCss.indexOf("@import 'tailwindcss/components';");
    const designComponentsIdx = indexCss.indexOf("@import './styles/components.css';");
    const tailwindUtilitiesIdx = indexCss.indexOf("@import 'tailwindcss/utilities';");
    expect(designComponentsIdx).toBeGreaterThanOrEqual(0);
    expect(tailwindComponentsIdx).toBeLessThan(designComponentsIdx);
    expect(designComponentsIdx).toBeLessThan(tailwindUtilitiesIdx);
  });
});

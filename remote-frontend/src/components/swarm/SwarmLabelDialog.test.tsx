import { describe, it, expect } from 'vitest';
import { getContrastColor } from './SwarmLabelDialog';

describe('getContrastColor', () => {
  it('returns black (#000000) for light colors', () => {
    expect(getContrastColor('#ffffff')).toBe('#000000');
  });

  it('returns white (#ffffff) for dark colors', () => {
    expect(getContrastColor('#000000')).toBe('#ffffff');
  });

  it('returns black for yellow (#ffff00 — bright)', () => {
    expect(getContrastColor('#ffff00')).toBe('#000000');
  });

  it('returns white for navy (#000080 — dark)', () => {
    expect(getContrastColor('#000080')).toBe('#ffffff');
  });

  it('handles hex color with leading #', () => {
    expect(getContrastColor('#ff0000')).toBe('#ffffff');
  });

  it('handles hex color without leading #', () => {
    expect(getContrastColor('00ff00')).toBe('#000000');
  });

  it('handles mid-gray correctly', () => {
    // #808080 has luminance ~0.502, just above 0.5 → black
    expect(getContrastColor('#808080')).toBe('#000000');
  });

  it('handles neutral gray at the boundary', () => {
    // #7f7f7f has luminance ~0.498, just below 0.5 → white
    const result = getContrastColor('#7f7f7f');
    expect(result).toBe('#ffffff');
  });

  it('handles uppercase hex', () => {
    // #FF5733 luminance = 0.522 > 0.5 → black
    expect(getContrastColor('#FF5733')).toBe('#000000');
  });

  it('handles mixed case hex', () => {
    expect(getContrastColor('#AbCdEf')).toBe('#000000');
  });

  it('returns white for 3-digit hex (#fff — invalid format)', () => {
    expect(getContrastColor('#fff')).toBe('#ffffff');
  });

  it('returns white for empty string', () => {
    expect(getContrastColor('')).toBe('#ffffff');
  });

  it('returns white for non-hex garbage', () => {
    expect(getContrastColor('not-a-color')).toBe('#ffffff');
  });

  it('returns white for 8-digit hex (#RRGGBBAA)', () => {
    expect(getContrastColor('#ff0000aa')).toBe('#ffffff');
  });
});

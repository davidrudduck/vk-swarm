import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { NodeCard } from './index';

describe('NodeCard (SC5)', () => {
  it('emits vks-node with the name in a __name span', () => {
    render(<NodeCard name="node-1" />);
    expect(screen.getByText('node-1').closest('.vks-node')).toBeTruthy();
    expect(screen.getByText('node-1')).toHaveClass('vks-node__name');
  });

  it('renders the OS glyph svg for linux', () => {
    const { container } = render(<NodeCard name="n" os="linux" />);
    expect(container.querySelector('.vks-node__os')).toBeTruthy();
    expect(container.querySelector('.vks-node__os svg')).toBeTruthy();
  });

  it('renders the online pulse when online=true', () => {
    const { container } = render(<NodeCard name="n" online />);
    expect(container.querySelector('.vks-node__pulse')).toBeTruthy();
    expect(container.querySelector('.vks-node__pulse--offline')).toBeFalsy();
  });

  it('renders the offline pulse when online=false', () => {
    const { container } = render(<NodeCard name="n" online={false} />);
    expect(container.querySelector('.vks-node__pulse--offline')).toBeTruthy();
  });

  it('renders the meta + right ReactNodes when provided', () => {
    const { container } = render(<NodeCard name="n" meta={<span data-testid="m" />} right={<span data-testid="r" />} />);
    expect(container.querySelector('[data-testid="m"]')).toBeTruthy();
    expect(container.querySelector('[data-testid="r"]')).toBeTruthy();
  });
});

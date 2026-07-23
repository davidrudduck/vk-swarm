import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Button, Badge, Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from './index';

describe('Button (SC4)', () => {
  it('emits vks-btn + vks-btn--primary + vks-btn--md for defaults', () => {
    render(<Button>Save</Button>);
    const btn = screen.getByText('Save');
    expect(btn).toHaveClass('vks-btn');
    expect(btn).toHaveClass('vks-btn--primary');
    expect(btn).toHaveClass('vks-btn--md');
  });

  it('emits the variant + size classes for ghost/icon', () => {
    render(
      <Button variant="ghost" size="icon">
        +
      </Button>
    );
    const btn = screen.getByText('+');
    expect(btn).toHaveClass('vks-btn--ghost');
    expect(btn).toHaveClass('vks-btn--icon');
  });

  it('passes through native button props (type, disabled, onClick)', () => {
    const onClick = vi.fn();
    render(
      <Button type="submit" disabled onClick={onClick}>
        Go
      </Button>
    );
    const btn = screen.getByText('Go') as HTMLButtonElement;
    expect(btn.type).toBe('submit');
    expect(btn.disabled).toBe(true);
  });
});

describe('Badge (SC4)', () => {
  it('emits vks-badge + vks-badge--default by default', () => {
    render(<Badge>new</Badge>);
    const el = screen.getByText('new');
    expect(el).toHaveClass('vks-badge');
    expect(el).toHaveClass('vks-badge--default');
  });

  it('renders a __dot span when dot={true}', () => {
    const { container } = render(<Badge dot>live</Badge>);
    expect(container.querySelector('.vks-badge__dot')).toBeTruthy();
  });

  it('emits the destructive variant class', () => {
    render(<Badge variant="destructive">err</Badge>);
    expect(screen.getByText('err')).toHaveClass('vks-badge--destructive');
  });
});

describe('Card (SC4)', () => {
  it('renders a vks-card root with header/title/desc/content/footer subcomponents', () => {
    const { container } = render(
      <Card>
        <CardHeader>
          <CardTitle>Title</CardTitle>
          <CardDescription>Desc</CardDescription>
        </CardHeader>
        <CardContent>Body</CardContent>
        <CardFooter>Foot</CardFooter>
      </Card>
    );
    expect(container.firstChild).toHaveClass('vks-card');
    expect(container.querySelector('.vks-card__header')).toBeTruthy();
    expect(container.querySelector('.vks-card__title')?.tagName).toBe('H3');
    expect(container.querySelector('.vks-card__desc')?.tagName).toBe('P');
    expect(container.querySelector('.vks-card__content')).toBeTruthy();
    expect(container.querySelector('.vks-card__footer')).toBeTruthy();
  });
});

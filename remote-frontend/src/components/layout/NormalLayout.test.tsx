import { render, screen } from '@testing-library/react';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { NormalLayout } from './NormalLayout';

describe('NormalLayout', () => {
  it('renders navbar, outlet, and bottom nav', () => {
    const router = createMemoryRouter([
      {
        path: '/',
        element: <NormalLayout />,
        children: [
          {
            index: true,
            element: <div data-testid="outlet-child">Test Child</div>,
          },
        ],
      },
    ]);

    render(<RouterProvider router={router} />);

    // Assert navbar renders
    const navbar = screen.getByTestId('navbar');
    expect(navbar).toBeInTheDocument();

    // Assert outlet child renders
    const outletChild = screen.getByTestId('outlet-child');
    expect(outletChild).toBeInTheDocument();
    expect(outletChild).toHaveTextContent('Test Child');

    // Assert bottom nav renders (check for role="navigation")
    const navs = screen.getAllByRole('navigation');
    expect(navs.length).toBeGreaterThanOrEqual(1);
  });
});

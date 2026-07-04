import { Outlet } from 'react-router-dom';
import { Navbar } from '@/components/layout/Navbar';
import { BottomNav } from '@/components/layout/BottomNav';

export function NormalLayout() {
  return (
    <>
      <Navbar />
      <div className="flex-1 min-h-0 overflow-hidden pb-14 sm:pb-0">
        <Outlet />
      </div>
      <BottomNav />
    </>
  );
}

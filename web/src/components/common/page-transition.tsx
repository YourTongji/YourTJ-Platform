import type { ReactNode } from "react";
import { useLocation } from "react-router";

export function PageTransition({ children }: { children: ReactNode }) {
  const location = useLocation();

  return (
    <div key={location.pathname} className="motion-page min-w-0">
      {children}
    </div>
  );
}

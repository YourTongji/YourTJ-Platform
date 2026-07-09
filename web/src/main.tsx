import { QueryClientProvider } from "@tanstack/react-query";
import React from "react";
import ReactDOM from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router";
import { Toaster } from "sonner";

import { AppLayout } from "@/components/layout/app-layout";
import { AuthProvider } from "@/context/auth-provider";
import { queryClient } from "@/lib/query";
import { AdminPage } from "@/pages/admin-page";
import { BookmarksPage } from "@/pages/bookmarks-page";
import { CourseDetailPage } from "@/pages/course-detail-page";
import { CoursesPage } from "@/pages/courses-page";
import { ForumPage } from "@/pages/forum-page";
import { HomePage } from "@/pages/home-page";
import { LoginPage } from "@/pages/login-page";
import { MessagesPage } from "@/pages/messages-page";
import { NotFoundPage } from "@/pages/not-found-page";
import { NotificationsPage } from "@/pages/notifications-page";
import { ProfilePage } from "@/pages/profile-page";
import { SchedulePage } from "@/pages/schedule-page";
import { SettingsPage } from "@/pages/settings-page";
import { ThreadDetailPage } from "@/pages/thread-detail-page";
import { WalletPage } from "@/pages/wallet-page";
import "@/styles/index.css";

const storedTheme = localStorage.getItem("yourtj.theme");
if (storedTheme === "dark") {
  document.documentElement.classList.add("dark");
}

const router = createBrowserRouter([
  {
    path: "/",
    element: <AppLayout />,
    errorElement: <NotFoundPage />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "login", element: <LoginPage /> },
      { path: "forum", element: <ForumPage /> },
      { path: "forum/threads/:id", element: <ThreadDetailPage /> },
      { path: "courses", element: <CoursesPage /> },
      { path: "courses/:id", element: <CourseDetailPage /> },
      { path: "schedule", element: <SchedulePage /> },
      { path: "wallet", element: <WalletPage /> },
      { path: "notifications", element: <NotificationsPage /> },
      { path: "messages", element: <MessagesPage /> },
      { path: "bookmarks", element: <BookmarksPage /> },
      { path: "profile/:handle", element: <ProfilePage /> },
      { path: "settings", element: <SettingsPage /> },
      { path: "admin", element: <AdminPage /> },
      { path: "*", element: <NotFoundPage /> },
    ],
  },
]);

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <RouterProvider router={router} />
        <Toaster position="top-center" richColors />
      </AuthProvider>
    </QueryClientProvider>
  </React.StrictMode>,
);

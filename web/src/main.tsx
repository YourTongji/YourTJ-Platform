import { QueryClientProvider } from "@tanstack/react-query";
import React from "react";
import ReactDOM from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router";
import { Toaster } from "sonner";

import { AppLayout } from "@/components/layout/app-layout";
import { AuthProvider } from "@/context/auth-provider";
import { queryClient } from "@/lib/query";
import { NotFoundPage } from "@/pages/not-found-page";
import "@/styles/index.css";

const AdminPage = React.lazy(async () => ({
  default: (await import("@/pages/admin-page")).AdminPage,
}));
const AccountRecoveryPage = React.lazy(async () => ({
  default: (await import("@/pages/account-recovery-page")).AccountRecoveryPage,
}));
const AnnouncementsPage = React.lazy(async () => ({
  default: (await import("@/pages/announcements-page")).AnnouncementsPage,
}));
const AppealsPage = React.lazy(async () => ({
  default: (await import("@/pages/appeals-page")).AppealsPage,
}));
const BookmarksPage = React.lazy(async () => ({
  default: (await import("@/pages/bookmarks-page")).BookmarksPage,
}));
const CourseDetailPage = React.lazy(async () => ({
  default: (await import("@/pages/course-detail-page")).CourseDetailPage,
}));
const CoursesPage = React.lazy(async () => ({
  default: (await import("@/pages/courses-page")).CoursesPage,
}));
const ForumPage = React.lazy(async () => ({
  default: (await import("@/pages/forum-page")).ForumPage,
}));
const HomePage = React.lazy(async () => ({
  default: (await import("@/pages/home-page")).HomePage,
}));
const LoginPage = React.lazy(async () => ({
  default: (await import("@/pages/login-page")).LoginPage,
}));
const MessagesPage = React.lazy(async () => ({
  default: (await import("@/pages/messages-page")).MessagesPage,
}));
const NotificationsPage = React.lazy(async () => ({
  default: (await import("@/pages/notifications-page")).NotificationsPage,
}));
const OnboardingPage = React.lazy(async () => ({
  default: (await import("@/pages/onboarding-page")).OnboardingPage,
}));
const ProfilePage = React.lazy(async () => ({
  default: (await import("@/pages/profile-page")).ProfilePage,
}));
const SchedulePage = React.lazy(async () => ({
  default: (await import("@/pages/schedule-page")).SchedulePage,
}));
const SearchPage = React.lazy(async () => ({
  default: (await import("@/pages/search-page")).SearchPage,
}));
const SettingsPage = React.lazy(async () => ({
  default: (await import("@/pages/settings-page")).SettingsPage,
}));
const ThreadDetailPage = React.lazy(async () => ({
  default: (await import("@/pages/thread-detail-page")).ThreadDetailPage,
}));
const WalletPage = React.lazy(async () => ({
  default: (await import("@/pages/wallet-page")).WalletPage,
}));

const storedTheme = localStorage.getItem("yourtj.theme");
if (storedTheme === "dark") {
  document.documentElement.classList.add("dark");
}

const router = createBrowserRouter(
  [
    {
      path: "/",
      element: <AppLayout />,
      errorElement: <NotFoundPage />,
      children: [
        { index: true, element: <HomePage /> },
        { path: "login", element: <LoginPage /> },
        { path: "recover-account", element: <AccountRecoveryPage /> },
        { path: "onboarding", element: <OnboardingPage /> },
        { path: "forum", element: <ForumPage /> },
        { path: "forum/threads/:id", element: <ThreadDetailPage /> },
        { path: "courses", element: <CoursesPage /> },
        { path: "courses/:id", element: <CourseDetailPage /> },
        { path: "schedule", element: <SchedulePage /> },
        { path: "search", element: <SearchPage /> },
        { path: "wallet", element: <WalletPage /> },
        { path: "announcements", element: <AnnouncementsPage /> },
        { path: "appeals", element: <AppealsPage /> },
        { path: "notifications", element: <NotificationsPage /> },
        { path: "messages", element: <MessagesPage /> },
        { path: "bookmarks", element: <BookmarksPage /> },
        { path: "profile/:handle", element: <ProfilePage /> },
        { path: "settings", element: <SettingsPage /> },
        { path: "admin", element: <AdminPage /> },
        { path: "*", element: <NotFoundPage /> },
      ],
    },
  ],
  { basename: import.meta.env.BASE_URL },
);

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

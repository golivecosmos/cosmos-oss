import { Route, createBrowserRouter, createRoutesFromElements, RouterProvider } from "react-router-dom";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { AppLayout } from "./components/AppLayout";
import { AppLayoutProvider } from "./contexts/AppLayoutContext";
import { IndexingJobsProvider } from "./contexts/IndexingJobsContext";
import { AILibrary } from "./components/routes/AILibrary";
import { FileSystem } from "./components/routes/FileSystem";
import { Drive } from "./components/routes/Drive";
import { Toaster } from "sonner";
import { Studio } from "./components/routes/Studio";
import { StudioEdit } from "./components/routes/StudioEdit";
import { StudioLayout } from "./components/routes/Studio/Layout";
import { QuickShell } from "./components/quick/QuickShell";

// Security: Disable console and devtools in production
if (process.env.NODE_ENV === "production") {
  // Disable console methods
  const noop = () => { };
  if (typeof window !== "undefined" && window.console) {
    window.console.log = noop;
    window.console.warn = noop;
    window.console.error = noop;
    window.console.info = noop;
    window.console.debug = noop;
    window.console.trace = noop;
    window.console.dir = noop;
    window.console.table = noop;
    window.console.clear = noop;
    window.console.count = noop;
    window.console.time = noop;
    window.console.timeEnd = noop;
    window.console.group = noop;
    window.console.groupEnd = noop;
    window.console.groupCollapsed = noop;
  }

  // Disable right-click context menu and devtools keyboard shortcuts
  document.addEventListener("contextmenu", (e) => e.preventDefault());

  document.addEventListener("keydown", (e) => {
    // Disable F12, Ctrl+Shift+I, Ctrl+Shift+J, Ctrl+U, Ctrl+Shift+C
    if (
      e.key === "F12" ||
      (e.ctrlKey && e.shiftKey && ["I", "J", "C"].includes(e.key)) ||
      (e.ctrlKey && e.key === "U")
    ) {
      e.preventDefault();
      e.stopPropagation();
      return false;
    }
  });

  // Additional protection against devtools
  let devtools = { open: false, orientation: null };
  setInterval(() => {
    if (
      window.outerHeight - window.innerHeight > 200 ||
      window.outerWidth - window.innerWidth > 200
    ) {
      if (!devtools.open) {
        devtools.open = true;
        // Could add additional security measures here if needed
      }
    } else {
      devtools.open = false;
    }
  }, 500);
}

const router = createBrowserRouter(
  createRoutesFromElements(
    <Route path="/" element={<AppLayout />}>
      <Route index element={<AILibrary />} />
      <Route path="/fs" element={<FileSystem />} />
      <Route path="/drive/:drive_id" element={<Drive />} />
      <Route path="/studio" element={<StudioLayout />}>
        <Route index element={<Studio />} />
        <Route path="edit" element={<StudioEdit />} loader={StudioEdit.loader} />
      </Route>
    </Route>
  )
);

const currentWindowLabel = (() => {
  try {
    return getCurrentWebviewWindow().label;
  } catch {
    return "main";
  }
})();

function AppToaster() {
  return (
    <Toaster
      toastOptions={{
        className: "bg-transparent dark:bg-[darkBg] dark:border-blueShadow",
        style: { backgroundColor: "var(--toast-bg, white)" },
        classNames: {
          icon: "dark:!text-customWhite",
          title: "dark:!text-customWhite",
          description: "text-xs !text-gray-600 dark:!text-customGray",
          actionButton: "!bg-blue-500 hover:!bg-blue-600 dark:hover:!bg-customBlue dark:!bg-blueShadow ",
          cancelButton: "dark:!bg-darkBgMid dark:!text-customWhite",
        },
      }}
    />
  );
}

export default function App() {
  if (currentWindowLabel === "quick") {
    return (
      <>
        <QuickShell />
        <AppToaster />
      </>
    );
  }

  return (
    <IndexingJobsProvider>
      <AppLayoutProvider>
        <RouterProvider router={router} />
        <AppToaster />
      </AppLayoutProvider>
    </IndexingJobsProvider>
  );
}

import { useLocation, useNavigate, Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { Onboarding } from "./onboarding/Onboarding";
import { InteractiveTour } from "./InteractiveTour";
import { Settings } from "./Settings";
import { AppStore } from "./AppStore";
import { UpdateNotification } from "./UpdateNotification";
import { ErrorReporting } from "./ErrorReporting";
import { QuickMenu } from "./QuickMenu";
import { IndexStatusSheet } from "./IndexStatusSheet";
import { TooltipProvider } from "./ui/tooltip";

import { Download, AlertCircle } from "lucide-react";
import { Button } from "./ui/button";
import { useAppLayout } from "../contexts/AppLayoutContext";
import { invoke } from "@tauri-apps/api/tauri";
import { useIndexingJobs } from "../contexts/IndexingJobsContext";

export const AppLayout = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const searchParams = new URLSearchParams(location.search);

  const {
    indexingPaths,
    modelDownload,
    retryModelDownload,
    showOnboarding,
    showInteractiveTour,
    showBugReport,
    showIndexingInfo,
    setShowIndexingInfo,
    clearSearch,
    handleAddToFavorites,
    handleShare,
    handleDelete,
    handleCreateFolder,
    handleOnboardingComplete,
    handleRestartOnboarding,
    handleStartTour,
    handleTourComplete,
    handleTourDismiss,
    handleCloseBugReport,
    handleOpenBugReport,
    isIndexingAllowed,
    showIndexingWarning,
    handleBulkIndex,
    showAppStore,
    handleOpenAppStore,
    handleCloseAppStore,
  } = useAppLayout();
  const { loadIndexedCount } = useIndexingJobs();
  // Check if settings modal should be shown based on query param
  const showSettings = searchParams.get('settings') === 'true';

  // Handle settings navigation
  const handleOpenSettings = () => {
    const newSearchParams = new URLSearchParams(location.search);
    newSearchParams.set('settings', 'true');
    navigate(`${location.pathname}?${newSearchParams.toString()}`, { replace: true });
  };

  const handleCloseSettings = () => {
    const newSearchParams = new URLSearchParams(location.search);
    newSearchParams.delete('settings');
    const queryString = newSearchParams.toString();
    navigate(`${location.pathname}${queryString ? `?${queryString}` : ''}`, { replace: true });
  };

  // Handle restarting tour from settings
  const handleRestartTourFromSettings = () => {
    handleCloseSettings();
    handleStartTour();
  };

  // Render model status indicator
  const renderModelStatus = () => {
    if (modelDownload.state === "ready") return null;

    const getStatusConfig = () => {
      switch (modelDownload.state) {
        case "checking":
          return {
            icon: Download,
            title: 'Checking AI Models',
            message: 'Verifying model availability...',
            bg: 'bg-blue-50 dark:bg-customBlue',
            border: 'border-blue-300 dark:border-blueShadow',
            text: "text-blue-600 dark:text-blueShadow",
            showProgress: false
          }
        case 'downloading':
          return {
            icon: Download,
            title: 'Downloading AI Models',
            message: 'Downloading required AI models...',
            bg: 'bg-blue-50 dark:bg-customBlue',
            border: 'border-blue-300 dark:border-blueShadow',
            text: "text-blue-600 dark:text-blueShadow",
            showProgress: true
          }
        case 'installing':
          return {
            icon: Download,
            bg: 'bg-blue-50 dark:bg-customBlue',
            border: 'border-blue-300 dark:border-blueShadow',
            text: "text-blue-600 dark:text-blueShadow",
            showProgress: true
          }
        case 'failed':
          return {
            icon: AlertCircle,
            title: 'AI Setup Failed',
            message: modelDownload.error || 'Failed to setup AI models',
            bg: 'bg-red-50 dark:bg-redHighlight',
            border: 'border-red-300 dark:border-redShadow',
            text: "text-red-600 dark:text-redShadow",
            showProgress: false
          }
        default:
          return null;
      }
    };

    const config = getStatusConfig();
    if (!config) return null;

    return (
      <div className="border-b dark:border-darkBgHighlight border-gray-200 flex items-center justify-between">
        <div className="flex-1">
          <div className={`border-b ${config.border} ${config.bg} px-6 py-3`}>
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-3">
                <config.icon className={`w-5 h-5 ${config.text} ${modelDownload.state === 'downloading' || modelDownload.state === 'installing' ? 'animate-spin' : ''}`} />
                <div>
                  <h3 className={`text-sm font-medium ${config.text}`}>{config.title}</h3>
                  <p className={`text-xs ${config.text}`}>{config.message}</p>
                </div>
              </div>

              <div className="flex items-center space-x-3">
                {config.showProgress && (
                  <div className="flex items-center space-x-2">
                    <div className={`w-32 h-2 ${config.bg} rounded-full overflow-hidden`}>
                      <div
                        className={`h-2 ${config.bg} rounded-full transition-all duration-500 ease-out`}
                        style={{ width: `${modelDownload.progress}%` }}
                      />
                    </div>
                    <span className={`text-xs ${config.text}-600 font-medium`}>
                      {Math.round(modelDownload.progress)}%
                    </span>
                  </div>
                )}

                {modelDownload.state === "failed" && (
                  <Button
                    onClick={retryModelDownload}
                    size="sm"
                    variant="outline"
                    className={`${config.border} ${config.text} dark:hover:bg-customRed hover:bg-red-100`}
                  >
                    Retry
                  </Button>
                )}

                {(modelDownload.state === "downloading" ||
                  modelDownload.state === "installing") && (
                    <Button
                      onClick={() => {
                        if (
                          confirm(
                            "Are you sure you want to cancel the download? You can resume it later."
                          )
                        ) {
                          // TODO: Add cancel download functionality
                          // invoke("cancel_download");
                          // setModelDownload((prev) => ({
                          //   ...prev,
                          //   state: "failed",
                          //   error: "Download cancelled by user",
                          // }));
                          // downloadInProgressRef.current = false;
                        }
                      }}
                      size="sm"
                      variant="outline"
                      className={`${config.border} ${config.text} hover:${config.bg}`}
                    >
                      Cancel
                    </Button>
                  )}

                <Button
                  onClick={handleOpenSettings}
                  size="sm"
                  variant="ghost"
                  className={`${config.text} hover:${config.text}`}
                >
                  Settings
                </Button>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  };

  return (
    <TooltipProvider>
      <div className="h-screen flex flex-col scroll-y-auto overflow-auto custom-scroll">
        <div className="flex-1 flex">
          <Sidebar
            onCreateFolder={handleCreateFolder}
            onDelete={handleDelete}
            onAddToFavorites={handleAddToFavorites}
            onShare={handleShare}
            indexingPaths={indexingPaths}
            onBulkIndex={handleBulkIndex}
            isIndexingDisabled={!isIndexingAllowed()}
            onClearSearch={clearSearch}
          />
          <div className="flex-1 bg-gray-50 dark:bg-darkBgMid overflow-y-hidden max-h-screen">
            {renderModelStatus()}
            <div className="flex-1 h-full">
              <Outlet />
            </div>
          </div>
        </div>

        {/* Modals and overlays */}
        {showOnboarding && !showInteractiveTour && (
          <Onboarding
            onComplete={handleOnboardingComplete}
          />
        )}
        {showInteractiveTour && (
          <InteractiveTour
            onComplete={handleTourComplete}
            onDismiss={handleTourDismiss}
            isVisible={showInteractiveTour}
            onIndexFile={async (_path: string) => {
              if (!isIndexingAllowed()) {
                showIndexingWarning();
                return;
              }

              try {
                await invoke("index_file", { path: _path });
                await loadIndexedCount();
              } catch (error) {
                console.error("Failed to index file:", error);
              }
            }}
          />
        )}
        {showSettings && (
          <Settings
            onClose={handleCloseSettings}
            onRestartTour={handleRestartTourFromSettings}
            isOpen={showSettings}
            modelDownloadState={modelDownload}
            onRetryDownload={retryModelDownload}
          />
        )}
        {showBugReport && (
          <ErrorReporting
            onClose={handleCloseBugReport}
            isOpen={showBugReport}
          />
        )}
        {showAppStore && (
          <AppStore
            onClose={handleCloseAppStore}
            isOpen={showAppStore}
          />
        )}
        <UpdateNotification
          checkOnMount={true}
          showStagingUpdates={process.env.NODE_ENV === "development"}
        />
        <QuickMenu
          onRestartWelcome={handleRestartOnboarding}
          onStartTour={handleStartTour}
          onOpenBugReport={handleOpenBugReport}
          onOpenSettings={handleOpenSettings}
          onOpenAppStore={handleOpenAppStore}
          showRestartWelcome={process.env.NODE_ENV === "development"}
        />
        <IndexStatusSheet
          isOpen={showIndexingInfo}
          onClose={() => setShowIndexingInfo(false)}
        />
      </div>
    </TooltipProvider>
  );
};

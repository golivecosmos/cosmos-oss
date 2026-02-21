import React, { useState, useEffect, useRef } from 'react';
import { Button } from './ui/button';
import { Download, RefreshCw, AlertCircle } from 'lucide-react';
import { updateService, UpdateCheckResult } from '../services/updateService';
import { useAppVersion } from '../hooks/useAppVersion';
import { toast } from 'sonner';

interface UpdateNotificationProps {
  checkOnMount?: boolean;
  showStagingUpdates?: boolean;
  enablePeriodicChecks?: boolean;
  checkIntervalMinutes?: number;
}

// Add this function to format markdown to HTML
const formatReleaseNotes = (markdown: string): string => {
  return markdown
    // Bold text: **text** -> <strong>text</strong>
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    // Headers: ### Added, ### Changed, etc. -> proper headings
    .replace(/^### (Added|Changed|Fixed|Removed)$/gm, '<div class="font-semibold text-gray-800 mt-3 mb-1">$1</div>')
    // Other headers: ### text -> <strong>text</strong>
    .replace(/^#{1,6}\s+(.*)$/gm, '<strong>$1</strong>')
    // Bullet points with dash: - text -> • text
    .replace(/^[\s]*-\s+(.*)$/gm, '<div class="ml-2">• $1</div>')
    // Bullet points with asterisk: * text -> • text
    .replace(/^[\s]*\*\s+(.*)$/gm, '<div class="ml-2">• $1</div>')
    // Bullet points with plus: + text -> • text
    .replace(/^[\s]*\+\s+(.*)$/gm, '<div class="ml-2">• $1</div>')
    // Double line breaks: \n\n -> <br/><br/>
    .replace(/\n\n/g, '<br/>')
    // Single line breaks: \n -> <br/>
    .replace(/\n/g, '<br/>');
};

export const UpdateNotification: React.FC<UpdateNotificationProps> = ({
  checkOnMount = true,
  showStagingUpdates = false,
  enablePeriodicChecks = true,
  checkIntervalMinutes = 60, // Check every hour by default
}) => {
  const [isInstalling, setIsInstalling] = useState(false);
  const { version } = useAppVersion();
  const intervalRef = useRef<NodeJS.Timeout | null>(null);

  const checkForUpdates = async (isBackground = false) => {
    try {
      const result = await updateService.checkForUpdates();
      if (result.hasUpdate) {
        renderUpdateToast(result)
      }
    } catch (error) {
      console.error('Update check failed:', error);
      renderUpdateToast({
        hasUpdate: false,
        currentVersion: version || '2.0.1',
        error: 'Failed to check for updates',
      })
    }
  };

  const handleInstallUpdate = async (update) => {
    if (!update.updateInfo) return;
    try {
      await updateService.installAndRestart();
      // If we reach here, something went wrong (app should have restarted)
      console.warn('Update installation completed but app did not restart');
      toast.dismiss("installing")
    } catch (error) {
      console.error('Update installation failed:', error);
      renderUpdateToast({
        error: error.message
      }, true)
    } finally {
      setIsInstalling(false)
    }
  };

  const handleRetry = () => {
    checkForUpdates();
  };

  const renderUpdateToast = (update, isInstalling = false) => {
    if (update.error) {
      toast(isInstalling ? "Update Failed" : "Update Check Failed", {
        action: {
          label: "Retry",
          onClick: handleRetry
        },
        cancel: {
          label: "Cancel",
          onClick: null
        }
      })
      console.log(update.error)
    } else {
      if (!isInstalling) {
        toast(<div className="bg-white dark:bg-[rgb(39,45,60)] border-gray-200 dark:border-transparent">
          <div className="flex items-center gap-2">
            <Download className="h-12 w-12 text-blue-500 dark:text-[rgb(117,135,163)]" />
            <div>
              <p className="text-gray-900 dark:text-[rgb(250,253,253)]">Update v{update.updateInfo.version} Available</p>
              <p className="text-xs text-gray-600 dark:text-[rgb(153,175,186)]"> {update.updateInfo.body} </p>
              <div className="flex mt-2 gap-2">
                <button onClick={() => toast.dismiss(update.updateInfo.version)}
                  className="p-1 rounded w-32 h-6 bg-gray-100 dark:bg-[rgb(59,66,86)] text-gray-900 dark:text-[rgb(250,253,253)]"> Remind Me Later </button>
                <button onClick={() => {
                  setIsInstalling(true)
                  toast.dismiss(update.updateInfo.version)
                  handleInstallUpdate(update)
                  toast.loading("Installing...", {
                    id: "installing",
                    duration: Infinity,
                  }
                  )
                }
                }
                  className="p-1 rounded w-32 h-6 text-white bg-blue-500 dark:bg-[rgb(117,135,163)] hover:bg-blue-600 dark:hover:bg-[rgb(181,204,216)]"> Install Now </button>
              </div>
            </div>
          </div>
        </div>, {
          //Use the found update as the toast key so the toast does not re render
          id: update.updateInfo.version,
          duration: 10000,
        })
      }
    }
  }

  // Initial check on mount
  useEffect(() => {
    if (checkOnMount) {
      // Check for updates 5 seconds after mount
      const timer = setTimeout(() => checkForUpdates(), 5000);
      return () => clearTimeout(timer);
    }
  }, [checkOnMount]);

  // Periodic background checks
  useEffect(() => {
    if (enablePeriodicChecks && checkIntervalMinutes > 0) {
      const intervalMs = checkIntervalMinutes * 60 * 1000;

      intervalRef.current = setInterval(() => {
        console.log('🔄 Performing background update check...');
        checkForUpdates(true);
      }, intervalMs);

      return () => {
        if (intervalRef.current) {
          clearInterval(intervalRef.current);
        }
      };
    }
  }, [enablePeriodicChecks, checkIntervalMinutes]);

  return null;
};

// Simplified update checker component for manual checks
export const UpdateChecker: React.FC<{ showStagingUpdates?: boolean }> = ({
  showStagingUpdates = false
}) => {
  const [isChecking, setIsChecking] = useState(false);
  const [result, setResult] = useState<UpdateCheckResult | null>(null);

  const checkForUpdates = async () => {
    setIsChecking(true);
    try {
      const update = await updateService.checkForUpdates();
      setResult(update);
    } catch (error) {
      console.error('Manual update check failed:', error);
      setResult({
        hasUpdate: false,
        currentVersion: '2.0.1',
        error: 'Update check failed',
      });
    } finally {
      setIsChecking(false);
    }
  };

  const handleInstallUpdate = async () => {
    if (!result?.updateInfo) return;

    try {
      await updateService.installAndRestart();
    } catch (error) {
      console.error('Update installation failed:', error);
    }
  };

  return (
    <div className="space-y-4">
      <Button
        onClick={checkForUpdates}
        disabled={isChecking}
        variant="outline"
        className="w-full"
      >
        {isChecking ? (
          <>
            <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
            Checking for Updates...
          </>
        ) : (
          <>
            <RefreshCw className="w-4 h-4 mr-2" />
            Check for Updates
          </>
        )}
      </Button>

      {result && (
        <div className="p-4 border rounded-lg">
          {result.hasUpdate ? (
            <div className="space-y-3">
              <div className="flex items-center space-x-2">
                <Download className="w-5 h-5 text-green-600" />
                <span className="font-medium text-green-800">Update Available!</span>
              </div>

              <div className="space-y-1 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-600">Current:</span>
                  <span className="font-mono">{result.currentVersion}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-600">Latest:</span>
                  <span className="font-mono text-green-600">{result.latestVersion}</span>
                </div>
              </div>

              {result.updateInfo?.body && (
                <div
                  className="text-sm text-gray-700 bg-gray-50 p-2 rounded"
                  dangerouslySetInnerHTML={{ __html: formatReleaseNotes(result.updateInfo.body) }}
                />
              )}

              <Button
                onClick={handleInstallUpdate}
                className="w-full"
                size="sm"
              >
                <Download className="w-4 h-4 mr-2" />
                Install & Restart
              </Button>
            </div>
          ) : result.error ? (
            <div className="flex items-center space-x-2 text-red-600">
              <AlertCircle className="w-5 h-5" />
              <span className="text-sm">{result.error}</span>
            </div>
          ) : (
            <div className="flex items-center space-x-2 text-green-200">
              <Download className="w-5 h-5" />
              <span className="text-sm">You're running the latest version!</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

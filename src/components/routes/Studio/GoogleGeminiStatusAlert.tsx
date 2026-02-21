import React from "react";
import { Button } from "../../ui/button";
import { AlertCircle, Video, Loader2 } from "lucide-react";

interface GoogleGeminiStatusAlertProps {
  onInstallVeo3: () => void;
  isChecking?: boolean;
}

export const GoogleGeminiStatusAlert: React.FC<GoogleGeminiStatusAlertProps> = ({
  onInstallVeo3,
  isChecking = false,
}) => {
  return (
    <div className="bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded-lg p-4">
      <div className="flex items-start space-x-3">
        <AlertCircle className="w-5 h-5 text-orange-600 dark:text-orange-400 mt-0.5 flex-shrink-0" />
        <div className="flex-1">
          <h4 className="text-sm font-medium text-orange-800 dark:text-orange-200 mb-1">
            {isChecking ? "Checking Google Gemini Status..." : "Google Gemini Required"}
          </h4>
          <p className="text-xs text-orange-700 dark:text-orange-300 mb-3">
            {isChecking 
              ? "Verifying Veo3 installation status..." 
              : "Install Veo3 from the App Store to generate videos"
            }
          </p>
          <Button 
            size="sm" 
            className="bg-orange-600 hover:bg-orange-700 text-white"
            onClick={onInstallVeo3}
            disabled={isChecking}
          >
            {isChecking ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                Checking...
              </>
            ) : (
              <>
                <Video className="w-4 h-4 mr-2" />
                Install Veo3
              </>
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}; 
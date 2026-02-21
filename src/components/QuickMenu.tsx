import React, { useState, useCallback, useRef } from 'react';
import { Sparkles, Target, Bug, Settings as SettingsIcon, Menu, Store } from 'lucide-react';
import { Button } from "./ui/button";

export interface QuickMenuProps {
  onRestartWelcome: () => void;
  onStartTour: () => void;
  onOpenBugReport: () => void;
  onOpenSettings: () => void;
  onOpenAppStore: () => void;
  showRestartWelcome?: boolean;
}

export function QuickMenu({ 
  onRestartWelcome, 
  onStartTour, 
  onOpenBugReport, 
  onOpenSettings,
  onOpenAppStore,
  showRestartWelcome = false
}: QuickMenuProps) {
  const [isOpen, setIsOpen] = useState(false);
  const closeTimeoutRef = useRef<number>();

  const handleMouseEnter = useCallback(() => {
    if (closeTimeoutRef.current) {
      window.clearTimeout(closeTimeoutRef.current);
    }
    setIsOpen(true);
  }, []);

  const handleMouseLeave = useCallback(() => {
    closeTimeoutRef.current = window.setTimeout(() => {
      setIsOpen(false);
    }, 150); // Small delay before closing
  }, []);

  return (
    <div 
      className="fixed bottom-4 left-4 z-40"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <div className="relative">
        {/* Menu items - positioned above the button */}
        <div 
          className={`absolute bottom-[calc(100%+2px)] left-0 min-w-[150px] ${
            isOpen ? 'opacity-100' : 'opacity-0 pointer-events-none'
          } transition-all duration-200`}
        >
          <div className="bg-white/90 dark:bg-darkBgHighlight backdrop-blur-sm rounded-lg shadow-lg p-1 space-y-0.5">
            {showRestartWelcome && (
              <Button
                onClick={onRestartWelcome}
                variant="ghost"
                size="sm"
                className="w-full justify-start hover:bg-gray-100 dark:hover:bg-blueShadow"
              >
                <Sparkles className="w-4 h-4 mr-2" />
                Restart Welcome
              </Button>
            )}
            <Button
              onClick={onStartTour}
              variant="ghost"
              size="sm"
              className="w-full justify-start hover:bg-gray-100 dark:hover:bg-blueShadow"
            >
              <Target className="w-4 h-4 mr-2" />
              Take Tour
            </Button>
            <Button
              onClick={onOpenAppStore}
              variant="ghost"
              size="sm"
              className="w-full justify-start hover:bg-gray-100 dark:hover:bg-blueShadow"
            >
              <Store className="w-4 h-4 mr-2" />
              App Store
            </Button>
            <Button
              onClick={onOpenBugReport}
              variant="ghost"
              size="sm"
              className="w-full justify-start hover:bg-gray-100 dark:hover:bg-blueShadow"
            >
              <Bug className="w-4 h-4 mr-2" />
              Report a Bug
            </Button>
            <Button
              onClick={onOpenSettings}
              variant="ghost"
              size="sm"
              className="w-full justify-start hover:bg-gray-100 dark:hover:bg-blueShadow"
            >
              <SettingsIcon className="w-4 h-4 mr-2" />
              Settings
            </Button>
          </div>
        </div>

        {/* Main menu button */}
        <Button
          variant="outline"
          size="sm"
          className="dark:bg-darkBgHighlight bg-white/90 backdrop-blur-sm border-gray-200 dark:border-darkBgHighlight hover:bg-gray-50 dark:hover:bg-blueShadow shadow-lg"
        >
          <Menu className="w-4 h-4 mr-2" />
          Quick Menu
        </Button>
      </div>
    </div>
  );
} 

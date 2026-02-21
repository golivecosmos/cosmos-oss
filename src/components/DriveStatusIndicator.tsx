import React from 'react';
import { HardDrive, MapPin, Wifi, WifiOff } from 'lucide-react';
import { cn } from '../lib/utils';

interface DriveStatusIndicatorProps {
  driveUuid?: string | null;
  driveName?: string | null;
  driveCustomName?: string | null;
  drivePhysicalLocation?: string | null;
  driveStatus?: string | null;
  size?: 'sm' | 'md' | 'lg';
  showLocation?: boolean;
}

export function DriveStatusIndicator({ 
  driveUuid,
  driveName,
  driveCustomName,
  drivePhysicalLocation,
  driveStatus,
  size = 'sm',
  showLocation = false
}: DriveStatusIndicatorProps) {
  // If no drive information, return null (local file)
  if (!driveUuid || !driveName) {
    return null;
  }

  const displayName = driveCustomName || driveName;
  const isOnline = driveStatus === 'connected' || driveStatus === 'indexing';
  
  const sizeClasses = {
    sm: 'text-xs px-2 py-1',
    md: 'text-sm px-3 py-1.5',
    lg: 'text-base px-4 py-2'
  };

  const iconSizes = {
    sm: 'w-3 h-3',
    md: 'w-4 h-4',
    lg: 'w-5 h-5'
  };

  return (
    <div className={cn(
      "inline-flex items-center space-x-1 rounded-full border",
      sizeClasses[size],
      isOnline 
        ? "bg-green-50 text-green-700 border-green-200 dark:bg-green-900/30 dark:text-green-400 dark:border-green-700"
        : "bg-orange-50 text-orange-700 border-orange-200 dark:bg-orange-900/30 dark:text-orange-400 dark:border-orange-700"
    )}>
      {isOnline ? (
        <Wifi className={iconSizes[size]} />
      ) : (
        <WifiOff className={iconSizes[size]} />
      )}
      
      <HardDrive className={iconSizes[size]} />
      <span className="font-medium truncate max-w-24">{displayName}</span>
      
      {showLocation && drivePhysicalLocation && size !== 'sm' && (
        <>
          <span className="text-gray-400 dark:text-gray-500">•</span>
          <MapPin className={iconSizes[size]} />
          <span className="truncate max-w-20">{drivePhysicalLocation}</span>
        </>
      )}
    </div>
  );
}
import React from 'react';
import { 
  HardDrive, 
  MapPin, 
  Clock, 
  FileIcon, 
  AlertTriangle,
  Database
} from 'lucide-react';
import { cn } from '../lib/utils';

interface OfflineDriveCardProps {
  file: {
    id: string;
    file_path: string;
    metadata: string;
    score: number;
    drive_uuid?: string | null;
    drive_name?: string | null;
    drive_custom_name?: string | null;
    drive_physical_location?: string | null;
    drive_status?: string | null;
    mime_type?: string | null;
    created_at: string;
    timestamp?: number | null;
    video_duration?: number | null;
  };
  onClick?: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);
  
  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

function getFileIcon(mimeType: string | null | undefined) {
  if (!mimeType) return FileIcon;
  
  if (mimeType.startsWith('image/')) return '🖼️';
  if (mimeType.startsWith('video/')) return '🎬';
  if (mimeType.startsWith('audio/')) return '🎵';
  if (mimeType.includes('pdf')) return '📄';
  return '📄';
}

export function OfflineDriveCard({ file, onClick }: OfflineDriveCardProps) {
  // Decode URL-encoded file path
  let decodedFilePath = file.file_path;
  try {
    if (file.file_path.includes('%')) {
      decodedFilePath = decodeURIComponent(file.file_path);
    }
  } catch (e) {
    decodedFilePath = file.file_path;
  }
  
  const fileName = decodedFilePath.split('/').pop() || 'Unknown file';
  const driveName = file.drive_custom_name || file.drive_name || 'Unknown Drive';
  const isOffline = file.drive_status === 'disconnected';
  
  // Parse metadata to get file size and other info
  let fileSize = 0;
  let dimensions = null;
  try {
    const metadata = JSON.parse(file.metadata);
    fileSize = metadata.fs_size || 0;
    if (metadata.dimensions) {
      dimensions = `${metadata.dimensions.width}×${metadata.dimensions.height}`;
    }
  } catch (e) {
    // Ignore parsing errors
  }

  return (
    <div 
      className={cn(
        "border rounded-lg p-4 space-y-3 cursor-pointer transition-all duration-200",
        "hover:shadow-md border-orange-200 bg-orange-50/30 dark:border-orange-700 dark:bg-orange-900/20",
        onClick && "hover:bg-orange-50/50 dark:hover:bg-orange-900/30"
      )}
      onClick={onClick}
    >
      {/* Header with file info */}
      <div className="flex items-start justify-between">
        <div className="flex items-center space-x-3 flex-1 min-w-0">
          <div className="text-2xl">
            {(() => {
              const icon = getFileIcon(file.mime_type);
              if (typeof icon === 'string') {
                return <span>{icon}</span>;
              }
              return <FileIcon className="w-6 h-6 text-gray-400 dark:text-gray-500" />;
            })()}
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-medium text-sm truncate text-gray-900 dark:text-gray-100">{fileName}</h3>
            <p className="text-xs text-gray-500 dark:text-gray-400 truncate">{decodedFilePath}</p>
          </div>
        </div>
        
        {/* Offline indicator */}
        <div className="flex items-center space-x-1 text-orange-600 bg-orange-100 dark:text-orange-400 dark:bg-orange-900/40 px-2 py-1 rounded-full text-xs">
          <AlertTriangle className="w-3 h-3" />
          <span>Drive Offline</span>
        </div>
      </div>

      {/* File metadata */}
      <div className="grid grid-cols-2 gap-2 text-xs text-gray-600 dark:text-gray-400">
        {fileSize > 0 && (
          <div className="flex items-center space-x-1">
            <Database className="w-3 h-3" />
            <span>{formatBytes(fileSize)}</span>
          </div>
        )}
        
        {dimensions && (
          <div className="flex items-center space-x-1">
            <span>📐</span>
            <span>{dimensions}</span>
          </div>
        )}
        
        {file.video_duration && (
          <div className="flex items-center space-x-1">
            <Clock className="w-3 h-3" />
            <span>{formatDuration(file.video_duration)}</span>
          </div>
        )}
        
        {file.timestamp && (
          <div className="flex items-center space-x-1">
            <span>⏱️</span>
            <span>Frame at {formatDuration(file.timestamp)}</span>
          </div>
        )}
      </div>

      {/* Drive information */}
      <div className="border-t pt-3 space-y-2">
        <div className="flex items-center space-x-2 text-sm">
          <HardDrive className="w-4 h-4 text-orange-500 dark:text-orange-400" />
          <span className="font-medium text-gray-900 dark:text-gray-100">{driveName}</span>
        </div>
        
        {file.drive_physical_location && (
          <div className="flex items-center space-x-2 text-xs text-gray-600 dark:text-gray-400">
            <MapPin className="w-3 h-3" />
            <span>{file.drive_physical_location}</span>
          </div>
        )}
        
        <div className="bg-orange-100 border border-orange-200 dark:bg-orange-900/40 dark:border-orange-700 rounded p-2 text-xs">
          <p className="text-orange-800 dark:text-orange-300 font-medium">
            🔌 Connect "{driveName}" to access this file
          </p>
          {file.drive_physical_location && (
            <p className="text-orange-600 dark:text-orange-400 mt-1">
              Look for the drive at: {file.drive_physical_location}
            </p>
          )}
        </div>
      </div>

      {/* Match score */}
      <div className="border-t dark:border-gray-700 pt-2">
        <div className="flex justify-between items-center text-xs text-gray-500 dark:text-gray-400">
          <span>Relevance Score</span>
          <span className="font-mono">{(file.score * 100).toFixed(1)}%</span>
        </div>
        <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1 mt-1">
          <div 
            className="bg-orange-400 dark:bg-orange-500 h-1 rounded-full transition-all duration-300"
            style={{ width: `${Math.min(file.score * 100, 100)}%` }}
          />
        </div>
      </div>
    </div>
  );
}
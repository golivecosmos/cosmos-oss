import React from 'react';
import { Folder } from 'lucide-react';
import { MediaFile } from './types';

interface FolderPreviewProps {
  folder: MediaFile;
  onSelect: () => void;
}

export function FolderPreview({ folder, onSelect }: FolderPreviewProps) {
  return (
    <div 
      className="w-full h-full bg-gradient-to-br dark:from-darkBg dark:to-darkBg from-gray-50 to-gray-100 flex flex-col items-center justify-center p-4 cursor-pointer transition-colors"
      onClick={onSelect}
    >
      <Folder className="h-12 w-12 dark:text-customBlue text-blue-500 mb-2" />
      <span className="text-sm dark:text-text text-gray-600 text-center truncate w-full">{folder.name}</span>
    </div>
  );
} 
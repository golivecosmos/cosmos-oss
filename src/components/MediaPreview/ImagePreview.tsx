import React, { useState, useEffect } from 'react';
import { MediaFile } from './types';
import { Image, AlertCircle } from 'lucide-react';
import { cn } from '../../lib/utils';

interface ImagePreviewProps {
  file: MediaFile;
}

export function ImagePreview({ file }: ImagePreviewProps) {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(false);
  const [errorDetails, setErrorDetails] = useState<string>('');

  // Check if this is a directory or not an image
  const isInvalidImage = file.type !== 'image' || !!file.metadata.isDirectory;

  useEffect(() => {
    // Reset state when file changes
    setIsLoading(true);
    setError(false);
    setErrorDetails('');
    
    // If this is a directory or not an image, set error immediately
    if (isInvalidImage) {
      console.error(`Invalid image: ${file.path} (type: ${file.type}, isDirectory: ${file.metadata.isDirectory ? 'true' : 'false'})`);
      setError(true);
      setErrorDetails(`Invalid image type: ${file.type}`);
      setIsLoading(false);
      return;
    }
    
    // Validate the path is actually usable
    if (!file.path || file.path === 'undefined' || file.path === 'null') {
      console.error(`Invalid image path: ${file.path}`);
      setError(true);
      setErrorDetails(`Invalid image path: ${file.path}`);
      setIsLoading(false);
    }
  }, [file.path, file.type, isInvalidImage]);

  const handleLoad = () => {
    setIsLoading(false);
  };

  const handleError = (e: React.SyntheticEvent<HTMLImageElement, Event>) => {
    console.error(`Image load error for ${file.path}`, e);
    setError(true);
    setErrorDetails(`Failed to load image. Path: ${file.path.substring(0, 100)}...`);
    setIsLoading(false);
  };

  return (
    <div className="w-full h-full dark:bg-darkBg bg-gray-50 rounded-lg overflow-hidden relative">
      {isLoading && !isInvalidImage && (
        <div className="absolute inset-0 flex items-center justify-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 dark:border-blueHighlight border-blue-500" />
        </div>
      )}
      
      {error ? (
        <div className="absolute inset-0 flex items-center justify-center">
          <div className="text-center">
            {isInvalidImage ? (
              <AlertCircle className="h-12 w-12 text-amber-500 mx-auto mb-2" />
            ) : (
              <Image className="h-12 w-12 text-gray-400 mx-auto mb-2" />
            )}
            <div className="text-sm text-gray-500">
              {isInvalidImage ? 'Not a valid image file' : 'Failed to load image'}
            </div>
            <div className="text-xs text-gray-400 mt-1">{errorDetails || file.path}</div>
            <div className="text-xs text-gray-400 mt-1">Type: {file.type}, Name: {file.name}</div>
          </div>
        </div>
      ) : (
        <>
          {!isInvalidImage && (
            <img
              src={file.path}
              alt={file.name}
              className={cn(
                "w-full h-full object-contain transition-opacity duration-200",
                isLoading ? "opacity-0" : "opacity-100"
              )}
              onLoad={handleLoad}
              onError={handleError}
            />
          )}
          {/* Show path info in debug mode */}
          {/* {process.env.NODE_ENV === 'development' && (
            <div className="absolute bottom-0 left-0 right-0 bg-black bg-opacity-50 text-white text-xs p-1 overflow-hidden text-ellipsis">
              {file.name} ({file.path.substring(0, 30)}...) - {file.type}
            </div>
          )} */}
        </>
      )}
    </div>
  );
} 
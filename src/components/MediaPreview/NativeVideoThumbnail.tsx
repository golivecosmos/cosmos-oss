import { useState, useEffect } from 'react';
import { MediaFile } from './types';
import { Play, Video } from 'lucide-react';
import { getCachedThumbnail } from '../../utils/thumbnailService';

interface VideoThumbnailProps {
  file: MediaFile;
  onClick?: () => void;
}

export function NativeVideoThumbnail({ file, onClick }: VideoThumbnailProps) {
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [hasError, setHasError] = useState(false);

  useEffect(() => {
    let isCancelled = false;

    const loadThumbnail = async () => {
      try {
        setIsLoading(true);
        setHasError(false);

        // Determine timestamp from metadata
        let timestamp = 1.0; // default timestamp

        // For video frames, use the specific timestamp from the search result
        if (file.metadata.isVideoFrame && typeof file.metadata.timestamp === 'number') {
          timestamp = file.metadata.timestamp;
        }
        // For regular videos, use timestamp if available
        else if (typeof file.metadata.timestamp === 'number') {
          timestamp = file.metadata.timestamp;
        }

        const url = await getCachedThumbnail(file.path, {
          timestamp,
          width: 480,
          height: 270,
        });

        if (!isCancelled) {
          setThumbnailUrl(url);
        }
      } catch (error) {
        console.error('Failed to load thumbnail:', error);
        if (!isCancelled) {
          setHasError(true);
        }
      } finally {
        if (!isCancelled) {
          setIsLoading(false);
        }
      }
    };

    loadThumbnail();

    return () => {
      isCancelled = true;
    };
  }, [file.path, file.metadata.timestamp]);

  return (
    <div
      className="relative w-full h-full bg-gray-900 group cursor-pointer overflow-hidden"
      onClick={onClick}
    >
      {/* Static thumbnail image */}
      {thumbnailUrl && !hasError && (
        <img
          src={thumbnailUrl}
          className="w-full h-full object-cover bg-gray-900"
          alt="Video thumbnail"
        />
      )}

      {/* Loading state */}
      {isLoading && (
        <div className="absolute inset-0 flex items-center justify-center bg-gray-900">
          <div className="animate-pulse">
            <Video className="h-12 w-12 text-gray-700" />
          </div>
        </div>
      )}

      {/* Error state */}
      {hasError && !isLoading && (
        <div className="absolute inset-0 flex items-center justify-center bg-gray-900">
          <Video className="h-12 w-12 text-gray-600" />
        </div>
      )}

      {/* Play icon overlay */}
      {thumbnailUrl && !hasError && (
        <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
          <div className="bg-black/40 group-hover:bg-black/60 rounded-full p-3 transition-all duration-200 group-hover:scale-110 backdrop-blur-sm">
            <Play className="h-6 w-6 text-white fill-white" />
          </div>
        </div>
      )}

      {/* Hover overlay */}
      <div className="absolute inset-0 bg-black/0 group-hover:bg-black/10 transition-all duration-200 pointer-events-none" />
    </div>
  );
}

import React, { useState, useRef } from 'react';
import { MediaFile } from './types';
import { Play, Video } from 'lucide-react';

interface VideoThumbnailProps {
  file: MediaFile;
  onClick?: () => void;
}

export function VideoThumbnail({ file, onClick }: VideoThumbnailProps) {
  const [hasError, setHasError] = useState(false);
  const [thumbnailReady, setThumbnailReady] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const videoRef = useRef<HTMLVideoElement>(null);

  const videoPath = file.path;

  // Handle initial frame display
  React.useEffect(() => {
    if (videoRef.current && thumbnailReady) {
      const video = videoRef.current;

      // For video frames, ensure we're at the right timestamp
      if (file.metadata.isVideoFrame &&
          file.metadata.timestamp !== undefined &&
          file.metadata.timestamp !== null) {
        video.currentTime = file.metadata.timestamp as number;
      }
    }
  }, [thumbnailReady, file.metadata.isVideoFrame, file.metadata.timestamp]);

  // Handle video load
  const handleLoadedData = () => {
    if (videoRef.current) {
      const video = videoRef.current;

      // For video frames, seek to timestamp
      if (file.metadata.isVideoFrame &&
          file.metadata.timestamp !== undefined &&
          file.metadata.timestamp !== null) {
        video.currentTime = file.metadata.timestamp as number;
      }

      setIsLoading(false);
      setThumbnailReady(true);
    }
  };

  return (
    <div
      className="relative w-full h-full bg-gray-900 group cursor-pointer overflow-hidden"
      onClick={onClick}
    >
      {/* Video element for thumbnail */}
      <video
        ref={videoRef}
        src={videoPath}
        className={`w-full h-full object-cover transition-opacity duration-300 ${
          thumbnailReady ? 'opacity-100' : 'opacity-0'
        }`}
        preload="auto"
        muted
        playsInline
        onLoadedData={handleLoadedData}
        onError={() => {
          console.error('VideoThumbnail: Failed to load video', videoPath);
          setHasError(true);
          setIsLoading(false);
          setThumbnailReady(true); // Show error state
        }}
      />

      {/* Loading/Error fallback */}
      {(isLoading || hasError) && (
        <div className="absolute inset-0 flex items-center justify-center bg-gray-900">
          {hasError ? (
            <Video className="h-12 w-12 text-gray-600" />
          ) : (
            <div className="animate-pulse">
              <Video className="h-12 w-12 text-gray-700" />
            </div>
          )}
        </div>
      )}

      {/* Play icon overlay - only show when thumbnail is ready */}
      {thumbnailReady && (
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

import React from "react";
import { Loader2 } from "lucide-react";
import { VideoCard } from "./VideoCard";
import { VideoGeneration } from "./types";

interface VideoGridProps {
  existingVideos: VideoGeneration[];
  generatedVideos: VideoGeneration[];
  isLoadingExisting: boolean;
  onVideoSelect: (video: VideoGeneration) => void;
}

export const VideoGrid: React.FC<VideoGridProps> = ({
  existingVideos,
  generatedVideos,
  isLoadingExisting,
  onVideoSelect,
}) => {
  return (
    <div className="space-y-4 px-6">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-700 dark:text-gray-300">
          Your Videos
        </h3>
        {isLoadingExisting && (
          <div className="flex items-center space-x-2">
            <Loader2 className="w-4 h-4 animate-spin" />
            <span className="text-sm text-gray-500">Loading...</span>
          </div>
        )}
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
        {/* Show existing videos first */}
        {existingVideos.map((video) => (
          <VideoCard
            key={video.id}
            video={video}
            onClick={() => onVideoSelect(video)}
          />
        ))}

        {/* Show generated videos */}
        {generatedVideos.map((video) => (
          <VideoCard
            key={video.id}
            video={video}
            onClick={() => onVideoSelect(video)}
          />
        ))}
      </div>
    </div>
  );
}; 
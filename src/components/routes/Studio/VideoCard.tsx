import React from "react";
import { Play, Loader2, AlertCircle } from "lucide-react";
import { VideoGeneration } from "./types";
import { cn } from "../../../lib/utils";

interface VideoCardProps {
  video: VideoGeneration;
  onClick: () => void;
}

export const VideoCard: React.FC<VideoCardProps> = ({ video, onClick }) => {
  const isCompleted = video.status === 'completed' && video.videoUrl;

  const handleOnVideoClick = () => {
    if (isCompleted) {
      onClick();
    }
  }

  return (
    <div
      className={cn(
        "flex-shrink-0 w-64 cursor-pointer group",
        isCompleted ? "opacity-100" : "opacity-60 cursor-not-allowed"
      )}
      onClick={handleOnVideoClick}
    >
      <div className="bg-white dark:bg-darkBgHighlight border border-gray-200 dark:border-gray-700 overflow-hidden hover:shadow-lg transition-all rounded-xl">
        <div className="aspect-video bg-gray-100 dark:bg-gray-800 relative">
          {video.status === 'completed' && video.videoUrl ? (
            <video
              src={video.videoUrl}
              className="w-full h-full object-cover"
            />
          ) : video.status === 'generating' ? (
            <div className="w-full h-full flex items-center justify-center bg-gray-100 dark:bg-gray-800">
              <Loader2 className="w-8 h-8 animate-spin text-purple-600 dark:text-purple-400" />
            </div>
          ) : (
            <div className="w-full h-full flex items-center justify-center bg-gray-100 dark:bg-gray-800">
              <AlertCircle className="w-8 h-8 text-red-500" />
            </div>
          )}
          <div className="absolute inset-0 flex items-center justify-center bg-black/20 opacity-0 group-hover:opacity-100 transition-opacity">
            <Play className="w-8 h-8 text-white" />
          </div>
        </div>
        <div className="p-3">
          <h4 className="text-sm font-medium text-gray-900 dark:text-white truncate">
            {video.prompt.substring(0, 40)}...
          </h4>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
            {video.duration || 8}s • {video.createdAt.toLocaleDateString()}
          </p>
        </div>
      </div>
    </div>
  );
}; 
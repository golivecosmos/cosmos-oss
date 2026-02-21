import { useState, useRef, useImperativeHandle, forwardRef } from "react";

import { FileItem } from "../../FileTree";
import { isSupportedVideoExtension, isSupportedImageExtension } from "../../../constants";
import { AudioPreview } from "../../MediaPreview/AudioPreview";
import { DocumentPreview } from "../../MediaPreview/DocumentPreview";
import { VideoPlayerWithTrim, VideoPlayerWithTrimRef } from "../../VideoEditor/VideoPlayerWithTrim";

interface ContentPreviewProps {
  file: FileItem & { normalizedPath: string } | null;
}

export interface ContentPreviewRef {
  seekTo: (timestamp: number) => void;
}

export const ContentPreview = forwardRef<ContentPreviewRef, ContentPreviewProps>(({ file }, ref) => {
  const [isVideoLoading, setIsVideoLoading] = useState(true);
  const videoPlayerRef = useRef<VideoPlayerWithTrimRef>(null);
  const audioRef = useRef<HTMLAudioElement>(null);

  useImperativeHandle(ref, () => ({
    seekTo: (timestamp: number) => {
      if (videoPlayerRef.current) {
        videoPlayerRef.current.seekTo(timestamp);
      } else if (audioRef.current) {
        audioRef.current.currentTime = timestamp;
      }
    }
  }));
  if (!file) {
    return (
      <div className="flex items-center justify-center text-muted-foreground">
        No file selected
      </div>
    );
  }

  const fileExtension = file.path.split('.').pop()?.toLowerCase() || '';
  const isVideo = isSupportedVideoExtension(fileExtension);
  const isImage = isSupportedImageExtension(fileExtension);
  const isAudio = ['mp3', 'wav', 'ogg', 'flac', 'aac', 'm4a'].includes(fileExtension);
  const isDocument = ['pdf', 'doc', 'docx', 'txt', 'rtf', 'md'].includes(fileExtension);

  const handleVideoCanPlay = async () => {
    setIsVideoLoading(false);
  };

  if (isVideo) {
    return (
      <div className="relative aspect-video w-full max-w-[90vw] bg-transparent rounded-xl shadow-2xl overflow-hidden">
        <VideoPlayerWithTrim
          ref={videoPlayerRef}
          src={file.normalizedPath}
          filePath={file.path}
          className="w-full h-full"
        />
      </div>
    );
  }

  if (isAudio) {
    return (
      <div className="w-full max-w-3xl h-full flex items-center justify-center">
        <AudioPreview
          file={{
            path: file.normalizedPath,
            name: file.name,
            type: 'audio',
            metadata: {
              size: file.size,
              modified: file.modified,
              isIndexed: false,
            }
          }}
          isTranscribing={false}
        />
      </div>
    );
  }

  if (isDocument) {
    return (
      <div className="w-full max-w-5xl mx-auto flex flex-col overflow-hidden bg-gray-50 dark:bg-darkBg rounded-xl shadow-2xl">
        <DocumentPreview
          file={{
            path: file.normalizedPath,
            name: file.name,
            type: 'document',
            metadata: {
              size: file.size,
              modified: file.modified,
              isIndexed: false,
            }
          }}
        />
      </div>
    );
  }

  if (isImage) {
    return (
      <div className="relative w-full max-w-[90vw] max-h-[80vh] flex items-center justify-center">
        <img
          src={file.normalizedPath}
          alt={file.name}
          className="max-w-full max-h-full object-contain rounded-xl shadow-2xl"
          draggable={false}
        />
      </div>
    );
  }

  // Default fallback for unknown file types
  return (
    <div className="relative w-full max-w-[90vw] max-h-[80vh] flex items-center justify-center">
      <img
        src={file.normalizedPath}
        alt={file.name}
        className="max-w-full max-h-full object-contain rounded-xl shadow-2xl"
        draggable={false}
      />
    </div>
  );
});

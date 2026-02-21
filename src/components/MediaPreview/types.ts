export type ViewMode = 'grid' | 'list';

export interface MediaFile {
  path: string;
  name: string;
  type: 'image' | 'video' | 'audio' | 'document' | 'directory' | 'grouped_video' | 'reference_image';
  originalIndex?: number;
  score?: number;
  metadata: {
    size?: number;
    modified?: string;
    created?: string;
    lastIndexed?: string | null;
    mimeType?: string | null;
    parentPath?: string | null;
    tags?: string | null;
    isDirectory?: boolean;
    score?: number;
    searchOrder?: number;
    isReferenceImage?: boolean;
    isIndexed?: boolean;
    
    // Video group specific
    frameCount?: number;
    bestMatchTimestamp?: string | number;
    bestMatchFrame?: string;
    isVideo?: boolean;
    isGroupedVideo?: boolean;
    sourceType?: string;
    
    // Video frame specific
    isVideoFrame?: boolean;
    timestamp?: number | null;
    timestampFormatted?: string | null;
    frameNumber?: number | null;
    videoDuration?: number | null;
    
    // Image dimensions
    dimensions?: {
      width: number;
      height: number;
    };
    
    // Drive information
    driveUuid?: string | null;
    driveName?: string | null;
    driveCustomName?: string | null;
    drivePhysicalLocation?: string | null;
    driveStatus?: string | null;
  };
}

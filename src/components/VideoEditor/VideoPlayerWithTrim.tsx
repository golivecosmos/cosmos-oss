import React, { useState, useRef, useEffect, useCallback, useImperativeHandle, forwardRef } from 'react';
import { cn } from '../../lib/utils';
import { Button } from '../ui/button';
import { Play, Pause, SkipBack, SkipForward, Scissors, Download, X, Monitor, Expand } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';

interface VideoPlayerWithTrimProps {
  src: string;
  filePath: string;
  className?: string;
}

export interface VideoPlayerWithTrimRef {
  seekTo: (timestamp: number) => void;
}

export const VideoPlayerWithTrim = forwardRef<VideoPlayerWithTrimRef, VideoPlayerWithTrimProps>(({ src, filePath, className }, ref) => {
  const videoRef = useRef<HTMLVideoElement>(null);
  const timelineRef = useRef<HTMLDivElement>(null);

  useImperativeHandle(ref, () => ({
    seekTo: (timestamp: number) => {
      if (videoRef.current) {
        videoRef.current.currentTime = timestamp;
      }
    }
  }));
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [showTrimMode, setShowTrimMode] = useState(false);
  const [trimIn, setTrimIn] = useState<number | null>(null);
  const [trimOut, setTrimOut] = useState<number | null>(null);
  const [showResizeMode, setShowResizeMode] = useState(false);
  const [selectedResolution, setSelectedResolution] = useState<string>('original');
  const [selectedAspectRatio, setSelectedAspectRatio] = useState<string>('original');
  const [aspectMode, setAspectMode] = useState<'crop' | 'pad'>('crop');
  const [cropPosition, setCropPosition] = useState({ x: 0, y: 0 }); // Crop position offset (0 = center, -1 to 1 range)
  const [isDragging, setIsDragging] = useState<'timeline' | 'in' | 'out' | 'crop' | null>(null);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0, initialCropX: 0, initialCropY: 0 });
  const [isExporting, setIsExporting] = useState(false);
  const [showControls, setShowControls] = useState(true);
  const [videoDimensions, setVideoDimensions] = useState({ width: 0, height: 0 });
  const controlsTimeoutRef = useRef<NodeJS.Timeout>();

  // Format time helper
  const formatTime = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  // Show controls on mouse move
  const handleControlsVisibility = () => {
    setShowControls(true);
    if (controlsTimeoutRef.current) {
      clearTimeout(controlsTimeoutRef.current);
    }
    controlsTimeoutRef.current = setTimeout(() => {
      if (isPlaying) {
        setShowControls(false);
      }
    }, 3000);
  };

  useEffect(() => {
    return () => {
      if (controlsTimeoutRef.current) {
        clearTimeout(controlsTimeoutRef.current);
      }
    };
  }, []);

  // Video event handlers
  const handleLoadedMetadata = () => {
    if (videoRef.current) {
      const videoDuration = videoRef.current.duration;
      const videoWidth = videoRef.current.videoWidth;
      const videoHeight = videoRef.current.videoHeight;
      console.log('Video metadata loaded:', { 
        duration: videoDuration, 
        width: videoWidth, 
        height: videoHeight 
      });
      setDuration(videoDuration);
      setCurrentTime(videoRef.current.currentTime);
      setVideoDimensions({ width: videoWidth, height: videoHeight });
    }
  };

  const handleTimeUpdate = () => {
    if (videoRef.current) {
      setCurrentTime(videoRef.current.currentTime);
    }
  };

  const togglePlayPause = () => {
    if (videoRef.current) {
      if (isPlaying) {
        videoRef.current.pause();
      } else {
        videoRef.current.play();
      }
      setIsPlaying(!isPlaying);
    }
  };

  // Timeline interaction
  const getTimeFromPosition = (clientX: number): number => {
    if (!timelineRef.current) return 0;
    const rect = timelineRef.current.getBoundingClientRect();
    const position = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    return position * duration;
  };

  const handleTimelineClick = (e: React.MouseEvent) => {
    const time = getTimeFromPosition(e.clientX);
    if (videoRef.current) {
      videoRef.current.currentTime = time;
      setCurrentTime(time);
    }
  };

  const handleTimelineMouseDown = (e: React.MouseEvent) => {
    // Handle timeline dragging for seeking
    e.preventDefault();
    const time = getTimeFromPosition(e.clientX);
    if (videoRef.current) {
      videoRef.current.currentTime = time;
      setCurrentTime(time);
    }
    setIsDragging('timeline');
  };

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging) return;
    
    if (isDragging === 'crop' && videoRef.current) {
      // Handle crop dragging - track relative movement from drag start
      const rect = videoRef.current.getBoundingClientRect();
      
      // Calculate movement delta from start position
      const deltaX = (e.clientX - dragStart.x) / (rect.width / 4); // Normalized movement
      const deltaY = (e.clientY - dragStart.y) / (rect.height / 4); // Normalized movement
      
      // Add delta to initial position
      const newX = Math.max(-1, Math.min(1, dragStart.initialCropX + deltaX));
      const newY = Math.max(-1, Math.min(1, dragStart.initialCropY + deltaY));
      
      console.log('Crop drag:', {
        mouseX: e.clientX,
        startX: dragStart.x,
        deltaX,
        initialX: dragStart.initialCropX,
        newX,
        rectWidth: rect.width
      });
      
      setCropPosition({ x: newX, y: newY });
    } else {
      // Handle timeline dragging
      const time = getTimeFromPosition(e.clientX);
      
      if (isDragging === 'timeline') {
        if (videoRef.current) {
          videoRef.current.currentTime = time;
        }
      } else if (isDragging === 'in') {
        setTrimIn(Math.min(time, trimOut ?? duration));
      } else if (isDragging === 'out') {
        setTrimOut(Math.max(time, trimIn ?? 0));
      }
    }
  }, [isDragging, trimIn, trimOut, duration, dragStart]);

  const handleMouseUp = useCallback(() => {
    setIsDragging(null);
  }, []);

  useEffect(() => {
    if (isDragging) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };
    }
  }, [isDragging, handleMouseMove, handleMouseUp]);

  // Trim controls
  const setInPoint = () => {
    const time = videoRef.current?.currentTime ?? currentTime;
    const videoDuration = videoRef.current?.duration ?? duration;
    
    console.log('Setting In Point:', time, 'Duration:', videoDuration, 'Current trimOut:', trimOut);
    
    if (trimOut !== null && time >= trimOut) {
      // If trying to set in point after out point, don't allow it
      console.log('In point cannot be after or equal to out point');
      return;
    }
    
    setTrimIn(time);
    if (trimOut === null && videoDuration > 0) {
      setTrimOut(videoDuration);
    }
  };

  const setOutPoint = () => {
    const time = videoRef.current?.currentTime ?? currentTime;
    
    console.log('Setting Out Point:', time, 'Current trimIn:', trimIn);
    
    if (trimIn === null) {
      // If no in point set, set in point to 0 and out point to current time
      setTrimIn(0);
      setTrimOut(time);
    } else if (time <= trimIn) {
      // If trying to set out point before or at in point, don't allow it
      console.log('Out point cannot be before or equal to in point');
      return;
    } else {
      // Normal case: set out point to current time
      setTrimOut(time);
    }
  };

  const clearTrimPoints = () => {
    setTrimIn(null);
    setTrimOut(null);
  };

  const resetCropPosition = () => {
    setCropPosition({ x: 0, y: 0 });
  };

  // Simple helper to parse aspect ratio string
  const parseAspectRatio = (ratio: string): number => {
    if (ratio === 'original') return videoDimensions.width / videoDimensions.height;
    const [w, h] = ratio.split(':').map(Number);
    return w / h;
  };

  // Calculate actual crop dimensions that will fit within video
  const calculateCropDimensions = (targetRatio: number) => {
    const { width: videoW, height: videoH } = videoDimensions;
    if (!videoW || !videoH) return null;

    // Calculate dimensions that fit within the video
    let cropWidth, cropHeight;
    
    // Try to use full width first
    cropWidth = videoW;
    cropHeight = videoW / targetRatio;
    
    // If height exceeds video, use full height instead
    if (cropHeight > videoH) {
      cropHeight = videoH;
      cropWidth = videoH * targetRatio;
    }
    
    return {
      width: Math.floor(cropWidth),
      height: Math.floor(cropHeight),
      maxOffsetX: (videoW - cropWidth) / 2,
      maxOffsetY: (videoH - cropHeight) / 2
    };
  };

  const handleExport = async () => {
    const hasTrim = trimIn !== null && trimOut !== null;
    const hasResize = selectedResolution !== 'original' || selectedAspectRatio !== 'original';
    
    if (!hasTrim && !hasResize) return;
    
    setIsExporting(true);
    try {
      // Get resize dimensions
      let width: number | undefined;
      let height: number | undefined;
      
      if (selectedResolution !== 'original') {
        const resolutions = {
          '4k': { width: 3840, height: 2160 },
          '1080p': { width: 1920, height: 1080 },
          '720p': { width: 1280, height: 720 },
          '480p': { width: 854, height: 480 }
        };
        const res = resolutions[selectedResolution as keyof typeof resolutions];
        if (res) {
          width = res.width;
          height = res.height;
        }
      }
      
      // Calculate actual crop dimensions if cropping
      let cropDimensions = null;
      if (selectedAspectRatio !== 'original' && aspectMode === 'crop') {
        const targetRatio = parseAspectRatio(selectedAspectRatio);
        cropDimensions = calculateCropDimensions(targetRatio);
        if (!cropDimensions) {
          console.error('Cannot calculate crop dimensions - video may be too small for this aspect ratio');
          // TODO: Show user-friendly error message
          return;
        }
        
        // Basic validation - ensure crop isn't too small
        if (cropDimensions.width < 100 || cropDimensions.height < 100) {
          console.error('Crop dimensions too small:', cropDimensions);
          // TODO: Show user-friendly error message
          return;
        }
      }
      
      console.log('Export with:', { 
        cropPosition, 
        aspectRatio: selectedAspectRatio,
        cropDimensions,
        videoDimensions 
      });
      
      const result = await invoke<string>('edit_video', {
        request: {
          input_path: filePath,
          start_time: hasTrim ? trimIn : undefined,
          end_time: hasTrim ? trimOut : undefined,
          preserve_timecodes: false,
          width,
          height,
          maintain_aspect_ratio: true,
          aspect_ratio: selectedAspectRatio !== 'original' ? selectedAspectRatio : undefined,
          aspect_mode: aspectMode,
          crop_x: cropPosition.x,
          crop_y: cropPosition.y,
          // Send actual dimensions for simpler backend processing
          source_width: videoDimensions.width || undefined,
          source_height: videoDimensions.height || undefined,
          crop_width: cropDimensions?.width,
          crop_height: cropDimensions?.height
        }
      });
      console.log('Video edited successfully:', result);
      // TODO: Show success notification
      setShowTrimMode(false);
      setShowResizeMode(false);
      clearTrimPoints();
    } catch (error) {
      console.error('Failed to edit video:', error);
      // TODO: Show error notification
    } finally {
      setIsExporting(false);
    }
  };

  // Skip forward/backward
  const skip = (seconds: number) => {
    if (videoRef.current) {
      videoRef.current.currentTime = Math.max(0, Math.min(duration, currentTime + seconds));
    }
  };

  const timelinePosition = duration > 0 ? (currentTime / duration) * 100 : 0;
  const trimInPosition = trimIn !== null && duration > 0 ? (trimIn / duration) * 100 : 0;
  const trimOutPosition = trimOut !== null && duration > 0 ? (trimOut / duration) * 100 : 100;
  
  const resolutionOptions = [
    { value: 'original', label: 'Original' },
    { value: '4k', label: '4K (3840×2160)' },
    { value: '1080p', label: '1080p (1920×1080)' },
    { value: '720p', label: '720p (1280×720)' },
    { value: '480p', label: '480p (854×480)' }
  ];
  
  const aspectRatioOptions = [
    { value: 'original', label: 'Original' },
    { value: '16:9', label: '16:9 (Widescreen)' },
    { value: '4:3', label: '4:3 (Standard)' },
    { value: '1:1', label: '1:1 (Square)' },
    { value: '9:16', label: '9:16 (Vertical)' }
  ];
  
  // Calculate crop guide dimensions - simplified version
  const getCropGuides = () => {
    if (!showResizeMode || selectedAspectRatio === 'original' || aspectMode !== 'crop') {
      return null;
    }
    
    const targetRatio = parseAspectRatio(selectedAspectRatio);
    const cropDims = calculateCropDimensions(targetRatio);
    if (!cropDims || !videoDimensions.width || !videoDimensions.height) return null;
    
    // Calculate percentage crops based on actual dimensions
    const { width: videoW, height: videoH } = videoDimensions;
    const { width: cropW, height: cropH } = cropDims;
    
    // Calculate crop amounts as percentages
    const totalXCrop = ((videoW - cropW) / videoW) * 100;
    const totalYCrop = ((videoH - cropH) / videoH) * 100;
    
    // Apply position offset (-1 to 1 mapped to crop range)
    // When cropPosition.x is negative (left), show more of left side
    const xOffset = cropPosition.x * (totalXCrop / 2);
    const yOffset = cropPosition.y * (totalYCrop / 2);
    
    const leftCrop = Math.max(0, Math.min(totalXCrop, (totalXCrop / 2) + xOffset));
    const rightCrop = Math.max(0, Math.min(totalXCrop, (totalXCrop / 2) - xOffset));
    const topCrop = Math.max(0, Math.min(totalYCrop, (totalYCrop / 2) - yOffset));
    const bottomCrop = Math.max(0, Math.min(totalYCrop, (totalYCrop / 2) + yOffset));
    
    return {
      top: `${topCrop}%`,
      bottom: `${bottomCrop}%`,
      left: `${leftCrop}%`,
      right: `${rightCrop}%`,
      canDragVertically: totalYCrop > 0,
      canDragHorizontally: totalXCrop > 0,
      dragX: totalXCrop > 0 ? cropPosition.x : 0,
      dragY: totalYCrop > 0 ? cropPosition.y : 0
    };
  };
  
  const cropGuides = getCropGuides();

  return (
    <div 
      className={cn("relative bg-black rounded-lg overflow-hidden group", className)}
      onMouseMove={handleControlsVisibility}
      onMouseEnter={() => setShowControls(true)}
    >
      <video
        ref={videoRef}
        src={src}
        className="w-full h-full"
        onLoadedMetadata={handleLoadedMetadata}
        onTimeUpdate={handleTimeUpdate}
        onPlay={() => setIsPlaying(true)}
        onPause={() => setIsPlaying(false)}
        onClick={togglePlayPause}
      />

      {/* Crop Guide Overlay */}
      {cropGuides && (
        <div className="absolute inset-0 pointer-events-none">
          {/* Top crop area */}
          {cropGuides.top !== '0%' && (
            <div 
              className="absolute top-0 left-0 right-0 bg-black/60 border-b-2 border-red-500/80"
              style={{ height: cropGuides.top }}
            />
          )}
          {/* Bottom crop area */}
          {cropGuides.bottom !== '0%' && (
            <div 
              className="absolute bottom-0 left-0 right-0 bg-black/60 border-t-2 border-red-500/80"
              style={{ height: cropGuides.bottom }}
            />
          )}
          {/* Left crop area */}
          {cropGuides.left !== '0%' && (
            <div 
              className="absolute top-0 bottom-0 left-0 bg-black/60 border-r-2 border-red-500/80"
              style={{ width: cropGuides.left }}
            />
          )}
          {/* Right crop area */}
          {cropGuides.right !== '0%' && (
            <div 
              className="absolute top-0 bottom-0 right-0 bg-black/60 border-l-2 border-red-500/80"
              style={{ width: cropGuides.right }}
            />
          )}
          
          {/* Draggable Crop Handle */}
          <div 
            className="absolute pointer-events-auto cursor-move bg-white/20 border-2 border-white/60 backdrop-blur-sm rounded-lg flex items-center justify-center shadow-lg hover:bg-white/30 transition-colors"
            style={{
              top: '50%',
              left: '50%',
              width: '60px',
              height: '60px',
              transform: `translate(-50%, -50%) translate(${(cropGuides.dragX || 0) * 100}px, ${(cropGuides.dragY || 0) * 100}px)`
            }}
            onMouseDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              setDragStart({
                x: e.clientX,
                y: e.clientY,
                initialCropX: cropPosition.x,
                initialCropY: cropPosition.y
              });
              setIsDragging('crop');
            }}
          >
            <div className="text-white text-xs font-medium select-none">⋮⋮</div>
          </div>
        </div>
      )}

      {/* Pad Guide Overlay */}
      {showResizeMode && selectedAspectRatio !== 'original' && aspectMode === 'pad' && (
        <div className="absolute inset-0 pointer-events-none">
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-black/70 text-white px-3 py-1 rounded text-sm font-medium">
            Pad to {selectedAspectRatio} (Black bars will be added)
          </div>
        </div>
      )}

      {/* Video Controls Overlay */}
      <div className={cn(
        "absolute inset-0 bg-gradient-to-t from-black/70 via-transparent to-transparent pointer-events-none transition-opacity duration-300",
        showControls ? "opacity-100" : "opacity-0"
      )}>
        {/* Top bar with mode indicators */}
        {(showTrimMode || showResizeMode) && (
          <div className="absolute top-4 left-4 right-4 flex justify-between items-center pointer-events-auto">
            <div className="bg-black/50 backdrop-blur-sm rounded-lg px-4 py-2 flex items-center gap-3">
              {showTrimMode && showResizeMode && (
                <span className="text-white text-sm font-medium">Edit Mode</span>
              )}
              {showTrimMode && !showResizeMode && (
                <span className="text-white text-sm font-medium">Trim Mode</span>
              )}
              {!showTrimMode && showResizeMode && (
                <span className="text-white text-sm font-medium">Resize Mode</span>
              )}
              
              {/* Trim info */}
              {showTrimMode && trimIn !== null && trimOut !== null && (
                <>
                  <span className="text-white/70 text-sm">
                    Trim: {formatTime(trimIn)} - {formatTime(trimOut)} ({formatTime(trimOut - trimIn)})
                  </span>
                  {showResizeMode && <span className="text-white/40">•</span>}
                </>
              )}
              
              {/* Resize info */}
              {showResizeMode && (selectedResolution !== 'original' || selectedAspectRatio !== 'original') && (
                <span className="text-white/70 text-sm">
                  {selectedResolution !== 'original' ? resolutionOptions.find(r => r.value === selectedResolution)?.label : ''}
                  {selectedResolution !== 'original' && selectedAspectRatio !== 'original' ? ' • ' : ''}
                  {selectedAspectRatio !== 'original' ? aspectRatioOptions.find(a => a.value === selectedAspectRatio)?.label : ''}
                </span>
              )}
            </div>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                setShowTrimMode(false);
                setShowResizeMode(false);
                clearTrimPoints();
              }}
              className="text-white hover:bg-white/20"
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        )}

        {/* Bottom controls */}
        <div className="absolute bottom-0 left-0 right-0 p-4 pointer-events-auto">
          {/* Timeline */}
          <div className="mb-4 space-y-2">
            {/* Main timeline bar */}
            <div 
              ref={timelineRef}
              className="relative h-3 bg-white/30 rounded-full cursor-pointer group/timeline"
              onClick={handleTimelineClick}
              onMouseDown={handleTimelineMouseDown}
            >
              {/* Base timeline background */}
              <div className="absolute inset-0 rounded-full pointer-events-none" />
              
              {showTrimMode && trimIn !== null && trimOut !== null ? (
                <>
                  {/* Left dimmed area (will be trimmed) */}
                  <div 
                    className="absolute top-0 left-0 h-full bg-white/20 rounded-l-full pointer-events-none"
                    style={{ width: `${trimInPosition}%` }}
                  />
                  
                  {/* Center bright area (will be kept) */}
                  <div 
                    className="absolute top-0 h-full bg-white/80 pointer-events-none"
                    style={{ 
                      left: `${trimInPosition}%`, 
                      width: `${trimOutPosition - trimInPosition}%` 
                    }}
                  />
                  
                  {/* Right dimmed area (will be trimmed) */}
                  <div 
                    className="absolute top-0 right-0 h-full bg-white/20 rounded-r-full pointer-events-none"
                    style={{ width: `${100 - trimOutPosition}%` }}
                  />
                </>
              ) : (
                /* Normal progress bar when not in trim mode */
                <div 
                  className="absolute top-0 left-0 h-full bg-white/60 rounded-full pointer-events-none"
                  style={{ width: `${timelinePosition}%` }}
                />
              )}
              
              {/* Playhead */}
              <div 
                className="absolute top-1/2 -translate-y-1/2 w-4 h-4 bg-white rounded-full shadow-lg group-hover/timeline:scale-110 pointer-events-none border-2 border-black/20 transition-transform"
                style={{ left: `${timelinePosition}%`, transform: 'translateX(-50%) translateY(-50%)' }}
              />
            </div>

            {/* I/O Markers Row (only shown in trim mode) */}
            {showTrimMode && (
              <div className="relative h-6 flex items-center">
                {/* In marker */}
                {trimIn !== null && (
                  <div
                    className="absolute flex items-center justify-center w-8 h-6 cursor-ew-resize hover:scale-110 transition-transform"
                    style={{ left: `${trimInPosition}%`, transform: 'translateX(-50%)' }}
                    onMouseDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      setIsDragging('in');
                    }}
                  >
                    <div className="w-6 h-4 bg-green-500 rounded flex items-center justify-center text-white text-xs font-bold shadow-lg pointer-events-none">
                      I
                    </div>
                  </div>
                )}
                
                {/* Out marker */}
                {trimOut !== null && (
                  <div
                    className="absolute flex items-center justify-center w-8 h-6 cursor-ew-resize hover:scale-110 transition-transform"
                    style={{ left: `${trimOutPosition}%`, transform: 'translateX(-50%)' }}
                    onMouseDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      setIsDragging('out');
                    }}
                  >
                    <div className="w-6 h-4 bg-red-500 rounded flex items-center justify-center text-white text-xs font-bold shadow-lg pointer-events-none">
                      O
                    </div>
                  </div>
                )}
                
                {/* Helper text */}
                <div className="absolute right-0 top-0 text-white/60 text-xs pointer-events-none">
                  Drag I/O markers or click timeline to set points
                </div>
              </div>
            )}
          </div>

          {/* Control buttons */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              {/* Play/Pause */}
              <Button
                size="sm"
                variant="ghost"
                onClick={togglePlayPause}
                className="text-white hover:bg-white/20"
              >
                {isPlaying ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
              </Button>
              
              {/* Skip buttons */}
              <Button
                size="sm"
                variant="ghost"
                onClick={() => skip(-10)}
                className="text-white hover:bg-white/20"
              >
                <SkipBack className="h-4 w-4" />
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => skip(10)}
                className="text-white hover:bg-white/20"
              >
                <SkipForward className="h-4 w-4" />
              </Button>
              
              {/* Time display */}
              <span className="text-white text-sm ml-2">
                {formatTime(currentTime)} / {formatTime(duration)}
              </span>
            </div>

            <div className="flex items-center gap-2">
              {/* Combined Mode Controls */}
              {showTrimMode || showResizeMode ? (
                <div className="flex items-center gap-2 flex-wrap">
                  {/* Trim Controls */}
                  {showTrimMode && (
                    <>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={setInPoint}
                        disabled={duration <= 0}
                        className="text-white hover:bg-white/20 disabled:opacity-50"
                      >
                        Set In
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={setOutPoint}
                        disabled={duration <= 0}
                        className="text-white hover:bg-white/20 disabled:opacity-50"
                      >
                        Set Out
                      </Button>
                      {trimIn !== null && trimOut !== null && (
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={clearTrimPoints}
                          className="text-white hover:bg-white/20"
                        >
                          Clear Trim
                        </Button>
                      )}
                      {showResizeMode && <div className="w-px h-4 bg-white/30" />}
                    </>
                  )}
                  
                  {/* Resize Controls */}
                  {showResizeMode && (
                    <>
                      <select
                        value={selectedResolution}
                        onChange={(e) => setSelectedResolution(e.target.value)}
                        className="bg-black/50 text-white text-sm rounded px-2 py-1 border border-white/20"
                      >
                        {resolutionOptions.map(option => (
                          <option key={option.value} value={option.value} className="bg-black">
                            {option.label}
                          </option>
                        ))}
                      </select>
                      <select
                        value={selectedAspectRatio}
                        onChange={(e) => setSelectedAspectRatio(e.target.value)}
                        className="bg-black/50 text-white text-sm rounded px-2 py-1 border border-white/20"
                      >
                        {aspectRatioOptions.map(option => (
                          <option key={option.value} value={option.value} className="bg-black">
                            {option.label}
                          </option>
                        ))}
                      </select>
                      {selectedAspectRatio !== 'original' && (
                        <div className="flex items-center gap-1 bg-black/50 rounded px-2 py-1">
                          <label className="text-white text-xs">
                            <input
                              type="radio"
                              name="aspectMode"
                              value="crop"
                              checked={aspectMode === 'crop'}
                              onChange={(e) => setAspectMode(e.target.value as 'crop' | 'pad')}
                              className="mr-1"
                            />
                            Crop
                          </label>
                          <label className="text-white text-xs ml-2">
                            <input
                              type="radio"
                              name="aspectMode"
                              value="pad"
                              checked={aspectMode === 'pad'}
                              onChange={(e) => setAspectMode(e.target.value as 'crop' | 'pad')}
                              className="mr-1"
                            />
                            Pad
                          </label>
                        </div>
                      )}
                      {selectedAspectRatio !== 'original' && aspectMode === 'crop' && (cropPosition.x !== 0 || cropPosition.y !== 0) && (
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={resetCropPosition}
                          className="text-white/70 hover:bg-white/20 text-xs px-2 py-1 h-auto"
                        >
                          Reset Position
                        </Button>
                      )}
                    </>
                  )}
                  
                  {/* Add/Remove Mode Buttons */}
                  <div className="flex gap-1">
                    {!showTrimMode && (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => {
                          setShowTrimMode(true);
                          if (videoRef.current) {
                            setCurrentTime(videoRef.current.currentTime);
                            setDuration(videoRef.current.duration || duration);
                          }
                        }}
                        className="text-white border-white/30 hover:bg-white/20 text-xs px-2 py-1 h-auto"
                      >
                        + Trim
                      </Button>
                    )}
                    {!showResizeMode && (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => setShowResizeMode(true)}
                        className="text-white border-white/30 hover:bg-white/20 text-xs px-2 py-1 h-auto"
                      >
                        + Resize
                      </Button>
                    )}
                  </div>
                </div>
              ) : (
                /* Initial mode selection */
                <>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setShowTrimMode(true);
                      if (videoRef.current) {
                        setCurrentTime(videoRef.current.currentTime);
                        setDuration(videoRef.current.duration || duration);
                      }
                    }}
                    className="text-white hover:bg-white/20"
                  >
                    <Scissors className="h-4 w-4 mr-1" />
                    Trim
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setShowResizeMode(true)}
                    className="text-white hover:bg-white/20"
                  >
                    <Monitor className="h-4 w-4 mr-1" />
                    Resize
                  </Button>
                </>
              )}
              
              {/* Export button - shows when any mode is active */}
              {(showTrimMode || showResizeMode) && (
                <Button
                  size="sm"
                  variant="default"
                  onClick={handleExport}
                  disabled={isExporting || (showTrimMode && (trimIn === null || trimOut === null)) || (showResizeMode && selectedResolution === 'original' && selectedAspectRatio === 'original')}
                  className="bg-blue-600 hover:bg-blue-700 ml-2"
                >
                  <Download className="h-4 w-4 mr-1" />
                  {isExporting ? 'Exporting...' : 'Export'}
                </Button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});
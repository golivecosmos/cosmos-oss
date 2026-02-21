import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { MediaFile } from './types';
import { Play, Pause, Video, Volume2, VolumeX, SkipBack, SkipForward, Square, Settings } from 'lucide-react';

interface VideoPreviewProps {
  file: MediaFile;
  initialTimestamp?: number;
  lazy?: boolean;
  showControls?: boolean;
}

export function VideoPreview({ file, initialTimestamp, lazy = true, showControls = false }: VideoPreviewProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const scrubBarRef = useRef<HTMLDivElement>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isScrubbing, setIsScrubbing] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [scrubPosition, setScrubPosition] = useState(0);
  const [isMuted, setIsMuted] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [showInteractionHint, setShowInteractionHint] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [playbackRate, setPlaybackRate] = useState(1);

  // **NEW: Lazy loading state**
  const [isInView, setIsInView] = useState(!lazy); // If lazy is false, assume it's always in view
  const [shouldLoad, setShouldLoad] = useState(!lazy); // Controls when to actually load the video

  // **NEW: Intersection Observer for lazy loading**
  useEffect(() => {
    if (!lazy || !containerRef.current) {
      setShouldLoad(true);
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        const [entry] = entries;
        setIsInView(entry.isIntersecting);

        // Load video when it's about to come into view (with some margin)
        if (entry.isIntersecting) {
          setShouldLoad(true);
        }
      },
      {
        // Load when 10% of the element is visible or when it's within 200px of viewport
        threshold: [0, 0.1],
        rootMargin: '200px'
      }
    );

    if (containerRef.current) {
      observer.observe(containerRef.current);
    }

    return () => {
      observer.disconnect();
    };
  }, [lazy]);

  // Handle video source path for video frames
  const videoPath = useMemo(() => {
    // Don't compute path until we need to load
    if (!shouldLoad) return '';

    // For video frames, the file.path already points to the video file
    if (file.metadata.isVideoFrame) {
      return file.path;
    }
    // For grouped videos, use the file path directly
    else if (file.metadata.isGroupedVideo || file.type === 'video') {
      return file.path;
    }
    // Otherwise just use the file path
    return file.path;
  }, [file, shouldLoad]);

  useEffect(() => {
    if (!shouldLoad) return;

    setIsLoading(true);
    setError(false);
    setScrubPosition(0);
    setIsPlaying(false);

    // Check for potential issues with video frames
    if (file.metadata.isVideoFrame) {
      if (!file.metadata.parentPath) {
        console.warn('Warning: Video frame is missing parent path');
        setError(true);
        setIsLoading(false);
      }
    }
  }, [videoPath, file.metadata.isVideoFrame, file.metadata.parentPath, shouldLoad]);

  // Handle changes to initialTimestamp (for seeking from transcription)
  useEffect(() => {
    if (!videoRef.current || !videoRef.current.duration) return;
    const video = videoRef.current;
    if (initialTimestamp !== undefined && initialTimestamp !== null &&
        initialTimestamp >= 0 && initialTimestamp <= video.duration) {

      const currentVideoTime = video.currentTime;
      const timeDifference = Math.abs(currentVideoTime - initialTimestamp);

      // This prevents constant small adjustments while still allowing proper seeking
      if (timeDifference > 0.5) {
        // Store current playing state to restore it after seeking
        const wasPlaying = !video.paused;
        video.currentTime = initialTimestamp;
        setCurrentTime(initialTimestamp);
        setScrubPosition(initialTimestamp / video.duration);

        // Only pause if the video wasn't already playing
        // This prevents the snap-back behavior when clicking transcript during playback
        if (!wasPlaying) {
          video.pause();
          setIsPlaying(false);
        }
      } else {
        console.log('❌ VideoPreview: Skipping seek - difference too small:', timeDifference);
      }
    }
  }, [initialTimestamp, file.name]);

  // Modify the handleLoadedMetadata function
  const handleLoadedMetadata = () => {
    if (videoRef.current) {
      const video = videoRef.current;

      // Immediately pause and ensure it stays paused
      video.pause();
      setIsPlaying(false);

      setDuration(video.duration);
      setIsLoading(false);

      // Show interaction hint briefly for the first video in a session
      if (!sessionStorage.getItem('video-hint-shown')) {
        setShowInteractionHint(true);
        setTimeout(() => {
          setShowInteractionHint(false);
          sessionStorage.setItem('video-hint-shown', 'true');
        }, 3000);
      }

      // Determine which timestamp to use
      let targetTimestamp = null;

      if (initialTimestamp !== undefined && initialTimestamp !== null) {
        targetTimestamp = initialTimestamp;
      } else if (file.metadata.timestamp !== undefined && file.metadata.timestamp !== null) {
        targetTimestamp = file.metadata.timestamp;
      }

      // Seek to timestamp but don't automatically pause
      if (targetTimestamp !== null && targetTimestamp >= 0 && targetTimestamp <= video.duration) {
        video.currentTime = targetTimestamp;
        setCurrentTime(targetTimestamp);
        setScrubPosition(targetTimestamp / video.duration);
        // Don't automatically pause - let the user control playback
        video.pause();
        setIsPlaying(false);
      } else {
        video.currentTime = 0;
        setCurrentTime(0);
        setScrubPosition(0);
        video.pause();
        setIsPlaying(false);
      }
    }
  };

  // Update the current time as the video plays
  const handleTimeUpdate = () => {
    if (videoRef.current) {
      setCurrentTime(videoRef.current.currentTime);
      setScrubPosition(videoRef.current.currentTime / videoRef.current.duration);
    }
  };

  const handlePlayPause = () => {
    if (videoRef.current) {
      if (isPlaying) {
        videoRef.current.pause();
        setIsPlaying(false);
      } else {
        videoRef.current.play();
        setIsPlaying(true);
      }
    }
  };

  // NEW: Better video interaction - double-click to play, single click for controls
  const [showControlsTemporarily, setShowControlsTemporarily] = useState(false);
  const [controlsTimeout, setControlsTimeout] = useState<NodeJS.Timeout | null>(null);
  const [lastClickTime, setLastClickTime] = useState(0);

  const handleVideoClick = useCallback((e: React.MouseEvent) => {
    // Don't handle clicks if we're dragging the scrub bar
    if (isDragging) return;

    // Don't trigger if clicking on the control areas
    if (scrubBarRef.current) {
      const scrubBarRect = scrubBarRef.current.getBoundingClientRect();
      const clickY = e.clientY;
      if (clickY >= scrubBarRect.top && clickY <= scrubBarRect.bottom) {
        return;
      }
    }

    e.preventDefault();
    e.stopPropagation();

    const now = Date.now();
    const timeSinceLastClick = now - lastClickTime;
    setLastClickTime(now);

    // Double-click to play/pause
    if (timeSinceLastClick < 300) {
      handlePlayPause();
      return;
    }

    // Single click: show controls temporarily and focus for keyboard control
    setShowControlsTemporarily(true);
    containerRef.current?.focus();

    // Clear existing timeout
    if (controlsTimeout) {
      clearTimeout(controlsTimeout);
    }

    // Hide controls after 3 seconds if not playing
    const timeout = setTimeout(() => {
      if (!isPlaying) {
        setShowControlsTemporarily(false);
      }
    }, 3000);

    setControlsTimeout(timeout);
  }, [isDragging, handlePlayPause, lastClickTime, controlsTimeout, isPlaying]);

  // Clear controls timeout when component unmounts
  useEffect(() => {
    return () => {
      if (controlsTimeout) {
        clearTimeout(controlsTimeout);
      }
    };
  }, [controlsTimeout]);

  // Close settings menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (showSettings && containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setShowSettings(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [showSettings]);


  const handleVideoScrub = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();

    const videoElement = videoRef.current;
    if (!videoElement || !scrubBarRef.current) return;

    const rect = scrubBarRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const percentage = Math.max(0, Math.min(1, x / rect.width));

    if (videoElement.duration) {
      const newTime = videoElement.duration * percentage;
      videoElement.currentTime = newTime;
      videoElement.pause(); // Always pause when scrubbing
      setIsPlaying(false);
      setCurrentTime(newTime);
      setScrubPosition(percentage);
    }
  }, []);

  // NEW: Handle scrub bar interactions
  const handleScrubBarMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
    handleVideoScrub(e);
  }, [handleVideoScrub]);

  const handleScrubBarMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (!isDragging) return;
    e.preventDefault();
    e.stopPropagation();
    handleVideoScrub(e);
  }, [isDragging, handleVideoScrub]);

  const handleScrubBarMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  // NEW: Global mouse up listener to handle dragging outside the scrub bar
  useEffect(() => {
    if (!isDragging) return;

    const handleGlobalMouseUp = () => {
      setIsDragging(false);
    };

    const handleGlobalMouseMove = (e: MouseEvent) => {
      if (!isDragging || !scrubBarRef.current || !videoRef.current) return;

      const rect = scrubBarRef.current.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const percentage = Math.max(0, Math.min(1, x / rect.width));

      if (videoRef.current.duration) {
        const newTime = videoRef.current.duration * percentage;
        videoRef.current.currentTime = newTime;
        videoRef.current.pause();
        setIsPlaying(false);
        setCurrentTime(newTime);
        setScrubPosition(percentage);
      }
    };

    document.addEventListener('mouseup', handleGlobalMouseUp);
    document.addEventListener('mousemove', handleGlobalMouseMove);

    return () => {
      document.removeEventListener('mouseup', handleGlobalMouseUp);
      document.removeEventListener('mousemove', handleGlobalMouseMove);
    };
  }, [isDragging]);

  // NEW: Keyboard controls for video - much more specific targeting
  useEffect(() => {
    if (!isInView || !shouldLoad) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Only handle keyboard events when the video container is focused (not just hovered)
      // This prevents interference with search bar typing
      if (document.activeElement !== containerRef.current) return;

      const video = videoRef.current;
      if (!video || !video.duration) return;

      switch (e.code) {
        case 'Space':
          e.preventDefault();
          e.stopPropagation();
          handlePlayPause();
          break;
        case 'ArrowLeft':
          e.preventDefault();
          e.stopPropagation();
          if (e.shiftKey) {
            // Frame-by-frame navigation (shift + arrow)
            const newTime = Math.max(0, video.currentTime - 1/30);
            video.currentTime = newTime;
            setCurrentTime(newTime);
            setScrubPosition(newTime / video.duration);
          } else {
            // 5-second skip
            const newTimeLeft = Math.max(0, video.currentTime - 5);
            video.currentTime = newTimeLeft;
            setCurrentTime(newTimeLeft);
            setScrubPosition(newTimeLeft / video.duration);
          }
          break;
        case 'ArrowRight':
          e.preventDefault();
          e.stopPropagation();
          if (e.shiftKey) {
            // Frame-by-frame navigation (shift + arrow)
            const newTime = Math.min(video.duration, video.currentTime + 1/30);
            video.currentTime = newTime;
            setCurrentTime(newTime);
            setScrubPosition(newTime / video.duration);
          } else {
            // 5-second skip
            const newTimeRight = Math.min(video.duration, video.currentTime + 5);
            video.currentTime = newTimeRight;
            setCurrentTime(newTimeRight);
            setScrubPosition(newTimeRight / video.duration);
          }
          break;
        case 'KeyM':
          e.preventDefault();
          e.stopPropagation();
          toggleMute();
          break;
        case 'Home':
          e.preventDefault();
          e.stopPropagation();
          video.currentTime = 0;
          setCurrentTime(0);
          setScrubPosition(0);
          video.pause();
          setIsPlaying(false);
          break;
        case 'End':
          e.preventDefault();
          e.stopPropagation();
          video.currentTime = video.duration;
          setCurrentTime(video.duration);
          setScrubPosition(1);
          video.pause();
          setIsPlaying(false);
          break;
        case 'Escape':
          e.preventDefault();
          e.stopPropagation();
          // Remove focus to stop keyboard controls
          containerRef.current?.blur();
          break;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isInView, shouldLoad, handlePlayPause]);

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const handleMouseEnter = useCallback(() => {
    // This is now handled by the timeline component directly
  }, []);

  const handleMouseLeave = useCallback(() => {
    // This is now handled by the timeline component directly
  }, []);

  const toggleMute = () => {
    if (videoRef.current) {
      videoRef.current.muted = !isMuted;
      setIsMuted(!isMuted);
    }
  };


  const changePlaybackRate = (rate: number) => {
    if (videoRef.current) {
      videoRef.current.playbackRate = rate;
      setPlaybackRate(rate);
    }
    setShowSettings(false);
  };

  return (
    <div
      ref={containerRef}
      className={`w-full h-full bg-gray-900 rounded-lg overflow-hidden relative group transition-all duration-200 ${
        document.activeElement === containerRef.current
          ? 'ring-2 ring-blue-500/50 ring-offset-2 ring-offset-gray-900'
          : 'focus:outline-none focus:ring-2 focus:ring-blue-500/50'
      }`}
      tabIndex={0}
      onFocus={() => {
        setShowControlsTemporarily(true);
        // Auto-hide controls after 4 seconds if not playing
        if (controlsTimeout) clearTimeout(controlsTimeout);
        const timeout = setTimeout(() => {
          if (!isPlaying) {
            setShowControlsTemporarily(false);
          }
        }, 4000);
        setControlsTimeout(timeout);
      }}
    >
      {/* **NEW: Show placeholder when not loading yet** */}
      {!shouldLoad ? (
        <div className="absolute inset-0 flex items-center justify-center bg-gradient-to-br from-gray-800 to-gray-900">
          <div className="text-center">
            <Video className="h-12 w-12 text-gray-400 mx-auto mb-2" />
            <div className="text-xs text-gray-500">{file.name}</div>
            {file.metadata.isVideoFrame && file.metadata.timestampFormatted && (
              <div className="text-xs text-gray-400 mt-1">{file.metadata.timestampFormatted}</div>
            )}
          </div>
        </div>
      ) : (
        <>
          {isLoading && (
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" />
            </div>
          )}

          {error ? (
            <div className="absolute inset-0 flex items-center justify-center bg-gray-100">
              <div className="text-center p-4">
                <Video className="h-12 w-12 text-gray-400 mx-auto mb-2" />
                <div className="text-sm text-gray-600 font-medium mb-1">Video not found</div>
                <div className="text-xs text-gray-500 max-w-sm">
                  {file.metadata.isVideoFrame
                    ? `The parent video file may have been moved or deleted`
                    : `This file may have been moved or deleted`
                  }
                </div>
                <div className="text-xs text-gray-400 mt-2 font-mono truncate max-w-xs">
                  {file.metadata.isVideoFrame ? file.metadata.parentPath : file.path}
                </div>
              </div>
            </div>
          ) : (
            <>
              <div
                className="relative w-full h-full"
                onMouseEnter={handleMouseEnter}
                onMouseLeave={handleMouseLeave}
                onClick={handleVideoClick}
              >
                <video
                  ref={videoRef}
                  src={videoPath}
                  className="w-full h-full object-contain"
                  autoPlay={false}
                  playsInline
                  preload="metadata"
                  muted={isMuted}
                  onLoadedMetadata={handleLoadedMetadata}
                  onTimeUpdate={handleTimeUpdate}
                  onError={(e) => {
                    // Check if this is a 404/file not found error
                    const target = e.target as HTMLVideoElement;
                    if (target.error?.code === MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED ||
                        videoPath.includes('404') ||
                        !videoPath) {
                      console.warn(`Video file not found (likely moved): ${file.name}`);
                    } else {
                      console.error(`Video error for: ${file.name}`, videoPath, target.error);
                    }
                    setError(true);
                    setIsLoading(false);
                  }}
                  style={{
                    backgroundColor: 'transparent',
                    objectFit: 'contain',
                    maxWidth: '100%',
                    maxHeight: '100%'
                  }}
                />

                {/* Professional Video Controls */}
                {(showControls || isScrubbing || isDragging || showControlsTemporarily || isPlaying) && (
                  <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 via-black/40 to-transparent">
                    {/* Timeline */}
                    <div
                      ref={scrubBarRef}
                      className="px-4 py-3 cursor-pointer"
                      onMouseDown={handleScrubBarMouseDown}
                      onMouseMove={handleScrubBarMouseMove}
                      onMouseUp={handleScrubBarMouseUp}
                      onMouseEnter={() => setIsScrubbing(true)}
                      onMouseLeave={() => {
                        if (!isDragging) {
                          setIsScrubbing(false);
                        }
                      }}
                    >
                      {/* Time markers */}
                      <div className="flex justify-between text-xs text-white/70 mb-2">
                        <span>{formatTime(currentTime)}</span>
                        <span>{formatTime(duration)}</span>
                      </div>

                      {/* Timeline track */}
                      <div className="relative h-1 bg-white/20 rounded-full group">
                        {/* Progress bar */}
                        <div
                          className={`h-full bg-red-500 rounded-full transition-all relative
                            ${isDragging ? 'duration-0' : 'duration-150'}
                          `}
                          style={{ width: `${scrubPosition * 100}%` }}
                        >
                          {/* Scrub handle */}
                          <div
                            className={`absolute right-0 top-1/2 transform translate-x-1/2 -translate-y-1/2
                              w-3 h-3 bg-red-500 rounded-full border border-white shadow-lg transition-all duration-200
                              ${(isScrubbing || isDragging) ? 'scale-125' : 'scale-100 group-hover:scale-110'}
                            `}
                          />
                        </div>

                        {/* Time tooltip during scrubbing */}
                        {isDragging && (
                          <div
                            className="absolute bottom-full mb-2 transform -translate-x-1/2 bg-black/90 text-white text-xs px-2 py-1 rounded whitespace-nowrap pointer-events-none"
                            style={{ left: `${scrubPosition * 100}%` }}
                          >
                            {formatTime(currentTime)}
                          </div>
                        )}
                      </div>
                    </div>

                    {/* Control Bar */}
                    <div className="flex items-center justify-between px-4 pb-3">
                      {/* Left controls */}
                      <div className="flex items-center space-x-3">
                        {/* Frame controls */}
                        <button
                          onClick={() => {
                            if (videoRef.current && videoRef.current.duration) {
                              const newTime = Math.max(0, videoRef.current.currentTime - 1/30); // Previous frame (~33ms)
                              videoRef.current.currentTime = newTime;
                              setCurrentTime(newTime);
                              setScrubPosition(newTime / videoRef.current.duration);
                            }
                          }}
                          className="text-white/80 hover:text-white transition-colors p-1 hover:bg-white/10 rounded"
                          title="Previous frame"
                        >
                          <SkipBack className="h-4 w-4" />
                        </button>

                        {/* Play/Pause */}
                        <button
                          onClick={handlePlayPause}
                          className="text-white hover:text-white transition-colors p-2 hover:bg-white/10 rounded-full"
                          title={isPlaying ? "Pause" : "Play"}
                        >
                          {isPlaying ? (
                            <Pause className="h-6 w-6" />
                          ) : (
                            <Play className="h-6 w-6 ml-0.5" />
                          )}
                        </button>

                        {/* Frame controls */}
                        <button
                          onClick={() => {
                            if (videoRef.current && videoRef.current.duration) {
                              const newTime = Math.min(videoRef.current.duration, videoRef.current.currentTime + 1/30); // Next frame (~33ms)
                              videoRef.current.currentTime = newTime;
                              setCurrentTime(newTime);
                              setScrubPosition(newTime / videoRef.current.duration);
                            }
                          }}
                          className="text-white/80 hover:text-white transition-colors p-1 hover:bg-white/10 rounded"
                          title="Next frame"
                        >
                          <SkipForward className="h-4 w-4" />
                        </button>

                        {/* Stop */}
                        <button
                          onClick={() => {
                            if (videoRef.current) {
                              videoRef.current.pause();
                              videoRef.current.currentTime = 0;
                              setIsPlaying(false);
                              setCurrentTime(0);
                              setScrubPosition(0);
                            }
                          }}
                          className="text-white/80 hover:text-white transition-colors p-1 hover:bg-white/10 rounded"
                          title="Stop"
                        >
                          <Square className="h-4 w-4" />
                        </button>

                        {/* Volume */}
                        <button
                          onClick={toggleMute}
                          className="text-white/80 hover:text-white transition-colors p-1 hover:bg-white/10 rounded"
                          title={isMuted ? "Unmute" : "Mute"}
                        >
                          {isMuted ? (
                            <VolumeX className="h-4 w-4" />
                          ) : (
                            <Volume2 className="h-4 w-4" />
                          )}
                        </button>
                      </div>

                      {/* Right controls */}
                      <div className="flex items-center space-x-3">
                        {/* Time display */}
                        <div className="text-white/90 text-sm font-mono">
                          {formatTime(currentTime)} / {formatTime(duration)}
                        </div>

                        {/* Settings */}
                        <div className="relative">
                          <button
                            onClick={() => setShowSettings(!showSettings)}
                            className="text-white/80 hover:text-white transition-colors p-1 hover:bg-white/10 rounded"
                            title="Playback Settings"
                          >
                            <Settings className="h-4 w-4" />
                          </button>

                          {/* Settings Menu */}
                          {showSettings && (
                            <div className="absolute bottom-full right-0 mb-2 bg-black/90 text-white text-xs rounded-lg py-2 min-w-[120px] shadow-lg">
                              <div className="px-3 py-1 text-white/70 font-medium border-b border-white/20">
                                Playback Speed
                              </div>
                              {[0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2].map(rate => (
                                <button
                                  key={rate}
                                  onClick={() => changePlaybackRate(rate)}
                                  className={`w-full text-left px-3 py-1 hover:bg-white/10 transition-colors ${
                                    playbackRate === rate ? 'text-red-400' : 'text-white/90'
                                  }`}
                                >
                                  {rate}x {rate === 1 ? '(Normal)' : ''}
                                </button>
                              ))}
                            </div>
                          )}
                        </div>

                      </div>
                    </div>
                  </div>
                )}
              </div>

              {/* Interaction Hint */}
              {showInteractionHint && !isLoading && (
                <div className="absolute inset-0 flex items-center justify-center bg-black/40 backdrop-blur-sm pointer-events-none animate-fade-in">
                  <div className="bg-white/95 rounded-lg p-4 max-w-md mx-4 text-center shadow-xl transform transition-all duration-500 ease-out scale-95 animate-pulse">
                    <div className="text-sm text-gray-800 mb-2 font-medium">
                      Professional Video Controls
                    </div>
                    <div className="text-xs text-gray-600 space-y-1">
                      <div>• Double-click to play/pause</div>
                      <div>• Space: Play/Pause • M: Mute</div>
                      <div>• Arrow keys: Skip 5s • Shift+Arrow: Frame-by-frame</div>
                      <div>• Home/End: Go to start/end</div>
                      <div>• Drag red timeline to scrub</div>
                    </div>
                  </div>
                </div>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
}

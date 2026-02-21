import React, { useState, useRef, useEffect, useCallback } from 'react';
import { cn } from '../../lib/utils';

interface VideoTimelineProps {
  duration: number;
  currentTime: number;
  onTimeChange: (time: number) => void;
  onTrimPointsChange: (inPoint: number | null, outPoint: number | null) => void;
  className?: string;
}

export const VideoTimeline: React.FC<VideoTimelineProps> = ({
  duration,
  currentTime,
  onTimeChange,
  onTrimPointsChange,
  className,
}) => {
  const [inPoint, setInPoint] = useState<number | null>(null);
  const [outPoint, setOutPoint] = useState<number | null>(null);
  const [isDragging, setIsDragging] = useState<'in' | 'out' | 'playhead' | null>(null);
  const timelineRef = useRef<HTMLDivElement>(null);

  // Format time to MM:SS or HH:MM:SS
  const formatTime = (seconds: number): string => {
    const hrs = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    
    if (hrs > 0) {
      return `${hrs.toString().padStart(2, '0')}:${mins
        .toString()
        .padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    }
    return `${mins.toString().padStart(2, '0')}:${secs
      .toString()
      .padStart(2, '0')}`;
  };

  // Convert position to time
  const positionToTime = (clientX: number): number => {
    if (!timelineRef.current) return 0;
    const rect = timelineRef.current.getBoundingClientRect();
    const position = (clientX - rect.left) / rect.width;
    return Math.max(0, Math.min(duration, position * duration));
  };

  // Convert time to position percentage
  const timeToPosition = (time: number): string => {
    return `${(time / duration) * 100}%`;
  };

  const handleMouseDown = (e: React.MouseEvent, type: 'in' | 'out' | 'playhead') => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(type);
  };

  const handleTimelineClick = (e: React.MouseEvent) => {
    if (isDragging) return;
    const time = positionToTime(e.clientX);
    onTimeChange(time);
  };

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isDragging) return;

      const time = positionToTime(e.clientX);

      if (isDragging === 'playhead') {
        onTimeChange(time);
      } else if (isDragging === 'in') {
        const newInPoint = Math.min(time, outPoint ?? duration);
        setInPoint(newInPoint);
        onTrimPointsChange(newInPoint, outPoint);
      } else if (isDragging === 'out') {
        const newOutPoint = Math.max(time, inPoint ?? 0);
        setOutPoint(newOutPoint);
        onTrimPointsChange(inPoint, newOutPoint);
      }
    },
    [isDragging, inPoint, outPoint, duration, onTimeChange, onTrimPointsChange]
  );

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

  // Set in point at current time
  const setInAtCurrentTime = () => {
    const newInPoint = Math.min(currentTime, outPoint ?? duration);
    setInPoint(newInPoint);
    onTrimPointsChange(newInPoint, outPoint);
  };

  // Set out point at current time
  const setOutAtCurrentTime = () => {
    const newOutPoint = Math.max(currentTime, inPoint ?? 0);
    setOutPoint(newOutPoint);
    onTrimPointsChange(inPoint, newOutPoint);
  };

  // Clear trim points
  const clearTrimPoints = () => {
    setInPoint(null);
    setOutPoint(null);
    onTrimPointsChange(null, null);
  };

  // Calculate trim duration
  const trimDuration = outPoint !== null && inPoint !== null 
    ? outPoint - inPoint 
    : duration;

  return (
    <div className={cn("space-y-4", className)}>
      {/* Timeline Bar */}
      <div className="relative">
        {/* Time labels */}
        <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400 mb-2">
          <span>{formatTime(0)}</span>
          <span className="font-semibold">{formatTime(currentTime)}</span>
          <span>{formatTime(duration)}</span>
        </div>

        {/* Timeline track */}
        <div
          ref={timelineRef}
          className="relative h-12 bg-gray-200 dark:bg-gray-700 rounded cursor-pointer overflow-hidden"
          onClick={handleTimelineClick}
        >
          {/* Trimmed region highlight */}
          {inPoint !== null && outPoint !== null && (
            <div
              className="absolute top-0 h-full bg-blue-500/20 dark:bg-blue-400/20"
              style={{
                left: timeToPosition(inPoint),
                width: `${((outPoint - inPoint) / duration) * 100}%`,
              }}
            />
          )}

          {/* Dimmed regions outside trim */}
          {inPoint !== null && (
            <div
              className="absolute top-0 left-0 h-full bg-black/40"
              style={{ width: timeToPosition(inPoint) }}
            />
          )}
          {outPoint !== null && (
            <div
              className="absolute top-0 h-full bg-black/40 right-0"
              style={{ width: `${100 - (outPoint / duration) * 100}%` }}
            />
          )}

          {/* In point marker */}
          {inPoint !== null && (
            <div
              className="absolute top-0 w-3 h-full bg-green-500 cursor-ew-resize hover:bg-green-600 transition-colors"
              style={{ left: `calc(${timeToPosition(inPoint)} - 6px)` }}
              onMouseDown={(e) => handleMouseDown(e, 'in')}
              title={`In: ${formatTime(inPoint)}`}
            >
              <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 text-white text-xs font-bold">
                I
              </div>
            </div>
          )}

          {/* Out point marker */}
          {outPoint !== null && (
            <div
              className="absolute top-0 w-3 h-full bg-red-500 cursor-ew-resize hover:bg-red-600 transition-colors"
              style={{ left: `calc(${timeToPosition(outPoint)} - 6px)` }}
              onMouseDown={(e) => handleMouseDown(e, 'out')}
              title={`Out: ${formatTime(outPoint)}`}
            >
              <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 text-white text-xs font-bold">
                O
              </div>
            </div>
          )}

          {/* Playhead */}
          <div
            className="absolute top-0 w-1 h-full bg-white cursor-ew-resize"
            style={{ left: timeToPosition(currentTime) }}
            onMouseDown={(e) => handleMouseDown(e, 'playhead')}
          >
            <div className="absolute -top-1 left-1/2 -translate-x-1/2 w-0 h-0 border-l-[6px] border-l-transparent border-r-[6px] border-r-transparent border-t-[8px] border-t-white" />
          </div>
        </div>
      </div>

      {/* Controls */}
      <div className="flex items-center justify-between">
        <div className="flex gap-2">
          <button
            onClick={setInAtCurrentTime}
            className="px-3 py-1.5 text-sm bg-green-500 text-white rounded hover:bg-green-600 transition-colors"
            title="Set in point at current time (I)"
          >
            Set In
          </button>
          <button
            onClick={setOutAtCurrentTime}
            className="px-3 py-1.5 text-sm bg-red-500 text-white rounded hover:bg-red-600 transition-colors"
            title="Set out point at current time (O)"
          >
            Set Out
          </button>
          {(inPoint !== null || outPoint !== null) && (
            <button
              onClick={clearTrimPoints}
              className="px-3 py-1.5 text-sm bg-gray-500 text-white rounded hover:bg-gray-600 transition-colors"
            >
              Clear
            </button>
          )}
        </div>

        {/* Trim info */}
        {inPoint !== null && outPoint !== null && (
          <div className="text-sm text-gray-600 dark:text-gray-300">
            Trim: {formatTime(inPoint)} - {formatTime(outPoint)} ({formatTime(trimDuration)})
          </div>
        )}
      </div>
    </div>
  );
};
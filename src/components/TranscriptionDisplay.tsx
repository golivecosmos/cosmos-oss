import React, { useState, useEffect, useCallback, useRef } from 'react';
import { FileAudio, Copy, Loader2, AlertCircle } from 'lucide-react';
import { Button } from './ui/button';
import { Card } from './ui/card';
import { Badge } from './ui/badge';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';

interface TranscriptionSegment {
  start: number;
  end: number;
  text: string;
  confidence?: number;
}

interface TranscriptionResult {
  text: string;
  segments: TranscriptionSegment[];
  duration: number;
  language?: string;
  model_name?: string;
  confidence_score?: number;
}

interface TranscriptionDisplayProps {
  filePath: string;
  className?: string;
  compact?: boolean;
  refreshTrigger?: number; // Optional prop to force refresh
  onSeekToTime?: (timestamp: number) => void; // Callback for seeking video to timestamp
  isTranscribing?: boolean; // Optional prop to show transcribing state
}

export const TranscriptionDisplay: React.FC<TranscriptionDisplayProps> = ({
  filePath,
  className = '',
  compact = false,
  refreshTrigger,
  onSeekToTime,
  isTranscribing = false
}) => {
  const [transcription, setTranscription] = useState<TranscriptionResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showSegments, setShowSegments] = useState(!compact);
  const isLoadingRef = useRef(false);

  const loadTranscription = useCallback(async () => {
    // Prevent multiple simultaneous loads
    if (isLoadingRef.current) {
      return;
    }

    // Additional check - if we're already loading, abort
    if (isLoading) {
      return;
    }

    try {
      isLoadingRef.current = true;
      setIsLoading(true);
      setError(null);

      // Clean up the file path - remove asset protocol and decode URL encoding
      let cleanPath = filePath;
      if (cleanPath.startsWith('asset://localhost/')) {
        cleanPath = decodeURIComponent(cleanPath.replace('asset://localhost/', ''));
      } else if (cleanPath.startsWith('asset://')) {
        cleanPath = decodeURIComponent(cleanPath.replace('asset://', ''));
      } else if (cleanPath.startsWith('file://')) {
        cleanPath = decodeURIComponent(cleanPath.replace('file://', ''));
      }

      // Add timeout to prevent hanging
      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error('Transcription load timeout')), 10000)
      );

      const result = await Promise.race([
        invoke<any>('get_transcription_by_path', { filePath: cleanPath }),
        timeoutPromise
      ]);

      if (result) {
        // Backend already parses segments JSON, so use directly
        let segments = Array.isArray(result.segments) ? result.segments : [];

        // Safety limit to prevent UI freezing with too many segments
        if (segments.length > 1000) {
          console.warn('TranscriptionDisplay: Too many segments, truncating to 1000');
          segments = segments.slice(0, 1000);
        }

        const parsedResult: TranscriptionResult = {
          ...result,
          text: result.transcription_text || '',
          duration: result.duration_seconds || 0,
          segments: segments
        };

        setTranscription(parsedResult);
      } else {
        console.log('TranscriptionDisplay: No transcription found');
        setTranscription(null);
      }
    } catch (err) {
      console.error('Failed to load transcription:', err);
      setError(err instanceof Error ? err.message : 'Failed to load transcription');
    } finally {
      setIsLoading(false);
      isLoadingRef.current = false;
    }
  }, [filePath]);

  // Remove loadTranscription from useCallback dependencies to prevent infinite loops
  // eslint-disable-next-line react-hooks/exhaustive-deps

  useEffect(() => {
    // Only load if we're not already loading and have a valid file path
    if (!isLoadingRef.current && filePath) {
      loadTranscription();
    }

    // Cleanup function to prevent state updates on unmounted component
    return () => {
      isLoadingRef.current = false;
    };
  }, [filePath, refreshTrigger]); // Depend on filePath and refreshTrigger

  const formatTime = (seconds: number) => {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`;
  };

  const handleCopyText = () => {
    if (transcription) {
      navigator.clipboard.writeText(transcription.text);
      toast.success('Transcription copied to clipboard!');
    }
  };

  const handleSegmentClick = (segment: TranscriptionSegment) => {
    if (onSeekToTime) {
      onSeekToTime(segment.start);
      toast.success(`Jumped to ${formatTime(segment.start)}`);
    }
  };

  const renderTranscriptionWithTimestamps = () => {
    if (!transcription || !transcription.segments || transcription.segments.length === 0) {
      return (
        <p className="text-sm text-gray-800 dark:text-customGray leading-relaxed whitespace-pre-wrap">
          {transcription?.text || 'No transcription text available'}
        </p>
      );
    }

    // If no onSeekToTime callback, render plain text with timestamps
    if (!onSeekToTime) {
      return (
        <div className="text-sm text-gray-800 dark:text-customGray leading-relaxed space-y-1">
          {transcription.segments.map((segment, index) => (
            <div key={index} className="flex items-start space-x-2">
              <span className="text-xs text-gray-500 dark:text-customGray font-mono flex-shrink-0">
                [{formatTime(segment.start)}]
              </span>
              <span className="flex-1">{segment.text}</span>
            </div>
          ))}
        </div>
      );
    }

    // Render clickable segments with timestamps
    return (
      <div className="text-sm text-gray-800 dark:text-customGray leading-relaxed space-y-1">
        {transcription.segments.map((segment, index) => (
          <div
            key={index}
            onClick={() => handleSegmentClick(segment)}
            className="flex items-start space-x-2 cursor-pointer hover:bg-blue-100 dark:hover:bg-blue-900/30 hover:text-blue-700 dark:hover:text-blue-300 rounded px-2 py-1 transition-colors duration-150"
            title={`Click to jump to ${formatTime(segment.start)}`}
          >
            <span className="text-xs text-gray-500 dark:text-customGray font-mono flex-shrink-0">
              [{formatTime(segment.start)}]
            </span>
            <span className="flex-1">{segment.text}</span>
          </div>
        ))}
      </div>
    );
  };

  if (isLoading) {
    return (
      <Card className={`p-4 ${className}`}>
        <div className="flex items-center space-x-2">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span className="text-sm text-gray-600 dark:text-customGray">Loading transcription...</span>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className={`p-4 ${className}`}>
        <div className="flex items-center space-x-2 text-red-600 dark:text-customRed">
          <AlertCircle className="h-4 w-4" />
          <span className="text-sm">Error: {error}</span>
        </div>
      </Card>
    );
  }

  if (!transcription) {
    if (isTranscribing) {
      return (
        <Card className={`p-4 ${className}`}>
          <div className="flex items-center space-x-2 text-blue-600 dark:text-blueHighlight">
            <Loader2 className="h-4 w-4 animate-spin" />
            <span className="text-sm">Transcribing...</span>
          </div>
        </Card>
      );
    }
    
    return (
      <Card className={`p-4 ${className}`}>
        <div className="flex items-center space-x-2 text-gray-500 dark:text-customGray">
          <FileAudio className="h-4 w-4" />
          <span className="text-sm">No transcription available</span>
        </div>
      </Card>
    );
  }

  // Safety check for malformed transcription data
  if (!transcription.text || !Array.isArray(transcription.segments)) {
    return (
      <Card className={`p-4 ${className}`}>
        <div className="flex items-center space-x-2 text-red-600 dark:text-customRed">
          <AlertCircle className="h-4 w-4" />
          <span className="text-sm">Invalid transcription data format</span>
        </div>
      </Card>
    );
  }

  if (compact) {
    return (
      <Card className={`p-3 ${className}`}>
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <FileAudio className="h-4 w-4 text-blue-600 dark:text-blueHighlight" />
              <span className="text-sm font-medium">Transcription</span>
              {transcription.language && (
                <Badge variant="secondary" className="text-xs">
                  {transcription.language}
                </Badge>
              )}
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopyText}
              className="h-6 px-2"
            >
              <Copy className="h-3 w-3" />
            </Button>
          </div>
          <div className="text-sm text-gray-700 dark:text-customGray line-clamp-3">
            {renderTranscriptionWithTimestamps()}
          </div>
          {transcription.segments.length > 0 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowSegments(!showSegments)}
              className="text-xs h-6"
            >
              {showSegments ? 'Hide' : 'Show'} {transcription.segments.length} segments
            </Button>
          )}
        </div>
      </Card>
    );
  }

  return (
    <Card className={`p-4 ${className}`}>
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <FileAudio className="h-4 w-4 text-blue-600 dark:text-blueHighlight" />
            <span className="font-medium text-sm dark:text-white">Transcription</span>
            {transcription.duration && (
              <span className="text-xs text-gray-500 dark:text-customGray">
                ({formatTime(transcription.duration)})
              </span>
            )}
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleCopyText}
            className="h-6 px-2"
          >
            <Copy className="h-3 w-3" />
          </Button>
        </div>

        <div className="bg-gray-50 dark:bg-darkBgMid p-3 rounded-lg">
          {renderTranscriptionWithTimestamps()}
        </div>
      </div>
    </Card>
  );
};

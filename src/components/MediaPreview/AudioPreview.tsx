import React, { useState } from 'react';
import { MediaFile } from './types';
import { Music, Play, FileText } from 'lucide-react';
import { cn } from '../../lib/utils';
import { TranscriptionDisplay } from '../TranscriptionDisplay';
import { Button } from '../ui/button';
import { Card } from '../ui/card';
import { invoke } from '@tauri-apps/api/core';

interface AudioPreviewProps {
  file: MediaFile;
  isTranscribing?: boolean;
  onTranscribeFile?: (path: string) => void;
}

export function AudioPreview({ file, isTranscribing = false, onTranscribeFile }: AudioPreviewProps) {
  const [showTranscription, setShowTranscription] = useState(false);
  const [hasTranscription, setHasTranscription] = useState(false);


  // Check if transcription exists
  React.useEffect(() => {
    const checkTranscription = async () => {
      try {
        const result = await invoke('get_transcription_by_path', {
          filePath: file.path.replace('file://', '')
        });
        setHasTranscription(!!result);
      } catch (error) {
        setHasTranscription(false);
      }
    };

    checkTranscription();
  }, [file.path]);


  return (
    <div className="w-full h-full bg-gray-50 rounded-lg p-4 flex flex-col">
      {/* Audio Player Section */}
      <div className="flex-shrink-0 text-center mb-4">
        <Music className="h-16 w-16 text-blue-500 mb-4 mx-auto" />
        <div className="text-sm font-medium mb-1">{file.name}</div>
        <div className="text-xs text-gray-500 mb-4">Audio File</div>

        <div className="flex items-center justify-center gap-4 mb-4">
          <button className="p-2 rounded-full bg-blue-500 text-white hover:bg-blue-600 transition-colors">
            <Play className="h-6 w-6" />
          </button>
        </div>

        {/* Progress Bar or Transcription Progress */}
        <div className="w-full max-w-xs mx-auto mb-4">
          {isTranscribing ? (
            <div>
              <div className="h-1 bg-gray-200 rounded-full overflow-hidden">
                <div className="h-full bg-blue-500 rounded-full animate-pulse" />
              </div>
              <div className="text-center mt-1 text-xs text-blue-600">
                Transcribing audio...
              </div>
            </div>
          ) : (
            <div>
              <div className="h-1 bg-gray-200 rounded-full">
                <div className="h-full w-1/3 bg-blue-500 rounded-full" />
              </div>
              <div className="flex justify-between mt-1 text-xs text-gray-500">
                <span>0:00</span>
                <span>3:45</span>
              </div>
            </div>
          )}
        </div>

        {/* Transcription Controls */}
        <div className="flex gap-2 justify-center mb-4">
          {hasTranscription && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowTranscription(!showTranscription)}
              className="flex items-center space-x-2"
            >
              <FileText className="h-4 w-4" />
              <span>{showTranscription ? 'Hide' : 'Show'} Transcription</span>
            </Button>
          )}
        </div>
      </div>

      {/* Transcription Section */}
      {showTranscription && (
        <div className="flex-1 overflow-y-auto">
          <TranscriptionDisplay
            filePath={file.path}
            compact={false}
          />
        </div>
      )}
    </div>
  );
}

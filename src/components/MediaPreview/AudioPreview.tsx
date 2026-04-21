import React, { useState } from 'react';
import { MediaFile } from './types';
import { Music, FileText } from 'lucide-react';
import { TranscriptionDisplay } from '../TranscriptionDisplay';
import { Button } from '../ui/button';
import { invoke } from '@tauri-apps/api/core';
import { normalizeFilePath } from '../../lib/utils';

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
          filePath: normalizeFilePath(file.path)
        });
        setHasTranscription(!!result);
      } catch {
        setHasTranscription(false);
      }
    };

    checkTranscription();
  }, [file.path, isTranscribing]);

  return (
    <div className="w-full h-full bg-gray-50 rounded-lg p-6 flex flex-col">
      <div className="flex-shrink-0 text-center mb-6">
        <Music className="h-16 w-16 text-blue-500 mb-4 mx-auto" />
        <div className="text-sm font-medium mb-1">{file.name}</div>
        <div className="text-xs text-gray-500 mb-4">Audio File</div>

        <div className="w-full max-w-xl mx-auto mb-4">
          <audio
            controls
            preload="metadata"
            src={file.path}
            className="w-full"
          />
        </div>

        <div className="flex gap-2 justify-center mb-4">
          {onTranscribeFile && !hasTranscription && (
            <Button
              variant="default"
              size="sm"
              disabled={isTranscribing}
              onClick={() => onTranscribeFile(normalizeFilePath(file.path))}
            >
              {isTranscribing ? 'Transcribing...' : 'Transcribe'}
            </Button>
          )}
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

import React, { useState, useEffect } from 'react';
import { MediaFile } from './types';
import { FileText, AlertCircle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';

interface DocumentPreviewProps {
  file: MediaFile;
}

export function DocumentPreview({ file }: DocumentPreviewProps) {
  const [content, setContent] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const ext = file.name.split('.').pop()?.toLowerCase() || '';
  const isPdf = ext === 'pdf';
  const isText = [
    'txt', 'md', 'json', 'js', 'jsx', 'ts', 'tsx', 'css', 'html', 'xml',
    'py', 'rb', 'java', 'cpp', 'c', 'h', 'rs', 'go', 'php', 'sql'
  ].includes(ext);

  useEffect(() => {
    async function loadContent() {
      setIsLoading(true);
      setError(null);
      
      try {
        if (isText) {
          // Read text content using Tauri command
          const content = await invoke<string>('read_text_file', { path: file.path });
          setContent(content);
        }
      } catch (err) {
        setError('Failed to load document content');
        console.error('Error loading document:', err);
      } finally {
        setIsLoading(false);
      }
    }

    loadContent();
  }, [file.path, isText]);

  if (isLoading) {
    return (
      <div className="w-full h-full flex items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="w-full h-full flex flex-col items-center justify-center text-gray-500 gap-2">
        <AlertCircle className="h-8 w-8" />
        <p>{error}</p>
      </div>
    );
  }

  if (isPdf) {
    return (
      <div className="w-full h-[calc(100vh-3rem)] flex flex-col overflow-hidden">
        <iframe
          src={`${file.path}#toolbar=0`}
          className="w-full h-full border-none"
          title={file.name}
        />
      </div>
    );
  }

  if (isText && content !== null) {
    return (
      <div className="w-full h-[calc(100vh-3rem)] flex flex-col overflow-hidden">
        <pre className="font-mono text-sm whitespace-pre-wrap p-4 h-full overflow-auto">{content}</pre>
      </div>
    );
  }

  return (
    <div className="w-full h-[calc(100vh-3rem)] flex flex-col items-center justify-center text-gray-500">
      <FileText className="h-16 w-16 mb-2" />
      <p>Preview not available for this file type</p>
      <p className="text-sm mt-1">({file.name})</p>
    </div>
  );
} 
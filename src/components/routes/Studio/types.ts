export interface VideoGeneration {
  id: string;
  prompt: string;
  status: 'generating' | 'completed' | 'failed';
  videoUrl?: string;
  thumbnailUrl?: string;
  createdAt: Date;
  duration?: number;
  operationId?: string;
  videoPath?: string;
  jsonPrompt?: string;
  error?: string;
} 
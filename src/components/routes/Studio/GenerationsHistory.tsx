import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { Button } from "../../ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../ui/card";
import { Badge } from "../../ui/badge";
import { Separator } from "../../ui/separator";
import { Trash2, Play, Eye, Calendar, FileVideo, Copy } from "lucide-react";
import { toast } from "sonner";

interface Generation {
  id: string;
  user_prompt: string;
  json_prompt: string;
  source: string;
  generated_file_path: string | null;
  created_at: string;
  updated_at: string;
}

interface GenerationsHistoryProps {
  onViewJsonPrompt: (jsonPrompt: string) => void;
  onPlayVideo: (videoPath: string) => void;
}

export const GenerationsHistory: React.FC<GenerationsHistoryProps> = ({
  onViewJsonPrompt,
  onPlayVideo,
}) => {
  const [generations, setGenerations] = useState<Generation[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadGenerations = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const generationsData = await invoke<Generation[]>('get_all_generations');
      setGenerations(generationsData);
    } catch (err) {
      console.error('Failed to load generations:', err);
      setError('Failed to load generation history');
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadGenerations();
  }, []);

  const handleDeleteGeneration = async (generationId: string) => {
    try {
      await invoke('delete_generation', { generationId });
      // Reload generations after deletion
      await loadGenerations();
    } catch (err) {
      console.error('Failed to delete generation:', err);
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await invoke('copy_to_clipboard', { text });
      toast.success("Copied to clipboard")
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
      toast.error("Failed to copy to clipboard")
    }
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const truncatePrompt = (prompt: string, maxLength: number = 100) => {
    if (prompt.length <= maxLength) return prompt;
    return prompt.substring(0, maxLength) + '...';
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4"></div>
          <p className="text-muted-foreground">Loading generation history...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <p className="text-destructive mb-4">{error}</p>
          <Button onClick={loadGenerations} variant="outline">
            Try Again
          </Button>
        </div>
      </div>
    );
  }

  if (generations.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <div className="w-32 h-32 bg-purple-100 dark:bg-purple-900/20 rounded-full flex items-center justify-center mx-auto mb-4">
            <FileVideo className="w-16 h-16 text-purple-600 dark:text-purple-400" />
          </div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white text-center mb-2">No Generations Yet</h2>
          <p className="text-lg text-gray-600 dark:text-gray-400 text-center mb-8">
            Your video generation history will appear here once you create your first video.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4 w-11/12 mx-auto">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold">Generation History</h2>
        <Button onClick={loadGenerations} variant="outline" size="sm">
          Refresh
        </Button>
      </div>

      <div className="grid gap-4">
        {generations.map((generation) => (
          <Card key={generation.id} className="hover:shadow-md transition-shadow bg-white dark:bg-darkBgMid rounded-xl">
            <CardHeader className="pb-3">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <CardTitle className="text-base">
                    {truncatePrompt(generation.user_prompt)}
                  </CardTitle>
                  <CardDescription className="flex items-center gap-2 mt-1">
                    <Calendar className="h-4 w-4" />
                    {formatDate(generation.created_at)}
                  </CardDescription>
                </div>
                <div className="flex items-center gap-2">
                  <Badge variant="secondary" className="text-xs">
                    {generation.source}
                  </Badge>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => copyToClipboard(generation.user_prompt)}
                    className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
                  >
                    <Copy className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => handleDeleteGeneration(generation.id)}
                    className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            </CardHeader>

            <CardContent className="pt-0">
              <div className="flex items-center gap-2 mb-3">
                {generation.generated_file_path ? (
                  <>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => onViewJsonPrompt(generation.json_prompt)}
                      className="flex items-center gap-2"
                    >
                      <Eye className="h-4 w-4" />
                      View JSON
                    </Button>
                  </>
                ) : (
                  <Badge variant="outline" className="text-xs">
                    No video file
                  </Badge>
                )}
              </div>

              <Separator className="my-3" />

              <div className="text-sm text-muted-foreground">
                <p className="font-medium mb-1">User Prompt:</p>
                <p className="text-xs bg-muted p-2 rounded">
                  {generation.user_prompt}
                </p>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}; 
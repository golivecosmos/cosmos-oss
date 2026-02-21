import React, { useState } from "react";
import { useNavigate, useOutletContext } from "react-router-dom";

import { EmptyState } from "./EmptyState";
import { VideoGrid } from "./VideoGrid";
import { GenerationsHistory } from "./GenerationsHistory";
import { convertFileSrc } from "@tauri-apps/api/tauri";
import { VideoGeneration } from "./types";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../ui/tabs";

export const Studio: React.FC = () => {
  const navigate = useNavigate();

  const [activeTab, setActiveTab] = useState<string>("videos");

  const {
    existingVideos,
    generatedVideos,
    isLoadingExisting,

    setSelectedJsonPrompt,
    setSelectedVideoForPlayer,
    setShowJsonPrompt,
  } = useOutletContext<any>();

  const handleViewJsonPromptFromHistory = (jsonPrompt: string) => {
    setSelectedJsonPrompt(jsonPrompt);
    setShowJsonPrompt(true);
  };

  const handlePlayVideoFromHistory = (videoPath: string) => {
    // Create a video generation object for the player
    const videoGeneration: VideoGeneration = {
      id: 'history-video',
      prompt: 'Generated Video',
      status: 'completed',
      videoPath: videoPath,
      videoUrl: convertFileSrc(videoPath),
      createdAt: new Date(),
      duration: 8,
      operationId: undefined,
      jsonPrompt: undefined,
      error: undefined
    };
    setSelectedVideoForPlayer(videoGeneration);
  };

  return (
    <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
      <TabsList className="grid w-full grid-cols-2 p-0 bg-white dark:bg-darkBgMid h-auto sticky top-0 shadow-lg">
        <TabsTrigger className="py-4 hover:bg-blue-100/40 dark:hover:bg-darkBgHighlight/40 data-[state=active]:bg-blue-100 dark:data-[state=active]:bg-darkBgHighlight" value="videos">Videos</TabsTrigger>
        <TabsTrigger className="py-4 hover:bg-blue-100/40 dark:hover:bg-darkBgHighlight/40 data-[state=active]:bg-blue-100 dark:data-[state=active]:bg-darkBgHighlight" value="history">Generation History</TabsTrigger>
      </TabsList>

      <TabsContent value="videos" className="h-auto max-h-full overflow-y-auto py-8 m-0">
        {existingVideos.length === 0 && generatedVideos.length === 0 ? (
          <EmptyState />
        ) : (
          (existingVideos.length > 0 || generatedVideos.length > 0) && (
            <VideoGrid
              existingVideos={existingVideos}
              generatedVideos={generatedVideos}
              isLoadingExisting={isLoadingExisting}
              onVideoSelect={(video) => {
                navigate(`/studio/edit?path=${video.videoPath}`)
              }}
            />
          )
        )}
      </TabsContent>

      <TabsContent value="history" className="h-full max-h-full overflow-y-auto py-8 m-0">
        <GenerationsHistory
          onViewJsonPrompt={handleViewJsonPromptFromHistory}
          onPlayVideo={handlePlayVideoFromHistory}
        />
      </TabsContent>
    </Tabs>
  );
};
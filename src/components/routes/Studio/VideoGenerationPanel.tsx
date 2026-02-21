import React, { useEffect } from "react";

import { Button } from "../../ui/button";
import { Textarea } from "../../ui/textarea";
import { Sparkles, Loader2, X } from "lucide-react";
import { GoogleGeminiStatusAlert } from "./GoogleGeminiStatusAlert";
import { VideoGeneration } from "./types";
import { cn } from "../../../lib/utils";

interface VideoGenerationPanelProps {
  isOpen: boolean;
  prompt: string;
  setPrompt: (prompt: string) => void;
  isGenerating: boolean;
  onGenerate: () => void;
  selectedStyle: string;
  setSelectedStyle: (style: string) => void;
  selectedDuration: number;
  setSelectedDuration: (duration: number) => void;
  selectedModel: string;
  setSelectedModel: (model: string) => void;
  selectedAspectRatio: string;
  setSelectedAspectRatio: (ratio: string) => void;
  image: { bytesBase64Encoded: string; mimeType: string } | null;
  setImage: (img: { bytesBase64Encoded: string; mimeType: string } | null) => void;
  googleGeminiInstalled: boolean;
  isCheckingVeo3: boolean;
  onInstallVeo3: () => void;
  generatedVideos: VideoGeneration[];
  onClose: () => void;
}

export const VideoGenerationPanel: React.FC<VideoGenerationPanelProps> = ({
  isOpen,
  prompt,
  setPrompt,
  isGenerating,
  onGenerate,
  selectedStyle,
  setSelectedStyle,
  selectedDuration,
  setSelectedDuration,
  selectedModel,
  setSelectedModel,
  selectedAspectRatio,
  setSelectedAspectRatio,
  image,
  setImage,
  googleGeminiInstalled,
  isCheckingVeo3,
  onInstallVeo3,
  generatedVideos,
  onClose,
}) => {
  const isVeo2 = /\bveo-2\b/.test(selectedModel) || selectedModel.startsWith("veo-2");
  const allowedAspectRatios = isVeo2 ? ["16:9", "9:16"] : ["16:9"];

  useEffect(() => {
    if (!allowedAspectRatios.includes(selectedAspectRatio)) {
      setSelectedAspectRatio(allowedAspectRatios[0]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedModel]);
  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) {
      setImage(null);
      return;
    }
    const mimeType = file.type || "image/png";
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      // result is a data URL like "data:image/png;base64,...."; strip the prefix
      const base64 = result.includes(",") ? result.split(",")[1] : result;
      setImage({ bytesBase64Encoded: base64, mimeType });
    };
    reader.readAsDataURL(file);
  };
  return (
    <div
      className={
        cn(
          "group fixed inset-y-0 right-0 dark:bg-darkBg bg-white shadow-xl border-l dark:border-darkBgHighlight border-gray-200 flex flex-col z-50",
          "data-[state=open]:w-[500px] data-[state=closed]:w-0 transition-width duration-300",
          "data-[state=closed]:delay-150"
        )
      }
      data-state={isOpen ? "open" : "closed"}
    >
      <div className={
        cn(
          "w-full h-full transition-opacity duration-150",
          "group-data-[state=closed]:opacity-0 group-data-[state=open]:opacity-100 group-data-[state=open]:delay-300"
        )
      }>
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b dark:border-darkBgHighlight border-gray-200">
          <div className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-purple-600 dark:text-purple-400" />
            <h2 className="text-lg font-semibold dark:text-text text-gray-900">Video Creator</h2>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-gray-500 dark:text-gray-400">
              ⌘P to toggle
            </span>
            <Button variant="ghost" size="icon" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">
          <div className="space-y-6">
            {/* Veo3 Status */}
            {!googleGeminiInstalled && (
              <GoogleGeminiStatusAlert onInstallVeo3={onInstallVeo3} isChecking={isCheckingVeo3} />
            )}

            {/* Prompt Input */}
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Describe your video
                </label>
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  {prompt.length}/500
                </span>
              </div>
              <Textarea
                placeholder="A cinematic shot of a futuristic city at sunset with flying cars and neon lights..."
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                className="min-h-[120px] resize-none"
                disabled={isGenerating}
                maxLength={500}
              />
            </div>

            {/* Generation Settings */}
            <div className="space-y-3">
              <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300">Settings</h3>
              <div className="grid grid-cols-2 gap-3">
                <div className="space-y-1">
                  <label className="text-xs text-gray-600 dark:text-gray-400">Duration</label>
                  <select
                    value={selectedDuration}
                    onChange={(e) => setSelectedDuration(Number(e.target.value))}
                    className="w-full text-sm border border-gray-300 dark:border-gray-600 rounded-md px-3 py-2 bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                  >
                    <option value={8}>8 seconds</option>
                    <option value={12} disabled>12 seconds</option>
                    <option value={16} disabled>16 seconds</option>
                    <option value={24} disabled>24 seconds</option>
                  </select>
                </div>
                <div className="space-y-1">
                  <label className="text-xs text-gray-600 dark:text-gray-400">Style</label>
                  <select
                    value={selectedStyle}
                    onChange={(e) => setSelectedStyle(e.target.value)}
                    className="w-full text-sm border border-gray-300 dark:border-gray-600 rounded-md px-3 py-2 bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                  >
                    <option value="cinematic">Cinematic</option>
                    <option value="realistic">Realistic</option>
                    <option value="artistic">Artistic</option>
                    <option value="animated">Animated</option>
                  </select>
                </div>
                <div className="space-y-1">
                  <label className="text-xs text-gray-600 dark:text-gray-400">Aspect ratio</label>
                  <select
                    value={selectedAspectRatio}
                    onChange={(e) => setSelectedAspectRatio(e.target.value)}
                    className="w-full text-sm border border-gray-300 dark:border-gray-600 rounded-md px-3 py-2 bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                    disabled={isGenerating}
                  >
                    {allowedAspectRatios.map((ar) => (
                      <option key={ar} value={ar}>{ar}</option>
                    ))}
                  </select>
                </div>
                <div className="space-y-1">
                  <label className="text-xs text-gray-600 dark:text-gray-400">Reference image (optional)</label>
                  <div className="flex items-center gap-2">
                    <input
                      type="file"
                      accept="image/*"
                      onChange={handleFileChange}
                      disabled={isGenerating}
                      className="block w-full text-xs text-gray-700 dark:text-gray-300"
                    />
                    {image && (
                      <Button variant="secondary" onClick={() => setImage(null)} disabled={isGenerating}>Remove</Button>
                    )}
                  </div>
                  {image && (
                    <div className="text-[11px] text-gray-500 dark:text-gray-400 truncate">{image.mimeType}</div>
                  )}
                </div>
                <div className="space-y-1 col-span-2">
                  <label className="text-xs text-gray-600 dark:text-gray-400">Model</label>
                  <select
                    value={selectedModel}
                    onChange={(e) => setSelectedModel(e.target.value)}
                    className="w-full text-sm border border-gray-300 dark:border-gray-600 rounded-md px-3 py-2 bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                    disabled={isGenerating}
                  >
                    <option value="veo-3.1-generate-preview">veo-3.1-generate-preview</option>
                    <option value="veo-3.1-fast-generate-preview">veo-3.1-fast-generate-preview</option>
                  </select>
                </div>
              </div>
            </div>

            <Button
              onClick={onGenerate}
              disabled={!prompt.trim() || isGenerating}
              className="w-full bg-purple-600 hover:bg-purple-700 text-white"
            >
              {isGenerating ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Generating...
                </>
              ) : (
                <>
                  <Sparkles className="w-4 h-4 mr-2" />
                  Generate Video
                </>
              )}
            </Button>

            {/* Recent Prompts */}
            {generatedVideos.length > 0 && (
              <div className="space-y-3">
                <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300">Recent Prompts</h3>
                <div className="space-y-2">
                  {generatedVideos.slice(0, 5).map((video) => (
                    <button
                      key={video.id}
                      onClick={() => setPrompt(video.prompt)}
                      className="w-full text-left p-3 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800 text-sm text-gray-600 dark:text-gray-400 border border-gray-200 dark:border-gray-700 transition-colors"
                    >
                      <div className="font-medium text-gray-900 dark:text-gray-100 mb-1">
                        {video.prompt.substring(0, 60)}...
                      </div>
                      <div className="text-xs text-gray-500 dark:text-gray-400">
                        {video.createdAt.toLocaleDateString()}
                      </div>
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

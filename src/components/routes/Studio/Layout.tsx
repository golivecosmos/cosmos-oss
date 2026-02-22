import React, { useState, useEffect, useCallback } from "react";
import { Outlet } from "react-router-dom";
import { useAppLayout } from "../../../contexts/AppLayoutContext";
import { JsonPromptModal } from "./JsonPromptModal";
import { VideoGenerationPanel } from "./VideoGenerationPanel";
import { PanelToggle } from "./PanelToggle";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { VideoGeneration } from "./types";

const GEMINI_APP_NAME = "Google Gemini";

export const StudioLayout: React.FC = () => {
    const { handleOpenAppStore, showAppStore } = useAppLayout();

    const [isPromptPanelOpen, setIsPromptPanelOpen] = useState(false);
    const [prompt, setPrompt] = useState("");
    const [isGenerating, setIsGenerating] = useState(false);
    const [generatedVideos, setGeneratedVideos] = useState<VideoGeneration[]>([]);
    const [googleGeminiInstalled, setGoogleGeminiInstalled] = useState(false);
    const [isCheckingVeo3, setIsCheckingVeo3] = useState(false);
    const [showJsonPrompt, setShowJsonPrompt] = useState(false);
    const [selectedJsonPrompt, setSelectedJsonPrompt] = useState<string>("");
    const [existingVideos, setExistingVideos] = useState<VideoGeneration[]>([]);
    const [isLoadingExisting, setIsLoadingExisting] = useState(false);
    const [selectedStyle, setSelectedStyle] = useState<string>("cinematic");
    const [selectedDuration, setSelectedDuration] = useState<number>(8);
    const [selectedModel, setSelectedModel] = useState<string>("veo-3.1-fast-generate-preview");
    const [selectedAspectRatio, setSelectedAspectRatio] = useState<string>("16:9");
    const [image, setImage] = useState<{ bytesBase64Encoded: string; mimeType: string } | null>(null);

    // Check Veo3 installation status
    const checkVeo3Installation = useCallback(async () => {
        try {
            setIsCheckingVeo3(true);
            const installedApps = await invoke<any[]>('get_installed_apps');
            const googleGeminiInstalled = installedApps.some(app => app.app_name === GEMINI_APP_NAME);
            setGoogleGeminiInstalled(googleGeminiInstalled);
            return googleGeminiInstalled;
        } catch (error) {
            console.error('Failed to check Veo3 installation:', error);
            setGoogleGeminiInstalled(false);
            return false;
        } finally {
            setIsCheckingVeo3(false);
        }
    }, []);

    // Load existing videos
    useEffect(() => {
        const loadExistingVideos = async () => {
            setIsLoadingExisting(true);
            try {
                const desktopPath = await invoke<string>('get_desktop_path');
                const cosmosVideosPath = `${desktopPath}/cosmos_videos`;

                const files = await invoke<any[]>('list_directory', { path: cosmosVideosPath });

                const videoFiles = Array.isArray(files)
                    ? files.filter(file => {
                        if (!file || typeof file !== 'object' || !file.name) return false;
                        const fileName = file.name.toLowerCase();
                        return fileName.endsWith('.mp4') || fileName.endsWith('.mov') || fileName.endsWith('.avi');
                    })
                    : [];

                const existingVideosData = videoFiles.map((file, index) => ({
                    id: `existing-${index}`,
                    prompt: file.name.replace(/\.(mp4|mov|avi)$/i, '').replace(/_/g, ' '),
                    status: 'completed' as const,
                    videoPath: `${cosmosVideosPath}/${file.name}`,
                    videoUrl: convertFileSrc(`${cosmosVideosPath}/${file.name}`),
                    createdAt: new Date(),
                    duration: 8,
                    operationId: undefined,
                    jsonPrompt: undefined,
                    error: undefined
                }));

                setExistingVideos(existingVideosData);
            } catch (error) {
                console.error('❌ Failed to load existing videos:', error);
                setExistingVideos([]);
            } finally {
                setIsLoadingExisting(false);
            }
        };

        loadExistingVideos();
    }, []);

    // Check Veo3 installation
    useEffect(() => {
        checkVeo3Installation();
    }, [checkVeo3Installation]);

    // Refresh Veo3 installation status when window regains focus
    useEffect(() => {
        const handleFocus = () => {
            checkVeo3Installation();
        };

        window.addEventListener('focus', handleFocus);
        return () => window.removeEventListener('focus', handleFocus);
    }, [checkVeo3Installation]);

    // Refresh Veo3 installation status when App Store closes
    useEffect(() => {
        if (!showAppStore) {
            // App Store was closed, check if Veo3 was installed
            const checkAfterAppStoreClose = async () => {
                // Add a small delay to ensure the installation process has completed
                setTimeout(async () => {
                    await checkVeo3Installation();
                }, 1000);
            };

            checkAfterAppStoreClose();
        }
    }, [showAppStore, checkVeo3Installation]);

    // Listen for app installation events
    useEffect(() => {
        const setupAppInstallationListener = async () => {
            try {
                const unlistenInstalled = await listen('app_installed', (event) => {
                    console.log('🔔 App installed event received:', event.payload);
                    const payload = event.payload as { app_name: string; app_id: number; message: string };

                    if (payload.app_name === GEMINI_APP_NAME) {
                        console.log('✅ Veo3 installed, refreshing installation status...');
                        checkVeo3Installation();
                    }
                });

                const unlistenUninstalled = await listen('app_uninstalled', (event) => {
                    console.log('🔔 App uninstalled event received:', event.payload);
                    const payload = event.payload as { app_name: string; app_id: number; message: string };

                    if (payload.app_name === GEMINI_APP_NAME) {
                        console.log('❌ Veo3 uninstalled, refreshing installation status...');
                        checkVeo3Installation();
                    }
                });

                return () => {
                    unlistenInstalled();
                    unlistenUninstalled();
                };
            } catch (error) {
                console.error('Failed to setup app installation listener:', error);
            }
        };

        const cleanup = setupAppInstallationListener();

        return () => {
            cleanup.then(unlisten => {
                if (unlisten) {
                    unlisten();
                }
            });
        };
    }, [checkVeo3Installation]);

    // Keyboard shortcuts
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                setIsPromptPanelOpen(false);
            }
            if (e.key === 'p' && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                setIsPromptPanelOpen(!isPromptPanelOpen);
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [isPromptPanelOpen]);

    // Video generation handlers
    const handleGenerateVideo = async () => {
        if (!prompt.trim()) return;

        // Check if Veo3 is installed before proceeding
        const isVeo3Installed = await checkVeo3Installation();
        if (!isVeo3Installed) {
            console.warn('Veo3 is not installed. Please install Veo3 first.');
            return;
        }

        const newVideo: VideoGeneration = {
            id: Date.now().toString(),
            prompt: prompt,
            status: 'generating',
            createdAt: new Date(),
        };

        setGeneratedVideos(prev => [newVideo, ...prev]);
        setIsGenerating(true);
        setPrompt("");

        try {
            const isVeo2 = /\bveo-2\b/.test(selectedModel) || selectedModel.startsWith('veo-2');
            const requestPayload: any = {
                prompt: prompt,
                duration_seconds: selectedDuration,
                style: selectedStyle.toLowerCase(),
                model: selectedModel,
                image: image
                    ? { bytesBase64Encoded: image.bytesBase64Encoded, mimeType: image.mimeType }
                    : undefined,
            };
            if (isVeo2) {
                requestPayload.aspect_ratio = selectedAspectRatio;
            }

            const response = await invoke('generate_video_prompt', {
                request: requestPayload
            }) as { operation_id: string; status: string; message: string };

            setGeneratedVideos(prev =>
                prev.map(video =>
                    video.id === newVideo.id
                        ? { ...video, operationId: response.operation_id }
                        : video
                )
            );

            pollVideoStatus(response.operation_id, newVideo.id);
        } catch (error) {
            setGeneratedVideos(prev =>
                prev.map(video =>
                    video.id === newVideo.id
                        ? { ...video, status: 'failed', error: error as string }
                        : video
                )
            );
            setIsGenerating(false);
        }
    };

    const pollVideoStatus = async (operationId: string, videoId: string) => {
        const pollInterval = setInterval(async () => {
            try {
                const status = await invoke('get_video_generation_status', {
                    operationId: operationId
                }) as { status: string; progress?: number; video_path?: string; json_prompt?: string; error?: string };

                if (status.status === 'completed' && status.video_path) {
                    const updatedVideo = {
                        status: 'completed' as const,
                        videoPath: status.video_path,
                        videoUrl: convertFileSrc(status.video_path),
                        duration: 8,
                        jsonPrompt: status.json_prompt
                    };

                    setGeneratedVideos(prev => {
                        const updatedVideos = prev.map(video =>
                            video.id === videoId ? { ...video, ...updatedVideo } : video
                        );

                        return updatedVideos;
                    });

                    setIsGenerating(false);
                    clearInterval(pollInterval);
                } else if (status.status === 'failed' || status.error) {
                    const failedVideo = {
                        status: 'failed' as const,
                        error: status.error || 'Generation failed',
                        jsonPrompt: status.json_prompt
                    };

                    setGeneratedVideos(prev =>
                        prev.map(video =>
                            video.id === videoId ? { ...video, ...failedVideo } : video
                        )
                    );

                    setIsGenerating(false);
                    clearInterval(pollInterval);
                }
            } catch (error) {
                console.error('❌ Failed to get video status:', error);
                clearInterval(pollInterval);
            }
        }, 5000);
    };

    return (
        <div className="h-full bg-gray-50 dark:bg-darkBg">
            <div className="h-full flex flex-col">
                {!isPromptPanelOpen && (
                    <PanelToggle onOpen={() => setIsPromptPanelOpen(true)} />
                )}

                <div className="flex-1 overflow-hidden">
                    <Outlet
                        context={{
                            existingVideos,
                            generatedVideos,
                            isLoadingExisting,
                            selectedJsonPrompt,

                            setSelectedJsonPrompt,
                            setShowJsonPrompt,
                        }}
                    />
                </div>
            </div>

            <VideoGenerationPanel
                isOpen={isPromptPanelOpen}
                prompt={prompt}
                setPrompt={setPrompt}
                isGenerating={isGenerating}
                onGenerate={handleGenerateVideo}
                selectedStyle={selectedStyle}
                setSelectedStyle={setSelectedStyle}
                selectedDuration={selectedDuration}
                setSelectedDuration={setSelectedDuration}
                selectedModel={selectedModel}
                setSelectedModel={setSelectedModel}
                selectedAspectRatio={selectedAspectRatio}
                setSelectedAspectRatio={setSelectedAspectRatio}
                image={image}
                setImage={setImage}
                googleGeminiInstalled={googleGeminiInstalled}
                isCheckingVeo3={isCheckingVeo3}
                onInstallVeo3={handleOpenAppStore}
                generatedVideos={generatedVideos}
                onClose={() => setIsPromptPanelOpen(false)}
            />

            <JsonPromptModal
                isOpen={showJsonPrompt}
                jsonPrompt={selectedJsonPrompt}
                onClose={() => setShowJsonPrompt(false)}
            />
        </div>
    );
};

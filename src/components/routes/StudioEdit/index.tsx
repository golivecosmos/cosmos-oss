
import { useState, useEffect, useRef } from "react";
import { useLoaderData, useNavigate, useSearchParams } from "react-router-dom";
import { Camera, CircleMinus, CirclePlus, ImageMinus, Info, Mic, Loader2 } from "lucide-react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { formatDistanceToNow } from 'date-fns';

import { Button } from "../../ui/button"
import { Slider } from "../../ui/slider"
import { FileItem } from "../../FileTree";
import { cn, formatFileSize } from "../../../lib/utils";
import { ContentPreview, ContentPreviewRef } from "./ContentPreview";
import { Tooltip, TooltipContent, TooltipTrigger } from "../../ui/tooltip";
import { TranscriptionDisplay } from "../../TranscriptionDisplay";
import { useAppLayout } from "../../../contexts/AppLayoutContext";

export const StudioEdit = () => {
    const { file, whisperStatus, isTranscribable } = useLoaderData<typeof StudioEdit.loader>();
    const navigate = useNavigate();
    const [searchParams] = useSearchParams();
    const { transcribingPaths, handleTranscribeFile } = useAppLayout();

    const [scale, setScale] = useState<number>(0.8);
    const [showInfo, setShowInfo] = useState(false);
    const [transcriptionRefreshCounter, setTranscriptionRefreshCounter] = useState(0);
    const contentPreviewRef = useRef<ContentPreviewRef>(null);

    const handleSeekToTime = (timestamp: number) => {
        contentPreviewRef.current?.seekTo(timestamp);
    };

    // Seek to timestamp from URL params when file loads
    useEffect(() => {
        const timestamp = searchParams.get('timestamp');
        if (timestamp && file && contentPreviewRef.current) {
            const timeInSeconds = parseFloat(timestamp);
            if (!isNaN(timeInSeconds)) {
                // Small delay to ensure video/audio is loaded
                const timeoutId = setTimeout(() => {
                    contentPreviewRef.current?.seekTo(timeInSeconds);
                }, 300);
                return () => clearTimeout(timeoutId);
            }
        }
    }, [searchParams, file]);

    // Track when transcription completes to trigger refresh
    useEffect(() => {
        const wasTranscribing = transcribingPaths.has(file?.path || '');

        // If this file is no longer transcribing (completed or failed), trigger refresh
        if (!wasTranscribing && file?.path) {
            const timeoutId = setTimeout(() => {
                setTranscriptionRefreshCounter(prev => prev + 1);
            }, 500); // Small delay to ensure backend has updated

            return () => clearTimeout(timeoutId);
        }
    }, [transcribingPaths, file?.path]);

    const getTransformMatrix = (scale: number) => `translate3d(0, 0, 0) scale(${scale})`

    return (
        <div className="h-full bg-gray-50 dark:bg-darkBg relative">
            <header className="flex items-center justify-between p-4 absolute top-0 z-10">
                <Button
                    variant="outline"
                    size="sm"
                    className="text-xs"
                    onClick={() => navigate(-1)}
                >
                    Back
                </Button>
            </header>
            <main className="flex-1 overflow-hidden h-full w-full">
                <div
                    className={cn(
                        "absolute inset-0 flex items-center justify-center w-full",
                    )}
                    style={{ touchAction: "none" }}
                >
                    <div
                        className="transition-transform duration-75 ease-out w-full"
                        style={{
                            transform: getTransformMatrix(scale),
                            transformOrigin: "center center",
                        }}
                    >
                        <div className="relative">
                            <ContentPreview ref={contentPreviewRef} file={file} />
                        </div>
                    </div>
                </div>
            </main>
            <div className="flex items-center justify-center absolute top-24 left-4">
                <div className="grid grid-cols-1 items-center gap-2 p-4 bg-white dark:bg-darkBgMid w-fit rounded-xl shadow-lg">
                    <Tooltip>
                        <TooltipTrigger className="w-full">
                            <Button variant="outline" disabled className="min-h-20 text-xs flex flex-col items-center h-auto py-2 px-1 gap-2 rounded-xl w-full">
                                <Camera className="max-w-4 h-auto" />
                                <p className="text-xs text-muted-foreground max-w-20 text-wrap">Screenshot</p>
                            </Button>
                        </TooltipTrigger>
                        <TooltipContent side="right">
                            Coming soon
                        </TooltipContent>
                    </Tooltip>
                    <Tooltip>
                        <TooltipTrigger className="w-full">
                            <Button variant="outline" disabled className="min-h-20 text-xs flex flex-col items-center h-auto py-2 px-1 gap-2 rounded-xl w-full">
                                <ImageMinus className="max-w-4 h-auto" />
                                <p className="text-xs text-muted-foreground max-w-20 text-wrap">Remove Background</p>
                            </Button>
                        </TooltipTrigger>
                        <TooltipContent side="right">
                            Coming soon
                        </TooltipContent>
                    </Tooltip>
                    <Button
                        variant="outline"
                        className={
                            cn(
                                "min-h-20 text-xs flex flex-col items-center h-auto py-2 px-1 gap-2 rounded-xl transition-colors duration-300 hover:bg-blue-100/40 dark:hover:bg-darkBgHighlight/40",
                                {
                                    "bg-blue-100 dark:bg-darkBgHighlight !hover:bg-transparent": showInfo
                                }
                            )
                        }
                        onClick={() => setShowInfo(!showInfo)}
                    >
                        <Info className="max-w-4 h-auto" />
                        <p className="text-xs text-muted-foreground max-w-20 text-wrap">File Metadata</p>
                    </Button>
                </div>
            </div>

            <div className="w-full flex items-center justify-center absolute bottom-4">
                <div className="flex items-center gap-4 py-2 px-4 rounded-xl bg-white dark:bg-darkBgMid w-fit shadow-lg">
                    <span className="text-sm text-muted-foreground">Size</span>
                    <div className="flex items-center gap-2">
                        <Button variant="ghost" size="icon" onClick={() => setScale((prevScale) => Math.max(0, prevScale - 0.1))}>
                            <CircleMinus className="w-4 h-4" />
                        </Button>
                        <Slider
                            value={[scale]}
                            onValueChange={(value) => setScale(value[0])}
                            max={3}
                            min={0.25}
                            step={0.05}
                            className="w-32"
                        />
                        <Button variant="ghost" size="icon" onClick={() => setScale((prevScale) => Math.min(3, prevScale + 0.1))}>
                            <CirclePlus className="w-4 h-4" />
                        </Button>
                    </div>
                    <span className="text-sm text-muted-foreground">{Math.round(scale * 10)}</span>
                </div>
            </div>

            {/* File Info Panel */}
            {file && (
                <div
                    className={cn(
                        "group absolute right-4 top-20 bottom-20 bg-white dark:bg-darkBg border border-gray-200 dark:border-darkBgHighlight rounded-xl shadow-xl overflow-hidden flex flex-col z-40",
                        "transition-all duration-300 ease-in-out",
                        showInfo ? "w-80 opacity-100" : "w-0 opacity-0"
                    )}
                >
                    <div className={
                        cn(
                            "w-full h-full transition-opacity duration-100",
                            {
                                "opacity-100 delay-300": showInfo,
                                "opacity-0": !showInfo
                            }
                        )
                    }>
                        {/* Header */}
                        <div className="p-4 border-b border-gray-200 dark:border-darkBgHighlight">
                            <h2 className="text-lg font-semibold text-gray-900 dark:text-white break-words">
                                {file.name}
                            </h2>
                        </div>

                        {/* Content */}
                        <div className="flex-1 overflow-y-auto p-4 space-y-4">
                            {/* File Information */}
                            <div>
                                <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-3">
                                    File Information
                                </h4>
                                <div className="space-y-2 text-sm">
                                    <div className="flex justify-between">
                                        <span className="text-gray-500 dark:text-customGray">Size</span>
                                        <span className="text-gray-900 dark:text-white">
                                            {file.size ? formatFileSize(file.size) : 'Unknown'}
                                        </span>
                                    </div>
                                    <div className="flex justify-between">
                                        <span className="text-gray-500 dark:text-customGray">Modified</span>
                                        <span className="text-gray-900 dark:text-white">
                                            {(() => {
                                                const dateStr = file.modified;
                                                if (!dateStr) return 'Unknown';

                                                const date = new Date(dateStr);
                                                if (isNaN(date.getTime())) return 'Unknown';

                                                return formatDistanceToNow(date, { addSuffix: true });
                                            })()}
                                        </span>
                                    </div>
                                    <div className="flex justify-between">
                                        <span className="text-gray-500 dark:text-customGray">Type</span>
                                        <span className="text-gray-900 dark:text-white">
                                            {file.path.split('.').pop()?.toUpperCase() || 'Unknown'}
                                        </span>
                                    </div>
                                </div>
                            </div>

                            {/* Transcription Section */}
                            {isTranscribable && (
                                <div className="space-y-3">
                                    <div className="flex items-center justify-between">
                                        <h4 className="text-sm font-medium text-gray-900 dark:text-white">Transcription</h4>
                                        {!transcribingPaths?.has(file.path) ? (
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                onClick={() => handleTranscribeFile(file.path)}
                                                className="text-xs px-2 py-1 h-6"
                                                disabled={whisperStatus !== 'ready'}
                                                title={whisperStatus === 'downloading' ? 'Whisper model is downloading. Please wait...' :
                                                    whisperStatus === 'failed' ? 'Whisper model failed to load' :
                                                        whisperStatus === 'not_available' ? 'Whisper model not available' : 'Transcribe audio'}
                                            >
                                                <Mic className="h-3 w-3 mr-1" />
                                                {whisperStatus === 'ready' ? 'Transcribe' :
                                                    whisperStatus === 'downloading' ? 'Model Downloading...' :
                                                        whisperStatus === 'failed' ? 'Model Failed' : 'Model Not Available'}
                                            </Button>
                                        ) : (
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                className="text-xs px-2 py-1 h-6"
                                                disabled
                                            >
                                                <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                                                Transcribing...
                                            </Button>
                                        )}
                                    </div>
                                    <div className="h-80 overflow-y-auto">
                                        <TranscriptionDisplay
                                            filePath={file.path}
                                            compact={false}
                                            className="shadow-none border-0 p-0 bg-transparent"
                                            refreshTrigger={transcriptionRefreshCounter}
                                            isTranscribing={transcribingPaths?.has(file.path) || false}
                                            onSeekToTime={handleSeekToTime}
                                        />
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            )}
        </div>
    )
}

StudioEdit.loader = async ({ request }) => {
    const url = new URL(request.url);
    const path = url.searchParams.get('path');

    if (path) {
        const normalizedPath = path.replace("asset://localhost/", "");

        try {
            // Start all async operations in parallel
            const fileMetadataPromise = invoke<FileItem>("get_file_metadata", {
                path: normalizedPath,
            });

            const whisperStatusPromise = invoke<string>('is_whisper_model_available');

            // Check if file is transcribable based on extension
            const ext = normalizedPath.toLowerCase().split('.').pop();
            const transcribableExtensions = ['wav', 'mp3', 'mp4', 'm4a', 'flac', 'ogg', 'mov', 'avi', 'mkv', 'webm'];
            const isTranscribable = ext ? transcribableExtensions.includes(ext) : false;

            // Wait for all operations to complete in parallel
            const [file, whisperStatus] = await Promise.allSettled([
                fileMetadataPromise,
                isTranscribable ? whisperStatusPromise : Promise.resolve('not_applicable')
            ]);

            const fileResult = file.status === 'fulfilled' ? file.value : null;
            const whisperResult = whisperStatus.status === 'fulfilled' ? whisperStatus.value : 'failed';

            return {
                file: fileResult ? {
                    ...fileResult,
                    normalizedPath: convertFileSrc(normalizedPath),
                } : null,
                whisperStatus: whisperResult,
                isTranscribable
            };
        } catch (error) {
            console.error(`Something went wrong when loading data for ${normalizedPath}: ${error}`);
            return {
                file: null,
                whisperStatus: 'failed',
                isTranscribable: false
            }
        }
    }

    return {
        file: null,
        whisperStatus: 'not_applicable',
        isTranscribable: false
    }
}
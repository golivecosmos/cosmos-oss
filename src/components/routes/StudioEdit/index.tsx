
import { PointerEvent, useState, useEffect, useRef } from "react";
import { useLoaderData, useNavigate, useSearchParams } from "react-router-dom";
import { CircleMinus, CirclePlus, Info, Mic, Loader2 } from "lucide-react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { formatDistanceToNow } from 'date-fns';

import { Button } from "../../ui/button"
import { Slider } from "../../ui/slider"
import { FileItem } from "../../FileTree";
import { cn, formatFileSize } from "../../../lib/utils";
import { ContentPreview, ContentPreviewRef } from "./ContentPreview";
import { TranscriptionDisplay } from "../../TranscriptionDisplay";
import { useAppLayout } from "../../../contexts/AppLayoutContext";
import { isSupportedImageExtension, isSupportedVideoExtension } from "../../../constants";

const getFileNameFromPath = (path: string): string => {
    const normalized = path.replace(/\\/g, "/");
    return normalized.split("/").pop() || path;
};

export const StudioEdit = () => {
    const { file, whisperStatus, isTranscribable } = useLoaderData<typeof StudioEdit.loader>();
    const navigate = useNavigate();
    const [searchParams] = useSearchParams();
    const { transcribingPaths, handleTranscribeFile } = useAppLayout();

    const [scale, setScale] = useState<number>(0.8);
    const [panOffset, setPanOffset] = useState({ x: 0, y: 0 });
    const [showInfo, setShowInfo] = useState(false);
    const [transcriptionRefreshCounter, setTranscriptionRefreshCounter] = useState(0);
    const contentPreviewRef = useRef<ContentPreviewRef>(null);
    const dragStateRef = useRef({
        isDragging: false,
        pointerId: -1,
        startX: 0,
        startY: 0,
        startOffsetX: 0,
        startOffsetY: 0,
    });

    const handleBack = () => {
        const returnTo = searchParams.get('returnTo');
        if (returnTo && returnTo.startsWith('/')) {
            navigate(returnTo);
            return;
        }
        navigate(-1);
    };

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

    // Arrow-key navigation through the sibling file list stashed by whatever
    // view the user came from (grid, list, search results). The previous view
    // is responsible for writing the list into `sessionStorage['studio.navigation']`
    // right before navigating here; if it's missing we no-op gracefully.
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (
                e.key !== 'ArrowLeft' &&
                e.key !== 'ArrowRight' &&
                e.key !== 'ArrowUp' &&
                e.key !== 'ArrowDown'
            ) {
                return;
            }

            const target = e.target as HTMLElement | null;
            if (
                target &&
                (target.tagName === 'INPUT' ||
                    target.tagName === 'TEXTAREA' ||
                    target.tagName === 'SELECT' ||
                    target.isContentEditable)
            ) {
                return;
            }

            if (e.metaKey || e.ctrlKey || e.altKey) return;

            const raw = sessionStorage.getItem('studio.navigation');
            if (!raw) return;

            let paths: string[] | undefined;
            try {
                const parsed = JSON.parse(raw) as { paths?: string[] };
                paths = Array.isArray(parsed.paths) ? parsed.paths : undefined;
            } catch {
                return;
            }
            if (!paths || paths.length < 2) return;

            const currentPath = searchParams.get('path');
            if (!currentPath) return;

            const idx = paths.indexOf(currentPath);
            if (idx === -1) return;

            const delta = e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 1;
            const nextIdx = idx + delta;
            if (nextIdx < 0 || nextIdx >= paths.length) return;

            e.preventDefault();

            const nextPath = paths[nextIdx];
            const params = new URLSearchParams();
            params.set('path', nextPath);
            const returnTo = searchParams.get('returnTo');
            if (returnTo) params.set('returnTo', returnTo);
            navigate(`/studio/edit?${params.toString()}`);
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [searchParams, navigate]);

    const fileExtension = file?.path?.split(".").pop()?.toLowerCase() || "";
    const isPannablePreview = isSupportedImageExtension(fileExtension) || isSupportedVideoExtension(fileExtension);
    const isPanActive = isPannablePreview;

    useEffect(() => {
        // Reset pan whenever file changes.
        setPanOffset({ x: 0, y: 0 });
    }, [file?.path]);

    const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
        if (!isPannablePreview) return;
        dragStateRef.current = {
            isDragging: true,
            pointerId: event.pointerId,
            startX: event.clientX,
            startY: event.clientY,
            startOffsetX: panOffset.x,
            startOffsetY: panOffset.y,
        };
        event.currentTarget.setPointerCapture(event.pointerId);
    };

    const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
        const dragState = dragStateRef.current;
        if (!dragState.isDragging || dragState.pointerId !== event.pointerId) return;

        const deltaX = event.clientX - dragState.startX;
        const deltaY = event.clientY - dragState.startY;
        setPanOffset({
            x: dragState.startOffsetX + deltaX,
            y: dragState.startOffsetY + deltaY,
        });
    };

    const handlePointerEnd = (event: PointerEvent<HTMLDivElement>) => {
        if (dragStateRef.current.pointerId === event.pointerId) {
            dragStateRef.current.isDragging = false;
            dragStateRef.current.pointerId = -1;
            if (event.currentTarget.hasPointerCapture(event.pointerId)) {
                event.currentTarget.releasePointerCapture(event.pointerId);
            }
        }
    };

    const getTransformMatrix = (scale: number) => `translate3d(${panOffset.x}px, ${panOffset.y}px, 0) scale(${scale})`

    return (
        <div className="h-full bg-gray-50 dark:bg-darkBg relative">
            <header className="flex items-center justify-between p-4 absolute top-0 z-10">
                <Button
                    variant="outline"
                    size="sm"
                    className="text-xs"
                    onClick={handleBack}
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
                        className={cn(
                            "transition-transform duration-75 ease-out inline-flex items-center justify-center",
                            isPanActive ? "cursor-grab active:cursor-grabbing" : "cursor-default"
                        )}
                        style={{
                            transform: getTransformMatrix(scale),
                            transformOrigin: "center center",
                            touchAction: "none",
                        }}
                        onPointerDown={handlePointerDown}
                        onPointerMove={handlePointerMove}
                        onPointerUp={handlePointerEnd}
                        onPointerCancel={handlePointerEnd}
                    >
                        <div className="relative">
                            <ContentPreview ref={contentPreviewRef} file={file} />
                        </div>
                    </div>
                </div>
            </main>
            <div className="flex items-center justify-center absolute top-24 left-4">
                <div className="grid grid-cols-1 items-center gap-2 p-4 bg-white dark:bg-darkBgMid w-fit rounded-xl shadow-lg">
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
    const toFilesystemPath = (input: string): string => {
        let normalized = input;

        try {
            normalized = decodeURIComponent(normalized);
        } catch {
            // Keep raw input when decode fails (for literal % in file names).
        }

        if (normalized.startsWith("asset://") || normalized.startsWith("file://")) {
            try {
                const parsed = new URL(normalized);
                if (parsed.pathname) {
                    normalized = parsed.pathname;
                }
            } catch {
                // Fall through to string-based normalization below.
            }
        }

        if (normalized.startsWith("asset://localhost")) {
            normalized = normalized.slice("asset://localhost".length);
        } else if (normalized.startsWith("asset://")) {
            normalized = normalized.slice("asset://".length);
        } else if (normalized.startsWith("file://")) {
            normalized = normalized.slice("file://".length);
        }

        if (normalized.startsWith("//")) {
            normalized = normalized.replace(/^\/+/, "/");
        }

        const isWindowsPath = /^[a-zA-Z]:[\\/]/.test(normalized);
        if (!isWindowsPath && normalized && !normalized.startsWith("/")) {
            normalized = `/${normalized}`;
        }

        return normalized;
    };

    const url = new URL(request.url);
    const rawPath = url.searchParams.get('path');
    const path = rawPath;

    if (path) {
        const normalizedPath = toFilesystemPath(path);

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
            const resolvedFileSystemPath = fileResult?.path
                ? toFilesystemPath(fileResult.path)
                : normalizedPath;
            const fallbackFile: FileItem = {
                name: getFileNameFromPath(resolvedFileSystemPath),
                path: resolvedFileSystemPath,
                is_dir: false,
                size: 0,
                modified: new Date().toISOString(),
                file_type: "file",
            };
            const effectiveFile = fileResult || fallbackFile;

            return {
                file: {
                    ...effectiveFile,
                    normalizedPath: convertFileSrc(resolvedFileSystemPath),
                },
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

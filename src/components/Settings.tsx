import { useState, useEffect } from 'react'
import {
  Settings as SettingsIcon,
  Database,
  Brain,
  Trash2,
  RefreshCw,
  Download,
  AlertTriangle,
  CheckCircle,
  X,
  Image,
  Video,
  Monitor,
  Target,
  HardDrive,
  Loader2,
  Edit3,
  Check,
  MapPin,
  Calendar,
  Save
} from 'lucide-react'
import { Button } from "./ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card"
import { Badge } from "./ui/badge"
import { Progress } from "./ui/progress"
import { invoke } from '@tauri-apps/api/core'
import { UpdateChecker } from './UpdateNotification'
import { useAppVersion } from '../hooks/useAppVersion'
import { homeDir } from '@tauri-apps/api/path'

import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'sonner'
import { useAppLayout } from '../contexts/AppLayoutContext'
import { Input } from "./ui/input"
import { Label } from "./ui/label"
import { cn } from '../lib/utils'

interface ModelDownloadProgress {
  state: 'checking' | 'ready' | 'downloading' | 'failed' | 'installing'
  progress: number
  currentFile?: string
  error?: string
  filesCompleted: number
  totalFiles: number
}

interface SettingsProps {
  isOpen: boolean
  onClose: () => void
  onRestartTour: () => void
  modelDownloadState?: ModelDownloadProgress
  onRetryDownload?: () => void
}

// TODO: add sqlite memory usage
interface IndexStats {
  total_files: number
  total_size_bytes: number
  image_count: number
  video_count: number
  last_updated: string
  sqlite_db_path?: [string, boolean]
  sqlite_db_bytes?: number
  sqlite_db_mb?: number
}

interface SystemStatus {
  models_loaded: boolean
  ffmpeg_available: boolean
  index_healthy: boolean
}

interface DriveInfo {
  uuid: string;
  name: string;
  custom_name?: string | null;
  physical_location?: string | null;
  mount_path: string;
  status: 'connected' | 'disconnected' | 'indexing' | 'error';
  indexed_files_count: number;
  is_removable: boolean;
}

interface DriveItemEditableProps {
  drive: DriveInfo;
  onUpdate: (uuid: string, customName: string | null, physicalLocation: string | null) => Promise<void>;
  onDelete: (uuid: string) => Promise<void>;
}

function DriveItemEditable({ drive, onUpdate, onDelete }: DriveItemEditableProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [customName, setCustomName] = useState(drive.custom_name || '');
  const [physicalLocation, setPhysicalLocation] = useState(drive.physical_location || '');
  const [isUpdating, setIsUpdating] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);

  const displayName = drive.custom_name || drive.name;
  const isOnline = drive.status === 'connected' || drive.status === 'indexing';

  const handleSave = async () => {
    setIsUpdating(true);
    try {
      await onUpdate(
        drive.uuid,
        customName.trim() || null,
        physicalLocation.trim() || null
      );
      setIsEditing(false);
    } catch (error) {
      console.error('Failed to update drive metadata:', error);
    } finally {
      setIsUpdating(false);
    }
  };

  const handleCancel = () => {
    setCustomName(drive.custom_name || '');
    setPhysicalLocation(drive.physical_location || '');
    setIsEditing(false);
  };

  const handleDelete = async () => {
    setIsDeleting(true);
    try {
      await onDelete(drive.uuid);
      setShowDeleteConfirm(false);
    } catch (error) {
      console.error('Failed to delete drive:', error);
      const errorMessage = error as string;
      if (errorMessage.includes('indexed files')) {
        alert(`Cannot delete drive: It contains indexed files. Please remove the indexed content first using the AI Library search.`);
      } else {
        alert('Failed to delete drive. Please try again.');
      }
    } finally {
      setIsDeleting(false);
    }
  };

  const getStatusIcon = () => {
    switch (drive.status) {
      case 'connected':
        return <HardDrive className="w-5 h-5 text-green-500" />;
      case 'disconnected':
        return <HardDrive className="w-5 h-5 text-gray-400" />;
      case 'indexing':
        return <Loader2 className="w-5 h-5 text-blue-500 animate-spin" />;
      case 'error':
        return <AlertTriangle className="w-5 h-5 text-red-500" />;
      default:
        return <HardDrive className="w-5 h-5 text-gray-500" />;
    }
  };

  const getStatusBadge = () => {
    const statusColors = {
      connected: 'bg-green-100 text-green-800 border-green-200 dark:bg-green-900/30 dark:text-green-400 dark:border-green-700',
      disconnected: 'bg-gray-100 text-gray-800 border-gray-200 dark:bg-gray-800 dark:text-gray-300 dark:border-gray-600',
      indexing: 'bg-blue-100 text-blue-800 border-blue-200 dark:bg-blue-900/30 dark:text-blue-400 dark:border-blue-700',
      error: 'bg-red-100 text-red-800 border-red-200 dark:bg-red-900/30 dark:text-red-400 dark:border-red-700'
    };

    return (
      <span className={cn(
        'px-2 py-1 text-xs rounded-full border',
        statusColors[drive.status]
      )}>
        {drive.status.charAt(0).toUpperCase() + drive.status.slice(1)}
      </span>
    );
  };

  return (
    <div className={cn(
      'border rounded-lg p-4 space-y-3 transition-colors bg-white dark:bg-gray-800',
      isOnline ? 'border-green-200 dark:border-green-700' : 'border-gray-200 dark:border-gray-700'
    )}>
      {/* Header Row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-3">
          {getStatusIcon()}
          <div>
            <h3 className="font-medium text-sm text-gray-900 dark:text-gray-100">{displayName}</h3>
            {drive.custom_name && (
              <p className="text-xs text-gray-600 dark:text-gray-400">System: {drive.name}</p>
            )}
          </div>
        </div>
        <div className="flex items-center space-x-2">
          {getStatusBadge()}
          {!isEditing && !showDeleteConfirm && (
            <>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setIsEditing(true)}
                className="h-7 w-7 p-0"
                title="Edit drive details"
              >
                <Edit3 className="w-3 h-3" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowDeleteConfirm(true)}
                className="h-7 w-7 p-0 hover:bg-red-100 hover:text-red-600 dark:hover:bg-red-900/30 dark:hover:text-red-400"
                title="Delete drive from database"
              >
                <Trash2 className="w-3 h-3" />
              </Button>
            </>
          )}
        </div>
      </div>

      {/* Editable Fields */}
      {isEditing ? (
        <div className="space-y-3 border-t border-gray-200 dark:border-gray-700 pt-3">
          <div className="space-y-2">
            <Label htmlFor={`name-${drive.uuid}`} className="text-xs font-medium text-gray-900 dark:text-gray-100">
              Custom Name
            </Label>
            <Input
              id={`name-${drive.uuid}`}
              value={customName}
              onChange={(e) => setCustomName(e.target.value)}
              placeholder={drive.name}
              className="h-8 text-sm"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor={`location-${drive.uuid}`} className="text-xs font-medium text-gray-900 dark:text-gray-100">
              Physical Location
            </Label>
            <Input
              id={`location-${drive.uuid}`}
              value={physicalLocation}
              onChange={(e) => setPhysicalLocation(e.target.value)}
              placeholder="e.g., Shelf B-3, Red Cabinet"
              className="h-8 text-sm"
            />
          </div>
          <div className="flex justify-end space-x-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleCancel}
              disabled={isUpdating}
              className="h-7 px-3"
            >
              <X className="w-3 h-3 mr-1" />
              Cancel
            </Button>
            <Button
              size="sm"
              onClick={handleSave}
              disabled={isUpdating}
              className="h-7 px-3"
            >
              {isUpdating ? (
                <Loader2 className="w-3 h-3 mr-1 animate-spin" />
              ) : (
                <Check className="w-3 h-3 mr-1" />
              )}
              Save
            </Button>
          </div>
        </div>
      ) : showDeleteConfirm ? (
        /* Delete Confirmation */
        <div className="space-y-3 border-t border-red-200 dark:border-red-700 pt-3">
          <div className="text-sm text-gray-700 dark:text-gray-300">
            <p className="font-medium text-red-600 dark:text-red-400 mb-2">Are you sure you want to delete this drive?</p>
            <p className="text-xs">This will remove the drive "{displayName}" from your database.</p>
            {drive.indexed_files_count > 0 && (
              <div className="text-xs mt-2 p-2 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-700 rounded">
                <p className="font-medium text-yellow-800 dark:text-yellow-400">⚠️ Warning:</p>
                <p className="text-yellow-700 dark:text-yellow-300">
                  This drive has {drive.indexed_files_count} indexed files. You cannot delete it until you remove the indexed content first.
                </p>
              </div>
            )}
            <p className="text-xs mt-1">Note: This only removes the drive from Cosmos. Your actual drive and its data will not be affected.</p>
          </div>
          <div className="flex justify-end space-x-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowDeleteConfirm(false)}
              disabled={isDeleting}
              className="h-7 px-3"
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={handleDelete}
              disabled={isDeleting || drive.indexed_files_count > 0}
              className="h-7 px-3"
              title={drive.indexed_files_count > 0 ? "Cannot delete drive with indexed content" : "Delete drive"}
            >
              {isDeleting ? (
                <Loader2 className="w-3 h-3 mr-1 animate-spin" />
              ) : (
                <Trash2 className="w-3 h-3 mr-1" />
              )}
              Delete
            </Button>
          </div>
        </div>
      ) : (
        /* Info Display */
        <div className="space-y-2 text-xs text-gray-700 dark:text-gray-300">
          {drive.physical_location && (
            <div className="flex items-center space-x-2">
              <MapPin className="w-3 h-3" />
              <span>{drive.physical_location}</span>
            </div>
          )}

          <div className="grid grid-cols-2 gap-2">
            <div className="flex items-center space-x-2">
              <Database className="w-3 h-3" />
              <span className={drive.indexed_files_count > 0 ? "font-medium text-blue-600 dark:text-blue-400" : ""}>
                {drive.indexed_files_count} files indexed
              </span>
            </div>
            <div className="flex items-center space-x-2">
              <Calendar className="w-3 h-3" />
              <span>Status: {drive.status}</span>
            </div>
          </div>

          {drive.mount_path && (
            <div className="text-xs text-gray-500 dark:text-gray-400 truncate">
              Path: {drive.mount_path}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function Settings({ isOpen, onClose, onRestartTour, modelDownloadState, onRetryDownload }: SettingsProps) {
  const [homeDirectory, setHomeDirectory] = useState(' ')

  const [activeTab, setActiveTab] = useState('index')
  const [indexStats, setIndexStats] = useState<IndexStats | null>(null)

  const [systemStatus, setSystemStatus] = useState<SystemStatus | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [isSettingIndexDir, setIsSettingIndexDir] = useState(false)
  const { version } = useAppVersion()
  const { drives, isDrivesLoading, updateDrive, deleteDrive, setShowBugReport } = useAppLayout()

  useEffect(() => {
    const getHomeDir = async () => {
      try {
        const dir = await homeDir()
        setHomeDirectory(dir)
      } catch (e) {
        console.log('Failed to get home directory', e)
      }
    }
    getHomeDir()
  }, [])

  useEffect(() => {
    if (isOpen) {
      loadSettingsData()
    }
  }, [isOpen])

  const toggleDisplayMode = (displayMode: string) => {
    const currMode = localStorage.getItem('theme')
    if (displayMode === "dark" && currMode !== 'dark') {
      document.documentElement.classList.add('dark');
      localStorage.setItem('theme', 'dark');
    } else if (displayMode === "light" && currMode !== 'light') {
      document.documentElement.classList.remove('dark');
      localStorage.setItem('theme', 'light');
    }
  }

  const loadSettingsData = async () => {
    setIsLoading(true)
    try {
      // Load index statistics
      await loadIndexStats()

      // Load system status
      await loadSystemStatus()
    } catch (error) {
      console.error('Failed to load settings data:', error)
    } finally {
      setIsLoading(false)
    }
  }

  const loadIndexStats = async () => {
    try {
      const [count, path, status, files] = await Promise.all([
        invoke<number>('get_indexed_count'),
        invoke<[string, boolean]>('get_indexed_directory'),
        invoke<any>('check_search_status'),
        invoke<any[]>('get_indexed_files_grouped')
      ])

      // Calculate actual counts
      const uniqueVideoPaths = new Set()
      const imageFiles = new Set()
      const framesByVideo = new Map() // Track frames per video

      files.forEach(f => {
        const metadata = typeof f.metadata === 'string' ? JSON.parse(f.metadata) : f.metadata

        // Normalize paths by removing any "asset://localhost/" prefix
        const normalizedPath = f.file_path.replace('asset://localhost/', '')

        // Check if this is a video frame or video file
        const isVideoFrame = metadata?.source_type === 'video_frame'
        const isVideo = f.mime_type?.startsWith('video/') || metadata?.is_video_group

        if (isVideoFrame) {
          uniqueVideoPaths.add(normalizedPath)

          // Track frames for this video
          const frames = framesByVideo.get(normalizedPath) || []
          frames.push(f)
          framesByVideo.set(normalizedPath, frames)
        } else if (isVideo) {
          uniqueVideoPaths.add(normalizedPath)
        } else if (!isVideoFrame && (f.mime_type?.startsWith('image/') || metadata?.width)) {
          // Count as image if it's not a video frame and either:
          // 1. Has image mime type
          // 2. Has width in metadata (indicating it's an image)
          imageFiles.add(normalizedPath)
        }
      })

      const videoCount = uniqueVideoPaths.size
      const imageCount = imageFiles.size

      // Log detailed stats for debugging
      console.log('Calculated counts:', {
        totalUniqueFiles: videoCount + imageCount,
        uniqueVideos: videoCount,
        imageCount,
        uniqueVideoPaths: Array.from(uniqueVideoPaths),
        imageFiles: Array.from(imageFiles),
        framesPerVideo: Array.from(framesByVideo.entries()).map(([video, frames]) => ({
          video,
          frameCount: frames.length
        }))
      })

      setIndexStats({
        total_files: videoCount + imageCount, // Use the count of unique files
        total_size_bytes: count * 2.5 * 1024 * 1024, // Estimate 2.5MB per file
        image_count: imageCount,
        video_count: videoCount,
        last_updated: new Date().toISOString(),
        sqlite_db_path: path ? path : ["Not found", true],
        sqlite_db_bytes: status.sqlite_stats?.database_size_bytes || 0,
        sqlite_db_mb: status.sqlite_stats?.database_size_mb || 0,
      })
    } catch (error) {
      console.error('Failed to load index stats:', error)
    }
  }

  const loadSystemStatus = async () => {
    try {
      const [searchStatus, ffmpegStatus] = await Promise.all([
        invoke<any>('check_search_status'),
        invoke<boolean>('is_ffmpeg_available')
      ])

      setSystemStatus({
        models_loaded: searchStatus.model_loaded,
        ffmpeg_available: ffmpegStatus,
        index_healthy: searchStatus.indexed_count > 0
      })
    } catch (error) {
      console.error('Failed to load system status:', error)
    }
  }

  const formatBytes = (bytes: number) => {
    if (!bytes) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`
  }

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString() + ' ' + new Date(dateString).toLocaleTimeString()
  }

  const handleClearIndex = async () => {
    if (!confirm('Are you sure you want to clear the entire search index? This action cannot be undone and will also clear all pending indexing jobs.')) {
      return
    }

    setIsLoading(true)
    try {
      // First clear the search index
      const result = await invoke<string>('clear_search_index')
      console.log('✅ Clear index result:', result)

      // Then clear all jobs since they're now obsolete
      try {
        // Clear pending and running jobs
        const queueResult = await invoke<any>('manage_job_queue', { action: 'clear' })
        console.log('✅ Clear queue result:', queueResult.message)

        // Clear all old jobs (completed, failed, cancelled) by using 0 days
        const oldJobsResult = await invoke<any>('bulk_job_operations', { action: 'cleanup_old', daysOld: 0 })
        console.log('✅ Clear old jobs result:', oldJobsResult.message)

        console.log('✅ All jobs cleared successfully')
      } catch (jobError) {
        console.error('⚠️ Failed to clear some jobs:', jobError)
        // Don't fail the entire operation if job clearing fails
      }

      // Immediately reset stats for instant UI feedback
      setIndexStats({
        total_files: 0,
        total_size_bytes: 0,
        image_count: 0,
        video_count: 0,
        last_updated: new Date().toISOString()
      })

      // Then reload stats from backend to ensure we're in sync
      await loadIndexStats()

      alert(`Index and jobs cleared successfully!\n\n${result}`)
    } catch (error) {
      console.error('Failed to clear index:', error)
      alert(`Failed to clear index: ${error}`)
    } finally {
      setIsLoading(false)
    }
  }

  const handleSetIndexDir = async (isSetDefault = false) => {
    if (!isSettingIndexDir) {
      setIsSettingIndexDir(true)
      try {
        const chosen = isSetDefault ? " " : await open({
          directory: true,
          multiple: false,
          title: 'Choose a folder',
          defaultPath: homeDirectory
        })

        // User cancelled the dialog
        if (!chosen) {
          return
        }

        const toastId = `handleSetIndexToast_${Date.now()}`;
        toast.loading("Setting Index Directory...", {
          id: toastId,
          duration: Infinity
        })

        const response = await invoke<string>("set_indexed_directory", {
          isSetDefault: isSetDefault,
          newDir: chosen
        })

        toast.success("Index Directory successfully set!", {
          id: toastId,
          duration: 5000
        })

        try {
          setIndexStats(prev => ({
            ...prev,
            sqlite_db_path: [response, isSetDefault],
          }))
        } catch (statsError) {
          console.error("Error updating index stats:", statsError);
          // Don't show error toast for stats update failure
        }
        
      } catch (e) {
        if (e && typeof e === 'string' && !e.includes('User cancelled')) {
          toast.error(`Failed to set index directory`, {
            id: `handleSetIndexToast_error_${Date.now()}`,
            description: "An unexpected error occurred. Please try again or report this issue.",
            action: {
              label: "Report Bug",
              onClick: () => {
                // Close settings modal first, then open bug report
                onClose()
                setTimeout(() => {
                  setShowBugReport(true)
                }, 300)
              }
            },
            duration: 8000
          })
        }
      } finally {
        setIsSettingIndexDir(false)
      }
    }
  }

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <div className="dark:bg-darkBg bg-white rounded-2xl shadow-2xl w-full max-w-4xl max-h-[90vh] overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b dark:border-darkBgHighlight border-gray-200">
          <div className="flex items-center">
            <div className="w-10 h-10 bg-gradient-to-r dark:from-customBlue from-blue-500 dark:to-blueShadow to-indigo-600 rounded-full flex items-center justify-center mr-3">
              <SettingsIcon className="w-5 h-5 text-white" />
            </div>
            <div>
              <h2 className="text-xl font-bold dark:text-text text-gray-900">Settings</h2>
              <p className="text-sm dark:text-customGray text-gray-500">Manage your AI search system</p>
            </div>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="dark:text-customGray dark:hover:text-red text-gray-400 hover:text-gray-600"
          >
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-hidden">
          <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full">
            <div className="border-b dark:border-darkBgHighlight border-gray-200">
              <TabsList className="flex justify-between px-8">
                <TabsTrigger value="index" className="flex items-center">
                  <Database className="w-4 h-4 mr-2" />
                  Index Management
                </TabsTrigger>
                <TabsTrigger value="drives" className="flex items-center">
                  <HardDrive className="w-4 h-4 mr-2" />
                  Drives
                </TabsTrigger>
                <TabsTrigger value="models" className="flex items-center">
                  <Brain className="w-4 h-4 mr-2" />
                  AI Models
                </TabsTrigger>
                <TabsTrigger value="updates" className="flex items-center">
                  <Download className="w-4 h-4 mr-2" />
                  Updates
                </TabsTrigger>
                <TabsTrigger value="display" className="flex items-center">
                  <Monitor className="w-4 h-4 mr-2" />
                  Display
                </TabsTrigger>
              </TabsList>
            </div>

            <div className="p-6 overflow-y-auto max-h-[calc(90vh-200px)]">
              {/* Index Management Tab */}
              <TabsContent value="index" className="space-y-6">
                <div>
                  {/* <h3 className="text-lg font-semibold mb-4">Search Index Overview</h3> */}

                  {indexStats && (
                    <>
                      {/* <div className="bg-green-50 border border-green-200 rounded-lg p-4 mb-4">
                        <div className="flex items-start">
                          <CheckCircle className="w-5 h-5 text-green-500 mt-0.5 mr-3 flex-shrink-0" />
                          <div>
                            <h4 className="font-medium text-green-800 mb-1">AI Knowledge Base Active</h4>
                            <p className="text-sm text-green-700">
                              Your AI has learned the visual patterns of {indexStats.total_files.toLocaleString()} files, creating a{' '}
                              <span className="font-semibold">TBD</span> neural index
                              ({((0 / indexStats.total_size_bytes) * 100).toFixed(1)}% of your content size).
                              Your original files remain untouched.
                            </p>
                          </div>
                        </div>
                      </div> */}

                      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6 overflow-hidden">
                        <Card>
                          <CardHeader className="pb-3">
                            <CardTitle className="text-sm font-medium dark:text-customGray text-gray-600">Total Files</CardTitle>
                          </CardHeader>
                          <CardContent>
                            <div className="text-2xl font-bold">{indexStats.total_files.toLocaleString()}</div>
                            <div className="flex items-center mt-2 space-x-4">
                              <div className="flex items-center">
                                <Image className="w-4 h-4 dark:text-customYellow text-blue-500 mr-1" />
                                <span className="text-sm dark:text-customGray text-gray-600">{indexStats.image_count} images</span>
                              </div>
                              <div className="flex items-center">
                                <Video className="w-4 h-4 dark:text-customPurple text-purple-500 mr-1" />
                                <span className="text-sm dark:text-customGray text-gray-600">{indexStats.video_count} videos</span>
                              </div>
                            </div>
                          </CardContent>
                        </Card>

                        <Card>
                          <CardHeader className="pb-3">
                            <CardTitle className="text-sm font-medium dark:text-customGray text-gray-600">AI Index Size</CardTitle>
                          </CardHeader>
                          <CardContent>
                            <div className="space-y-3">
                              <div>
                                <div className="text-2xl font-bold dark:text-customGreen text-green-600">
                                  {indexStats.sqlite_db_bytes ? formatBytes(indexStats.sqlite_db_bytes) : 'TBD'}
                                </div>
                              </div>
                              <div className="border-t pt-2 dark:border-darkBgHighlight">
                                <div className="text-sm text-gray-600">
                                  Your files: {formatBytes(indexStats.total_size_bytes)}
                                </div>
                                <div className="text-xs dark:text-customGreen text-green-600 font-medium">
                                  {indexStats.total_size_bytes > 0 && indexStats.sqlite_db_bytes
                                    ? `${((indexStats.sqlite_db_bytes / indexStats.total_size_bytes) * 100).toFixed(1)}% neural compression ratio`
                                    : '0.0% neural compression ratio'}
                                </div>
                              </div>
                            </div>
                          </CardContent>
                        </Card>
                      </div>
                    </>
                  )}

                  <Card className="mt-6">
                    <CardHeader>
                      <CardTitle>Index Actions</CardTitle>
                      <CardDescription>
                        Manage your search index and optimize performance
                      </CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-4">
                      <div className="flex items-center justify-between p-4 border dark:border-darkBgHighlight border-gray-200 rounded-lg">
                        <div>
                          <div className="font-medium"> Index Directory </div>
                          <div className="text-sm dark:text-customGray text-gray-600"> Choose where your AI Library is stored</div>
                        </div>
                        <div className="flex gap-2 items-center">
                          <div className="w-48 mr-2 text-gray-400 text-right truncate text-sm dark:text-darkBgHighlight"> {indexStats && indexStats.sqlite_db_path ? indexStats.sqlite_db_path[0] : "Path Not Found"} </div>
                          <button onClick={() => handleSetIndexDir()} className="h-8 px-2 text-sm rounded bg-blue-500 leading-none text-customWhite hover:bg-blue-600 dark:bg-blueShadow dark:hover:bg-customBlue"> Choose Folder </button>
                          <button onClick={indexStats && indexStats.sqlite_db_path && (() => !indexStats.sqlite_db_path[1] && handleSetIndexDir(true))} className={`h-8 px-2 text-sm rounded leading-none text-customWhiten
                          ${indexStats && indexStats.sqlite_db_path && indexStats.sqlite_db_path[1] ?
                              'bg-gray-300 dark:bg-darkBgMid text-gray-100 dark:text-darkBgHighlight cursor-default' :
                              'dark:bg-greenShadow bg-green-600 hover:bg-green-700 dark:hover:bg-customGreen'}`}>
                            Set Default
                          </button>
                        </div>

                      </div>
                      <div className="flex items-center justify-between p-4 border dark:border-darkBgHighlight border-gray-200 rounded-lg">
                        <div>
                          <h4 className="font-medium">Clear Index</h4>
                          <p className="text-sm dark:text-customGray text-gray-600">Remove all indexed files and start fresh</p>
                        </div>
                        <Button
                          variant="outline"
                          onClick={handleClearIndex}
                          disabled={isLoading}
                          className="text-red-600 border-red-200 dark:border-customRed dark:text-customRed dark:hover:bg-customRed/90 hover:bg-red-50"
                        >
                          <Trash2 className="w-4 h-4 mr-2" />
                          Clear
                        </Button>
                      </div>
                    </CardContent>
                  </Card>
                </div>
              </TabsContent>

              {/* Drives Tab */}
              <TabsContent value="drives" className="space-y-6">
                <div>
                  <Card>
                    <CardHeader>
                      <CardTitle className="flex items-center space-x-2">
                        <HardDrive className="w-5 h-5" />
                        <span>Drive Management</span>
                      </CardTitle>
                      <CardDescription>
                        Manage your external drives, assign custom names, and set physical storage locations.
                      </CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-6">
                      {isDrivesLoading ? (
                        <div className="flex items-center justify-center py-8 text-gray-600 dark:text-gray-400">
                          <Loader2 className="w-6 h-6 animate-spin mr-2" />
                          <span>Loading drives...</span>
                        </div>
                      ) : drives.length === 0 ? (
                        <div className="text-center py-8 text-gray-600 dark:text-gray-400">
                          <HardDrive className="w-12 h-12 mx-auto mb-2 text-gray-400 dark:text-gray-500" />
                          <p className="text-gray-900 dark:text-gray-100">No drives found</p>
                          <p className="text-sm text-gray-600 dark:text-gray-400">Connect an external drive to get started</p>
                        </div>
                      ) : (
                        <>
                          {/* Connected Drives */}
                          {drives.filter(d => d.status === 'connected' || d.status === 'indexing').length > 0 && (
                            <div>
                              <h3 className="font-medium text-sm mb-3 text-green-800 dark:text-green-400">
                                Connected Drives ({drives.filter(d => d.status === 'connected' || d.status === 'indexing').length})
                              </h3>
                              <div className="space-y-3">
                                {drives.filter(d => d.status === 'connected' || d.status === 'indexing').map((drive) => (
                                  <DriveItemEditable
                                    key={drive.uuid}
                                    drive={drive}
                                    onUpdate={updateDrive}
                                    onDelete={deleteDrive}
                                  />
                                ))}
                              </div>
                            </div>
                          )}

                          {/* Disconnected Drives */}
                          {drives.filter(d => d.status === 'disconnected').length > 0 && (
                            <div>
                              <h3 className="font-medium text-sm mb-3 text-gray-800 dark:text-gray-200">
                                Disconnected Drives ({drives.filter(d => d.status === 'disconnected').length})
                              </h3>
                              <div className="space-y-3">
                                {drives.filter(d => d.status === 'disconnected').map((drive) => (
                                  <DriveItemEditable
                                    key={drive.uuid}
                                    drive={drive}
                                    onUpdate={updateDrive}
                                    onDelete={deleteDrive}
                                  />
                                ))}
                              </div>
                            </div>
                          )}
                        </>
                      )}
                    </CardContent>
                  </Card>
                </div>
              </TabsContent>

              {/* AI Models Tab */}
              <TabsContent value="models" className="space-y-6">
                <div>
                  <Card>
                    <CardHeader>
                      <CardTitle>Model Status</CardTitle>
                      <CardDescription>
                        AI models required for semantic search and visual analysis
                      </CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-4">
                      {modelDownloadState && (
                        <div className="space-y-4">
                          {/* Status indicator */}
                          <div className="flex items-center justify-between p-4 border dark:border-darkBgHighlight rounded-lg">
                            <div className="flex items-center space-x-3">
                              {modelDownloadState.state === 'ready' ? (
                                <CheckCircle className="w-5 h-5 dark:text-customGreen text-green-500" />
                              ) : modelDownloadState.state === 'failed' ? (
                                <AlertTriangle className="w-5 h-5 dark:text-customRed text-red-500" />
                              ) : (
                                <Download className="w-5 h-5 dark:text-customBlue text-blue-500 animate-spin" />
                              )}
                              <div>
                                <h4 className="font-medium">
                                  {modelDownloadState.state === 'ready' && 'Models Ready'}
                                  {modelDownloadState.state === 'checking' && 'Checking Models'}
                                  {modelDownloadState.state === 'downloading' && 'Downloading Models'}
                                  {modelDownloadState.state === 'installing' && 'Installing Models'}
                                  {modelDownloadState.state === 'failed' && 'Setup Failed'}
                                </h4>
                                <p className="text-sm dark:text-customGray text-gray-600">
                                  {modelDownloadState.state === 'ready' && 'All AI models are loaded and ready for use'}
                                  {modelDownloadState.state === 'checking' && 'Verifying model availability...'}
                                  {modelDownloadState.state === 'downloading' && 'Downloading required AI models for search...'}
                                  {modelDownloadState.state === 'installing' && 'Finalizing AI model setup...'}
                                  {modelDownloadState.state === 'failed' && (modelDownloadState.error || 'Failed to setup AI models')}
                                </p>
                              </div>
                            </div>

                            {modelDownloadState.state === 'failed' && onRetryDownload && (
                              <Button
                                onClick={onRetryDownload}
                                size="sm"
                                variant="outline"
                                className="border-red-300 dark:text-customRed text-red-700 hover:bg-red-100"
                              >
                                <RefreshCw className="w-4 h-4 mr-2" />
                                Retry
                              </Button>
                            )}
                          </div>

                          {/* Progress bar for downloading/installing */}
                          {(modelDownloadState.state === 'downloading' || modelDownloadState.state === 'installing') && (
                            <div className="space-y-2">
                              <div className="flex justify-between text-sm">
                                <span>Progress</span>
                                <span>{Math.round(modelDownloadState.progress)}%</span>
                              </div>
                              <Progress value={modelDownloadState.progress} className="h-2" />
                              {modelDownloadState.totalFiles > 0 && (
                                <p className="text-xs text-gray-500">
                                  {modelDownloadState.filesCompleted} of {modelDownloadState.totalFiles} files completed
                                </p>
                              )}
                            </div>
                          )}
                        </div>
                      )}
                    </CardContent>
                  </Card>
                </div>

                {systemStatus && (
                  <Card>
                    <CardHeader>
                      <CardTitle>System Status</CardTitle>
                      <CardDescription>
                        Overall system health and connectivity
                      </CardDescription>
                    </CardHeader>
                    <CardContent>
                      <div className="grid grid-cols-2 gap-4">
                        <div className="flex items-center justify-between">
                          <span className="text-sm dark:text-customGray text-gray-600">Database Connected</span>
                          <Badge variant={systemStatus.index_healthy ? "default" : "destructive"}>
                            {systemStatus.index_healthy ? 'Healthy' : 'Empty'}
                          </Badge>
                        </div>
                        <div className="flex items-center justify-between">
                          <span className="text-sm dark:text-customGray text-gray-600">Video Processing</span>
                          <Badge variant={systemStatus.ffmpeg_available ? "default" : "secondary"}>
                            {systemStatus.ffmpeg_available ? 'Available' : 'Unavailable'}
                          </Badge>
                        </div>
                        <div className="flex items-center justify-between">
                          <span className="text-sm dark:text-customGray text-gray-600">Search Index</span>
                          <Badge variant={systemStatus.index_healthy ? "default" : "secondary"}>
                            {systemStatus.index_healthy ? 'Healthy' : 'Empty'}
                          </Badge>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                )}
              </TabsContent>

              {/* Updates Tab */}
              <TabsContent value="updates" className="space-y-6">
                <div>
                  <Card>
                    <CardHeader>
                      <CardTitle>Update Settings</CardTitle>
                      <CardDescription>
                        Check for and download the latest version of Cosmos
                      </CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-6">
                      <UpdateChecker />
                    </CardContent>
                  </Card>
                </div>
              </TabsContent>

              {/* Display Tab */}
              <TabsContent value="display" className="space-y-6">
                <div>
                  <Card>
                    <CardHeader>
                      <CardTitle> Display Mode </CardTitle>
                      <CardDescription> Toggle between light and dark mode </CardDescription>

                    </CardHeader>

                    <div className="flex px-6 pb-6 gap-2">
                      <Button onClick={() => toggleDisplayMode("light")} variant="outline"
                        className="bg-blue-500 text-customWhite border-black hover:text-customWhite dark:bg-darkBg hover:bg-blue-500 dark:hover:bg-darkBgHighlight"

                      >
                        Light
                      </Button>
                      <Button onClick={() => toggleDisplayMode("dark")} variant="outline"
                        className="hover:bg-gray-200 dark:hover:bg-blueShadow dark:bg-blueShadow">
                        Dark
                      </Button>
                    </div>

                  </Card>
                </div>
              </TabsContent>

            </div>
          </Tabs>
        </div>

        {/* Footer */}
        <div className="border-t dark:border-darkBgHighlight border-gray-200 p-4 dark:bg-darkBgMid bg-gray-50">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <Button
                variant="outline"
                onClick={onRestartTour}
                className="flex items-center dark:border-darkBgHighlight"
              >
                <Target className="w-4 h-4 mr-2" />
                Restart Tour
              </Button>
              <div className="text-xs dark:text-customGray text-gray-500">
                Cosmos v{version}
              </div>
            </div>
            <div className="flex space-x-3">
              <Button variant="outline" className="dark:border-darkBgHighlight" onClick={onClose}>
                Cancel
              </Button>
              <Button onClick={onClose} className="dark:bg-blueShadow">
                Done
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

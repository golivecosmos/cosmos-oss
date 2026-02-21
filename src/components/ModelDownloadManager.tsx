import React, { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import { Download, CheckCircle, AlertCircle, RefreshCw, Brain } from 'lucide-react'
import { Button } from "./ui/button"
import { Progress } from "./ui/progress"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card"
import { Badge } from "./ui/badge"

interface DownloadProgress {
  file_name: string
  downloaded_bytes: number
  total_bytes?: number
  percentage: number
  status: 'Pending' | 'Downloading' | 'Completed' | { Failed: string }
}

interface ModelStatus {
  models_available: boolean
  missing_models: string[]
  total_missing: number
  timestamp: string
}

export function ModelDownloadManager() {
  const [modelStatus, setModelStatus] = useState<ModelStatus | null>(null)
  const [isDownloading, setIsDownloading] = useState(false)
  const [downloadProgress, setDownloadProgress] = useState<Record<string, DownloadProgress>>({})
  const [error, setError] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isLoadingModels, setIsLoadingModels] = useState(false)

  useEffect(() => {
    checkModelStatus()
    
    // Listen for download progress events
    const unlisten = listen<DownloadProgress>('download_progress', (event) => {
      const progress = event.payload
      setDownloadProgress(prev => ({
        ...prev,
        [progress.file_name]: progress
      }))
    })

    return () => {
      unlisten.then(f => f())
    }
  }, [])

  const checkModelStatus = async () => {
    try {
      setIsLoading(true)
      setError(null)
      const status = await invoke<ModelStatus>('check_models_status')
      setModelStatus(status)
    } catch (err) {
      console.error('Failed to check model status:', err)
      setError(err as string)
    } finally {
      setIsLoading(false)
    }
  }

  const startDownload = async () => {
    try {
      setIsDownloading(true)
      setError(null)
      setDownloadProgress({})
      
      await invoke('download_models')
      
      // Recheck status to confirm models are loaded (auto-reload is now handled in backend)
      await checkModelStatus()
    } catch (err) {
      console.error('Failed to download/load models:', err)
      setError(err as string)
    } finally {
      setIsDownloading(false)
      setIsLoadingModels(false)
    }
  }

  const clearAndRedownload = async () => {
    try {
      setIsDownloading(true)
      setError(null)
      setDownloadProgress({})
      
      await invoke('clear_and_redownload_models')
      
      // Recheck status to confirm models are loaded
      await checkModelStatus()
    } catch (err) {
      console.error('Failed to clear and re-download models:', err)
      setError(err as string)
    } finally {
      setIsDownloading(false)
      setIsLoadingModels(false)
    }
  }

  const getOverallProgress = () => {
    const progressValues = Object.values(downloadProgress)
    if (progressValues.length === 0) return 0
    
    const totalProgress = progressValues.reduce((sum, progress) => sum + progress.percentage, 0)
    return totalProgress / progressValues.length
  }

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  const getTotalDownloadedBytes = () => {
    return Object.values(downloadProgress).reduce((sum, progress) => sum + progress.downloaded_bytes, 0)
  }

  const getTotalBytes = () => {
    return Object.values(downloadProgress).reduce((sum, progress) => {
      return sum + (progress.total_bytes || progress.downloaded_bytes)
    }, 0)
  }

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center">
            <Brain className="w-5 h-5 mr-2" />
            AI Models
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center space-x-2">
            <RefreshCw className="w-4 h-4 animate-spin" />
            <span>Checking model status...</span>
          </div>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center">
          <Brain className="w-5 h-5 mr-2" />
          AI Models
        </CardTitle>
        <CardDescription>
          Manage the AI models used for intelligent search and document understanding
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4">
            <div className="flex items-start">
              <AlertCircle className="w-5 h-5 text-red-500 mt-0.5 mr-3 flex-shrink-0" />
              <div>
                <h3 className="text-sm font-semibold text-red-800">Error</h3>
                <p className="text-sm text-red-700 mt-1">{error}</p>
              </div>
            </div>
          </div>
        )}

        {/* Model Status */}
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">Model Status</h3>
            <Button
              variant="outline"
              size="sm"
              onClick={checkModelStatus}
              disabled={isLoading}
            >
              <RefreshCw className={`w-4 h-4 mr-2 ${isLoading ? 'animate-spin' : ''}`} />
              Refresh
            </Button>
          </div>

          {modelStatus && (
            <div className="flex items-center space-x-3">
              {modelStatus.models_available ? (
                <CheckCircle className="w-5 h-5 text-green-500" />
              ) : (
                <AlertCircle className="w-5 h-5 text-orange-500" />
              )}
              <div>
                <p className="font-medium">
                  {modelStatus.models_available ? 'All models available' : 'Missing models detected'}
                </p>
                <p className="text-sm text-gray-600">
                  {modelStatus.models_available 
                    ? 'AI search functionality is fully operational'
                    : `${modelStatus.total_missing} model files need to be downloaded`
                  }
                </p>
              </div>
              <div className="ml-auto">
                <Badge variant={modelStatus.models_available ? "default" : "secondary"}>
                  {modelStatus.models_available ? 'Ready' : 'Not Ready'}
                </Badge>
              </div>
            </div>
          )}
        </div>

        {/* Download Progress */}
        {(isDownloading || isLoadingModels) && (
          <div className="space-y-4">
            <div className="flex items-center space-x-3">
              <div className="w-8 h-8 bg-gradient-to-r from-blue-500 to-indigo-600 rounded-full flex items-center justify-center">
                {isLoadingModels ? (
                  <Brain className="w-4 h-4 text-white animate-pulse" />
                ) : (
                  <Download className="w-4 h-4 text-white animate-bounce" />
                )}
              </div>
              <h3 className="text-lg font-semibold">
                {isLoadingModels ? 'Loading AI Models' : 'Downloading AI Models'}
              </h3>
            </div>
            
            <div className="space-y-3">
              <div className="flex justify-between text-sm font-medium text-gray-700">
                <span>
                  {isLoadingModels ? 'Initializing models...' : 'Download Progress'}
                </span>
                <div className="text-right">
                  {isLoadingModels ? (
                    <span className="text-lg font-bold text-indigo-600">Loading...</span>
                  ) : (
                    <>
                      <span className="text-lg font-bold text-indigo-600">{Math.round(getOverallProgress())}%</span>
                      <div className="text-xs text-gray-500 mt-1">
                        {formatBytes(getTotalDownloadedBytes())} / {formatBytes(getTotalBytes())}
                      </div>
                    </>
                  )}
                </div>
              </div>
              <div className="relative">
                <div className="w-full h-3 bg-gray-200 rounded-full overflow-hidden">
                  <div 
                    className={`h-3 rounded-full transition-all duration-500 ease-out ${
                      isLoadingModels 
                        ? 'bg-gradient-to-r from-purple-500 to-indigo-600 animate-pulse' 
                        : 'bg-gradient-to-r from-blue-500 to-indigo-600'
                    }`}
                    style={{ 
                      width: isLoadingModels ? '100%' : `${getOverallProgress()}%` 
                    }}
                  />
                </div>
              </div>
              <p className="text-sm text-gray-600">
                {isLoadingModels 
                  ? 'AI models are being initialized...' 
                  : 'Downloading neural networks for visual and semantic search...'
                }
              </p>
            </div>
          </div>
        )}

        {/* Download Action */}
        {modelStatus && !modelStatus.models_available && !isDownloading && !isLoadingModels && (
          <div className="space-y-4">
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
              <h4 className="font-medium mb-2">Missing Models:</h4>
              <ul className="space-y-1">
                {modelStatus.missing_models.map((model) => (
                  <li key={model} className="flex items-center text-sm text-gray-700">
                    <div className="w-2 h-2 bg-orange-400 rounded-full mr-3"></div>
                    {model}
                  </li>
                ))}
              </ul>
            </div>
            
            <div className="space-y-2">
              <Button
                onClick={startDownload}
                disabled={isDownloading}
                className="w-full"
              >
                <Download className="w-4 h-4 mr-2" />
                Download Missing Models
              </Button>
              
              {error && (
                <Button
                  onClick={clearAndRedownload}
                  disabled={isDownloading}
                  variant="outline"
                  className="w-full text-orange-600 border-orange-200 hover:bg-orange-50"
                >
                  <RefreshCw className="w-4 h-4 mr-2" />
                  Clear & Re-download Models
                </Button>
              )}
            </div>
          </div>
        )}

        {/* Model Information */}
        <div className="bg-gray-50 border border-gray-200 rounded-lg p-4">
          <h3 className="text-lg font-semibold mb-2">Model Information</h3>
          <div className="space-y-3 text-sm">
            <div>
              <span className="font-medium">Vision Model:</span>
              <span className="ml-2 text-gray-600">CLIP vision encoder for image understanding</span>
            </div>
            <div>
              <span className="font-medium">Text Model:</span>
              <span className="ml-2 text-gray-600">CLIP text encoder for semantic search</span>
            </div>
            <div>
              <span className="font-medium">Tokenizer:</span>
              <span className="ml-2 text-gray-600">Text preprocessing and tokenization</span>
            </div>
          </div>
          
          <div className="mt-4 pt-4 border-t border-gray-200">
            <p className="text-xs text-gray-500">
              Models are stored locally in your application data directory and are only used for local processing.
              No data is sent to external servers.
            </p>
          </div>
        </div>
      </CardContent>
    </Card>
  )
} 
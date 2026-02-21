import React, { useState, useEffect } from 'react'
import { AlertTriangle, Download, Clock, CheckCircle } from 'lucide-react'
import { Button } from "./ui/button"

interface DownloadWarningProps {
  isOpen: boolean
  onClose: () => void
  onForceClose: () => void
  downloadProgress: number
  currentFile?: string
}

export function DownloadWarning({ 
  isOpen, 
  onClose, 
  onForceClose, 
  downloadProgress, 
  currentFile 
}: DownloadWarningProps) {
  // Smooth progress state to prevent flickering
  const [smoothProgress, setSmoothProgress] = useState(downloadProgress)

  // Smooth out rapid progress changes
  useEffect(() => {
    const timer = setTimeout(() => {
      setSmoothProgress(downloadProgress)
    }, 50) // Small delay to smooth out rapid updates

    return () => clearTimeout(timer)
  }, [downloadProgress])

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl max-w-md w-full mx-4 p-6">
        <div className="flex items-center mb-4">
          <div className="w-12 h-12 bg-orange-100 rounded-full flex items-center justify-center mr-4">
            <AlertTriangle className="w-6 h-6 text-orange-600" />
          </div>
          <div>
            <h3 className="text-lg font-semibold text-gray-900">Download in Progress</h3>
            <p className="text-sm text-gray-600">AI models are still downloading</p>
          </div>
        </div>

        <div className="mb-6">
          <div className="flex items-center justify-between text-sm text-gray-600 mb-2">
            <span>Progress</span>
            <span>{Math.round(smoothProgress)}%</span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2 overflow-hidden">
            <div 
              className="bg-blue-500 h-2 rounded-full transition-all duration-500 ease-out"
              style={{ width: `${Math.max(0, Math.min(100, smoothProgress))}%` }}
            />
          </div>
          <p className="text-xs text-gray-500 mt-2">
            Downloading AI models for search functionality
            {currentFile && (
              <span className="block text-gray-400 mt-1">Current: {currentFile}</span>
            )}
          </p>
        </div>

        <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
          <div className="flex items-start">
            <Download className="w-5 h-5 text-blue-600 mt-0.5 mr-3 flex-shrink-0" />
            <div className="text-sm text-blue-800">
              <p className="font-medium mb-1">What happens if you close now?</p>
              <ul className="space-y-1 text-xs">
                <li className="flex items-center">
                  <CheckCircle className="w-3 h-3 mr-2 text-green-600" />
                  Downloaded files are saved and won't be lost
                </li>
                <li className="flex items-center">
                  <Clock className="w-3 h-3 mr-2 text-orange-600" />
                  Download will resume from where it left off next time
                </li>
                <li className="flex items-center">
                  <CheckCircle className="w-3 h-3 mr-2 text-green-600" />
                  No data or progress will be lost
                </li>
              </ul>
            </div>
          </div>
        </div>

        <div className="flex space-x-3">
          <Button
            onClick={onClose}
            variant="outline"
            className="flex-1"
          >
            Keep Downloading
          </Button>
          <Button
            onClick={onForceClose}
            className="flex-1 bg-orange-600 hover:bg-orange-700"
          >
            Close Anyway
          </Button>
        </div>
      </div>
    </div>
  )
} 
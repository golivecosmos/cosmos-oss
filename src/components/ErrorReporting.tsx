import React, { useState, useEffect } from 'react'
import { Button } from "./ui/button"
import { Textarea } from "./ui/textarea"
import { Input } from "./ui/input"
import { Label } from "./ui/label"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card"
import {
  Bug,
  Send,
  Copy,
  CheckCircle,
  AlertCircle,
  Loader2,
  X
} from 'lucide-react'
import { errorReportingService, UploadResult } from '../services/errorReportingService'

// Report type constants
export const REPORT_TYPE_BUG = 'bug';
export const REPORT_TYPE_CRASH = 'crash';
export const REPORT_TYPE_FEATURE = 'feature';
export const REPORT_TYPE_OTHER = 'other';

interface ErrorReportingProps {
  isOpen: boolean
  onClose: () => void
  initialError?: {
    type: string
    message: string
    stack?: string
  }
}

type ReportType = typeof REPORT_TYPE_BUG | typeof REPORT_TYPE_CRASH | typeof REPORT_TYPE_FEATURE | typeof REPORT_TYPE_OTHER;

export const ErrorReporting: React.FC<ErrorReportingProps> = ({
  isOpen,
  onClose,
  initialError
}) => {
  const [reportType, setReportType] = useState<ReportType>(REPORT_TYPE_BUG)
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [reproductionSteps, setReproductionSteps] = useState('')
  const [actualBehavior, setActualBehavior] = useState('')
  const [userEmail, setUserEmail] = useState('')
  const [uploadConsent, setUploadConsent] = useState(true)

  const [isSubmitting, setIsSubmitting] = useState(false)
  const [isUploading, setIsUploading] = useState(false)
  const [submitStatus, setSubmitStatus] = useState<'idle' | 'success' | 'error'>('idle')
  const [reportId, setReportId] = useState<string>('')
  const [uploadResult, setUploadResult] = useState<UploadResult | null>(null)
  const [uploadAvailable, setUploadAvailable] = useState(false)

  // Load upload availability when component opens
  useEffect(() => {
    if (isOpen) {
      loadUploadAvailability()

      // Pre-fill form if there's an initial error
      if (initialError) {
        const validTypes = [REPORT_TYPE_BUG, REPORT_TYPE_CRASH, REPORT_TYPE_FEATURE, REPORT_TYPE_OTHER] as const;
        const type: ReportType = validTypes.includes(initialError.type as ReportType)
          ? (initialError.type as ReportType)
          : REPORT_TYPE_CRASH;
        setReportType(type)
        setTitle(`${initialError.type}: ${initialError.message}`)
        setDescription(`An error occurred: ${initialError.message}`)
        if (initialError.stack) {
          setActualBehavior(`Stack trace:\n${initialError.stack}`)
        }
      }
    }
  }, [isOpen, initialError])

  const loadUploadAvailability = async () => {
    try {
      const uploadAvail = await errorReportingService.isUploadAvailable()
      setUploadAvailable(uploadAvail)
    } catch (error) {
      console.error('Failed to check upload availability:', error)
    }
  }

  const handleSubmit = async () => {
    if (!title.trim() || !description.trim()) {
      alert('Please fill in the title and description fields.')
      return
    }

    setIsSubmitting(true)
    setSubmitStatus('idle')
    setUploadResult(null)

    try {
      if (reportType === REPORT_TYPE_CRASH && initialError) {
        // Report as JavaScript error (no upload option for crashes)
        const reportId = await errorReportingService.reportJavaScriptError(
          new Error(initialError.message),
          description,
          reproductionSteps,
          {
            title,
            actual_behavior: actualBehavior,
            report_type: reportType
          }
        )
        setReportId(reportId)
      } else {
        // Report as user bug report with optional upload
        setIsUploading(uploadConsent && uploadAvailable)

        const result = await errorReportingService.createBugReportWithUpload(
          title,
          description,
          reproductionSteps,
          actualBehavior,
          userEmail || undefined,
          uploadConsent && uploadAvailable
        )

        setReportId(result.reportId)
        if (result.uploadResult) {
          setUploadResult(result.uploadResult)
        }
      }

      setSubmitStatus('success')
    } catch (error) {
      console.error('Failed to submit error report:', error)
      setSubmitStatus('error')
    } finally {
      setIsSubmitting(false)
      setIsUploading(false)
    }
  }

  const copyReportId = () => {
    if (reportId) {
      navigator.clipboard.writeText(reportId)
    }
  }

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className="dark:bg-darkBg bg-white rounded-lg shadow-xl w-full max-w-2xl max-h-[90vh] overflow-hidden">
        <div className="flex items-center justify-between p-6 dark:border-darkBgHighlight border-b">
          <div className="flex items-center space-x-3">
            <Bug className="w-6 h-6 dark:text-customRed text-red-600" />
            <div>
              <h2 className="text-xl font-semibold">Report an Issue</h2>
              <p className="text-sm dark:text-customGray text-gray-500">
                Help us improve Cosmos by reporting bugs and issues
              </p>
            </div>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="h-8 w-8 p-0"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

        <div className="p-6 overflow-y-auto max-h-[calc(90vh-120px)]">
              {submitStatus === 'success' ? (
                <Card className="dark:bg-customGreen dark:border-greenShadow border-green-200 bg-green-50">
                  <CardHeader>
                    <CardTitle className="flex items-center space-x-2 dark:text-greenShadow">
                      <CheckCircle className="w-5 h-5" />
                      <span>Report Submitted Successfully</span>
                    </CardTitle>
                    <CardDescription className="dark:text-greenHighlight text-green-700">
                      Your error report has been created and saved locally.
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    <div>
                      <Label className="text-sm font-medium dark:text-greenShadow text-green-800">Report ID</Label>
                      <div className="flex items-center space-x-2 mt-1">
                        <code className="bg-green-100 px-2 py-1 rounded text-sm font-mono text-green-800">
                          {reportId}
                        </code>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={copyReportId}
                          className="h-8 dark:border-greenShadow"
                        >
                          <Copy className="w-4 h-4" />
                        </Button>
                      </div>
                    </div>

                {uploadResult && (
                  <div className="mt-3 pt-3 border-t border-green-200">
                    {uploadResult.success ? (
                      <div className="text-green-700">
                        <div className="flex items-center space-x-2">
                          <CheckCircle className="w-4 h-4" />
                          <span className="font-medium">Logs uploaded successfully</span>
                        </div>
                        <p className="text-sm mt-1">
                          Upload ID: <code className="bg-green-100 px-1 rounded text-xs">{uploadResult.upload_id}</code>
                        </p>
                        <p className="text-sm">
                          Size: {(uploadResult.file_size / 1024 / 1024).toFixed(2)} MB
                        </p>
                      </div>
                    ) : (
                      <div className="text-orange-700">
                        <div className="flex items-center space-x-2">
                          <AlertCircle className="w-4 h-4" />
                          <span className="font-medium">Log upload failed</span>
                        </div>
                        <p className="text-sm mt-1">
                          {uploadResult.error_message || 'Unknown upload error'}
                        </p>
                        <p className="text-sm text-green-700">
                          Your report was still submitted successfully.
                        </p>
                      </div>
                    )}
                  </div>
                )}

                <p className="text-sm dark:text-greenShadow text-green-700">
                  Please include this Report ID when contacting support.
                </p>
                  </CardContent>
                </Card>
              ) : (
                <div className="space-y-6">
                  <div>
                    <Label htmlFor="report-type">Report Type</Label>
                    <select
                      id="report-type"
                      value={reportType}
                      onChange={(e) => setReportType(e.target.value as ReportType)}
                      className="w-full mt-1 p-2 border dark:bg-darkBgHighlight dark:border-darkBgHighlight border-gray-300 rounded-md"
                    >
                      <option value={REPORT_TYPE_BUG}>Bug Report</option>
                      <option value={REPORT_TYPE_CRASH}>Application Crash</option>
                      <option value={REPORT_TYPE_FEATURE}>Feature Request</option>
                      <option value={REPORT_TYPE_OTHER}>Other Issue</option>
                    </select>
                  </div>

                  <div>
                    <Label htmlFor="title">Title *</Label>
                    <Input
                      id="title"
                      value={title}
                      onChange={(e) => setTitle(e.target.value)}
                      placeholder="Brief description of the issue"
                      className="mt-1 dark:border-darkBgHighlight"
                    />
                  </div>

                  <div>
                    <Label htmlFor="description">Description *</Label>
                    <Textarea
                      id="description"
                      value={description}
                      onChange={(e) => setDescription(e.target.value)}
                      placeholder="Detailed description of what happened"
                      rows={4}
                      className="mt-1 dark:border-darkBgHighlight"
                    />
                  </div>

                  <div>
                    <Label htmlFor="reproduction">Steps to Reproduce</Label>
                    <Textarea
                      id="reproduction"
                      value={reproductionSteps}
                      onChange={(e) => setReproductionSteps(e.target.value)}
                      placeholder="1. Go to...&#10;2. Click on...&#10;3. See error"
                      rows={3}
                      className="mt-1 dark:border-darkBgHighlight"
                    />
                  </div>

                  {/* Upload consent section */}
                  {uploadAvailable && reportType !== REPORT_TYPE_CRASH && (
                    <div className="border rounded-lg p-4 bg-blue-50 border-blue-200">
                      <div className="flex items-start space-x-3">
                        <input
                          type="checkbox"
                          id="upload-consent"
                          checked={uploadConsent}
                          onChange={(e) => setUploadConsent(e.target.checked)}
                          className="mt-1"
                        />
                        <div className="flex-1">
                          <Label htmlFor="upload-consent" className="text-blue-800 font-medium">
                            Share logs to help us fix this issue faster
                          </Label>
                          <p className="text-sm text-blue-700 mt-1">
                            By default, we'll securely upload your application logs along with your report.
                            You can uncheck this box if you prefer not to share logs.
                          </p>
                          {uploadConsent && (
                            <div className="mt-3">
                              <Label htmlFor="user-email" className="text-sm">
                                Email (optional - for follow-up)
                              </Label>
                              <Input
                                id="user-email"
                                type="email"
                                value={userEmail}
                                onChange={(e) => setUserEmail(e.target.value)}
                                placeholder="your.email@example.com"
                                className="mt-1"
                              />
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                  )}

                  {submitStatus === 'error' && (
                    <div className="bg-red-50 border border-red-200 rounded-lg p-4">
                      <div className="flex items-center space-x-2 text-red-800">
                        <AlertCircle className="w-5 h-5" />
                        <span className="font-medium">Failed to submit report</span>
                      </div>
                                        <p className="text-sm text-red-700 mt-1">
                    Please try again or contact support directly.
                  </p>
                    </div>
                  )}

                  <div className="flex justify-end space-x-3">
                    <Button variant="outline" onClick={onClose}>
                      Cancel
                    </Button>
                    <Button
                      onClick={handleSubmit}
                      disabled={isSubmitting || !title.trim() || !description.trim()}
                    >
                      {isSubmitting ? (
                        <>
                          <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                          {isUploading ? 'Submitting & Uploading...' : 'Submitting...'}
                        </>
                      ) : (
                        <>
                          <Send className="w-4 h-4 mr-2 dark:text-customBlue" />
                          {uploadConsent && uploadAvailable ? 'Submit & Upload Logs' : 'Submit Report'}
                        </>
                      )}
                    </Button>
                  </div>
                </div>
              )}
        </div>
      </div>
    </div>
  )
}

export default ErrorReporting

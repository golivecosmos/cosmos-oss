import React, { useState, useEffect, useMemo } from 'react'
import { X, FolderPlus, AlertCircle, CheckCircle2, FileText, ChevronDown, ChevronUp, Clock, Bug, StopCircle, PlayCircle, Trash2 } from 'lucide-react'
import { Button } from './ui/button'
import { formatDistanceToNow } from 'date-fns'
import ErrorReporting from './ErrorReporting'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import { useIndexingJobs } from '../contexts/IndexingJobsContext'

interface Job {
  id: string;
  path: string;
  progress: {
    current_file: string;
    processed: number;
    total: number;
    status: string;
    errors: string[];
    directory_path: string;
  };
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  startTime?: Date;
}

interface IndexStatusSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

// Job display limits for performance
const JOB_LIMITS = {
  PENDING: 50,
  COMPLETED: 20,
  FAILED: 20,
  CANCELLED: 10,
} as const;

export function IndexStatusSheet({ isOpen, onClose }: IndexStatusSheetProps) {
  const { indexingJobs: jobs, loadJobs } = useIndexingJobs();
  const [showBugReport, setShowBugReport] = useState(false);
  const [bugReportError, setBugReportError] = useState<{ type: string; message: string } | undefined>(undefined);
  const [showRunningJobs, setShowRunningJobs] = useState(true);
  const [showPendingJobs, setShowPendingJobs] = useState(true);
  const [showCompletedJobs, setShowCompletedJobs] = useState(true);
  const [showFailedJobs, setShowFailedJobs] = useState(true);
  const [showCancelledJobs, setShowCancelledJobs] = useState(true);
  const [cancellingJobs, setCancellingJobs] = useState<Set<string>>(new Set());
  const [queuePaused, setQueuePaused] = useState(false);
  const [queueActionLoading, setQueueActionLoading] = useState<null | 'stop' | 'resume' | 'clear' | 'clear_all'>(null);

  // Memoized job categorization for performance
  const jobCategories = useMemo(() => {
    const runningJobs = jobs.filter(job => job.status === 'running');
    const pendingJobs = jobs.filter(job => job.status === 'pending');
    const completedJobs = jobs.filter(job => job.status === 'completed');
    const failedJobs = jobs.filter(job => job.status === 'failed');
    const cancelledJobs = jobs.filter(job => job.status === 'cancelled');

    return {
      running: runningJobs,
      pending: {
        all: pendingJobs,
        displayed: pendingJobs.slice(0, JOB_LIMITS.PENDING),
        total: pendingJobs.length
      },
      completed: {
        all: completedJobs,
        displayed: completedJobs.slice(0, JOB_LIMITS.COMPLETED),
        total: completedJobs.length
      },
      failed: {
        all: failedJobs,
        displayed: failedJobs.slice(0, JOB_LIMITS.FAILED),
        total: failedJobs.length
      },
      cancelled: {
        all: cancelledJobs,
        displayed: cancelledJobs.slice(0, JOB_LIMITS.CANCELLED),
        total: cancelledJobs.length
      },
      hasMore: {
        pending: pendingJobs.length > JOB_LIMITS.PENDING,
        completed: completedJobs.length > JOB_LIMITS.COMPLETED,
        failed: failedJobs.length > JOB_LIMITS.FAILED,
        cancelled: cancelledJobs.length > JOB_LIMITS.CANCELLED,
      }
    };
  }, [jobs]);

  // Add stabilization for running jobs section to prevent frenetic behavior
  const [showRunningSection, setShowRunningSection] = useState(false);
  const [runningJobsTimeout, setRunningJobsTimeout] = useState<NodeJS.Timeout | null>(null);

  useEffect(() => {
    const hasRunningJobs = jobCategories.running.length > 0;
    const hasPendingJobs = jobCategories.pending.total > 0;
    
    if (hasRunningJobs) {
      // Immediately show running section when jobs are running
      if (runningJobsTimeout) {
        clearTimeout(runningJobsTimeout);
        setRunningJobsTimeout(null);
      }
      setShowRunningSection(true);
    } else if (hasPendingJobs) {
      // Keep showing for a brief moment when jobs are pending but not yet running
      if (!runningJobsTimeout) {
        const timeout = setTimeout(() => {
          setShowRunningSection(false);
          setRunningJobsTimeout(null);
        }, 1000); // 1 second grace period
        setRunningJobsTimeout(timeout);
      }
    } else {
      // No jobs at all, hide immediately
      if (runningJobsTimeout) {
        clearTimeout(runningJobsTimeout);
        setRunningJobsTimeout(null);
      }
      setShowRunningSection(false);
    }

    return () => {
      if (runningJobsTimeout) {
        clearTimeout(runningJobsTimeout);
      }
    };
  }, [jobCategories.running.length, jobCategories.pending.total]);

  useEffect(() => {
    if (!isOpen) return;

    let unlistenQueueChanged: (() => void) | null = null;

    const loadQueueStatus = async () => {
      try {
        const status = await invoke<any>('manage_job_queue', { action: 'status' });
        setQueuePaused(Boolean(status?.paused));
      } catch (error) {
        console.error('Failed to load queue status:', error);
      }
    };

    const setup = async () => {
      await loadQueueStatus();
      unlistenQueueChanged = await listen('queue_processing_changed', (event: any) => {
        const paused = Boolean(event?.payload?.paused);
        setQueuePaused(paused);
      });
    };

    setup();

    return () => {
      if (unlistenQueueChanged) {
        unlistenQueueChanged();
      }
    };
  }, [isOpen]);

  // Early return if not open
  if (!isOpen) return null;

  const handleCancelJob = async (jobId: string) => {
    setCancellingJobs(prev => new Set([...prev, jobId]));
    
    try {
      console.log('🚀 Sending cancel request with job_id:', jobId);
      await invoke('manage_job_queue', { 
        action: 'cancel', 
        job_id: jobId 
      });
      loadJobs();
    } catch (error) {
      console.error('❌ Failed to cancel job:', error);
    } finally {
      setCancellingJobs(prev => {
        const next = new Set(prev);
        next.delete(jobId);
        return next;
      });
    }
  };



  const handleRetryJob = async (job: Job) => {
    console.log('🔄 Retry button clicked for job:', job);
    console.log('🔍 Job ID:', job.id, 'Type:', typeof job.id);
    
    if (!job.id) {
      console.error('❌ Job ID is missing:', job);
      return;
    }
    
    try {
      console.log('🚀 Sending retry request with job_id:', job.id);
      await invoke('retry_job', { 
        jobId: job.id
      });
      loadJobs();
    } catch (error) {
      console.error('❌ Failed to retry job:', error);
    }
  };

  const handleQueueAction = async (action: 'stop' | 'resume' | 'clear' | 'clear_all') => {
    if (queueActionLoading) return;

    if (action === 'clear_all') {
      const confirmed = window.confirm('Clear all jobs (including completed/failed/cancelled)? This cannot be undone.');
      if (!confirmed) return;
    } else if (action === 'clear') {
      const confirmed = window.confirm('Clear pending and running jobs?');
      if (!confirmed) return;
    }

    setQueueActionLoading(action);
    try {
      await invoke('manage_job_queue', { action });
      if (action === 'stop' || action === 'clear' || action === 'clear_all') {
        setQueuePaused(true);
      } else if (action === 'resume') {
        setQueuePaused(false);
      }
      await loadJobs();
    } catch (error) {
      console.error(`Failed queue action ${action}:`, error);
    } finally {
      setQueueActionLoading(null);
    }
  };

  // Helper to get base name from path
  const getBaseName = (path: string): string => {
    return path.split('/').pop() || path;
  };

  const JobSection = ({ 
    title, 
    jobs, 
    bgColor, 
    textColor, 
    borderColor, 
    icon: Icon, 
    isExpanded, 
    onToggle, 
    hasMore = false,
    limit = 0,
    totalCount
  }: {
    title: string;
    jobs: Job[];
    bgColor: string;
    textColor: string;
    borderColor: string;
    icon: React.ComponentType<{ className: string }>;
    isExpanded: boolean;
    onToggle: () => void;
    hasMore?: boolean;
    limit?: number;
    totalCount?: number;
  }) => {
    if (jobs.length === 0) return null;

    const displayCount = totalCount || jobs.length;

    return (
      <div 
        className={`${bgColor} rounded-lg p-3 transition-all duration-300 ease-in-out transform`}
        style={{
          opacity: jobs.length > 0 ? 1 : 0,
          transform: jobs.length > 0 ? 'translateY(0) scale(1)' : 'translateY(-10px) scale(0.95)',
        }}
      >
        <button
          className={`w-full flex items-center justify-between font-medium ${textColor} text-sm mb-2 border-b ${borderColor} pb-1 bg-transparent hover:bg-opacity-20 hover:bg-gray-500 rounded transition-colors duration-200`}
          onClick={onToggle}
        >
          <span className="flex items-center gap-2">
            <Icon className="h-4 w-4" />
            {title} ({displayCount} files){hasMore && ` - showing first ${limit}`}
          </span>
          {isExpanded ? (
            <ChevronUp className="h-4 w-4 ml-2" />
          ) : (
            <ChevronDown className="h-4 w-4 ml-2" />
          )}
        </button>
        
        <div 
          className={`transition-all duration-300 ease-in-out ${
            isExpanded ? 'max-h-96 opacity-100 overflow-y-auto' : 'max-h-0 opacity-0 overflow-hidden'
          }`}
        >
          <ul className="space-y-2 mt-2 pr-2">
            {jobs.map(job => (
              <JobItem 
                key={job.id} 
                job={job} 
                onCancel={handleCancelJob}
                onRetry={handleRetryJob}
                cancellingJobs={cancellingJobs}
                setBugReportError={setBugReportError}
                setShowBugReport={setShowBugReport}
              />
            ))}
          </ul>
        </div>
      </div>
    );
  };

  const JobItem = ({ 
    job, 
    onCancel, 
    onRetry,
    cancellingJobs, 
    setBugReportError, 
    setShowBugReport, 
  }: {
    job: Job;
    onCancel: (jobId: string) => void;
    onRetry: (job: Job) => void;
    cancellingJobs: Set<string>;
    setBugReportError: (error: { type: string; message: string }) => void;
    setShowBugReport: (show: boolean) => void;
  }) => {
    const isRunning = job.status === 'running';
    const isFailed = job.status === 'failed';
    const isPending = job.status === 'pending';
    
    return (
      <li className={`
        border rounded-md p-2 dark:bg-darkBgMid bg-white transition-all duration-200 hover:shadow-sm
        ${isRunning ? 'border-blue-200 dark:border-customBlue' : ''}
        ${isFailed ? 'border-red-200 dark:border-customRed' : ''}
        ${isPending ? 'border-orange-200 dark:border-customYellow' : ''}
        ${job.status === 'completed' ? 'dark:border-customGreen border-green-200' : ''}
        ${job.status === 'cancelled' ? 'dark:border-customGray border-gray-200' : ''}
      `}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 flex-1 min-w-0">
            {isRunning && <FolderPlus className="h-4 w-4 text-blue-500 dark:text-blueHighlight flex-shrink-0" />}
            {isPending && <FileText className="h-4 w-4 text-orange-500 dark:text-yellowHighlight flex-shrink-0" />}
            {isFailed && <AlertCircle className="h-4 w-4 text-red-500 dark:text-redHighlight flex-shrink-0" />}
            {job.status === 'completed' && <CheckCircle2 className="h-4 w-4 text-green-500  dark:text-greenHighlight flex-shrink-0" />}
            {job.status === 'cancelled' && <StopCircle className="h-4 w-4 text-gray-500 dark:text-customGray flex-shrink-0" />}
            
            <div className="min-w-0 flex-1">
              <div className={`font-medium text-sm truncate ${
                isRunning ? 'text-blue-800 dark:text-customBlue' : 
                isPending ? 'text-orange-800 dark:text-customYellow' : 
                isFailed ? 'text-red-800 dark:text-customRed' : 
                job.status === 'completed' ? 'dark:text-customGreen text-green-800' : 'text-gray-800'
              }`}>
                {getBaseName(job.progress.directory_path)}
              </div>
              
              <div className={`text-xs truncate ${
                isRunning ? 'text-blue-600 dark:text-blueHighlight' : 
                isPending ? 'text-orange-600 dark:text-yellowHighlight' : 
                isFailed ? 'text-red-600 dark:text-redHighlight' : 
                job.status === 'completed' ? 'text-green-600 dark:text-greenHighlight' : 'text-gray-600'
              }`}>
                {job.progress.directory_path}
              </div>

              {isRunning && job.progress.current_file && (
                <div className="text-xs dark:text-blueHighlight text-blue-500 font-mono mt-1">
                  📄 {getBaseName(job.progress.current_file)}
                </div>
              )}

              {isFailed && job.progress.errors && job.progress.errors.length > 0 && (
                <div className="text-xs dark:text-redHighlight text-red-500 mt-1 font-mono truncate">
                  {job.progress.errors[0]}
                </div>
              )}

              {job.startTime && (
                <div className="text-xs dark:text-customGray text-gray-500 mt-1">
                  {job.status === 'completed' && `${formatDistanceToNow(job.startTime)} ago`}
                  {job.status === 'failed' && `Failed ${formatDistanceToNow(job.startTime)} ago`}
                  {job.status === 'cancelled' && `Cancelled ${formatDistanceToNow(job.startTime)} ago`}
                  {isRunning && `Started ${formatDistanceToNow(job.startTime)} ago`}
                </div>
              )}
            </div>
          </div>

          <div className="flex items-center gap-1 ml-2">
            {isRunning && (
              <div className="text-xs dark:text-blueHighlight text-blue-600 mr-2">
                {job.progress.processed} of {job.progress.total}
              </div>
            )}
            
            {isFailed && (
              <Button
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0 dark:text-customRed text-red-600 dark:hover:bg-customRed hover:bg-red-100 dark:hover:text-redHighlight hover:text-red-700"
                title="Report bug for this failed job"
                onClick={() => {
                  const errorMessage = job.progress.errors?.[0] || 'Job failed without specific error message';
                  setBugReportError({
                    type: 'indexing_failure',
                    message: `Failed to index file: ${getBaseName(job.progress.directory_path)} - ${errorMessage}`
                  });
                  setShowBugReport(true);
                }}
              >
                <Bug className="h-3 w-3" />
              </Button>
            )}

            {job.status === 'failed' && (
              <Button
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0 text-blue-600 dark:text-customYellow dark:hover:text-yellowHighlight dark:hover:bg-customYellow hover:bg-blue-100 hover:text-blue-700"
                title="Retry this job"
                onClick={() => onRetry(job)}
              >
                <Clock className="h-3 w-3" />
              </Button>
            )}

            {(isRunning || isPending) && (
              <Button
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0 dark:text-customRed dark:hover:bg-customRed dark:hover:text-redHighlight text-red-600 hover:bg-red-100 hover:text-red-700"
                title={cancellingJobs.has(job.id) ? "Cancelling..." : "Cancel job"}
                onClick={() => onCancel(job.id)}
                disabled={cancellingJobs.has(job.id)}
              >
                {cancellingJobs.has(job.id) ? (
                  <div className="h-3 w-3 animate-spin rounded-full border border-red-600 dark:border-customRed border-t-transparent" />
                ) : (
                  <StopCircle className="h-3 w-3" />
                )}
              </Button>
            )}
          </div>
        </div>

        {isRunning && (
          <>
            <div className="w-full dark:bg-customBlue bg-blue-200 rounded-full h-1.5 mt-2">
              <div
                className="dark:bg-blueHighlight bg-blue-600 h-1.5 rounded-full transition-all duration-500 ease-out"
                style={{ width: `${Math.min((job.progress.processed / job.progress.total) * 100, 100)}%` }}
              />
            </div>
          </>
        )}
      </li>
    );
  };

  const hasAnyJobs = jobs.length > 0;

  return (
    <div className="fixed inset-y-0 right-0 w-[500px] dark:bg-darkBg bg-white shadow-xl border-l dark:border-darkBgHighlight border-gray-200 flex flex-col z-50">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b dark:border-darkBgHighlight border-gray-200">
        <div className="flex items-center gap-2">
          <FileText className="h-5 w-5 dark:text-customBlue text-blue-600" />
          <h2 className="text-lg font-semibold dark:text-text text-gray-900">Indexing Status</h2>
          <span className={`text-xs px-2 py-0.5 rounded-full ${queuePaused ? 'bg-orange-100 text-orange-700 dark:bg-yellowShadow dark:text-yellowHighlight' : 'bg-green-100 text-green-700 dark:bg-greenShadow dark:text-greenHighlight'}`}>
            {queuePaused ? 'Paused' : 'Active'}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => handleQueueAction(queuePaused ? 'resume' : 'stop')}
            disabled={queueActionLoading !== null}
            title={queuePaused ? 'Resume queue processing' : 'Stop queue processing'}
            className="h-8 px-2"
          >
            {queuePaused ? <PlayCircle className="h-4 w-4 mr-1" /> : <StopCircle className="h-4 w-4 mr-1" />}
            {queuePaused ? (queueActionLoading === 'resume' ? 'Resuming...' : 'Resume') : (queueActionLoading === 'stop' ? 'Stopping...' : 'Stop')}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => handleQueueAction('clear')}
            disabled={queueActionLoading !== null}
            title="Clear pending and running jobs"
            className="h-8 px-2 text-orange-700 dark:text-yellowHighlight"
          >
            <Trash2 className="h-4 w-4 mr-1" />
            {queueActionLoading === 'clear' ? 'Clearing...' : 'Clear Queue'}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => handleQueueAction('clear_all')}
            disabled={queueActionLoading !== null}
            title="Clear all jobs"
            className="h-8 px-2 text-red-700 dark:text-customRed"
          >
            <Trash2 className="h-4 w-4 mr-1" />
            {queueActionLoading === 'clear_all' ? 'Clearing...' : 'Clear All'}
          </Button>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6">
        {!hasAnyJobs ? (
          <div className="text-center dark:text-customGray text-gray-500 py-8">
            No recent indexing jobs
          </div>
        ) : (
          <div className="space-y-4">
                         {/* Running Jobs - Stabilized to prevent frenetic behavior */}
             {(showRunningSection || jobCategories.running.length > 0) && (
               <div 
                 className={`bg-blue-50 dark:bg-blueShadow rounded-lg p-3 transition-all duration-500 ease-in-out transform ${
                   jobCategories.running.length > 0 ? 'opacity-100 scale-100' : 'opacity-60 scale-95'
                 }`}
               >
                 <button
                   className="w-full flex items-center justify-between font-medium text-blue-700 dark:text-blueHighlight text-sm mb-2 border-b dark:border-customBlue border-blue-200 pb-1 bg-transparent hover:bg-opacity-20 hover:bg-gray-500 rounded transition-colors duration-200"
                   onClick={() => setShowRunningJobs(v => !v)}
                 >
                   <span className="flex items-center gap-2">
                     <div className="animate-spin rounded-full h-4 w-4 border-b-2 dark:border-blueHighlight border-blue-500" />
                     {jobCategories.running.length > 0 
                       ? `Indexing in Progress (${jobCategories.running.length} files)` 
                       : `Preparing to index (${jobCategories.pending.total} queued)`
                     }
                   </span>
                   {showRunningJobs ? (
                     <ChevronUp className="h-4 w-4 ml-2" />
                   ) : (
                     <ChevronDown className="h-4 w-4 ml-2" />
                   )}
                 </button>
                 
                 <div 
                   className={`transition-all duration-300 ease-in-out ${
                     showRunningJobs ? 'max-h-96 opacity-100 overflow-y-auto' : 'max-h-0 opacity-0 overflow-hidden'
                   }`}
                 >
                   {jobCategories.running.length > 0 ? (
                     <ul className="space-y-2 mt-2 pr-2">
                       {jobCategories.running.map(job => (
                         <JobItem 
                           key={job.id} 
                           job={job} 
                           onCancel={handleCancelJob}
                           onRetry={handleRetryJob}
                           cancellingJobs={cancellingJobs}
                           setBugReportError={setBugReportError}
                           setShowBugReport={setShowBugReport}
                         />
                       ))}
                     </ul>
                   ) : (
                     <div className="mt-2 text-sm dark:text-customBlue text-blue-600 italic">
                       Jobs are being prepared for processing...
                     </div>
                   )}
                 </div>
               </div>
             )}

             {/* Pending Jobs */}
             {jobCategories.pending.total > 0 && (
               <JobSection
                 title="Pending Queue"
                 jobs={jobCategories.pending.displayed}
                 bgColor="bg-orange-50 dark:bg-yellowShadow"
                 textColor="text-orange-700 dark:text-yellowHighlight"
                 borderColor="border-orange-200 dark:border-customYellow"
                 icon={Clock}
                 isExpanded={showPendingJobs}
                 onToggle={() => setShowPendingJobs(v => !v)}
                 hasMore={jobCategories.hasMore.pending}
                 limit={JOB_LIMITS.PENDING}
                 totalCount={jobCategories.pending.total}
               />
             )}

             {/* Completed Jobs */}
             {jobCategories.completed.total > 0 && (
               <JobSection
                 title="Completed"
                 jobs={jobCategories.completed.displayed}
                 bgColor="bg-green-50 dark:bg-greenShadow"
                 textColor="text-green-700 dark:text-greenHighlight"
                 borderColor="border-green-200 dark:border-customGreen"
                 icon={CheckCircle2}
                 isExpanded={showCompletedJobs}
                 onToggle={() => setShowCompletedJobs(v => !v)}
                 hasMore={jobCategories.hasMore.completed}
                 limit={JOB_LIMITS.COMPLETED}
                 totalCount={jobCategories.completed.total}
               />
             )}

             {/* Failed Jobs */}
             {jobCategories.failed.total > 0 && (
               <JobSection
                 title="Failed"
                 jobs={jobCategories.failed.displayed}
                 bgColor="bg-red-50 dark:bg-redShadow"
                 textColor="text-red-700 dark:text-redHighlight"
                 borderColor="border-red-200 dark:border-customRed"
                 icon={AlertCircle}
                 isExpanded={showFailedJobs}
                 onToggle={() => setShowFailedJobs(v => !v)}
                 hasMore={jobCategories.hasMore.failed}
                 limit={JOB_LIMITS.FAILED}
                 totalCount={jobCategories.failed.total}
               />
             )}

             {/* Cancelled Jobs */}
             {jobCategories.cancelled.total > 0 && (
               <JobSection
                 title="Cancelled"
                 jobs={jobCategories.cancelled.displayed}
                 bgColor="bg-gray-50 dark:bg-darkBgHighlight"
                 textColor="text-gray-700 dark:text-customGray"
                 borderColor="border-gray-200 dark:border-customGray"
                 icon={StopCircle}
                 isExpanded={showCancelledJobs}
                 onToggle={() => setShowCancelledJobs(v => !v)}
                 hasMore={jobCategories.hasMore.cancelled}
                 limit={JOB_LIMITS.CANCELLED}
                 totalCount={jobCategories.cancelled.total}
               />
             )}
          </div>
        )}
      </div>

      {/* ErrorReporting Modal */}
      <ErrorReporting
        isOpen={showBugReport}
        onClose={() => setShowBugReport(false)}
        initialError={bugReportError}
      />
    </div>
  );
} 

import React, { useState, useEffect, useMemo } from 'react'
import { X, FolderPlus, AlertCircle, CheckCircle2, FileText, ChevronDown, ChevronUp, Clock, Bug, StopCircle, PlayCircle, Trash2 } from 'lucide-react'
import { Button } from './ui/button'
import { formatDistanceToNow } from 'date-fns'
import ErrorReporting from './ErrorReporting'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useIndexingJobs } from '../contexts/IndexingJobsContext'
import type { Job } from '../contexts/IndexingJobsContext'

type JobStatus = Job['status'];

interface IndexStatusSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

interface QueueHealthStatus {
  total: number;
  pending: number;
  running: number;
  completed: number;
  failed: number;
  cancelled: number;
  retry_scheduled: number;
  retry_ready: number;
  stale_running: number;
  orphaned_pending_claims: number;
  completed_last_hour: number;
  failed_last_hour: number;
  oldest_pending_age_seconds: number | null;
  longest_running_age_seconds: number | null;
  latest_update_at: string | null;
  paused: boolean;
}

// Job display limits for performance
const JOB_LIMITS = {
  PENDING: 50,
  COMPLETED: 20,
  FAILED: 20,
  CANCELLED: 10,
} as const;

const JOB_STATUS_UI: Record<JobStatus, {
  borderClass: string;
  titleClass: string;
  pathClass: string;
  badgeClass: string;
  badgeText: string;
  icon: React.ComponentType<{ className: string }>;
}> = {
  pending: {
    borderClass: 'border-orange-200 dark:border-customYellow',
    titleClass: 'text-orange-800 dark:text-customYellow',
    pathClass: 'text-orange-600 dark:text-yellowHighlight',
    badgeClass: 'bg-orange-100 text-orange-700 dark:bg-yellowShadow dark:text-yellowHighlight',
    badgeText: 'Pending',
    icon: Clock,
  },
  running: {
    borderClass: 'border-blue-200 dark:border-customBlue',
    titleClass: 'text-blue-800 dark:text-customBlue',
    pathClass: 'text-blue-600 dark:text-blueHighlight',
    badgeClass: 'bg-blue-100 text-blue-700 dark:bg-blueShadow dark:text-blueHighlight',
    badgeText: 'Running',
    icon: FolderPlus,
  },
  completed: {
    borderClass: 'border-green-200 dark:border-customGreen',
    titleClass: 'text-green-800 dark:text-customGreen',
    pathClass: 'text-green-600 dark:text-greenHighlight',
    badgeClass: 'bg-green-100 text-green-700 dark:bg-greenShadow dark:text-greenHighlight',
    badgeText: 'Completed',
    icon: CheckCircle2,
  },
  failed: {
    borderClass: 'border-red-200 dark:border-customRed',
    titleClass: 'text-red-800 dark:text-customRed',
    pathClass: 'text-red-600 dark:text-redHighlight',
    badgeClass: 'bg-red-100 text-red-700 dark:bg-redShadow dark:text-redHighlight',
    badgeText: 'Failed',
    icon: AlertCircle,
  },
  cancelled: {
    borderClass: 'border-gray-200 dark:border-customGray',
    titleClass: 'text-gray-800 dark:text-customGray',
    pathClass: 'text-gray-600 dark:text-customGray',
    badgeClass: 'bg-gray-100 text-gray-700 dark:bg-darkBgHighlight dark:text-customGray',
    badgeText: 'Cancelled',
    icon: StopCircle,
  },
};

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
  const [queueStatus, setQueueStatus] = useState<QueueHealthStatus | null>(null);
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
    let pollInterval: ReturnType<typeof setInterval> | null = null;
    let active = true;

    const loadQueueStatus = async () => {
      try {
        const status = await invoke<QueueHealthStatus>('manage_job_queue', { action: 'status' });
        if (!active) return;
        setQueuePaused(Boolean(status?.paused));
        setQueueStatus(status);
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
      pollInterval = setInterval(() => {
        void loadQueueStatus();
      }, 5000);
    };

    void setup();

    return () => {
      active = false;
      if (unlistenQueueChanged) {
        unlistenQueueChanged();
      }
      if (pollInterval) {
        clearInterval(pollInterval);
      }
    };
  }, [isOpen]);

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

  const formatAge = (seconds: number | null | undefined): string => {
    if (seconds === null || seconds === undefined) return 'n/a';
    if (seconds < 60) return `${seconds}s`;

    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m`;

    const hours = Math.floor(minutes / 60);
    const remainingMinutes = minutes % 60;
    return remainingMinutes > 0 ? `${hours}h ${remainingMinutes}m` : `${hours}h`;
  };

  const queueHealthLabel = useMemo(() => {
    if (!queueStatus) return { text: 'Unknown', className: 'bg-gray-100 text-gray-700 dark:bg-darkBgHighlight dark:text-customGray' };

    if (queueStatus.stale_running > 0 || queueStatus.orphaned_pending_claims > 0) {
      return { text: 'Degraded', className: 'bg-red-100 text-red-700 dark:bg-redShadow dark:text-redHighlight' };
    }

    if (queueStatus.retry_scheduled > 0 || queueStatus.failed_last_hour > 0) {
      return { text: 'Warning', className: 'bg-orange-100 text-orange-700 dark:bg-yellowShadow dark:text-yellowHighlight' };
    }

    return { text: 'Healthy', className: 'bg-green-100 text-green-700 dark:bg-greenShadow dark:text-greenHighlight' };
  }, [queueStatus]);

  const queueLastUpdatedLabel = useMemo(() => {
    if (!queueStatus?.latest_update_at) return 'No queue updates yet';
    const timestamp = new Date(queueStatus.latest_update_at);
    if (Number.isNaN(timestamp.getTime())) return 'Unknown update time';
    return `Updated ${formatDistanceToNow(timestamp)} ago`;
  }, [queueStatus?.latest_update_at]);

  // Early return only after all hooks have run to keep hook order stable.
  if (!isOpen) return null;

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
        className={`${bgColor} rounded-xl border ${borderColor} p-3 transition-all duration-300 ease-in-out`}
        style={{
          opacity: jobs.length > 0 ? 1 : 0,
          transform: jobs.length > 0 ? 'translateY(0)' : 'translateY(-6px)',
        }}
      >
        <button
          className={`w-full flex items-center justify-between font-semibold ${textColor} text-sm mb-2 border-b ${borderColor} pb-2 bg-transparent hover:bg-white/60 dark:hover:bg-darkBgHighlight/40 rounded-md transition-colors duration-200 px-1`}
          onClick={onToggle}
        >
          <span className="flex items-center gap-2 tracking-tight">
            <Icon className="h-4 w-4" />
            {title} ({displayCount}){hasMore && ` - first ${limit}`}
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
    const statusUi = JOB_STATUS_UI[job.status];
    const StatusIcon = statusUi.icon;
    const progressPercent = job.progress.total > 0
      ? Math.min((job.progress.processed / job.progress.total) * 100, 100)
      : 0;
    
    return (
      <li className={`group border rounded-lg p-3 dark:bg-darkBgMid bg-white transition-all duration-200 hover:shadow-md hover:-translate-y-0.5 ${statusUi.borderClass}`}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3 flex-1 min-w-0">
            <StatusIcon className={`h-4 w-4 flex-shrink-0 ${statusUi.pathClass}`} />
            
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <div className={`font-semibold text-sm truncate tracking-tight ${statusUi.titleClass}`}>
                  {getBaseName(job.progress.directory_path)}
                </div>
                <span className={`text-[11px] px-2 py-0.5 rounded-full font-medium ${statusUi.badgeClass}`}>
                  {statusUi.badgeText}
                </span>
              </div>
              
              <div className={`text-xs truncate mt-0.5 ${statusUi.pathClass}`}>
                {job.progress.directory_path}
              </div>

              {isRunning && job.progress.current_file && (
                <div className="text-xs dark:text-blueHighlight text-blue-500 font-mono mt-1">
                  {getBaseName(job.progress.current_file)}
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
              <div className="text-xs dark:text-blueHighlight text-blue-600 mr-2 font-medium">
                {job.progress.processed} of {job.progress.total}
              </div>
            )}
            
            {isFailed && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 w-7 p-0 rounded-md dark:text-customRed text-red-600 dark:hover:bg-customRed/20 hover:bg-red-100 dark:hover:text-redHighlight hover:text-red-700"
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
                className="h-7 w-7 p-0 rounded-md text-blue-600 dark:text-customYellow dark:hover:text-yellowHighlight dark:hover:bg-customYellow/20 hover:bg-blue-100 hover:text-blue-700"
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
                className="h-7 w-7 p-0 rounded-md dark:text-customRed dark:hover:bg-customRed/20 dark:hover:text-redHighlight text-red-600 hover:bg-red-100 hover:text-red-700"
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
            <div className="w-full dark:bg-customBlue/30 bg-blue-200 rounded-full h-1.5 mt-3">
              <div
                className="dark:bg-blueHighlight bg-blue-600 h-1.5 rounded-full transition-all duration-500 ease-out"
                style={{ width: `${progressPercent}%` }}
              />
            </div>
          </>
        )}
      </li>
    );
  };

  const hasAnyJobs = jobs.length > 0;

  return (
    <div className="fixed inset-y-0 right-0 w-full sm:w-[560px] dark:bg-darkBg bg-white shadow-2xl border-l dark:border-darkBgHighlight border-gray-200 flex flex-col z-50">
      {/* Header */}
      <div className="px-6 py-4 border-b dark:border-darkBgHighlight border-gray-200 bg-white/95 dark:bg-darkBg/95 backdrop-blur-sm">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <FileText className="h-5 w-5 dark:text-customBlue text-blue-600" />
              <h2 className="text-lg font-semibold tracking-tight dark:text-text text-gray-900">Indexing Status</h2>
            </div>
            <p className="text-xs text-gray-500 dark:text-customGray mt-1">
              Live queue activity and processing health
            </p>
            <div className="flex items-center gap-2 mt-2">
              <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${queuePaused ? 'bg-orange-100 text-orange-700 dark:bg-yellowShadow dark:text-yellowHighlight' : 'bg-green-100 text-green-700 dark:bg-greenShadow dark:text-greenHighlight'}`}>
                {queuePaused ? 'Paused' : 'Active'}
              </span>
              <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${queueHealthLabel.className}`}>
                {queueHealthLabel.text}
              </span>
            </div>
          </div>
          <div className="flex items-start gap-2">
            <div className="flex items-center gap-1.5 rounded-2xl border border-slate-200/90 dark:border-darkBgHighlight bg-slate-100/80 dark:bg-darkBgMid/70 p-1.5 shadow-[0_6px_18px_rgba(15,23,42,0.08)] dark:shadow-none">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleQueueAction(queuePaused ? 'resume' : 'stop')}
                disabled={queueActionLoading !== null}
                title={queuePaused ? 'Resume queue processing' : 'Stop queue processing'}
                className={`h-9 px-4 rounded-xl font-semibold tracking-tight transition-all ${
                  queuePaused
                    ? 'bg-emerald-600 text-white hover:bg-emerald-700 dark:bg-emerald-500 dark:hover:bg-emerald-400'
                    : 'bg-slate-900 text-white hover:bg-slate-800 dark:bg-slate-100 dark:text-slate-900 dark:hover:bg-white'
                }`}
              >
                {queuePaused ? <PlayCircle className="h-4 w-4 mr-1.5" /> : <StopCircle className="h-4 w-4 mr-1.5" />}
                {queuePaused ? (queueActionLoading === 'resume' ? 'Resuming...' : 'Resume') : (queueActionLoading === 'stop' ? 'Stopping...' : 'Stop')}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleQueueAction('clear')}
                disabled={queueActionLoading !== null}
                title="Clear pending and running jobs"
                className="h-9 px-3 rounded-xl text-amber-700 dark:text-yellowHighlight font-semibold tracking-tight bg-transparent hover:bg-amber-100/90 dark:hover:bg-yellowShadow/40 hover:text-amber-800 border border-transparent hover:border-amber-200 dark:hover:border-customYellow/50"
              >
                <Trash2 className="h-4 w-4 mr-1.5" />
                {queueActionLoading === 'clear' ? 'Clearing...' : 'Clear Queue'}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleQueueAction('clear_all')}
                disabled={queueActionLoading !== null}
                title="Clear all jobs"
                className="h-9 px-3 rounded-xl text-rose-700 dark:text-customRed font-semibold tracking-tight bg-transparent hover:bg-rose-100/90 dark:hover:bg-redShadow/40 hover:text-rose-800 border border-transparent hover:border-rose-200 dark:hover:border-customRed/50"
              >
                <Trash2 className="h-4 w-4 mr-1.5" />
                {queueActionLoading === 'clear_all' ? 'Clearing...' : 'Clear All'}
              </Button>
            </div>
            <Button
              variant="ghost"
              size="icon"
              onClick={onClose}
              className="h-9 w-9 rounded-xl text-slate-500 hover:text-slate-900 hover:bg-slate-100 dark:text-customGray dark:hover:text-text dark:hover:bg-darkBgHighlight/60"
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6">
        {queueStatus && (
          <div className="mb-4 rounded-xl border border-gray-200 dark:border-darkBgHighlight bg-gray-50 dark:bg-darkBgMid p-4">
            <div className="flex items-center justify-between gap-2">
              <div className="text-xs uppercase tracking-wide text-gray-600 dark:text-customGray">Queue Health</div>
              <div className="text-[11px] text-gray-500 dark:text-customGray">{queueLastUpdatedLabel}</div>
            </div>
            <div className="mt-3 grid grid-cols-2 gap-2 text-sm">
              <div className="rounded-lg border border-orange-200 dark:border-customYellow px-2 py-1.5 bg-white/70 dark:bg-darkBg/30">
                <div className="text-[11px] text-gray-600 dark:text-customGray">Queued</div>
                <div className="font-semibold text-orange-700 dark:text-yellowHighlight">{queueStatus.pending}</div>
              </div>
              <div className="rounded-lg border border-blue-200 dark:border-customBlue px-2 py-1.5 bg-white/70 dark:bg-darkBg/30">
                <div className="text-[11px] text-gray-600 dark:text-customGray">Running</div>
                <div className="font-semibold text-blue-700 dark:text-blueHighlight">{queueStatus.running}</div>
              </div>
              <div className="rounded-lg border border-yellow-200 dark:border-customYellow px-2 py-1.5 bg-white/70 dark:bg-darkBg/30">
                <div className="text-[11px] text-gray-600 dark:text-customGray">Retries Waiting</div>
                <div className="font-semibold text-yellow-700 dark:text-yellowHighlight">{queueStatus.retry_scheduled}</div>
              </div>
              <div className="rounded-lg border border-red-200 dark:border-customRed px-2 py-1.5 bg-white/70 dark:bg-darkBg/30">
                <div className="text-[11px] text-gray-600 dark:text-customGray">Stale Running</div>
                <div className="font-semibold text-red-700 dark:text-redHighlight">{queueStatus.stale_running}</div>
              </div>
              <div className="rounded-lg border border-gray-200 dark:border-customGray px-2 py-1.5 bg-white/70 dark:bg-darkBg/30">
                <div className="text-[11px] text-gray-600 dark:text-customGray">Oldest Queue Age</div>
                <div className="font-semibold text-gray-800 dark:text-text">{formatAge(queueStatus.oldest_pending_age_seconds)}</div>
              </div>
              <div className="rounded-lg border border-gray-200 dark:border-customGray px-2 py-1.5 bg-white/70 dark:bg-darkBg/30">
                <div className="text-[11px] text-gray-600 dark:text-customGray">Longest Running</div>
                <div className="font-semibold text-gray-800 dark:text-text">{formatAge(queueStatus.longest_running_age_seconds)}</div>
              </div>
              <div className="col-span-2 rounded-lg border border-green-200 dark:border-customGreen px-2 py-1.5 bg-white/70 dark:bg-darkBg/30">
                <div className="text-[11px] text-gray-600 dark:text-customGray">Last Hour Throughput</div>
                <div className="font-semibold text-green-700 dark:text-greenHighlight">
                  {queueStatus.completed_last_hour} completed / {queueStatus.failed_last_hour} failed
                </div>
              </div>
            </div>
          </div>
        )}

        {!hasAnyJobs ? (
          <div className="text-center dark:text-customGray text-gray-500 py-8">
            No recent indexing jobs
          </div>
        ) : (
          <div className="space-y-4">
                         {/* Running Jobs - Stabilized to prevent frenetic behavior */}
             {(showRunningSection || jobCategories.running.length > 0) && (
               <div
                 className={`bg-blue-50 dark:bg-blueShadow rounded-xl border border-blue-200 dark:border-customBlue p-3 transition-all duration-500 ease-in-out transform ${
                   jobCategories.running.length > 0 ? 'opacity-100 scale-100' : 'opacity-60 scale-95'
                 }`}
               >
                 <button
                   className="w-full flex items-center justify-between font-semibold text-blue-700 dark:text-blueHighlight text-sm mb-2 border-b dark:border-customBlue border-blue-200 pb-2 bg-transparent hover:bg-white/60 dark:hover:bg-darkBgHighlight/40 rounded-md transition-colors duration-200 px-1"
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

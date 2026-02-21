import { invoke } from '@tauri-apps/api/tauri'

export interface SystemInfo {
  os: string
  os_version: string
  arch: string
  app_version: string
  rust_version: string
  memory_usage?: number
  disk_space?: number
  cpu_count?: number
  uptime?: number
}

export interface LogEntry {
  timestamp: string
  level: string
  message: string
  module?: string
  file?: string
  line?: number
  thread?: string
  session_id: string
}

export interface ErrorReportData {
  error_type: string
  error_message: string
  stack_trace?: string
  user_description?: string
  reproduction_steps?: string
  app_state?: string
}

export interface AppStateInfo {
  model_loaded: boolean
  indexed_count: number
  ffmpeg_available: boolean
  models_available: boolean
  app_version: string
  timestamp: string
  session_id: string
}

export interface UploadResult {
  upload_id: string
  artifact_path?: string
  timestamp: string
  file_size: number
  success: boolean
  error_message?: string
}

class ErrorReportingService {

  /**
   * Create a comprehensive error report
   */
  async createErrorReport(data: ErrorReportData): Promise<string> {
    try {
      const reportId = await invoke<string>('create_error_report', {
        errorType: data.error_type,
        errorMessage: data.error_message,
        stackTrace: data.stack_trace,
        userDescription: data.user_description,
        reproductionSteps: data.reproduction_steps,
        appState: data.app_state,
      })

      return reportId
    } catch (error) {
      console.error('❌ Failed to create error report:', error)
      throw new Error(`Failed to create error report: ${error}`)
    }
  }

  /**
   * Get system information for debugging
   */
  async getSystemInfo(): Promise<SystemInfo> {
    try {
      const systemInfo = await invoke<SystemInfo>('get_system_info')
      return systemInfo
    } catch (error) {
      console.error('❌ Failed to get system info:', error)
      throw new Error(`Failed to get system info: ${error}`)
    }
  }

  /**
   * Get recent log entries
   */
  async getRecentLogs(count: number = 50): Promise<LogEntry[]> {
    try {
      const logs = await invoke<LogEntry[]>('get_recent_logs', { count })
      return logs
    } catch (error) {
      console.error('❌ Failed to get recent logs:', error)
      throw new Error(`Failed to get recent logs: ${error}`)
    }
  }

  /**
   * Package logs for support team
   */
  async packageLogsForSupport(): Promise<string> {
    try {
      const packagePath = await invoke<string>('package_logs_for_support')
      return packagePath
    } catch (error) {
      console.error('❌ Failed to package logs:', error)
      throw new Error(`Failed to package logs: ${error}`)
    }
  }

  /**
   * Get current application state information
   */
  async getAppStateInfo(): Promise<AppStateInfo> {
    try {
      const appState = await invoke<AppStateInfo>('get_app_state_info')
      return appState
    } catch (error) {
      console.error('❌ Failed to get app state info:', error)
      throw new Error(`Failed to get app state info: ${error}`)
    }
  }

  /**
   * Create an error report from a JavaScript error
   */
  async reportJavaScriptError(
    error: Error,
    userDescription?: string,
    reproductionSteps?: string,
    additionalContext?: Record<string, any>
  ): Promise<string> {
    try {
      // Get current app state
      const appState = await this.getAppStateInfo()

      const errorData: ErrorReportData = {
        error_type: 'JavaScript Error',
        error_message: error.message,
        stack_trace: error.stack,
        user_description: userDescription,
        reproduction_steps: reproductionSteps,
        app_state: JSON.stringify({
          ...appState,
          additional_context: additionalContext,
          url: window.location.href,
          user_agent: navigator.userAgent,
        }, null, 2),
      }

      return await this.createErrorReport(errorData)
    } catch (reportError) {
      console.error('❌ Failed to report JavaScript error:', reportError)
      throw reportError
    }
  }

  /**
   * Create an error report for a backend/Tauri error
   */
  async reportBackendError(
    operation: string,
    error: string,
    userDescription?: string,
    reproductionSteps?: string
  ): Promise<string> {
    try {
      const appState = await this.getAppStateInfo()

      const errorData: ErrorReportData = {
        error_type: 'Backend Error',
        error_message: `Operation: ${operation}\nError: ${error}`,
        user_description: userDescription,
        reproduction_steps: reproductionSteps,
        app_state: JSON.stringify(appState, null, 2),
      }

      return await this.createErrorReport(errorData)
    } catch (reportError) {
      console.error('❌ Failed to report backend error:', reportError)
      throw reportError
    }
  }

  /**
   * Create a user-initiated bug report
   */
  async createUserBugReport(
    title: string,
    description: string,
    reproductionSteps?: string,
    expectedBehavior?: string,
    actualBehavior?: string
  ): Promise<string> {
    try {
      const appState = await this.getAppStateInfo()

      const errorMessage = [
        `Title: ${title}`,
        `Description: ${description}`,
        expectedBehavior ? `Expected Behavior: ${expectedBehavior}` : '',
        actualBehavior ? `Actual Behavior: ${actualBehavior}` : '',
      ].filter(Boolean).join('\n\n')

      const errorData: ErrorReportData = {
        error_type: 'User Bug Report',
        error_message: errorMessage,
        user_description: description,
        reproduction_steps: reproductionSteps,
        app_state: JSON.stringify(appState, null, 2),
      }

      return await this.createErrorReport(errorData)
    } catch (reportError) {
      console.error('❌ Failed to create user bug report:', reportError)
      throw reportError
    }
  }

  /**
   * Open the logs directory in the file explorer
   */
  async openLogsDirectory(): Promise<void> {
    try {
      const { shell } = await import('@tauri-apps/api')
      const logPath = await invoke<string>('get_log_file_path')
      const logDir = logPath.substring(0, logPath.lastIndexOf('/'))
      await shell.open(logDir)
    } catch (error) {
      console.error('❌ Failed to open logs directory:', error)
      throw new Error(`Failed to open logs directory: ${error}`)
    }
  }

  /**
   * Check if log upload is available
   */
  async isUploadAvailable(): Promise<boolean> {
    // Cosmos OSS runs fully offline; remote uploads are disabled.
    return false
  }

  /**
   * Create a simple text archive from the log package directory
   */
  private async createArchiveFromDirectory(directoryPath: string): Promise<string> {
    try {
      // Get list of files in the directory
      const directoryContents = await invoke<any[]>('list_directory_contents', { path: directoryPath })
      let combinedContent = `Cosmos Log Package\n`
      combinedContent += `Generated: ${new Date().toISOString()}\n`
      combinedContent += `Directory: ${directoryPath}\n`
      combinedContent += `${'='.repeat(80)}\n\n`

      // Read each file and add to combined content
      for (const item of directoryContents) {
        if (!item.is_dir) {
          try {
            const content = await invoke<string>('read_file_content', { path: item.path })
            combinedContent += `\n${'='.repeat(40)} ${item.name} ${'='.repeat(40)}\n`
            combinedContent += content
            combinedContent += `\n${'='.repeat(80)}\n`
          } catch (error) {
            console.warn(`Failed to read file ${item.path}:`, error)
            combinedContent += `\n${'='.repeat(40)} ${item.name} (ERROR) ${'='.repeat(40)}\n`
            combinedContent += `Error reading file: ${error}\n`
            combinedContent += `${'='.repeat(80)}\n`
          }
        }
      }

      // Convert to base64 for consistent handling
      const base64Content = btoa(unescape(encodeURIComponent(combinedContent)))
      return base64Content
    } catch (error) {
      console.error('❌ Failed to create archive from directory:', error)
      throw new Error(`Failed to create archive: ${error}`)
    }
  }

  /**
   * Convert file to base64 string
   */
  private async fileToBase64(filePath: string): Promise<string> {
    try {
      // Use the dedicated binary file reader for zip files and other binary content
      const content = await invoke<string>('read_file_as_base64', { path: filePath })
      return content
    } catch (error) {
      console.error('❌ Failed to convert file to base64:', error)
      throw new Error(`Failed to read file: ${error}`)
    }
  }

  /**
   * Upload logs with user consent
   */
  async uploadLogsWithConsent(
    packagePath: string,
    reportId: string,
    userEmail?: string,
    userDescription?: string,
    reproductionSteps?: string
  ): Promise<UploadResult> {
    return {
      upload_id: '',
      artifact_path: undefined,
      timestamp: new Date().toISOString(),
      file_size: 0,
      success: false,
      error_message: 'Log upload is disabled in Cosmos OSS.'
    }
  }

  /**
   * Create a bug report and optionally upload logs
   */
  async createBugReportWithUpload(
    title: string,
    description: string,
    reproductionSteps?: string,
    actualBehavior?: string,
    userEmail?: string,
    uploadLogs: boolean = false
  ): Promise<{ reportId: string; uploadResult?: UploadResult }> {
    try {
      // First create the bug report
      const reportId = await this.createUserBugReport(
        title,
        description,
        reproductionSteps,
        undefined, // expectedBehavior removed
        actualBehavior
      )

      let uploadResult: UploadResult | undefined

      // If user consented to upload logs
      if (uploadLogs) {
        uploadResult = await this.uploadLogsWithConsent('', reportId, userEmail, description, reproductionSteps)
      }

      return { reportId, uploadResult }
    } catch (error) {
      console.error('❌ Failed to create bug report with upload:', error)
      throw error
    }
  }

  /**
   * Format error information for display
   */
  formatErrorForDisplay(error: any): string {
    if (error instanceof Error) {
      return `${error.name}: ${error.message}`
    }

    if (typeof error === 'string') {
      return error
    }

    try {
      return JSON.stringify(error, null, 2)
    } catch {
      return String(error)
    }
  }

  /**
   * Check if error reporting is available
   */
  async isErrorReportingAvailable(): Promise<boolean> {
    try {
      await this.getSystemInfo()
      return true
    } catch {
      return false
    }
  }
}

// Export singleton instance
export const errorReportingService = new ErrorReportingService()
export default errorReportingService

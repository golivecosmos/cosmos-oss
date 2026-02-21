import { getVersion } from '@tauri-apps/api/app';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export interface UpdateInfo {
  version: string;
  date?: string;
  body?: string;
  available: boolean;
}

export interface UpdateCheckResult {
  hasUpdate: boolean;
  currentVersion: string;
  latestVersion?: string;
  updateInfo?: UpdateInfo;
  error?: string;
}

class UpdateService {
  private readonly TIMEOUT = 30000; // 30 seconds for update operations

  private isUpdaterNotConfiguredError(error: unknown): boolean {
    const message = error instanceof Error ? error.message : String(error);
    const normalized = message.toLowerCase();
    return (
      normalized.includes('updater does not have any endpoints set') ||
      (normalized.includes('updater') && normalized.includes('endpoints'))
    );
  }

  // Check if we're in development/staging environment
  private isDevelopmentBuild(): boolean {
    // Check if we're in development mode
    if (process.env.NODE_ENV === 'development') {
      return true;
    }

    // Check if version contains staging indicators
    const version = this.getCurrentVersionSync();
    return version.includes('staging') || version.includes('dev') || version.includes('alpha') || version.includes('beta');
  }

  // Synchronous version getter for internal use
  private getCurrentVersionSync(): string {
    try {
      // In development, we might not have access to Tauri APIs
      return process.env.NODE_ENV === 'development' ? '2.0.1-dev' : '2.0.1';
    } catch {
      return '2.0.1';
    }
  }

  // Get current version from Tauri app info
  private async getCurrentVersion(): Promise<string> {
    try {
      return await getVersion();
    } catch (error) {
      console.warn('Failed to get app version:', error);
      return this.getCurrentVersionSync();
    }
  }

  // Check for updates using Tauri's built-in updater
  async checkForUpdates(): Promise<UpdateCheckResult> {
    const currentVersion = await this.getCurrentVersion();

    try {
      console.log('🔍 Checking for updates using Tauri updater...');
      console.log('🔍 Current version:', currentVersion);
      console.log('🔍 Environment:', process.env.NODE_ENV);
      console.log('🔍 Is development build:', this.isDevelopmentBuild());

      const updateResult = await check({ timeout: this.TIMEOUT });

      console.log('📦 Update check result:', updateResult);

      if (updateResult) {
        const updateInfo: UpdateInfo = {
          version: updateResult.version || 'Unknown',
          date: updateResult.date,
          body: updateResult.body || 'New version available',
          available: updateResult.available ?? true,
        };

        return {
          hasUpdate: true,
          currentVersion,
          latestVersion: updateInfo.version,
          updateInfo,
        };
      } else {
        return {
          hasUpdate: false,
          currentVersion,
        };
      }

    } catch (error) {
      if (this.isUpdaterNotConfiguredError(error)) {
        console.info('ℹ️ Automatic updates are not configured for this build.');
        return {
          hasUpdate: false,
          currentVersion,
        };
      }

      console.error('❌ Update check failed:', error);

      // Special handling for signature errors in development builds
      const errorMessage = error instanceof Error ? error.message : String(error);
      if (errorMessage.toLowerCase().includes('signature') && this.isDevelopmentBuild()) {
        console.warn('⚠️ Signature verification failed in development build - this is expected');
        return {
          hasUpdate: false,
          currentVersion,
          error: 'Signature verification failed (expected in development mode). This is normal for development builds and doesn\'t affect app functionality.',
        };
      }

      return {
        hasUpdate: false,
        currentVersion,
        error: this.formatError(error),
      };
    }
  }

  // Install update and restart app
  async installAndRestart(): Promise<void> {
    try {
      console.log('📥 Starting update installation...');

      const update = await check({ timeout: this.TIMEOUT });
      if (!update) {
        throw new Error('No update available.');
      }

      await update.downloadAndInstall();

      console.log('✅ Update installed successfully, restarting app...');

      // Restart the application
      await relaunch();

    } catch (error) {
      if (this.isUpdaterNotConfiguredError(error)) {
        throw new Error('Automatic updates are not configured for this build.');
      }

      console.error('❌ Failed to install update:', error);
      throw new Error(this.formatError(error));
    }
  }

  // Format error messages for user display
  private formatError(error: any): string {
    if (error instanceof Error) {
      const message = error.message.toLowerCase();

      // Signature verification errors
      if (message.includes('verify signature') || message.includes('minisign') || message.includes('signature')) {
        if (this.isDevelopmentBuild()) {
          return 'Signature verification failed (expected in development mode). This is normal for development builds and doesn\'t affect app functionality.';
        }
        return 'Update signature verification failed. This may be a staging build or the update server is having issues. Please try again later.';
      }

      // Network-related errors
      if (message.includes('timeout')) {
        return 'Update check timed out. Please check your internet connection and try again.';
      }
      if (message.includes('network') || message.includes('fetch') || message.includes('connection')) {
        return 'Network error. Please check your internet connection and try again.';
      }

      // Permission errors
      if (message.includes('permission') || message.includes('access denied')) {
        return 'Permission denied. Please run as administrator and try again.';
      }

      // Download errors
      if (message.includes('download') || message.includes('404') || message.includes('not found')) {
        return 'Update download failed. The update may no longer be available.';
      }

      // Installation errors
      if (message.includes('install') || message.includes('extract')) {
        return 'Update installation failed. Please try downloading the update manually.';
      }

      // Generic error with original message
      return `Update failed: ${error.message}`;
    }

    return 'An unknown error occurred during the update process. Please try again or download the update manually.';
  }

  // Format date for display
  formatDate(dateString?: string): string {
    if (!dateString) return 'Unknown date';

    try {
      const date = new Date(dateString);
      return date.toLocaleDateString() + ' at ' + date.toLocaleTimeString();
    } catch {
      return dateString;
    }
  }

  async downloadUpdate(updateInfo: UpdateInfo): Promise<void> {
    console.warn('⚠️ downloadUpdate is deprecated. Use installAndRestart() instead.');
    await this.installAndRestart();
  }

  formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 Bytes';

    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));

    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
}

export const updateService = new UpdateService();

import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { 
  downloadDir, 
  documentDir, 
  desktopDir, 
  pictureDir, 
  videoDir, 
  audioDir 
} from '@tauri-apps/api/path';

export interface PermissionManager {
  checkDirectoryAccess: (path: string) => Promise<boolean>;
  requestDirectoryAccess: (path: string) => Promise<string | null>;
  hasStoredPermission: (path: string) => boolean;
  storePermission: (path: string) => void;
  getStoredPermissions: () => string[];
  isProtectedDirectory: (path: string) => Promise<boolean>;
  clearProtectedDirectoriesCache: () => void;
  getProtectedDirectoriesDebug: () => Promise<string[]>;
}

class PermissionManagerImpl implements PermissionManager {
  private readonly STORAGE_KEY = 'desktopDocsSelectedPaths';
  
  // Cache for protected directories to avoid repeated API calls
  private protectedDirsCache: string[] | null = null;

  /**
   * Check if we have stored permission for a directory
   */
  hasStoredPermission(path: string): boolean {
    const storedPaths = this.getStoredPermissions();
    return storedPaths.some(storedPath => {
      // Normalize paths to handle different path formats
      const normalizedPath = path.replace(/\/$/, '');
      const normalizedStoredPath = storedPath.replace(/\/$/, '');
      
      return normalizedPath.startsWith(normalizedStoredPath) || 
             normalizedStoredPath.startsWith(normalizedPath);
    });
  }

  /**
   * Get all stored permissions from localStorage
   */
  getStoredPermissions(): string[] {
    try {
      return JSON.parse(localStorage.getItem(this.STORAGE_KEY) || '[]');
    } catch {
      return [];
    }
  }

  /**
   * Store a new permission path
   */
  storePermission(path: string): void {
    const storedPaths = this.getStoredPermissions();
    if (!storedPaths.includes(path)) {
      storedPaths.push(path);
      localStorage.setItem(this.STORAGE_KEY, JSON.stringify(storedPaths));
      console.log(`[PermissionManager] Stored permission for: ${path}`);
    }
  }

  /**
   * Get protected directories using dynamic path resolution
   */
  private async getProtectedDirectories(): Promise<string[]> {
    if (this.protectedDirsCache) {
      return this.protectedDirsCache;
    }

    try {
      const [downloads, documents, desktop, pictures, videos, audio] = await Promise.all([
        downloadDir(),
        documentDir(),
        desktopDir(),
        pictureDir(),
        videoDir(),
        audioDir()
      ]);

      this.protectedDirsCache = [downloads, documents, desktop, pictures, videos, audio];
      console.log(`[PermissionManager] Protected directories resolved:`, this.protectedDirsCache);
      return this.protectedDirsCache;
    } catch (error) {
      console.error('Failed to get protected directories:', error);
      // Fallback to hardcoded paths if dynamic resolution fails
      return ['/Downloads', '/Documents', '/Desktop', '/Pictures', '/Movies', '/Music'];
    }
  }

  /**
   * Check if a path is a protected directory
   */
  async isProtectedDirectory(path: string): Promise<boolean> {
    const protectedDirs = await this.getProtectedDirectories();
    return protectedDirs.some(protectedDir => {
      // Normalize paths to handle different path formats
      const normalizedPath = path.replace(/\/$/, '');
      const normalizedProtectedDir = protectedDir.replace(/\/$/, '');
      
      return normalizedPath.startsWith(normalizedProtectedDir) || 
             normalizedProtectedDir.startsWith(normalizedPath);
    });
  }

  /**
   * Clear the protected directories cache (useful for testing or cache invalidation)
   */
  clearProtectedDirectoriesCache(): void {
    this.protectedDirsCache = null;
  }

  /**
   * Debug method to get current protected directories (for troubleshooting)
   */
  async getProtectedDirectoriesDebug(): Promise<string[]> {
    return await this.getProtectedDirectories();
  }

  /**
   * Test actual directory access by attempting to list it
   */
  async checkDirectoryAccess(path: string): Promise<boolean> {
    try {
      await invoke('list_directory', { path });
      return true;
    } catch (error) {
      const errorMessage = error as string;
      // Check if it's specifically a permission error
      return !(
        errorMessage.includes('Operation not permitted') ||
        errorMessage.includes('Permission denied') ||
        errorMessage.includes('Failed to read directory')
      );
    }
  }

  /**
   * Request directory access through system dialog
   */
  async requestDirectoryAccess(path: string): Promise<string | null> {
    try {
      const directoryName = path.split('/').pop() || 'folder';
      
      // Show user-friendly confirmation first
      const userWantsToGrant = confirm(
        `${directoryName} requires permission to access. This will open a folder picker where you can grant access.`
      );
      
      if (!userWantsToGrant) {
        return null;
      }

      const selected = await open({
        directory: true,
        multiple: false,
        title: `Grant access to ${directoryName}`,
        defaultPath: path
      });
      
      if (selected && typeof selected === 'string') {
        this.storePermission(selected);
        return selected;
      }
      
      return null;
    } catch (error) {
      console.error('Failed to request directory access:', error);
      return null;
    }
  }
}

// Export singleton instance
export const permissionManager: PermissionManager = new PermissionManagerImpl();

/**
 * Enhanced directory loading with automatic permission handling
 */
export async function loadDirectoryWithPermissions(path: string): Promise<any[]> {
  // Handle empty or invalid paths (e.g., unmounted drives)
  if (!path || path.trim() === '') {
    console.warn('Attempted to load directory with empty path');
    return [];
  }

  // First, try direct access
  const hasAccess = await permissionManager.checkDirectoryAccess(path);
  
  if (hasAccess) {
    // Direct access works, return the files
    return await invoke('list_directory', { path });
  }

  // Check if this is a protected directory
  const isProtected = await permissionManager.isProtectedDirectory(path);
  
  // Check if this might be an external drive path (often in /Volumes on macOS)
  const isExternalDrive = path.startsWith('/Volumes/') || path.includes('/media/') || path.includes('/mnt/');
  
  if (!isProtected && isExternalDrive) {
    // This is likely an unmounted or inaccessible external drive
    // Don't show permission dialog, just return empty
    console.warn(`External drive path not accessible: ${path}`);
    return [];
  }
  
  if (!isProtected) {
    // Not a protected directory and not an external drive, throw original error
    throw new Error(`Failed to access directory: ${path}`);
  }

  // Check if we have stored permission
  if (permissionManager.hasStoredPermission(path)) {
    // We should have permission but it's not working
    // This might be a dev vs prod build issue - request fresh permission
    console.warn(`Stored permission for ${path} not working, requesting fresh permission`);
  }

  // Request permission
  const grantedPath = await permissionManager.requestDirectoryAccess(path);
  
  if (grantedPath) {
    // Try with the granted path
    return await invoke('list_directory', { path: grantedPath });
  }

  // User denied permission or something went wrong
  throw new Error(`Permission denied for directory: ${path}`);
}

/**
 * Utility to ensure Downloads directory access specifically
 */
export async function ensureDownloadsAccess(): Promise<string | null> {
  // Get the actual Downloads path from Tauri
  const downloadsPath = await downloadDir();
  
  try {
    // Test if we already have access
    await invoke('list_directory', { path: downloadsPath });
    return downloadsPath;
  } catch {
    // Request access
    return await permissionManager.requestDirectoryAccess(downloadsPath);
  }
}

/**
 * Enhanced permission check that tests actual access
 */
export async function hasDirectoryAccess(path: string): Promise<boolean> {
  return await permissionManager.checkDirectoryAccess(path);
}

/**
 * Force refresh permissions for a directory (useful for dev vs prod issues)
 */
export async function refreshDirectoryPermission(path: string): Promise<string | null> {
  console.log(`Refreshing permission for: ${path}`);
  return await permissionManager.requestDirectoryAccess(path);
}
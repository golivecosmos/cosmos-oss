import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { 
  HardDrive, 
  Loader2, 
  AlertCircle, 
  CheckCircle, 
  RefreshCw,
  FolderPlus,
  Wifi,
  WifiOff
} from 'lucide-react';
import { Button } from './ui/button';

interface DriveInfo {
  uuid: string;
  name: string;
  mount_path: string;
  total_space: number;
  free_space: number;
  is_removable: boolean;
  last_seen: string;
  status: 'connected' | 'disconnected' | 'indexing' | 'error';
  indexed_files_count: number;
  total_size_indexed: number;
}

interface DriveItemProps {
  drive: DriveInfo;
  onIndex: (driveUuid: string) => void;
  onRefresh: () => void;
}

function DriveItem({ drive, onIndex, onRefresh }: DriveItemProps) {
  const [isIndexing, setIsIndexing] = useState(false);

  const getStatusIcon = () => {
    switch (drive.status) {
      case 'connected':
        return <HardDrive className="w-5 h-5 text-green-500" />;
      case 'disconnected':
        return <HardDrive className="w-5 h-5 text-gray-400" />;
      case 'indexing':
        return <Loader2 className="w-5 h-5 text-blue-500 animate-spin" />;
      case 'error':
        return <AlertCircle className="w-5 h-5 text-red-500" />;
      default:
        return <HardDrive className="w-5 h-5 text-gray-500" />;
    }
  };

  const getStatusColor = () => {
    switch (drive.status) {
      case 'connected': return 'border-green-200 bg-green-50';
      case 'disconnected': return 'border-gray-200 bg-gray-50';
      case 'indexing': return 'border-blue-200 bg-blue-50';
      case 'error': return 'border-red-200 bg-red-50';
      default: return 'border-gray-200';
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  const getUsagePercentage = () => {
    if (drive.total_space === 0) return 0;
    return ((drive.total_space - drive.free_space) / drive.total_space) * 100;
  };

  const handleIndex = async () => {
    setIsIndexing(true);
    try {
      await onIndex(drive.uuid);
    } finally {
      setIsIndexing(false);
    }
  };

  return (
    <div className={`p-4 rounded-lg border-2 transition-all duration-200 ${getStatusColor()}`}>
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center space-x-3">
          {getStatusIcon()}
          <div>
            <h3 className="font-semibold text-gray-900 truncate max-w-32">
              {drive.name}
            </h3>
            <p className="text-xs text-gray-500">
              {drive.status === 'disconnected' ? 'Offline' : drive.mount_path}
            </p>
          </div>
        </div>
        
        <div className="flex items-center space-x-1">
          {drive.status === 'connected' && (
            <Wifi className="w-4 h-4 text-green-500" />
          )}
          {drive.status === 'disconnected' && (
            <WifiOff className="w-4 h-4 text-gray-400" />
          )}
        </div>
      </div>

      {/* Storage info */}
      {drive.total_space > 0 && (
        <div className="mb-3">
          <div className="flex justify-between text-xs text-gray-600 mb-1">
            <span>{formatSize(drive.total_space - drive.free_space)} used</span>
            <span>{formatSize(drive.total_space)} total</span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2">
            <div 
              className="bg-blue-500 h-2 rounded-full transition-all duration-300"
              style={{ width: `${getUsagePercentage()}%` }}
            />
          </div>
        </div>
      )}

      {/* Index info */}
      <div className="mb-3">
        <p className="text-xs text-gray-600">
          {drive.indexed_files_count > 0 
            ? `${drive.indexed_files_count} files indexed`
            : 'No files indexed'
          }
        </p>
        {drive.total_size_indexed > 0 && (
          <p className="text-xs text-gray-500">
            {formatSize(drive.total_size_indexed)} indexed
          </p>
        )}
      </div>

      {/* Actions */}
      <div className="flex space-x-2">
        {drive.status === 'connected' && (
          <Button
            size="sm"
            onClick={handleIndex}
            disabled={isIndexing}
            className="flex-1 text-xs"
          >
            {isIndexing ? (
              <>
                <Loader2 className="w-3 h-3 mr-1 animate-spin" />
                Indexing...
              </>
            ) : (
              <>
                <FolderPlus className="w-3 h-3 mr-1" />
                Index Drive
              </>
            )}
          </Button>
        )}
        
        {drive.status === 'disconnected' && (
          <Button
            size="sm"
            variant="outline"
            onClick={onRefresh}
            className="flex-1 text-xs"
          >
            <RefreshCw className="w-3 h-3 mr-1" />
            Refresh
          </Button>
        )}
      </div>
    </div>
  );
}

export function DriveManager() {
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadDrives = async () => {
    try {
      setIsLoading(true);
      setError(null);
      const connectedDrives = await invoke<DriveInfo[]>('get_connected_drives');
      setDrives(connectedDrives);
    } catch (err) {
      console.error('Failed to load drives:', err);
      setError(err as string);
    } finally {
      setIsLoading(false);
    }
  };

  const refreshDrives = async () => {
    try {
      const refreshedDrives = await invoke<DriveInfo[]>('refresh_drives');
      setDrives(refreshedDrives);
    } catch (err) {
      console.error('Failed to refresh drives:', err);
      setError(err as string);
    }
  };

  const handleIndexDrive = async (driveUuid: string) => {
    try {
      // Update drive status to indexing
      setDrives(prev => prev.map(drive => 
        drive.uuid === driveUuid 
          ? { ...drive, status: 'indexing' as const }
          : drive
      ));

      // TODO: Implement drive indexing command
      // await invoke('index_drive', { driveUuid });
      
      console.log(`Starting indexing for drive: ${driveUuid}`);
      
      // For now, simulate indexing completion after 3 seconds
      setTimeout(() => {
        setDrives(prev => prev.map(drive => 
          drive.uuid === driveUuid 
            ? { ...drive, status: 'connected' as const, indexed_files_count: 42 }
            : drive
        ));
      }, 3000);
      
    } catch (err) {
      console.error('Failed to index drive:', err);
      setError(err as string);
      
      // Reset drive status on error
      setDrives(prev => prev.map(drive => 
        drive.uuid === driveUuid 
          ? { ...drive, status: 'connected' as const }
          : drive
      ));
    }
  };

  useEffect(() => {
    loadDrives();

    // Listen for drive events
    const setupEventListeners = async () => {
      const unlistenConnected = await listen<DriveInfo>('drive_connected', (event) => {
        console.log('Drive connected:', event.payload);
        setDrives(prev => {
          const existing = prev.find(d => d.uuid === event.payload.uuid);
          if (existing) {
            return prev.map(d => d.uuid === event.payload.uuid ? event.payload : d);
          } else {
            return [...prev, event.payload];
          }
        });
      });

      const unlistenDisconnected = await listen<{uuid: string, name: string}>('drive_disconnected', (event) => {
        console.log('Drive disconnected:', event.payload);
        setDrives(prev => prev.map(drive => 
          drive.uuid === event.payload.uuid 
            ? { ...drive, status: 'disconnected' as const }
            : drive
        ));
      });

      return () => {
        unlistenConnected();
        unlistenDisconnected();
      };
    };

    const cleanup = setupEventListeners();
    
    return () => {
      cleanup.then(fn => fn());
    };
  }, []);

  if (isLoading) {
    return (
      <div className="p-4">
        <div className="flex items-center space-x-2 text-gray-600">
          <Loader2 className="w-4 h-4 animate-spin" />
          <span>Loading external drives...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-4">
        <div className="flex items-center space-x-2 text-red-600 mb-2">
          <AlertCircle className="w-4 h-4" />
          <span className="text-sm">Failed to load drives</span>
        </div>
        <Button size="sm" onClick={loadDrives} variant="outline">
          <RefreshCw className="w-3 h-3 mr-1" />
          Retry
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-gray-900">External Drives</h2>
        <Button size="sm" onClick={refreshDrives} variant="outline">
          <RefreshCw className="w-3 h-3 mr-1" />
          Refresh
        </Button>
      </div>

      {drives.length === 0 ? (
        <div className="text-center py-8 text-gray-500">
          <HardDrive className="w-12 h-12 mx-auto mb-3 text-gray-300" />
          <p>No external drives detected</p>
          <p className="text-sm mt-1">Connect a drive and click Refresh</p>
        </div>
      ) : (
        <div className="grid gap-4">
          {drives.map((drive) => (
            <DriveItem
              key={drive.uuid}
              drive={drive}
              onIndex={handleIndexDrive}
              onRefresh={refreshDrives}
            />
          ))}
        </div>
      )}
    </div>
  );
}
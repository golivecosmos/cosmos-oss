import { useState, useEffect } from 'react';
import { getVersion } from '@tauri-apps/api/app';

export function useAppVersion() {
  const [version, setVersion] = useState<string>('');
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.warn('Failed to get app version:', error);
        // Fallback to package.json version
        setVersion('2.0.1');
      } finally {
        setIsLoading(false);
      }
    };

    fetchVersion();
  }, []);

  return { version, isLoading };
} 
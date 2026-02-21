import React, { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import {
  Store,
  X,
  Download,
  CheckCircle,
  Package,
  ExternalLink,
  Key,
  Settings as SettingsIcon,
  Eye,
  EyeOff,
  RefreshCw
} from 'lucide-react'
import { Button } from "./ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card"
import { Badge } from "./ui/badge"
import { Input } from "./ui/input"
import { Label } from "./ui/label"
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "./ui/dialog"

interface AppInfo {
  id: number;
  app_name: string;
  app_version: string;
  installed_at: string;
  updated_at: string;
  has_api_key: boolean;
  metadata?: {
    description?: string;
    icon?: string;
    requires_api_key?: boolean;
  };
}

interface AppInstallResponse {
  success: boolean;
  message: string;
}

interface AppStoreProps {
  isOpen: boolean;
  onClose: () => void;
}

export function AppStore({ isOpen, onClose }: AppStoreProps) {
  const [activeTab, setActiveTab] = useState('installed')
  const [installedApps, setInstalledApps] = useState<AppInfo[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  
  const [availableApps] = useState<AppInfo[]>([
    {
      id: 0,
      app_name: 'Google Gemini',
      app_version: '1.0.0',
      installed_at: '',
      updated_at: '',
      has_api_key: false,
      metadata: {
        description: 'Google\'s Gemini with Gemini 2.5 Flash and Veo3 for AI video generation',
        icon: '/google_gemini_logo.webp',
        requires_api_key: true
      }
    }
  ])
  
  // API Key modal state
  const [showApiKeyModal, setShowApiKeyModal] = useState(false)
  const [apiKey, setApiKey] = useState('')
  const [showApiKey, setShowApiKey] = useState(false)
  const [selectedApp, setSelectedApp] = useState<AppInfo | null>(null)

  // Uninstall confirmation modal state
  const [showUninstallModal, setShowUninstallModal] = useState(false)
  const [appToUninstall, setAppToUninstall] = useState<AppInfo | null>(null)

  // Load installed apps on component mount
  useEffect(() => {
    if (isOpen) {
      loadInstalledApps()
    }
  }, [isOpen])

  const loadInstalledApps = async () => {
    setLoading(true)
    setError(null)

    try {
      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(() => reject(new Error('Timeout loading apps')), 5000)
      })

      const appsPromise = invoke<AppInfo[]>('get_installed_apps')
      const apps = await Promise.race([appsPromise, timeoutPromise])

      setInstalledApps(apps)
    } catch (err) {
      console.error('Failed to load installed apps:', err)
      setError(`Failed to load installed apps: ${err instanceof Error ? err.message : 'Unknown error'}`)

      // Set empty array as fallback to prevent infinite loading
      setInstalledApps([])
    } finally {
      setLoading(false)
    }
  }

  const handleInstallApp = (app: AppInfo) => {
    // Don't allow installation if app is already installed
    if (isAppInstalled(app.app_name)) {
      return
    }

    setSelectedApp(app)
    setShowApiKeyModal(true)
  }

  const handleApiKeySubmit = async () => {
    if (!apiKey.trim() || !selectedApp) return

    setLoading(true)
    setError(null)

    try {
      const result = await invoke<AppInstallResponse>('install_app', {
        request: {
          app_name: selectedApp.app_name,
          app_version: selectedApp.app_version,
          api_key: apiKey.trim(),
          metadata: selectedApp.metadata
        }
      })

      if (result.success) {
        console.log('App installed successfully:', result.message)
        // Reload installed apps
        await loadInstalledApps()
        
        // Reset state
        setApiKey('')
        setShowApiKey(false)
        setShowApiKeyModal(false)
        setSelectedApp(null)
      } else {
        setError(result.message || 'Installation failed')
      }
    } catch (err) {
      console.error('Failed to install app:', err)
      setError(err instanceof Error ? err.message : 'Failed to install app')
    } finally {
      setLoading(false)
    }
  }

  const handleUninstallApp = (app: AppInfo) => {
    setAppToUninstall(app)
    setShowUninstallModal(true)
  }

  const confirmUninstall = async () => {
    if (!appToUninstall) return

    setLoading(true)
    setError(null)

    try {
      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(() => reject(new Error('Uninstall timeout')), 10000)
      })

      const uninstallPromise = invoke<AppInstallResponse>('uninstall_app', {
        appId: appToUninstall.id
      })

      const result = await Promise.race([uninstallPromise, timeoutPromise])

      if (result.success) {
        // Reload installed apps
        await loadInstalledApps()
        // Close modal
        setShowUninstallModal(false)
        setAppToUninstall(null)
      } else {
        setError(result.message || 'Uninstallation failed')
      }
    } catch (err) {
      console.error('Failed to uninstall app:', err)
      setError(`Failed to uninstall app: ${err instanceof Error ? err.message : 'Unknown error'}`)
    } finally {
      setLoading(false)
    }
  }

  // Check if an app is installed
  const isAppInstalled = (appName: string) => {
    return installedApps.some(app => app.app_name === appName)
  }

  // Get button text and state for an app
  const getAppButtonState = (app: AppInfo) => {
    if (loading) {
      return { text: 'Installing...', disabled: true }
    }

    if (isAppInstalled(app.app_name)) {
      return { text: `${app.app_name} installed`, disabled: true }
    }

    return { text: `Install ${app.app_name}`, disabled: false }
  }

  if (!isOpen) return null

  return (
    <>
      <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
        <div className="dark:bg-darkBg bg-white rounded-2xl shadow-2xl w-full max-w-4xl max-h-[90vh] overflow-hidden">
          {/* Header */}
          <div className="flex items-center justify-between p-6 border-b dark:border-darkBgHighlight border-gray-200">
            <div className="flex items-center">
              <div className="w-10 h-10 bg-gradient-to-r dark:from-customBlue from-blue-500 dark:to-blueShadow to-indigo-600 rounded-full flex items-center justify-center mr-3">
                <Store className="w-5 h-5 text-white" />
              </div>
              <div>
                <h2 className="text-xl font-bold dark:text-text text-gray-900">App Store</h2>
                <p className="text-sm dark:text-customGray text-gray-500">Install and manage AI apps</p>
              </div>
            </div>
            <div className="flex items-center space-x-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={loadInstalledApps}
                disabled={loading}
                className="dark:text-customGray dark:hover:text-blue-400 text-gray-400 hover:text-blue-600"
              >
                <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={onClose}
                className="dark:text-customGray dark:hover:text-red text-gray-400 hover:text-gray-600"
              >
                <X className="w-5 h-5" />
              </Button>
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-hidden">
            <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full">
              <div className="border-b dark:border-darkBgHighlight border-gray-200">
                <TabsList className="flex justify-between px-8">
                  <TabsTrigger value="installed" className="flex items-center">
                    <Package className="w-4 h-4 mr-2" />
                    Installed Apps
                  </TabsTrigger>
                  <TabsTrigger value="available" className="flex items-center">
                    <Download className="w-4 h-4 mr-2" />
                    Available Apps
                  </TabsTrigger>
                </TabsList>
              </div>

              <div className="p-6 overflow-y-auto max-h-[calc(90vh-200px)]">
                {/* Error Display */}
                {error && (
                  <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                    <p className="text-sm text-red-800 dark:text-red-200">{error}</p>
                  </div>
                )}

                {/* Installed Apps Tab */}
                <TabsContent value="installed" className="space-y-6">
                  <div>
                    {loading ? (
                      <div className="text-center py-12">
                        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto"></div>
                        <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">Loading apps...</p>
                      </div>
                    ) : installedApps.length === 0 ? (
                      <div className="text-center py-12">
                        <Package className="w-16 h-16 mx-auto mb-4 text-gray-400 dark:text-gray-500" />
                        <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                          No apps installed
                        </h3>
                        <p className="text-sm text-gray-600 dark:text-gray-400 mb-6">
                          Install one of our apps below to get started
                        </p>
                        <Button
                          onClick={() => setActiveTab('available')}
                          className="dark:bg-blueShadow bg-blue-500 hover:bg-blue-600 dark:hover:bg-customBlue"
                        >
                          <Download className="w-4 h-4 mr-2" />
                          Browse Available Apps
                        </Button>
                      </div>
                    ) : (
                      <div className="space-y-4">
                        {installedApps.map((app) => (
                          <Card key={app.id}>
                            <CardContent className="p-6">
                              <div className="flex items-center justify-between">
                                <div className="flex items-center space-x-4">
                                  {app.metadata?.icon?.startsWith('/') ? (
                                    <img 
                                      src={app.metadata.icon} 
                                      alt={`${app.app_name} logo`}
                                      className="w-16 h-12 object-contain"
                                    />
                                  ) : (
                                    <div className="w-12 h-12 bg-gray-100 dark:bg-gray-700 rounded-lg flex items-center justify-center">
                                      <span className="text-2xl">{app.metadata?.icon || '📦'}</span>
                                    </div>
                                  )}
                                  <div>
                                    <h3 className="font-medium text-gray-900 dark:text-gray-100">
                                      {app.app_name}
                                    </h3>
                                    <p className="text-sm text-gray-600 dark:text-gray-400">
                                      {app.metadata?.description || 'No description available'}
                                    </p>
                                    <div className="flex items-center space-x-2 mt-2">
                                      <Badge variant="secondary" className="text-xs">
                                        v{app.app_version}
                                      </Badge>
                                      <Badge variant="default" className="text-xs">
                                        <CheckCircle className="w-3 h-3 mr-1" />
                                        Installed
                                      </Badge>
                                      {app.has_api_key && (
                                        <Badge variant="outline" className="text-xs">
                                          <Key className="w-3 h-3 mr-1" />
                                          API Key
                                        </Badge>
                                      )}
                                    </div>
                                    <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                      Installed: {new Date(app.installed_at).toLocaleDateString()}
                                    </p>
                                  </div>
                                </div>
                                <div className="flex items-center space-x-2">
                                  <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={() => handleUninstallApp(app)}
                                    disabled={loading}
                                    className="dark:border-red-500 border-red-500 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
                                  >
                                    <X className="w-4 h-4 mr-2" />
                                    Uninstall
                                  </Button>
                                </div>
                              </div>
                            </CardContent>
                          </Card>
                        ))}
                      </div>
                    )}
                  </div>
                </TabsContent>

                {/* Available Apps Tab */}
                <TabsContent value="available" className="space-y-6">
                  <div>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                      {availableApps.map((app) => (
                        <Card key={app.id} className="hover:shadow-lg transition-shadow">
                          <CardHeader>
                            <div className="flex items-center space-x-3">
                              {app.metadata?.icon?.startsWith('/') ? (
                                <img
                                  src={app.metadata.icon}
                                  alt={`${app.app_name} logo`}
                                  className="w-14 h-14 object-contain"
                                />
                              ) : (
                                <div className="w-10 h-10 bg-gray-100 dark:bg-gray-700 rounded-lg flex items-center justify-center">
                                  <span className="text-xl">{app.metadata?.icon || '📦'}</span>
                                </div>
                              )}
                              <div>
                                <CardTitle className="text-lg">{app.app_name}</CardTitle>
                                <CardDescription>{app.metadata?.description}</CardDescription>
                              </div>
                            </div>
                          </CardHeader>
                          <CardContent className="space-y-4">
                            <div className="flex items-center justify-between">
                              <Badge variant="secondary" className="text-xs">
                                v{app.app_version}
                              </Badge>
                              <div className="flex items-center space-x-2">
                                {isAppInstalled(app.app_name) && (
                                  <Badge variant="default" className="text-xs">
                                    <CheckCircle className="w-3 h-3 mr-1" />
                                    Installed
                                  </Badge>
                                )}
                                <Badge variant="outline" className="text-xs">
                                  Free
                                </Badge>
                              </div>
                            </div>
                            
                            <div className="space-y-2">
                              <div className="flex items-center text-sm text-gray-600 dark:text-gray-400">
                                <Key className="w-4 h-4 mr-2" />
                                <span>Requires API key</span>
                              </div>
                              <div className="flex items-center text-sm text-gray-600 dark:text-gray-400">
                                <ExternalLink className="w-4 h-4 mr-2" />
                                <span>External service</span>
                              </div>
                            </div>

                            <Button
                              onClick={() => handleInstallApp(app)}
                              disabled={getAppButtonState(app).disabled}
                              className="w-full dark:bg-blueShadow bg-blue-500 hover:bg-blue-600 dark:hover:bg-customBlue disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                              <Download className="w-4 h-4 mr-2" />
                              {getAppButtonState(app).text}
                            </Button>
                          </CardContent>
                        </Card>
                      ))}
                    </div>
                  </div>
                </TabsContent>
              </div>
            </Tabs>
          </div>

          {/* Footer */}
          <div className="border-t dark:border-darkBgHighlight border-gray-200 p-4 dark:bg-darkBgMid bg-gray-50">
            <div className="flex items-center justify-between">
              <div className="text-xs dark:text-customGray text-gray-500">
                Apps require API keys to function
              </div>
              <div className="flex space-x-3">
                <Button variant="outline" className="dark:border-darkBgHighlight" onClick={onClose}>
                  Cancel
                </Button>
                <Button onClick={onClose} className="dark:bg-blueShadow">
                  Done
                </Button>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* API Key Input Modal */}
      <Dialog open={showApiKeyModal} onOpenChange={setShowApiKeyModal}>
        <DialogContent className="sm:max-w-md bg-bg dark:bg-darkBg">
          <DialogHeader>
            <DialogTitle className="flex items-center text-gray-900 dark:text-white">
              {selectedApp?.metadata?.icon?.startsWith('/') ? (
                <img
                  src={selectedApp.metadata.icon}
                  alt={`${selectedApp.app_name} logo`}
                  className="w-10 h-8 object-contain mr-3"
                />
              ) : (
                <div className="w-8 h-8 bg-gray-100 dark:bg-gray-700 rounded-lg flex items-center justify-center mr-3">
                  <span className="text-lg">{selectedApp?.metadata?.icon}</span>
                </div>
              )}
              Install {selectedApp?.app_name}
            </DialogTitle>
            <DialogDescription className="text-gray-700 dark:text-gray-300">
              Enter your API key to install {selectedApp?.app_name}. Your API key will be stored securely and used to authenticate with the service.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="api-key" className="text-gray-900 dark:text-white">API Key</Label>
              <div className="relative">
                <Input
                  id="api-key"
                  type={showApiKey ? "text" : "password"}
                  placeholder="Enter your API key"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  className="pr-10"
                  disabled={loading}
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="absolute right-0 top-0 h-full px-3 hover:bg-transparent"
                  onClick={() => setShowApiKey(!showApiKey)}
                  disabled={loading}
                >
                  {showApiKey ? (
                    <EyeOff className="h-4 w-4 text-gray-600 dark:text-gray-400" />
                  ) : (
                    <Eye className="h-4 w-4 text-gray-600 dark:text-gray-400" />
                  )}
                </Button>
              </div>
            </div>

            <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-3">
              <div className="flex items-start">
                <Key className="w-4 h-4 text-blue-600 dark:text-blue-400 mt-0.5 mr-2 flex-shrink-0" />
                <div className="text-sm text-blue-900 dark:text-blue-200">
                  <p className="font-medium mb-1">API Key Security</p>
                  <p className="text-blue-800 dark:text-blue-100">Your API key will be encrypted and stored locally on your device. Never share your API key with others.</p>
                </div>
              </div>
            </div>
          </div>
          
          <div className="flex justify-end space-x-2 pt-4">
            <Button
              variant="outline"
              onClick={() => {
                setShowApiKeyModal(false)
                setApiKey('')
                setShowApiKey(false)
                setSelectedApp(null)
              }}
              disabled={loading}
            >
              Cancel
            </Button>
            <Button
              onClick={handleApiKeySubmit}
              disabled={!apiKey.trim() || loading}
              className="dark:bg-blueShadow bg-blue-500 hover:bg-blue-600 dark:hover:bg-customBlue"
            >
              {loading ? 'Installing...' : 'Install App'}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Uninstall Confirmation Modal */}
      <Dialog open={showUninstallModal} onOpenChange={setShowUninstallModal}>
        <DialogContent className="sm:max-w-md bg-bg">
          <DialogHeader>
            <DialogTitle className="flex items-center text-gray-900 dark:text-white">
              <div className="w-8 h-8 bg-red-100 dark:bg-red-600 rounded-lg flex items-center justify-center text-lg mr-3">
                ⚠️
              </div>
              Uninstall {appToUninstall?.app_name}
            </DialogTitle>
            <DialogDescription className="text-gray-700 dark:text-gray-300">
              Are you sure you want to uninstall {appToUninstall?.app_name}? This action cannot be undone and all app data will be permanently deleted.
            </DialogDescription>
          </DialogHeader>

          <div className="bg-red-50 dark:bg-red-950 border border-red-200 dark:border-red-700 rounded-lg p-3">
            <div className="flex items-start">
              <X className="w-4 h-4 text-red-600 dark:text-red-400 mt-0.5 mr-2 flex-shrink-0" />
              <div className="text-sm text-red-800 dark:text-red-100">
                <p className="font-medium mb-1">Warning</p>
                <p>This will permanently remove the app and all its associated data from your system.</p>
              </div>
            </div>
          </div>

          <div className="flex justify-end space-x-2 pt-4">
            <Button
              variant="outline"
              onClick={() => {
                setShowUninstallModal(false)
                setAppToUninstall(null)
              }}
              disabled={loading}
              className="border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800"
            >
              Cancel
            </Button>
            <Button
              onClick={confirmUninstall}
              disabled={loading}
              className="bg-red-600 hover:bg-red-700 dark:bg-red-600 dark:hover:bg-red-700 text-white"
            >
              {loading ? 'Uninstalling...' : 'Uninstall'}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  )
} 

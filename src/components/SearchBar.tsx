import React, { useState, useRef, useEffect, useCallback } from 'react'
import { Search, Image, Clock, X, Bug, RefreshCw, Settings, ChevronDown, Target } from 'lucide-react'
import { Button } from './ui/button'
import { Input } from './ui/input'
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/tooltip'
import { invoke } from '@tauri-apps/api/core'
import { debug } from '../utils/debug'
import { ReferenceImagePanel } from './ReferenceImagePanel'

// Add interface for reference image data
export interface ReferenceImageData {
  url: string;
  name: string;
}

export type SearchBackend = 'sqlite';

export type SearchType = 'text' | 'visual' | 'tag' | 'recent';
export type SearchMode = 'text' | 'visual' | 'recent';

interface SearchBarProps {
  onSearch: (query: string, type: SearchType) => Promise<void>;
  onFileUpload?: (file: File) => void;
  isSearchDisabled?: boolean;
  onReferenceImageChange?: (image: ReferenceImageData | null) => void;
  onShowReferenceImageChange?: (show: boolean) => void;
  referenceImage?: ReferenceImageData | null;
  showReferenceImage?: boolean;
  results_count?: number;
  onOpenBenchmark?: () => void;
  onClearSearch?: () => void;
}

// **NEW: Recent search history management**
interface RecentSearch {
  query: string;
  type: 'text' | 'visual';
  timestamp: number;
  results_count?: number;
}

const RECENT_SEARCHES_KEY = 'cosmos-recent-searches';
const MAX_RECENT_SEARCHES = 10;

// Helper functions for recent searches
const getRecentSearches = (): RecentSearch[] => {
  try {
    const stored = localStorage.getItem(RECENT_SEARCHES_KEY);
    return stored ? JSON.parse(stored) : [];
  } catch {
    return [];
  }
};

const saveRecentSearch = (query: string, setRecentSearches: (newSearches: RecentSearch[]) => void, resultsCount?: number) => {
  const recent = getRecentSearches();
  const newSearch: RecentSearch = {
    query: query.trim(),
    type: 'text',
    timestamp: Date.now(),
    results_count: resultsCount
  };
  
  // Remove duplicates and add to front
  const filtered = recent.filter(s => s.query !== newSearch.query || s.type !== newSearch.type);
  const updated = [newSearch, ...filtered].slice(0, MAX_RECENT_SEARCHES);
  
  localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify(updated));
  if(setRecentSearches){
    setRecentSearches(updated)
  }
};

export const SearchBar: React.FC<SearchBarProps> = ({ 
  onSearch, 
  isSearchDisabled,
  onReferenceImageChange,
  onShowReferenceImageChange,
  referenceImage,
  showReferenceImage,
  onClearSearch
}) => {
  const [searchMode, setSearchMode] = useState<SearchMode>('text')
  const [searchQuery, setSearchQuery] = useState('')
  const [isSearching, setIsSearching] = useState(false)
  const [showRecentDropdown, setShowRecentDropdown] = useState(false)
  const [showDebugMenu, setShowDebugMenu] = useState(false)
  const [recentSearches, setRecentSearches] = useState<RecentSearch[]>([])
  const fileInputRef = useRef<HTMLInputElement>(null)
  const recentDropdownRef = useRef<HTMLDivElement>(null)
  const debugMenuRef = useRef<HTMLDivElement>(null)
  
  // Load recent searches on mount
  useEffect(() => {
    setRecentSearches(getRecentSearches());
  }, []);
  
  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (recentDropdownRef.current && !recentDropdownRef.current.contains(event.target as Node)) {
        setShowRecentDropdown(false);
      }
    };
    
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);
  
  useEffect(() => {
    if (onReferenceImageChange) {
      onReferenceImageChange(referenceImage);
    }
  }, [referenceImage, onReferenceImageChange]);
  
  const performSearch = useCallback(async (query: string, searchType?: SearchType, setRecentSearches?: (newSearches: RecentSearch[]) => void) => {
    const actualType = searchType || searchMode;
    if (!query.trim() || isSearchDisabled) {
        return;
    }

    try {
        setIsSearching(true);
        await onSearch(query.trim(), actualType);
        if(actualType === "text"){
          saveRecentSearch(query, setRecentSearches)
        }
    } catch (error) {
        console.error('Search error:', error);
    } finally {
        setIsSearching(false);
    }
  }, [searchMode, isSearchDisabled, onSearch]);
  
  const handleFileSelect = async (file: File) => {
    try {
      if (file.type.startsWith('image/')) {
        // Clear any existing reference image URL first
        if (referenceImage?.url) {
          URL.revokeObjectURL(referenceImage.url);
        }
        
        // Store reference image first using blob URL for display
        const newReferenceImage = {
          url: URL.createObjectURL(file),
          name: file.name
        };
        onReferenceImageChange && onReferenceImageChange(newReferenceImage);
        onShowReferenceImageChange && onShowReferenceImageChange(true);
        
        // Then update search UI and perform search
        setSearchQuery(`Visual search: ${file.name}`)
        setIsSearching(true)
        
        // Read file as base64 for search
        const reader = new FileReader();
        reader.onload = async (event) => {
          const base64Data = event.target?.result as string;
          
          // Strip the data URL prefix (e.g., "data:image/jpeg;base64,") to get just the base64 data
          const base64Only = base64Data.split(',')[1] || base64Data;
          
          // Pass the clean base64 data to the search handler with selected backend
          onSearch(base64Only, 'visual')
        };
        reader.readAsDataURL(file);
        
        // Dispatch custom event for tour
        window.dispatchEvent(new CustomEvent('visual-search-performed', {
          detail: { query: `Visual search: ${file.name}`, mode: 'visual' }
        }));
        
        setIsSearching(false);
      }
    } catch (error) {
      console.error('Error handling file:', error);
      if (referenceImage) {
        URL.revokeObjectURL(referenceImage.url);
        onReferenceImageChange && onReferenceImageChange(null);
        onShowReferenceImageChange && onShowReferenceImageChange(false);
      }
    }
  };

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    // If there's text input, prioritize text search
    if (searchQuery.trim()) {
      performSearch(searchQuery, 'text', setRecentSearches)
    } else if (referenceImage) {
      // Only use visual search if there's no text and there's a reference image
      performSearch(searchQuery, 'visual')
    }
  }

  const handleKeyPress = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      
      if (searchMode === 'recent') {
        // In recent mode, Enter should open the dropdown
        setShowRecentDropdown(true)
      } else {
        // If there's text input, prioritize text search
        if (searchQuery.trim()) {
          performSearch(searchQuery, 'text', setRecentSearches)
        } else if (referenceImage) {
          // Only use visual search if there's no text and there's a reference image
          performSearch(searchQuery, 'visual')
        }
      }
    }
  }

  const clearSearch = () => {
    setSearchQuery('')
    
    // Also clear reference image if in visual mode
    if (referenceImage) {
      URL.revokeObjectURL(referenceImage.url);
      onReferenceImageChange && onReferenceImageChange(null);
      onShowReferenceImageChange && onShowReferenceImageChange(false);
    }

    // Always call onClearSearch to refresh the preview area
    onClearSearch && onClearSearch();
  }

  const handleModeChange = (newMode: SearchMode) => {
    //Disables a search mode if the user reclicks it's trigger
    if(newMode === searchMode){
      newMode = 'text'
    }

    setSearchMode(newMode);
   
    if (newMode === 'visual') {
      setSearchQuery('');
      if (fileInputRef.current) {
        fileInputRef.current.click();
        console.log('Tour: File input click triggered');
      } else {
        console.error('Tour: fileInputRef.current is null');
      }
      setShowRecentDropdown(false);
    } else if (newMode === 'recent') {
      setSearchQuery('');
      setShowRecentDropdown(true);
    } else {
      setSearchQuery('');
      setShowRecentDropdown(false);
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value
    setSearchQuery(value)
    
    // Close recent dropdown when typing
    if (showRecentDropdown) {
      setShowRecentDropdown(false)
    }
  }

  const handleRecentSearchSelect = (recentSearch: RecentSearch) => {
    setSearchQuery(recentSearch.query)
    setSearchMode(recentSearch.type)
    performSearch(recentSearch.query, recentSearch.type)
  }

  const clearRecentSearches = () => {
    localStorage.removeItem(RECENT_SEARCHES_KEY)
    setRecentSearches([])
    setShowRecentDropdown(false)
  }

  const formatTimestamp = (timestamp: number) => {
    const now = Date.now()
    const diff = now - timestamp
    const minutes = Math.floor(diff / (1000 * 60))
    const hours = Math.floor(diff / (1000 * 60 * 60))
    const days = Math.floor(diff / (1000 * 60 * 60 * 24))
    
    if (minutes < 1) return 'Just now'
    if (minutes < 60) return `${minutes}m ago`
    if (hours < 24) return `${hours}h ago`
    return `${days}d ago`
  }

  const checkSearchStatus = async () => {
    try {
      debug.log('🔍 Checking search status...')
      const status = await invoke('check_search_status')
      debug.log('📊 Search Status:', status)
      alert(`Search Status:\n${JSON.stringify(status, null, 2)}`)
    } catch (error) {
      debug.error('❌ Failed to check search status:', error)
      alert(`Failed to check search status: ${error}`)
    }
  }

  const recreateSqliteVirtualTable = async () => {
    try {
      const result = await invoke('recreate_sqlite_virtual_table');
      console.log('🔧 Virtual table result:', result);
      alert('Virtual table recreated successfully');
    } catch (error) {
      console.error('Failed to recreate virtual table:', error);
      alert(`Failed to recreate virtual table: ${error}`);
    }
  };

  const debugModelStatus = async () => {
    try {
      const status = await invoke('debug_model_status')
      console.log('🤖 Model Status:', status)
      alert('Model status logged to console - check developer tools')
    } catch (error) {
      console.error('Failed to debug model status:', error)
      alert(`Failed to debug model status: ${error}`)
    }
  }

  const reloadModels = async () => {
    try {
      const result = await invoke('reload_models')
      console.log('🔄 Reload result:', result)
      alert('Models reloaded successfully')
    } catch (error) {
      console.error('Failed to reload models:', error)
      alert(`Failed to reload models: ${error}`)
    }
  }

  const handleFileInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    console.log('Tour: File input change event triggered');
    const file = e.target.files?.[0]
    if (file) {
      console.log('Tour: File selected:', file.name);
      handleFileSelect(file)
    } else {
      console.log('Tour: No file selected');
    }
    
    // Clear the file input
    e.target.value = '';
  };

  // Add click outside handler for debug menu
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (debugMenuRef.current && !debugMenuRef.current.contains(event.target as Node)) {
        setShowDebugMenu(false);
      }
    };
    
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <>
      <form onSubmit={handleSearch} className="flex items-center gap-3 px-6 bg-white dark:bg-darkBg dark:border-darkBgHighlight border-gray-200" data-tour="search-bar">
        <div className="flex-1 max-w-4xl relative">
          <div className="relative">
            <Input
              type="text"
              value={searchQuery}
              onChange={handleInputChange}
              onKeyPress={handleKeyPress}
              disabled={isSearchDisabled}
              placeholder={
                isSearchDisabled ? "Install AI models to enable search..." :
                searchQuery.length >= 2 ? "Press Enter to search with AI-powered semantic search" :
                searchMode === 'recent' ? "Select from recent searches below..." :
                "Type your search and press Enter..."
              }
              className={`h-12 pl-12 pr-24 text-base rounded-xl border-gray-200 dark:bg-darkBgHighlight dark:border-blueShadow bg-gray-50 dark:focus:bg-blueShadow focus:bg-white transition-all duration-200 ${
                isSearchDisabled ? 'opacity-50 cursor-not-allowed' : ''
              }`}
              readOnly={searchMode === 'recent'}
              onClick={() => {
                if (searchMode === 'recent') {
                  setShowRecentDropdown(true)
                }
              }}
            />
            
            {/* **SIMPLIFIED: Search icon with loading state** */}
            {isSearching ? (
              <RefreshCw className="absolute left-4 top-1/2 -translate-y-1/2 h-5 w-5 text-blue-500 dark:text-text animate-spin" />
            ) : (
              <Search className="absolute left-4 top-1/2 -translate-y-1/2 h-5 w-5" />
            )}
            
            {/* **NEW: Search button for explicit search trigger** */}
            <div className="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
              {searchQuery && searchMode !== 'recent' && (
                <button
                  type="button"
                  onClick={clearSearch}
                  className="h-8 w-8 flex items-center justify-center dark:text-text dark:hover:text-red dark:hover:bg-transparent text-gray-400 hover:text-gray-600 transition-colors rounded-md hover:bg-gray-100"
                >
                  <X className="h-4 w-4" />
                </button>
              )}
              
              {searchMode === 'recent' && (
                <button
                  type="button"
                  onClick={() => handleModeChange('text')}
                  className="h-8 w-8 flex items-center justify-center text-gray-400 hover:text-gray-600 transition-colors rounded-md hover:bg-gray-100"
                >
                  <ChevronDown className={`h-4 w-4 transition-transform ${!showRecentDropdown ? 'rotate-180' : ''}`} />
                </button>
              )}
            </div>
          </div>
          
          {/* **NEW: Recent searches dropdown** */}
          {searchMode === 'recent' && showRecentDropdown && (
            <div 
              ref={recentDropdownRef}
              className="absolute top-full left-0 right-0 mt-1 bg-white border dark:border-darkBgHighlight border-gray-200 rounded-lg shadow-lg z-50 max-h-80 overflow-y-auto"
            >
              {recentSearches.length > 0 ? (
                <>
                  <div className="p-3 border-b border-gray-100 flex justify-between items-center">
                    <span className="text-sm font-medium text-gray-700">Recent Searches</span>
                    <button
                      type="button"
                      onClick={clearRecentSearches}
                      className="text-xs text-gray-500 hover:text-red-600 transition-colors"
                    >
                      Clear All
                    </button>
                  </div>
                  {recentSearches.map((recent, index) => (
                    <button
                      key={index}
                      type="button"
                      onClick={() => handleRecentSearchSelect(recent)}
                      className="w-full p-3 text-left hover:bg-gray-50 transition-colors border-b border-gray-50 last:border-b-0"
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            {recent.type === 'text' && <Search className="h-3 w-3 text-gray-400 flex-shrink-0" />}
                            {recent.type === 'visual' && <Image className="h-3 w-3 text-gray-400 flex-shrink-0" />}
                            <span className="text-sm text-gray-900 truncate">{recent.query}</span>
                          </div>
                          <div className="flex items-center gap-2 mt-1">
                            <span className="text-xs text-gray-500">{formatTimestamp(recent.timestamp)}</span>
                            {recent.results_count !== undefined && (
                              <span className="text-xs text-gray-400">• {recent.results_count} results</span>
                            )}
                          </div>
                        </div>
                      </div>
                    </button>
                  ))}
                </>
              ) : (
                <div className="p-6 text-center dark:bg-darkBg text-gray-500">
                  <Clock className="h-8 w-8 mx-auto mb-2 dark:text-customBlue text-gray-300" />
                  <p className="text-sm dark:text-text">No recent searches yet</p>
                  <p className="text-xs dark:text-customGray text-gray-400 mt-1">Your search history will appear here</p>
                </div>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  variant={searchMode === 'visual' ? 'default' : 'ghost'}
                  size="sm"
                  onClick={() => handleModeChange('visual')}
                  disabled={isSearchDisabled}
                  className= {`h-8 px-3 rounded-md
                    ${searchMode === 'visual' && 'dark:hover:bg-darkBgHighlight dark:bg-darkBgHighlight hover:bg-gray-100 bg-gray-100'}
                  `}
                  data-tour="visual-search"
                >
                  <Image className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                {isSearchDisabled ? 'Install AI models to enable visual search' : 'Visual search (Upload image)'}
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  variant={searchMode === 'recent' ? 'default' : 'ghost'}
                  size="sm"
                  onClick={() => handleModeChange('recent')}
                  className= {`h-8 px-3 rounded-md
                    ${searchMode === 'recent' && 'dark:hover:bg-darkBgHighlight dark:bg-darkBgHighlight hover:bg-gray-100 bg-gray-100'}
                  `}
                >
                  <Clock className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Recent searches</TooltipContent>
            </Tooltip>
          </div>

          {/* Debug button and menu - only show in development */}
          {process.env.NODE_ENV === 'development' && (
            <div className="relative" ref={debugMenuRef}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => setShowDebugMenu(!showDebugMenu)}
                    className="h-8 px-3 rounded-md dark:border-customYellow border-green-200 dark:text-customYellow text-green-600 dark:hover:bg-customYellow/80 hover:bg-green-50"
                  >
                    <Bug className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Debug Tools</TooltipContent>
              </Tooltip>

              {showDebugMenu && (
                <div className="absolute right-0 mt-2 w-48 dark:bg-darkBg bg-white rounded-lg shadow-lg border dark:border-darkBgHighlight border-gray-200 z-50">
                  <div className="p-2">
                    <button
                      type="button"
                      onClick={checkSearchStatus}
                      className="w-full px-3 py-2 text-left text-sm dark:text-text text-gray-700 hover:bg-gray-100 rounded-md flex items-center gap-2"
                    >
                      <Bug className="h-4 w-4 dark:text-customYellow text-green-600" />
                      Check Search Status
                    </button>
                    <button
                      type="button"
                      onClick={recreateSqliteVirtualTable}
                      className="w-full px-3 py-2 text-left text-sm dark:text-text text-gray-700 hover:bg-gray-100 rounded-md flex items-center gap-2"
                    >
                      <Target className="h-4 w-4 dark:text-customBlue text-blue-600" />
                      Recreate Virtual Table
                    </button>
                    <button
                      type="button"
                      onClick={debugModelStatus}
                      className="w-full px-3 py-2 text-left text-sm dark:text-text text-gray-700 hover:bg-gray-100 rounded-md flex items-center gap-2"
                    >
                      <Settings className="h-4 w-4 dark:text-customBlue text-blue-600" />
                      Model Status
                    </button>
                    <button
                      type="button"
                      onClick={reloadModels}
                      className="w-full px-3 py-2 text-left text-sm dark:text-text text-gray-700 hover:bg-gray-100 rounded-md flex items-center gap-2"
                    >
                      <RefreshCw className="h-4 w-4 dark:text-customYellow text-green-600" />
                      Reload Models
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* File input */}
        <input
          ref={fileInputRef}
          type="file"
          className="hidden"
          accept="image/*"
          onChange={handleFileInputChange}
        />
      </form>
    </>
  )
} 
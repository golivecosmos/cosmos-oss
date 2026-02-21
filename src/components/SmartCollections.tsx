import React from 'react'
import { Calendar, Image, FileText, Users, Palette } from 'lucide-react'
import { cn } from '../lib/utils'

interface Collection {
  id: string;
  name: string;
  type: 'time' | 'content' | 'faces' | 'colors' | 'similar' | 'index';
  count: number;
  icon: React.ElementType;
  preview?: string[];
}

interface SmartCollectionsProps {
  collections: Collection[];
  onCollectionSelect: (collection: Collection) => void;
  selectedCollection?: string;
  layout?: 'vertical' | 'horizontal';
}

export function SmartCollections({ 
  collections, 
  onCollectionSelect,
  selectedCollection,
  layout = 'vertical'
}: SmartCollectionsProps) {
  if (layout === 'horizontal') {
    return (
      <div className="flex items-center justify-between" data-tour="collections">
        <div>
          <h2 className="text-lg font-semibold text-gray-900">Smart Collections</h2>
          <p className="text-sm text-gray-500">Browse your indexed content</p>
        </div>
        
        <div className="flex items-center gap-3">
          {collections.map((collection) => (
            <button
              key={collection.id}
              className={cn(
                "flex items-center gap-3 px-4 py-2 rounded-lg border transition-all duration-200",
                selectedCollection === collection.id
                  ? "bg-blue-50 border-blue-200 text-blue-700"
                  : "hover:bg-gray-50 border-gray-200 hover:border-gray-300 text-gray-700"
              )}
              onClick={() => onCollectionSelect(collection)}
            >
              <div className={cn(
                "flex items-center justify-center w-8 h-8 rounded-md transition-colors",
                selectedCollection === collection.id
                  ? "bg-blue-100 text-blue-600"
                  : "bg-gray-100 text-gray-600"
              )}>
                <collection.icon className="h-4 w-4" />
              </div>
              
              <div className="text-left">
                <div className="font-medium text-sm">
                  {collection.name}
                </div>
                <div className="text-xs text-gray-500">
                  {collection.count.toLocaleString()} {collection.count === 1 ? 'item' : 'items'}
                </div>
              </div>
            </button>
          ))}
        </div>
      </div>
    )
  }

  // Vertical layout (original)
  return (
    <div className="p-6 bg-white h-full" data-tour="collections">
      <div className="space-y-6">
        <div>
          <h2 className="text-lg font-semibold text-gray-900">Smart Collections</h2>
          <p className="text-sm text-gray-500 mt-1">Browse your indexed content</p>
        </div>
        
        <div className="space-y-3">
          {collections.map((collection) => (
            <button
              key={collection.id}
              className={cn(
                "w-full flex items-center gap-4 p-4 rounded-xl border transition-all duration-200 text-left",
                selectedCollection === collection.id
                  ? "bg-blue-50 border-blue-200 shadow-sm"
                  : "hover:bg-gray-50 border-gray-200 hover:border-gray-300 hover:shadow-sm"
              )}
              onClick={() => onCollectionSelect(collection)}
            >
              <div className={cn(
                "flex items-center justify-center w-12 h-12 rounded-lg transition-colors",
                selectedCollection === collection.id
                  ? "bg-blue-100 text-blue-600"
                  : "bg-gray-100 text-gray-600"
              )}>
                <collection.icon className="h-6 w-6" />
              </div>
              
              <div className="flex-1 min-w-0">
                <h3 className="font-medium text-gray-900 truncate">
                  {collection.name}
                </h3>
                <p className="text-sm text-gray-500 mt-0.5">
                  {collection.count.toLocaleString()} {collection.count === 1 ? 'item' : 'items'}
                </p>
              </div>

              {collection.preview && collection.preview.length > 0 && (
                <div className="flex -space-x-2">
                  {collection.preview.slice(0, 3).map((url, i) => (
                    <div
                      key={i}
                      className="w-8 h-8 rounded-lg bg-gray-100 overflow-hidden border-2 border-white shadow-sm"
                    >
                      <img
                        src={url}
                        alt=""
                        className="w-full h-full object-cover"
                        onError={(e) => {
                          // Handle image load errors
                          const target = e.target as HTMLImageElement;
                          target.style.display = 'none';
                        }}
                      />
                    </div>
                  ))}
                </div>
              )}
            </button>
          ))}
        </div>
      </div>
    </div>
  )
} 
import React from 'react';
import { Search, X } from 'lucide-react';

interface ReferenceImageProps {
  imageUrl: string;
  imageName: string;
  onClose: () => void;
}

export function ReferenceImage({ imageUrl, imageName, onClose }: ReferenceImageProps) {
  return (
    <div className="flex justify-center w-full">
      <div className="group relative w-[480px] max-w-[90vw] bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden hover:shadow-lg hover:border-gray-200 transition-all duration-200 mb-6">
        <div className="px-3.5 py-2.5 border-b border-gray-100 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Search className="h-4 w-4 text-blue-500" />
            <div className="text-base font-normal text-gray-500">Search similar to ... </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-md hover:bg-gray-100 transition-colors"
            aria-label="Close reference image"
          >
            <X className="h-4 w-4 text-gray-400 hover:text-gray-600" />
          </button>
        </div>
        <div className="relative w-full h-[320px]">
          <img
            src={imageUrl}
            alt={imageName}
            className="w-full h-full object-contain bg-gray-50"
          />
        </div>
      </div>
    </div>
  );
} 
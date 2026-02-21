import React, { useState, useEffect } from 'react';
import { Search, X } from 'lucide-react';

interface ReferenceImagePanelProps {
  imageUrl: string;
  imageName: string;
  onClose: () => void;
}

function useImageDimensions(imageUrl: string) {
  const [dimensions, setDimensions] = useState<{ width: number; height: number } | null>(null);

  useEffect(() => {
    const img = new Image();
    img.src = imageUrl;
    img.onload = () => {
      setDimensions({
        width: img.naturalWidth,
        height: img.naturalHeight
      });
    };
  }, [imageUrl]);

  return dimensions;
}

export function ReferenceImagePanel({ imageUrl, imageName, onClose }: ReferenceImagePanelProps) {
  const dimensions = useImageDimensions(imageUrl);
  const [containerStyle, setContainerStyle] = useState({
    width: '320px',
    height: '240px'
  });

  useEffect(() => {
    if (!dimensions) return;

    const viewportHeight = window.innerHeight;
    const maxHeight = Math.min(600, viewportHeight * 0.7); // 70% of viewport height, max 600px
    const maxWidth = 320; // Fixed width for the panel

    const aspectRatio = dimensions.width / dimensions.height;
    
    if (aspectRatio > 1) {
      // Wide image
      const width = maxWidth;
      const height = Math.min(maxHeight, width / aspectRatio);
      setContainerStyle({
        width: `${width}px`,
        height: `${height}px`
      });
    } else {
      // Tall image
      const height = Math.min(maxHeight, maxWidth * (1/aspectRatio));
      setContainerStyle({
        width: `${maxWidth}px`,
        height: `${height}px`
      });
    }
  }, [dimensions]);

  return (
    <div className="flex justify-center w-full" data-tour="reference-image">
      <div className="fixed top-24 right-6 z-50 bg-white dark:bg-darkBg rounded-xl shadow-lg border dark:border-darkBgHighlight border-gray-200 overflow-hidden hover:shadow-xl transition-shadow duration-200" style={{ width: containerStyle.width }}>
        <div className="px-3.5 py-2.5 border-b dark:border-darkBg border-gray-100 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Search className="h-4 w-4 text-blue-500 dark:text-customBlue" />
            <div className="text-sm font-medium dark:text-customWhite text-gray-700 truncate">
              Reference Image
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-gray-400 dark:text-customGray dark:hover:text-customRed hover:text-gray-600 transition-colors"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
        <div 
          className="relative bg-gray-50 dark:bg-darkBg flex items-center justify-center"
          style={{ height: containerStyle.height }}
        >
          <img
            src={imageUrl}
            alt={imageName}
            className="max-w-full max-h-full object-contain"
          />
        </div>
        <div className="px-3.5 py-2.5 dark:bg-darkBg bg-gray-50 border-t dark:border-darkBgHighlight border-gray-100">
          <div className="text-xs dark:text-customGray text-gray-500 truncate">
            {imageName}
          </div>
        </div>
      </div>
    </div>
  );
} 
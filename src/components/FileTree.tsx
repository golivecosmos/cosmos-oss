import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { FolderClosed, File, ChevronRight, ChevronDown } from 'lucide-react';
import { cn } from '../lib/utils';

export interface FileItem {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: string;
  file_type: string;
  child_count?: number;
  children?: FileItem[];
}

interface FileTreeProps {
  initialPath: string;
  onSelect?: (file: FileItem) => void;
  selectedFile?: FileItem | null;
}

interface FileTreeNodeProps {
  name: string;
  path: string;
  isDirectory: boolean;
  level: number;
  onSelect?: (file: FileItem) => void;
  isSelected?: boolean;
  selectedFile?: FileItem | null;
}

const FileTreeNode: React.FC<FileTreeNodeProps> = ({ 
  name, 
  path, 
  isDirectory, 
  level,
  onSelect,
  isSelected,
  selectedFile
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [children, setChildren] = useState<FileItem[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const loadChildren = async () => {
    if (!isDirectory) return;
    
    setIsLoading(true);
    try {
      const items: FileItem[] = await invoke('list_directory', { path });
      setChildren(items);
    } catch (error) {
      console.error('Error loading directory:', error);
    }
    setIsLoading(false);
  };

  const handleClick = async () => {
    if (isDirectory) {
      const newIsOpen = !isOpen;
      setIsOpen(newIsOpen);
      if (newIsOpen) {
        await loadChildren();
      }
    }
    
    onSelect?.({
      name,
      path,
      is_dir: isDirectory,
      size: 0,
      modified: new Date().toISOString(),
      file_type: isDirectory ? "directory" : "file"
    });
  };

  return (
    <div style={{ paddingLeft: `${level * 12}px` }}>
      <div
        className={cn(
          "flex items-center gap-1 p-1 hover:bg-gray-100 rounded cursor-pointer",
          isSelected && "bg-blue-100 hover:bg-blue-200"
        )}
        onClick={handleClick}
      >
        {isDirectory ? (
          <>
            {isOpen ? (
              <ChevronDown className="h-4 w-4" />
            ) : (
              <ChevronRight className="h-4 w-4" />
            )}
            <FolderClosed className="h-4 w-4 text-blue-500" />
          </>
        ) : (
          <>
            <span className="w-4" />
            <File className="h-4 w-4 text-gray-500" />
          </>
        )}
        <span className="truncate">{name}</span>
      </div>
      
      {isOpen && isDirectory && (
        <div>
          {isLoading ? (
            <div className="pl-8 py-1 text-sm text-gray-500">Loading...</div>
          ) : (
            children.map((child) => (
              <FileTreeNode
                key={child.path}
                name={child.name}
                path={child.path}
                isDirectory={child.is_dir}
                level={level + 1}
                onSelect={onSelect}
                isSelected={child.path === selectedFile?.path}
                selectedFile={selectedFile}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
};

export const FileTree: React.FC<FileTreeProps> = ({ 
  initialPath, 
  onSelect,
  selectedFile 
}) => {
  const [rootItems, setRootItems] = useState<FileItem[]>([]);

  useEffect(() => {
    const loadRoot = async () => {
      try {
        const items: FileItem[] = await invoke('list_directory', { path: initialPath });
        setRootItems(items);
      } catch (error) {
        console.error('Error loading root directory:', error);
      }
    };
    loadRoot();
  }, [initialPath]);

  return (
    <div>
      {rootItems.map((item) => (
        <FileTreeNode
          key={item.path}
          name={item.name}
          path={item.path}
          isDirectory={item.is_dir}
          level={0}
          onSelect={onSelect}
          isSelected={selectedFile?.path === item.path}
          selectedFile={selectedFile}
        />
      ))}
    </div>
  );
}; 
import React from "react";
import { Button } from "../../ui/button";
import { Code, X } from "lucide-react";

interface JsonPromptModalProps {
  isOpen: boolean;
  jsonPrompt: string;
  onClose: () => void;
}

export const JsonPromptModal: React.FC<JsonPromptModalProps> = ({
  isOpen,
  jsonPrompt,
  onClose,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <div className="dark:bg-darkBg bg-white rounded-2xl shadow-2xl w-full max-w-4xl max-h-[90vh] overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b dark:border-darkBgHighlight border-gray-200">
          <div className="flex items-center">
            <div className="w-10 h-10 bg-gradient-to-r dark:from-purple-500 from-purple-500 dark:to-purple-600 to-purple-600 rounded-full flex items-center justify-center mr-3">
              <Code className="w-5 h-5 text-white" />
            </div>
            <div>
              <h2 className="text-xl font-bold dark:text-text text-gray-900">Generated JSON Prompt</h2>
              <p className="text-sm dark:text-customGray text-gray-500">The detailed prompt used for video generation</p>
            </div>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="dark:text-customGray dark:hover:text-red text-gray-400 hover:text-gray-600"
          >
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-hidden">
          <div className="p-6 overflow-y-auto max-h-[calc(90vh-200px)]">
            <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4">
              <pre className="text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap overflow-x-auto">
                {jsonPrompt}
              </pre>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="border-t dark:border-darkBgHighlight border-gray-200 p-4 dark:bg-darkBgMid bg-gray-50">
          <div className="flex items-center justify-between">
            <div className="text-xs dark:text-customGray text-gray-500">
              This JSON prompt was generated from your simple description
            </div>
            <div className="flex space-x-3">
              <Button variant="outline" className="dark:border-darkBgHighlight" onClick={onClose}>
                Close
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}; 
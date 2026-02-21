import React from "react";
import { Button } from "../../ui/button";
import { Sparkles } from "lucide-react";

interface PanelToggleProps {
  onOpen: () => void;
}

export const PanelToggle: React.FC<PanelToggleProps> = ({ onOpen }) => {
  return (
    <div className="absolute right-4 top-4 z-10">
      <Button
        variant="outline"
        size="sm"
        onClick={onOpen}
        className="bg-white dark:bg-transparent shadow-md"
      >
        <Sparkles className="w-4 h-4 mr-2" />
        Create Video
      </Button>
    </div>
  );
}; 
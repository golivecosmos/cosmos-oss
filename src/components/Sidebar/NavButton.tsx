import { createContext, useState, useContext } from "react";
import { NavLink, useMatch } from "react-router-dom";

import { cn } from "../../lib/utils";
import { ChevronDown, ChevronRight } from "lucide-react";

interface NavButtonContextType {
    expanded: boolean;
    setExpanded: (expanded: boolean) => void;
    isExpandable: boolean;
    to?: string;
    onClick?: () => void;
}

const NavButtonContext = createContext<NavButtonContextType | undefined>(undefined);

const useNavButton = () => {
    const context = useContext(NavButtonContext);
    if (!context) {
        throw new Error("useNavButton must be used within a NavButton component");
    }
    return context;
};

interface NavButtonProps {
    to?: string;
    onClick?: () => void;
    children: React.ReactNode;
    isExpandable?: boolean;
}

function NavButtonIcon({ className, icon }: { className: string, icon: React.ReactNode }) {
    return (
        <div className={cn(className)}>
            {icon}
        </div>
    );
}

function NavButtonLabel({ label, description }: { label: string, description?: string }) {
    return (
        <div className={
            cn(
                "flex flex-col items-start",
                description ? "" : "justify-center"
            )
        }>
            <span className="font-medium">{label}</span>
            {description ? <p className="text-xs text-gray-500 dark:text-gray-400" >{description}</p> : null}
        </div>
    );
}

function NavButtonExtendedContent({ children }: { children: React.ReactNode }) {
    return (
        <div className="min-h-0 max-h-[400px] overflow-y-auto">
            {children}
        </div>
    );
}

function NavButtonTrigger({ children, ...props }: { children: React.ReactNode } & React.HTMLAttributes<HTMLElement>) {
    const { expanded, setExpanded, isExpandable, to, onClick } = useNavButton();

    const Component = isExpandable ? "button" : NavLink;
    const isActive = isExpandable ? false : useMatch({ path: to ?? "", end: true });

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' || e.key === ' ') {
            if (isExpandable) {
                // For expandable buttons, prevent default and handle manually
                e.preventDefault();
                setExpanded(!expanded);
                onClick?.();
            } else {
                // For NavLink components, call onClick but let NavLink handle navigation
                onClick?.();
                // Don't prevent default - let NavLink handle the Enter key navigation
            }
        }
    };

    const handleClick = () => {
        if (isExpandable) {
            setExpanded(!expanded);
        }
        onClick?.();
    };

    return (
        <Component
            to={isExpandable ? undefined : to}
            className={
                cn(
                    "grid w-full gap-2 text-sm font-medium text-gray-700 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100 transition-colors px-2 py-3 cursor-pointer",
                    "grid-cols-[28px_auto_1fr]",
                    "focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-inset focus:bg-blue-50 dark:focus:bg-blue-900/20 rounded-md",
                    "focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-inset",
                    isActive ? "dark:bg-blueShadow bg-blue-100 dark:text-blueHighlight text-blue-600" : ""
                )
            }
            onClick={handleClick}
            onKeyDown={handleKeyDown}
            tabIndex={0}
            role={isExpandable ? "button" : undefined}
            aria-expanded={isExpandable ? expanded : undefined}
            data-nav-button
            {...props}
        >
            {isExpandable ? (
                <>
                    <div className="flex items-center justify-center h-full">
                        {expanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
                    </div>
                    <div className="flex items-center gap-2">
                        {children}
                    </div>
                </>
            ) : (
                <>
                    <div />
                    <div className="flex items-center gap-2">
                        {children}
                    </div>
                </>
            )}
        </Component>
    );
}

function NavButton({
    to,
    onClick,
    children,
    isExpandable = false,
}: NavButtonProps) {
    const [expanded, setExpanded] = useState(false);

    const contextValue: NavButtonContextType = {
        expanded,
        setExpanded,
        isExpandable,
        to,
        onClick,
    };

    return (
        <NavButtonContext.Provider value={contextValue}>
            <div 
                className={cn(
                    "w-full overflow-hidden transition-all duration-300 ease-in-out grid",
                    expanded ? "grid-rows-[60px_1fr]" : "grid-rows-[60px_0fr]",
                )}
                data-expanded={expanded}
            >
                {children}
            </div>
        </NavButtonContext.Provider>    );
}

NavButton.Icon = NavButtonIcon;
NavButton.Label = NavButtonLabel;
NavButton.Trigger = NavButtonTrigger;
NavButton.ExtendedContent = NavButtonExtendedContent;

export { NavButton, useNavButton };
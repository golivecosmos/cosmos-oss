import { 
  Home,
  Folder,
  Star,
  Clock,
  Tags,
  Settings,
  Share2,
  FolderInput,
  Trash2
} from 'lucide-react'

export interface NavItem {
  id: string;
  title: string;
  href: string;
  icon: any;
  badge?: number;
}

export const workspaces: NavItem[] = [
  {
    id: 'home',
    title: 'Home',
    href: '/',
    icon: Home,
  },
]

export const libraries: NavItem[] = [
  {
    id: 'photos',
    title: 'Photos Library',
    href: '/library/photos',
    icon: FolderInput,
    badge: 23, // New items
  },
  {
    id: 'documents',
    title: 'Documents',
    href: '/library/documents',
    icon: Folder,
  }
]

export const tools: NavItem[] = [
  {
    id: 'settings',
    title: 'Settings',
    href: '/settings',
    icon: Settings,
  }
] 
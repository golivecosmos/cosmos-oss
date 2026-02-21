import { useState, useEffect } from 'react'
import { Button } from "../ui/button"
import { Check, Folder, Plus, LucideIcon, Download } from "lucide-react"
import { open } from '@tauri-apps/plugin-dialog'
import { homeDir, desktopDir, downloadDir } from '@tauri-apps/api/path'
import { permissionManager } from '../../utils/permissionManager'
import { toast } from 'sonner'

interface Folder {
    name: string,
    path: string,
    selected: boolean
    icon: LucideIcon
}

const AddFolderButton = ({ homeDirectory, folders, setFolders }) => {
    const [isSelecting, setIsSelecting] = useState(false)

    const handleSelectCustomFolder = async () => {
        if (!isSelecting) {
            setIsSelecting(true)
            try {
                const selected = await open({
                    directory: true,
                    multiple: false,
                    title: 'Choose a folder',
                    defaultPath: homeDirectory
                })

                if (selected && typeof selected === 'string') {
                    // Check if folder is already selected
                    const alreadySelected = folders.some(folder => folder.path === selected)

                    if (!alreadySelected) {
                        const folderName = selected.split('/').pop() || 'Custom Folder'
                        const newFolder: Folder = {
                            name: folderName,
                            path: selected,
                            selected: true,
                            icon: Folder,
                        }

                        setFolders(prev => [...prev, newFolder])
                    }
                }
            } catch (error) {
                toast.error('Failed to select folder. Please try again.')
            } finally {
                setIsSelecting(false)
            }
        }
    }

    return (
        <button onClick={handleSelectCustomFolder} className="border-2 border-blueShadow border-dashed content-box text-left relative flex flex-row items-center px-4 gap-2 bg-darkBg/50 rounded">
            <Plus className="h-8 w-8 text-customBlue relative z-20" />
            <div className="w-8 h-8 absolute z-10 rounded-full blur-md opacity-50 bg-customBlue" />
            <div className="flex flex-col w-5/6">
                <div className="text-customWhite text-2xl md:text-3xl !leading-none w-full"> Custom </div>
                <div className="text-customGray text-l md:text-xl !leading-none w-full"> Add your own folder here </div>
            </div>
        </button>
    )
}

const FolderButton = ({ folder, toggleSelect }: { folder: Folder, toggleSelect: () => void }) => {
    return (
        <button onClick={toggleSelect} className={`hover:bg-darkBgHighlight content-box text-left relative flex flex-row items-center px-4 gap-2 shadow-onbButton bg-darkBg rounded 
            ${(folder.selected) &&
            'border-2 border-blueShadow'
            }`}>
            <folder.icon className="h-8 w-8 relative z-20 text-customBlue" />
            <div className="w-8 h-8 absolute z-10 rounded-full blur-md opacity-50 bg-customBlue" />
            <div className="flex flex-col w-5/6">
                <div className="text-customWhite text-2xl md:text-3xl !leading-none truncate w-full"> {folder.name} </div>
                <div className="text-customGray text-l md:text-xl !leading-none truncate flex-1 w-full"> {folder.path} </div>
            </div>
        </button>
    )
}

const FolderMenu = ({ folders, setFolders }) => {
    const [isAllSelected, setIsAllSelected] = useState<boolean>(false)
    const [homeDirectory, setHomeDirectory] = useState<string>(' ')

    const toggleSelect = (index: number) => {
        setFolders(prev => {
            return prev.map((folder, i) => (
                i === index ?
                    { ...folder, selected: !folder.selected }
                    : folder
            ))
        })
    }

    const toggleSelectAll = (isSelected: boolean) => {
        setFolders(prev => {
            return prev.map((folder, _) => (
                { ...folder, selected: isSelected }
            ))
        })
    }

    useEffect(() => {
        //Gets any needed system directories and adds default folders to the menu
        const getSystemDirs = async () => {
            try {
                const [home, desktop, downloads] = await Promise.all([
                    homeDir(),
                    desktopDir(),
                    downloadDir(),
                ])

                setHomeDirectory(home)
                // Pre-populate with suggested system folders
                const suggestedFolders: Folder[] = [
                    {
                        name: 'Desktop',
                        path: desktop,
                        selected: true,
                        icon: Folder,
                    },
                    {
                        name: 'Downloads',
                        path: downloads,
                        selected: true,
                        icon: Download
                    },
                ]
                setFolders(suggestedFolders)
            } catch (error) {
                toast.error('Failed to get system directories. Please try again.')
            }
        }

        getSystemDirs()
    }, [])


    useEffect(() => {
        setIsAllSelected(folders.every(folder => folder.selected))
    }, [folders])

    return (
        <div className="flex flex-col overflow-hidden w-full" >
            <div className="flex flex-row py-4 md:py-8 items-center gap-4">
                <button onClick={() => toggleSelectAll(!isAllSelected)} className={
                    `aspect-square h-5 md:h-6 border-2 border-blueShadow rounded
                        ${isAllSelected ?
                        'bg-blueShadow' :
                        'bg-transparent'
                    }`
                }
                >
                    {isAllSelected && (
                        <Check className="w-full h-full text-customWhite" />
                    )}
                </button>
                <div className="text-customWhite text-l md:text-xl"> Select All </div>
            </div>

            <div className="rounded flex-1 overflow-y-auto gap-6 w-full grid grid-cols-[repeat(auto-fill,_300px)] md:grid-cols-[repeat(auto-fill,_360px)] auto-rows-[70px] md:auto-rows-[85px] pb-10">
                {folders.map((currFolder, index) => (
                    <FolderButton folder={currFolder} toggleSelect={() => toggleSelect(index)} key={index} />
                ))}
                <AddFolderButton folders={folders} setFolders={setFolders} homeDirectory={homeDirectory} />
            </div>
        </div>
    )
}

export function PermissionsPage({ toNextPage }) {
    const [folders, setFolders] = useState<Folder[]>([])
    const [error, setError] = useState('')
    const [isSaving, setIsSaving] = useState(false)

    const handlePermissionSetupComplete = (selectedPaths: string[]) => {
        selectedPaths.forEach(path => {
            permissionManager.storePermission(path)
        })
    }

    const onContinue = async () => {
        const selected = folders.filter(folder => folder.selected).map(folder => folder.path)

        if (selected.length === 0) {
            setError('Select at least one folder to continue.')
            return
        }

        setError('')
        setIsSaving(true)
        try {
            handlePermissionSetupComplete(selected)
            toNextPage()
        } finally {
            setIsSaving(false)
        }
    }

    return (
        <div className="flex flex-col items-start py-10 px-12 md:px-16 font-jockey relative h-screen w-screen bg-gradient-to-b from-darkBgHighlight to-darkBg gap-10">
            <div className="flex flex-col w-full">
                <div className="text-customWhite text-3xl md:text-6xl"> Permissions </div>
                <div className="max-w-64 md:max-w-onbParagraph leading-tight pt-2 md:pt-6 text-customGray text-l md:text-xl">
                    Choose the folders you'd like Cosmos to have access to.
                    You'll have control over what to use later. This just grants permission.
                </div>
                <FolderMenu folders={folders} setFolders={setFolders} />
            </div>


            <div className="w-full max-w-md">
                {error && (
                    <p className="text-red-400 text-sm mb-2">{error}</p>
                )}
            </div>
            <Button
                onClick={onContinue}
                disabled={isSaving}
                className="text-l md:text-xl rounded w-32 h-7 md:w-48 md:h-10 bg-gradient-to-r from-customBlue to-blueShadow text-customWhite shadow-onbButton origin-top-left transition-all duration-300 hover:scale-110 hover:from-blueShadow hover:to-blueShadow disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 absolute bottom-10 left-16"
            >
                {isSaving ? 'Saving...' : 'Continue'}
            </Button>
        </div>
    )
}

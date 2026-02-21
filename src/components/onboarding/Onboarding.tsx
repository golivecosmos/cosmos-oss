import { WelcomePage } from './WelcomePage'
import { PermissionsPage } from './PermissionsPage'
import { CompletionPage } from './CompletionPage'
import { useState } from 'react'

const pages = [WelcomePage, PermissionsPage, CompletionPage]

export function Onboarding({ onComplete }: { onComplete: () => void }) {
    const [activePage, setActivePage] = useState<number>(0)

    return (
        <div className={`flex flex-row fixed z-50 
            ${pages[activePage] === PermissionsPage && 'animate-toPerms'}
            ${pages[activePage] === CompletionPage && 'animate-toComplete'}
        `}>
            <WelcomePage toNextPage={() => setActivePage(activePage + 1)} />
            <PermissionsPage toNextPage={() => setActivePage(activePage + 1)} />
            <CompletionPage inView={pages[activePage] === CompletionPage} onComplete={onComplete} />
        </div>
    )
}
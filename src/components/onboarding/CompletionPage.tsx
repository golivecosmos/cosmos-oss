import { Check } from "lucide-react"

export function CompletionPage({inView, onComplete}){
    const handleAnimationEnd = (e) => {
        if(e.animationName === "fadeOut"){
            onComplete()
        }
    }

    return(
        <div className={`[animation-delay:2.5s] relative font-jockey h-screen w-screen flex flex-col justify-center items-center bg-gradient-to-b from-darkBgHighlight to-darkBg
                ${inView && 'animate-fadeOut'}
            `}
            onAnimationEnd={handleAnimationEnd}>
            <Check className = {`h-16 w-16 stroke-white
                ${inView && 'animate-drawCheck'}
            `}/>
            <div className = {`text-xl text-customWhite opacity-0 [animation-delay:1s] 
                ${inView && 'animate-floatIn'}
            `}> 
                Setup complete! 
            </div>
        </div>
    )
}
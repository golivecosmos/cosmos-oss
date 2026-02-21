import { Button } from "../ui/button"
import { LucideIcon, Search, Folder, Zap, Bot } from "lucide-react"

interface Feature {
    name: string,
    description: string,
    color: string,
    icon: LucideIcon
}

const features: Feature[] = [
    {
        name: "Smart Search",
        description: "Find anything instantly with AI-powered search across all your documents.",
        color: "rgb(var(--blue))",
        icon: Search
    },
    {
        name: "Visual Recognition",
        description: "Search inside images and PDFs - find text, objects, or similar visual content.",
        color: "rgb(var(--purple))",
        icon: Bot

    },
    {
        name: "Smart Organization",
        description: "Automatically organize files by content, date, or custom collections.",
        color: "rgb(var(--red))",
        icon: Folder
    },
    {
        name: "Lightning Fast",
        description: "Instant previews and lightning-fast search results across thousands of files.",
        color: "rgb(var(--yellow))",
        icon: Zap
    }
]

const FeatureDisplay = ({feature}: {feature:Feature}) => {
    return (
        <div className="flex flex-row-reverse gap-6 p-4 w-full max-w-featureDisplay transition-all duration-300 hover:-translate-y-1">
            <div className="flex-col flex-1 min-w-0">
                <div className="text-2xl leading-snug text-customWhite font-medium mb-2"> {feature.name} </div>
                <div className="text-lg leading-relaxed text-customGray"> {feature.description} </div>
            </div>

            <div className="aspect-square relative flex justify-center items-center overflow-visible flex-shrink-0">
                <feature.icon className="h-14 w-14 relative z-10" style={{color: feature.color}}/>
                <div className="w-14 h-14 absolute z-0 rounded-full blur-lg opacity-80 animate-pulse"
                     style={{backgroundColor: feature.color}}/>
                <div className="w-20 h-20 absolute z-0 rounded-full blur-xl opacity-40"
                     style={{backgroundColor: feature.color}}/>
            </div>
        </div>
    );
}

const ImageCarousel = () => {
    const mainImages = [
        "/asset_1.webp",
        "/asset_2.webp",
        "/asset_3.webp",
        "/asset_4.webp",
        "/asset_5.webp",
        "/asset_6.webp",
        "/asset_7.webp",
        "/asset_8.webp"
    ]

    const ImageSet = ({images, isBackground = false}) => {
        return (
            <div className="flex flex-row pl-4 gap-10">
                {images.map((img, index) => (
                    <div 
                        className={`
                            my-auto w-onbImgHero h-onbImgHero overflow-hidden group relative
                            ${isBackground ? 
                                'opacity-20 blur-sm' : 
                                'rounded-lg transition-all duration-300 hover:scale-105'
                            }
                        `}
                        style={{ 
                            boxShadow: isBackground ? 'none' : '0 20px 50px -10px rgba(0, 0, 0, 0.4)',
                            border: isBackground ? 'none' : '1px solid rgba(255, 255, 255, 0.1)'
                        }} 
                        key={index}
                    >
                        <img 
                            src={img} 
                            alt={`Asset ${index + 1}`}
                            className={`
                                w-full h-full object-cover
                                ${!isBackground ? 'transition-transform duration-300 group-hover:scale-110' : ''}
                            `}
                        />
                    </div>
                ))}
                {/* Duplicate for seamless loop */}
                {images.map((img, index) => (
                    <div 
                        className={`
                            my-auto w-onbImgHero h-onbImgHero overflow-hidden group relative
                            ${isBackground ? 
                                'opacity-20 blur-sm' : 
                                'rounded-lg transition-all duration-300 hover:scale-105'
                            }
                        `}
                        style={{ 
                            boxShadow: isBackground ? 'none' : '0 20px 50px -10px rgba(0, 0, 0, 0.4)',
                            border: isBackground ? 'none' : '1px solid rgba(255, 255, 255, 0.1)'
                        }} 
                        key={`dup-${index}`}
                    >
                        <img 
                            src={img} 
                            alt={`Asset ${index + 1}`}
                            className={`
                                w-full h-full object-cover
                                ${!isBackground ? 'transition-transform duration-300 group-hover:scale-110' : ''}
                            `}
                        />
                    </div>
                ))}
            </div>
        )
    }

    return (
        <div className="my-auto relative w-full flex flex-col items-start -space-y-20">
            {/* Main Carousel */}
            <div className="relative z-10">
                <div className="flex animate-scroll">
                    <ImageSet images={mainImages} />
                </div>
            </div>
            {/* Background Carousel */}
            <div className="relative z-0">
                <div className="flex animate-scrollSlow">
                    <ImageSet images={mainImages} isBackground={true} />
                </div>
            </div>
            {/* Fade Gradients */}
            <div className="absolute blur-xl !mt-0 left-0 top-0 z-20 w-1/2 h-full bg-gradient-to-r from-customBlue/30 to-customBlue/0 pointer-events-none"/>
            <div className="absolute blur-xl !mt-0 left-1/2 top-0 z-20 w-1/2 h-full bg-gradient-to-l from-customBlue/30 to-customBlue/0 pointer-events-none"/>
        </div>
    )
}

export function WelcomePage({toNextPage}){
    return (
        <div className="relative font-jockey h-screen w-screen flex flex-row justify-between bg-gradient-to-b from-darkBgHighlight to-darkBg overflow-hidden no-scrollbar">
            {/* Left Section - Main Content */}
            <div className="w-[60%] flex flex-col py-12 gap-8 relative z-10 overflow-hidden">
                <div className="flex flex-col items-start px-16">
                    <div className="text-7xl leading-none text-customWhite"> Welcome To </div>
                    <div className="text-5xl leading-none self-auto bg-gradient-to-r from-blue-400 to-blue-600 bg-clip-text text-transparent"> Cosmos </div>
                    <div className="text-xl leading-relaxed max-w-onbParagraph pt-4 text-customGray"> Say goodbye to long hours of aimlessly wandering through media. Cosmos does all the heavy lifting for you! </div> 
                </div>
                <Button onClick={() => toNextPage()} className="mx-16 text-xl rounded-lg w-48 h-12 bg-gradient-to-r from-blue-400 to-blue-600 text-customWhite shadow-onbButton transition-all duration-300 hover:scale-105 hover:shadow-xl">
                    Get Started
                </Button>
                <div className="flex-1 min-h-0 pt-16">
                    <ImageCarousel />
                </div>
            </div>

            {/* Right Section - Features */}
            <div className="w-[40%] flex flex-col shadow-onbSidebar relative z-20 bg-darkBg h-screen">
                {/* Scrollable Features Container */}
                <div className="flex-1 overflow-y-auto py-16 no-scrollbar flex flex-col justify-center">
                    <div className="w-full max-w-lg px-8 space-y-12 mx-auto">
                        {features.map((feature, index) => (
                            <FeatureDisplay feature={feature} key={index}/>
                        ))}
                    </div>
                </div>
            </div> 
        </div>
    )
}
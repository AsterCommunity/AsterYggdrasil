import type { ComponentProps, ComponentType } from "react";
import { FaDocker, FaGithub } from "react-icons/fa6";
import { LuBookOpenText, LuScale } from "react-icons/lu";
import {
	PiArrowCounterClockwise,
	PiArrowDown,
	PiArrowLeft,
	PiArrowRight,
	PiArrowSquareOut,
	PiArrowsClockwise,
	PiArrowsInCardinal,
	PiArrowsOutCardinal,
	PiArrowUp,
	PiArrowUUpLeft,
	PiBracketsCurly,
	PiBrowsers,
	PiCaretDown,
	PiCaretLeft,
	PiCaretRight,
	PiCaretUp,
	PiChartBar,
	PiCheck,
	PiClipboardText,
	PiClockCounterClockwise,
	PiCloud,
	PiCopy,
	PiCpu,
	PiDeviceMobile,
	PiDeviceTablet,
	PiDotsThree,
	PiDownloadSimple,
	PiEnvelopeSimple,
	PiEye,
	PiEyeSlash,
	PiFile,
	PiFileAudio,
	PiFileCode,
	PiFileImage,
	PiFilePlus,
	PiFileText,
	PiFileVideo,
	PiFileZip,
	PiFlag,
	PiFloppyDisk,
	PiFolder,
	PiFolderOpen,
	PiFolderPlus,
	PiGauge,
	PiGear,
	PiGlobe,
	PiGridFour,
	PiHardDrive,
	PiHouse,
	PiImages,
	PiInfo,
	PiKey,
	PiLaptop,
	PiLink,
	PiLinkSimple,
	PiList,
	PiListBullets,
	PiListChecks,
	PiLock,
	PiLockOpen,
	PiMagnifyingGlass,
	PiMagnifyingGlassMinus,
	PiMagnifyingGlassPlus,
	PiMinus,
	PiMonitor,
	PiMoon,
	PiMusicNotes,
	PiPause,
	PiPencilSimple,
	PiPlay,
	PiPlus,
	PiPower,
	PiPresentation,
	PiQueue,
	PiRepeat,
	PiRepeatOnce,
	PiScroll,
	PiShield,
	PiShuffle,
	PiSignIn,
	PiSignOut,
	PiSkipBack,
	PiSkipForward,
	PiSortAscending,
	PiSortDescending,
	PiSpeakerHigh,
	PiSpeakerSlash,
	PiSpinner,
	PiSun,
	PiTable,
	PiTrash,
	PiUploadSimple,
	PiUser,
	PiVinylRecord,
	PiWarning,
	PiWarningCircle,
	PiWifiHigh,
	PiWifiX,
	PiWrench,
	PiX,
} from "react-icons/pi";

export type IconName =
	| "ArrowCounterClockwise"
	| "ArrowDown"
	| "ArrowLeft"
	| "ArrowRight"
	| "ArrowSquareOut"
	| "ArrowUp"
	| "ArrowsInCardinal"
	| "ArrowsOutCardinal"
	| "BookOpenText"
	| "BracketsCurly"
	| "CaretDown"
	| "CaretLeft"
	| "CaretRight"
	| "CaretUp"
	| "ChartBar"
	| "Check"
	| "CircleAlert"
	| "ClipboardText"
	| "Clock"
	| "Cloud"
	| "Copy"
	| "Cpu"
	| "Browsers"
	| "DeviceMobile"
	| "DeviceTablet"
	| "Docker"
	| "DotsThree"
	| "Download"
	| "EnvelopeSimple"
	| "Eye"
	| "EyeSlash"
	| "File"
	| "FileAudio"
	| "FileCode"
	| "FileImage"
	| "FilePlus"
	| "FileText"
	| "FileVideo"
	| "FileZip"
	| "Flag"
	| "FloppyDisk"
	| "Folder"
	| "FolderOpen"
	| "FolderPlus"
	| "Gauge"
	| "Gear"
	| "Github"
	| "Globe"
	| "Grid"
	| "HardDrive"
	| "House"
	| "Info"
	| "Images"
	| "Key"
	| "Laptop"
	| "Link"
	| "LinkSimple"
	| "List"
	| "ListBullets"
	| "ListChecks"
	| "Lock"
	| "LockOpen"
	| "MagnifyingGlass"
	| "MagnifyingGlassMinus"
	| "MagnifyingGlassPlus"
	| "Monitor"
	| "Moon"
	| "Minus"
	| "Pause"
	| "MusicNotes"
	| "PencilSimple"
	| "Play"
	| "Plus"
	| "Power"
	| "Presentation"
	| "Queue"
	| "Refresh"
	| "Repeat"
	| "RepeatOnce"
	| "Scale"
	| "Scroll"
	| "Shield"
	| "SignIn"
	| "SignOut"
	| "Shuffle"
	| "SkipBack"
	| "SkipForward"
	| "SortAscending"
	| "SortDescending"
	| "SpeakerHigh"
	| "SpeakerSlash"
	| "Spinner"
	| "Sun"
	| "Table"
	| "Trash"
	| "Undo"
	| "Upload"
	| "User"
	| "VinylRecord"
	| "Warning"
	| "WifiHigh"
	| "WifiX"
	| "Wrench"
	| "X";

const iconMap: Record<IconName, ComponentType<{ className?: string }>> = {
	ArrowCounterClockwise: PiArrowCounterClockwise,
	ArrowDown: PiArrowDown,
	ArrowLeft: PiArrowLeft,
	ArrowRight: PiArrowRight,
	ArrowSquareOut: PiArrowSquareOut,
	ArrowUp: PiArrowUp,
	ArrowsInCardinal: PiArrowsInCardinal,
	ArrowsOutCardinal: PiArrowsOutCardinal,
	BookOpenText: LuBookOpenText,
	BracketsCurly: PiBracketsCurly,
	CaretDown: PiCaretDown,
	CaretLeft: PiCaretLeft,
	CaretRight: PiCaretRight,
	CaretUp: PiCaretUp,
	ChartBar: PiChartBar,
	Check: PiCheck,
	CircleAlert: PiWarningCircle,
	ClipboardText: PiClipboardText,
	Clock: PiClockCounterClockwise,
	Cloud: PiCloud,
	Copy: PiCopy,
	Cpu: PiCpu,
	Browsers: PiBrowsers,
	DeviceMobile: PiDeviceMobile,
	DeviceTablet: PiDeviceTablet,
	Docker: FaDocker,
	DotsThree: PiDotsThree,
	Download: PiDownloadSimple,
	EnvelopeSimple: PiEnvelopeSimple,
	Eye: PiEye,
	EyeSlash: PiEyeSlash,
	File: PiFile,
	FileAudio: PiFileAudio,
	FileCode: PiFileCode,
	FileImage: PiFileImage,
	FilePlus: PiFilePlus,
	FileText: PiFileText,
	FileVideo: PiFileVideo,
	FileZip: PiFileZip,
	Flag: PiFlag,
	FloppyDisk: PiFloppyDisk,
	Folder: PiFolder,
	FolderOpen: PiFolderOpen,
	FolderPlus: PiFolderPlus,
	Gauge: PiGauge,
	Gear: PiGear,
	Github: FaGithub,
	Globe: PiGlobe,
	Grid: PiGridFour,
	HardDrive: PiHardDrive,
	House: PiHouse,
	Info: PiInfo,
	Images: PiImages,
	Key: PiKey,
	Laptop: PiLaptop,
	Link: PiLink,
	LinkSimple: PiLinkSimple,
	List: PiList,
	ListBullets: PiListBullets,
	ListChecks: PiListChecks,
	Lock: PiLock,
	LockOpen: PiLockOpen,
	MagnifyingGlass: PiMagnifyingGlass,
	MagnifyingGlassMinus: PiMagnifyingGlassMinus,
	MagnifyingGlassPlus: PiMagnifyingGlassPlus,
	Monitor: PiMonitor,
	Moon: PiMoon,
	Minus: PiMinus,
	MusicNotes: PiMusicNotes,
	Pause: PiPause,
	PencilSimple: PiPencilSimple,
	Play: PiPlay,
	Plus: PiPlus,
	Power: PiPower,
	Presentation: PiPresentation,
	Queue: PiQueue,
	Refresh: PiArrowsClockwise,
	Repeat: PiRepeat,
	RepeatOnce: PiRepeatOnce,
	Scale: LuScale,
	Scroll: PiScroll,
	Shield: PiShield,
	SignIn: PiSignIn,
	SignOut: PiSignOut,
	Shuffle: PiShuffle,
	SkipBack: PiSkipBack,
	SkipForward: PiSkipForward,
	SortAscending: PiSortAscending,
	SortDescending: PiSortDescending,
	SpeakerHigh: PiSpeakerHigh,
	SpeakerSlash: PiSpeakerSlash,
	Spinner: PiSpinner,
	Sun: PiSun,
	Table: PiTable,
	Trash: PiTrash,
	Undo: PiArrowUUpLeft,
	Upload: PiUploadSimple,
	User: PiUser,
	VinylRecord: PiVinylRecord,
	Warning: PiWarning,
	WifiHigh: PiWifiHigh,
	WifiX: PiWifiX,
	Wrench: PiWrench,
	X: PiX,
};

export interface IconProps {
	name: IconName;
	className?: string;
}

/**
 * Single refresh glyph for the whole app. Business code must not reference
 * the react-icons glyph directly: refresh actions go through RefreshButton,
 * non-button consumers (error reason lists, etc.) use this constant.
 */
export const REFRESH_ICON_NAME = "Refresh" as const;

export function isIconName(value: string): value is IconName {
	return Object.hasOwn(iconMap, value);
}

export function Icon({
	name,
	className,
	...props
}: IconProps & ComponentProps<"svg">) {
	const IconComponent = iconMap[name];
	if (!IconComponent) return null;
	return <IconComponent className={className} {...props} />;
}

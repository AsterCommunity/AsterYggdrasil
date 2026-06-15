import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitepress";
import { withMermaid } from "vitepress-plugin-mermaid";

const __dirname = dirname(fileURLToPath(import.meta.url));

function getVersion(): string {
	try {
		const cargoPath = resolve(__dirname, "../../Cargo.toml");
		const content = readFileSync(cargoPath, "utf-8");
		const match = content.match(/^version\s*=\s*"([^"]+)"/m);
		return match ? match[1] : "unknown";
	} catch {
		return "unknown";
	}
}

const version = getVersion();

export default withMermaid(
	defineConfig({
		title: "AsterYggdrasil",
		description:
			"Self-hosted Minecraft skin site and Yggdrasil authentication server.",
		lang: "zh-CN",
		cleanUrls: true,
		lastUpdated: true,
		head: [
			["meta", { name: "theme-color", content: "#111827" }],
			["link", { rel: "icon", href: "/favicon.svg" }],
		],
		locales: {
			root: {
				label: "简体中文",
				lang: "zh-CN",
				title: "AsterYggdrasil",
				description: "自建 Minecraft 皮肤站与 Yggdrasil 认证服务器。",
				themeConfig: {
					outline: {
						label: "本页内容",
					},
					lastUpdated: {
						text: "最后更新",
					},
					docFooter: {
						prev: "上一页",
						next: "下一页",
					},
					darkModeSwitchLabel: "外观",
					darkModeSwitchTitle: "切换到深色主题",
					lightModeSwitchTitle: "切换到浅色主题",
					sidebarMenuLabel: "菜单",
					returnToTopLabel: "返回顶部",
					langMenuLabel: "切换语言",
					skipToContentLabel: "跳到内容",
					search: {
						provider: "local",
						options: {
							translations: {
								button: {
									buttonText: "搜索",
									buttonAriaLabel: "搜索",
								},
								modal: {
									displayDetails: "显示详细列表",
									resetButtonTitle: "清除搜索",
									backButtonTitle: "关闭搜索",
									noResultsText: "没有找到相关结果",
									footer: {
										selectText: "选择",
										selectKeyAriaLabel: "Enter",
										navigateText: "切换",
										navigateUpKeyAriaLabel: "向上",
										navigateDownKeyAriaLabel: "向下",
										closeText: "关闭",
										closeKeyAriaLabel: "Escape",
									},
								},
							},
						},
					},
					nav: [
						{ text: "首页", link: "/" },
						{ text: "关于", link: "/guide/about" },
						{ text: "用户手册", link: "/guide/user-guide" },
						{ text: "接入", link: "/guide/yggdrasil-api" },
						{ text: "部署", link: "/deployment/docker" },
						{ text: `v${version}`, link: "https://github.com/AsterCommunity/AsterYggdrasil" },
					],
					sidebar: [
						{
							text: "运行 AsterYggdrasil",
							items: [
								{ text: "概览", link: "/" },
								{ text: "关于", link: "/guide/about" },
								{ text: "快速开始", link: "/guide/getting-started" },
								{ text: "用户手册", link: "/guide/user-guide" },
								{ text: "Yggdrasil API", link: "/guide/yggdrasil-api" },
								{ text: "启动器登录", link: "/guide/launcher-login" },
								{ text: "玩家档案", link: "/guide/profiles" },
								{ text: "材质处理", link: "/guide/yggdrasil-textures" },
								{ text: "配置和密钥", link: "/guide/configuration" },
								{ text: "材质存储", link: "/guide/storage" },
								{ text: "审计与后台任务", link: "/guide/audit-tasks" },
							],
						},
						{
							text: "部署",
							items: [{ text: "Docker", link: "/deployment/docker" }],
						},
					],
				},
			},
			en: {
				label: "English",
				lang: "en-US",
				title: "AsterYggdrasil",
				description:
					"Self-hosted Minecraft skin site and Yggdrasil authentication server.",
				themeConfig: {
					nav: [
						{ text: "Home", link: "/en/" },
						{ text: "About", link: "/en/guide/about" },
						{ text: "User Guide", link: "/en/guide/user-guide" },
						{ text: "Integration", link: "/en/guide/yggdrasil-api" },
						{ text: "Deployment", link: "/en/deployment/docker" },
						{ text: `v${version}`, link: "https://github.com/AsterCommunity/AsterYggdrasil" },
					],
					sidebar: [
						{
							text: "Run AsterYggdrasil",
							items: [
								{ text: "Overview", link: "/en/" },
								{ text: "About", link: "/en/guide/about" },
								{ text: "Getting Started", link: "/en/guide/getting-started" },
								{ text: "User Guide", link: "/en/guide/user-guide" },
								{ text: "Yggdrasil API", link: "/en/guide/yggdrasil-api" },
								{ text: "Launcher Login", link: "/en/guide/launcher-login" },
								{ text: "Minecraft Profiles", link: "/en/guide/profiles" },
								{ text: "Textures", link: "/en/guide/yggdrasil-textures" },
								{ text: "Config and Keys", link: "/en/guide/configuration" },
								{ text: "Texture Storage", link: "/en/guide/storage" },
								{ text: "Audit and Tasks", link: "/en/guide/audit-tasks" },
							],
						},
						{
							text: "Deployment",
							items: [{ text: "Docker", link: "/en/deployment/docker" }],
						},
					],
				},
			},
		},
		themeConfig: {
			search: {
				provider: "local",
			},
			socialLinks: [
				{ icon: "github", link: "https://github.com/AsterCommunity/AsterYggdrasil" },
			],
			footer: {
				message: "Released under the MIT License.",
				copyright: "Copyright (c) AptS-1547",
			},
		},
	}),
);

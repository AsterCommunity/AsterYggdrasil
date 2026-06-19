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
						{ text: "快速开始", link: "/guide/getting-started" },
						{ text: "使用指南", link: "/guide/" },
						{ text: "接入", link: "/guide/launcher-setup" },
						{ text: "部署", link: "/deployment/" },
						{ text: "关于", link: "/guide/about" },
						{ text: `v${version}`, link: "https://github.com/AsterCommunity/AsterYggdrasil" },
					],
					sidebar: [
						{
							text: "开始",
							items: [
								{ text: "概览", link: "/" },
								{ text: "使用指南总览", link: "/guide/" },
								{ text: "快速开始", link: "/guide/getting-started" },
							],
						},
						{
							text: "玩家使用",
							items: [
								{ text: "用户手册", link: "/guide/user-guide" },
								{ text: "玩家档案", link: "/guide/profiles" },
								{ text: "材质处理", link: "/guide/yggdrasil-textures" },
								{ text: "常见问题速查", link: "/guide/faq" },
							],
						},
						{
							text: "接入协议",
							items: [
								{ text: "启动器填写", link: "/guide/launcher-setup" },
								{ text: "启动器登录", link: "/guide/launcher-login" },
								{ text: "Yggdrasil 转发", link: "/guide/yggdrasil-forwarding" },
								{ text: "Yggdrasil API", link: "/guide/yggdrasil-api" },
								{ text: "故障排查", link: "/guide/troubleshooting" },
							],
						},
						{
							text: "管理维护",
							items: [
								{ text: "管理员指南", link: "/guide/admin-guide" },
								{ text: "配置和密钥", link: "/guide/configuration" },
								{ text: "对象存储", link: "/guide/storage" },
								{ text: "审计与后台任务", link: "/guide/audit-tasks" },
							],
						},
						{
							text: "部署",
							items: [
								{ text: "部署总览", link: "/deployment/" },
								{ text: "Docker", link: "/deployment/docker" },
							],
						},
						{
							text: "项目参考",
							items: [
								{ text: "关于", link: "/guide/about" },
								{ text: "文档贡献说明", link: "/guide/docs-contributing" },
							],
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
						{ text: "Getting Started", link: "/en/guide/getting-started" },
						{ text: "Guides", link: "/en/guide/" },
						{ text: "Integration", link: "/en/guide/launcher-setup" },
						{ text: "Deployment", link: "/en/deployment/" },
						{ text: "About", link: "/en/guide/about" },
						{ text: `v${version}`, link: "https://github.com/AsterCommunity/AsterYggdrasil" },
					],
					sidebar: [
						{
							text: "Start",
							items: [
								{ text: "Overview", link: "/en/" },
								{ text: "Guides Overview", link: "/en/guide/" },
								{ text: "Getting Started", link: "/en/guide/getting-started" },
							],
						},
						{
							text: "Player Usage",
							items: [
								{ text: "User Guide", link: "/en/guide/user-guide" },
								{ text: "Minecraft Profiles", link: "/en/guide/profiles" },
								{ text: "Textures", link: "/en/guide/yggdrasil-textures" },
								{ text: "FAQ", link: "/en/guide/faq" },
							],
						},
						{
							text: "Protocol Integration",
							items: [
								{ text: "Launcher Setup", link: "/en/guide/launcher-setup" },
								{ text: "Launcher Login", link: "/en/guide/launcher-login" },
								{ text: "Yggdrasil Forwarding", link: "/en/guide/yggdrasil-forwarding" },
								{ text: "Yggdrasil API", link: "/en/guide/yggdrasil-api" },
								{ text: "Troubleshooting", link: "/en/guide/troubleshooting" },
							],
						},
						{
							text: "Admin Maintenance",
							items: [
								{ text: "Admin Guide", link: "/en/guide/admin-guide" },
								{ text: "Config and Keys", link: "/en/guide/configuration" },
								{ text: "Object Storage", link: "/en/guide/storage" },
								{ text: "Audit and Tasks", link: "/en/guide/audit-tasks" },
							],
						},
						{
							text: "Deployment",
							items: [
								{ text: "Deployment Overview", link: "/en/deployment/" },
								{ text: "Docker", link: "/en/deployment/docker" },
							],
						},
						{
							text: "Project Reference",
							items: [
								{ text: "About", link: "/en/guide/about" },
								{ text: "Docs Contributing", link: "/en/guide/docs-contributing" },
							],
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

import type { ReactDoctorConfig } from "react-doctor/api";

// react-doctor 门禁配置。模式沿用 AsterDrive：全局只关有充分理由的规则，
// 其余豁免一律 per-file、per-rule，且每条必须带理由。
// 运行：bunx react-doctor（质量链集成位置见 AGENTS.md / issue #54）。
export default {
	ignore: {
		// 全局关闭：无。当前没有值得全仓豁免的规则。
		rules: [],
		files: ["dist/**", "coverage/**", "test-results/**"],
		overrides: [
			{
				// dateTime.ts 的格式化入口以 locale 作为每次调用的参数，
				// Intl formatter 必须跟随调用参数构造，无法提升为模块级单例。
				files: ["src/lib/dateTime.ts"],
				rules: ["react-doctor/js-hoist-intl"],
			},
			{
				// 会话相对时间（“3 分钟前”）依赖 render 期当前时间；
				// 模块级 RelativeTimeFormat 已是提升形态，剩余命中来自刻意的 now 取值。
				files: ["src/components/settings/LoginDevicesSection.tsx"],
				rules: ["react-doctor/js-hoist-intl"],
			},
			{
				// footer 版权年份在 render 期取 new Date().getFullYear()。
				// 前端是纯 CSR Vite SPA（无 SSR 水合路径），不存在水合不匹配风险。
				files: ["src/components/layout/AppFooter.tsx"],
				rules: ["react-doctor/rendering-hydration-mismatch-time"],
			},
		],
	},
} satisfies ReactDoctorConfig;

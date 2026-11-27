# AsterYggdrasil Frontend Panel

React 前端面板，嵌入 Rust 二进制分发。这里服务的是 Minecraft 皮肤站、玩家档案、材质管理和 Yggdrasil/authlib-injector 接入，不是云盘项目。别把文件管理、分享、团队、回收站、WebDAV 之类旧模板概念带回来。

## 技术栈

- React 19 + TypeScript (`tsgo` native-preview)
- Vite 8 + Tailwind CSS 4
- shadcn/ui 风格组件，Base UI 底层
- zustand 5
- axios
- biome
- Vitest + Playwright

## 开发命令

```bash
bun install
bun run dev
bun run build
bun run check
bun run test
bun run test:e2e
```

## 当前路由边界

路由分组放在 `src/routes/`，不要继续往旧 `src/router/index.tsx` 里塞东西；旧文件只是兼容 re-export。

```text
/                         公开入口
/login                    登录
/register                 注册
/init                     初始化

/account                  用户工作区
/account/profiles         Minecraft 角色档案
/account/wardrobe         材质衣柜
/account/settings         个人设置

/admin                    管理后台入口
/admin/users              用户管理
/admin/users/:id          用户详情
/admin/minecraft-profiles/:uuid
/admin/external-auth
/admin/tasks
/admin/settings
/admin/audit
/admin/about
```

旧路径 `/dashboard/*` 和 `/app/*` 只作为兼容 redirect 保留，不要在新代码里继续引用。

## Shell 约定

- `components/shell/AppShell.tsx`: 唯一 shell，按 `scope`（`account` / `admin`）参数化，由 `ShellSidebar.tsx`、`ShellTopbar.tsx` 和 `shellNavigation.ts` 提供分区导航。
- `routes/guards/`: 路由守卫都在这里：`AuthenticatedGate`（登录）、`GuestOnlyGate`（未登录限定）、`AdminOnlyGate`（管理权限）、`InitGate`（初始化状态）、`RouteAccessState`（拒绝态提示）。不要新建 `src/router/*` 别名文件。
- `components/layout/AdminPageShell.tsx`: 管理页页面级容器（配合 `AdminPageHeader.tsx`、`AdminSurface.tsx`）。
- `components/layout/PublicEntryShell.tsx`: 公开入口和认证入口的共享壳。
- `components/layout/AppLayout.tsx`: 现阶段仍是认证区 frame，后续应继续拆小，不要再加新业务逻辑。

## TypeScript 规则

- `erasableSyntaxOnly: true`: 禁止 TS enum，用 `as const` 对象。
- `verbatimModuleSyntax: true`: 类型导入用 `import type`。
- biome 用 tab 缩进、double quote。
- 类型优先从 `@/types/api` 导入，不要直接改 `api.generated.ts`。

## 前端产品方向

- 公开入口说明这个站点如何接入 Yggdrasil/authlib-injector，但首屏不要做营销页堆砌。
- 用户区围绕玩家档案、材质上传/绑定、启动器接入、个人安全设置。
- 管理区围绕用户、玩家档案、材质、认证源、系统设置、任务和审计。
- 管理界面要可扫描、可重复操作、信息密度适中，不要装饰性卡片堆满屏。
- 图标走统一 `Icon` 组件，不手写 SVG，不用 emoji 当图标。

## UI/UX 规范

完整 UI/UX 规范见 [`frontend-uiux-guidelines.md`](./frontend-uiux-guidelines.md)。它定义两块内容，改前端 UI 前先读：

- **二分范式**：用户面（公开入口/账户区/衣柜/公开材质库）是玩家偏好型皮肤站（BlessingSkin 传统），`/admin/**` 是唯一 app 模式区；两个区域各有各的规则，不做一套通用 app 模板。
- 操作入口分类法、Dialog 分类、异步提交 ref 门闩、加载状态规则（区域 Skeleton / 动作 spinner）、动效档位表、色彩 token 与品牌固定色登记表、组件状态清单、合并前自查清单。

组件级静态门禁用 react-doctor，配置在 [`doctor.config.ts`](./doctor.config.ts)；全局规则不轻易关，per-file 豁免必须带理由。

以下是最容易踩的硬规则摘要，细节以 guidelines 为准：

- 用户侧页面优先复用 `AppLayout` / `.app-shell` 的背景体系，不要每个页面再铺一套独立大背景。
- 绿色/emerald 是 Minecraft/Yggdrasil 的强调色，不是大面积底色；暗色模式尤其要控制绿色面积。
- 页面要有明确主次：一个主 hero 或主任务区，少量入口区，安静的辅助信息区。不要为了“丰富”把所有内容都包成 card，同一区块内不卡片套卡片。
- 禁止在组件里写 `bg-[#hex]` / `text-[#hex]` 等硬编码颜色：跟随主题的表面/文字用主题 token，公开区用 `public-*` token，有意固定的品牌色必须收口成 CSS 变量并登记进 guidelines §5.2。
- 不要在同一元素上用 `dark:bg-[linear-gradient(...)]` / `dark:bg-[url(...)]` 切 `background-image`；需要亮暗不同的背景图/渐变时，用亮/暗两层元素做 opacity 交叉淡入，并挂 `data-theme-surface="public-backdrop"`。
- 加载状态：列表/区域用 Skeleton，按钮/动作用 spinner；不用居中 spinner 占满区域。
- 浮层用 `src/components/ui/` 的 Base UI 封装（Dialog/Popover/Menu/Select），不手写 portal/定位/Escape。
- 图标走统一 `Icon` 组件，不手写 SVG，不用 emoji 当图标；icon-only 按钮必须有 `aria-label`。
- 动效用于减少突兀感，不用于装饰。普通主题和状态过渡约 160-220ms，必须尊重 `prefers-reduced-motion`。
- 新增或修改中文文案时同步更新英文 i18n；英文不要机器直译，要保持同样的信息密度和语气。用户侧文案面向玩家工作流，不写成内部模块清单。

## API 和服务层

- 项目后台 API 使用统一 envelope，由 `services/http.ts` 解包。
- Yggdrasil 协议接口保持协议响应格式，不要套项目 envelope。
- 服务层按领域命名：`authService`、`externalAuthService`、`yggdrasilService`、`admin*Service`。
- 安全敏感字段不要进日志、toast 或可见错误文本。

## i18n

当前 i18n 仍是单 namespace 合并模式，文件在 `src/i18n/locales/{zh-CN,en-US}/`。用户工作区文案放在 `account.json`，不要再新增 `dashboard.json` 或 `dashboard.*` key。

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

### 整体视觉

- 用户侧页面优先复用 `AppLayout` / `.app-shell` 的背景体系，不要每个页面再铺一套独立大背景。只有确实需要建立页面主视觉时，才在局部 hero 内做背景。
- 绿色/emerald 是 Minecraft/Yggdrasil 的强调色，不是大面积底色。大面积背景和卡片优先使用 `background`、`card`、`muted`、`border` 等 token；绿色只用于图标、状态点、当前项、主按钮和关键 accent。
- 避免整页都变成绿黑同色系。暗色模式下尤其要控制绿色面积，否则页面会闷、重、层级糊。
- 页面要有明确主次：一个主 hero 或主任务区，少量入口区，安静的辅助信息区。不要让 hero、流程卡、快捷操作、审计记录都像同等重要的大卡片。
- 不要为了“丰富”把所有内容都包成 card。卡片只用于真正的分组、工具面板和重复项；同一区块内避免卡片套卡片。

### 用户工作台 `/account`

- `/account` 是玩家工作台，不是管理端仪表盘。可以参考管理端布局的清晰层级，但不要照搬“系统状态、趋势图、服务器指标”这类管理员信息。
- 顶部 hero 应承担欢迎和方向感，不展示多余统计。不要在 hero 里额外展示“角色档案数量”这类对用户行动帮助不大的指标。
- hero 可以使用 Minecraft 相关图片作为右侧背景，但必须是柔和的侧面渐变融合，不要出现硬切边界。桌面和手机端都保持“文字侧到图片侧”的横向渐变，不要手机端改成上下渐变。
- hero 图片只作为氛围和识别，不应压过文字；文字区域必须保持足够对比度和可读性。
- “下一步”这类流程提示应轻量展示，像流程导航而不是三张同权重大入口卡。具体操作说明留给对应工具区块。
- 快速操作是纯入口，视觉上要轻。不要在每个入口 icon 外再套一层带背景的小方块；默认使用裸 `Icon`，适当放大到 `size-5`。
- 近期活动来自用户审计记录，应作为安静的辅助信息展示。避免做成过重的状态面板，也不要让重复登录/刷新事件抢主视觉。
- `LauncherSetupCard` 在工作台里是工具区块，不要再放“管理角色档案”这类重复导航按钮。启动器接入里的说明、API Root、拖拽添加和服务端折叠区即可。

### 图标和按钮

- 默认使用 `Icon` 组件，按钮内图标按按钮尺寸匹配。入口列表、流程项、说明步骤里的图标优先裸露展示，不额外包背景 div。
- 只有在需要明确点击目标、状态徽章、头像/实体占位或视觉分组时，才给 icon 加圆角背景。
- 不要用 emoji 作为 UI 图标或标题装饰。文案里也尽量避免 emoji，除非产品明确要求。
- 图标尺寸要和信息密度匹配：普通行入口常用 `size-5`，大型拖拽/空状态图标可以用 `size-8` 或更大。

### 主题和动效

- 主题切换参考 AsterDrive 的方式：使用 `theme-switching` 和 `data-theme-surface` 做柔和过渡，尊重 `prefers-reduced-motion`。
- 会随主题变化的 shell、topbar、sidebar、panel、overlay、control 要补 `data-theme-surface`，否则会硬切。
- 不要直接在同一个元素上用完全不同的 `dark:bg-[linear-gradient(...)]` / `dark:bg-[url(...)]` 切 `background-image`；这类背景无法平滑插值。需要切换图片或复杂渐变时，用亮/暗两层元素做 opacity 交叉淡入。
- 动效用于减少突兀感，不用于装饰。普通主题和状态过渡控制在约 160-220ms，避免夸张弹跳。

### 文案和 i18n

- 用户侧文案要面向玩家工作流，不要写成内部模块清单。例如描述应表达“准备角色档案和材质，然后进入游戏”，而不是“角色、材质、启动器和会话”。
- 首页/工作台的短描述要短，具体步骤放到对应工具卡。不要在小标题下塞完整说明书。
- 新增或修改中文文案时同步更新英文 i18n；英文不要机器直译，要保持同样的信息密度和语气。

## API 和服务层

- 项目后台 API 使用统一 envelope，由 `services/http.ts` 解包。
- Yggdrasil 协议接口保持协议响应格式，不要套项目 envelope。
- 服务层按领域命名：`authService`、`externalAuthService`、`yggdrasilService`、`admin*Service`。
- 安全敏感字段不要进日志、toast 或可见错误文本。

## i18n

当前 i18n 仍是单 namespace 合并模式，文件在 `src/i18n/locales/{zh-CN,en-US}/`。用户工作区文案放在 `account.json`，不要再新增 `dashboard.json` 或 `dashboard.*` key。

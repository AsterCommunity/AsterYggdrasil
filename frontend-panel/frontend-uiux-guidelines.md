# AsterYggdrasil 前端 UI/UX 规范

本文定义 AsterYggdrasil 前端的产品体验、视觉层级、交互规则和工程验收标准。它服务于前端结构迁移及之后的全部页面改动。

机器门禁：本仓库使用 react-doctor 做组件级静态检查，豁免配置见 [`doctor.config.ts`](./doctor.config.ts)，每条豁免必须带理由。本文与 `AGENTS.md` 的分工：这里放 UI/UX 规则本体，`AGENTS.md` 保留工程命令、API/i18n 契约和目录边界。

## 0. 二分范式：玩家偏好区 vs 应用管理区（先读这节）

Yggdrasil 是 BlessingSkin 传统的 Minecraft 皮肤站。这个产品有**两种并存且都合法的界面范式**，不要试图用一套通用 app 模板覆盖全站：

|      | 用户面（玩家偏好型）                                                         | 管理区（应用模式）                                     |
| ---- | ---------------------------------------------------------------------------- | ------------------------------------------------------ |
| 范围 | 公开入口 `/`、登录/注册/初始化、账户区 `/account/**`、公开材质库             | `/admin/**` 全部                                       |
| 气质 | 玩家偏好站点：有品牌氛围、有视觉叙事、绿色 Minecraft 识别、允许有性格的 hero | 运营工作台：可扫描、可重复操作、信息密度中高、视觉克制 |
| 参考 | BlessingSkin 系皮肤站的站点感                                                | AsterDrive 的后台形态                                  |
| 背景 | 允许氛围背景（图片、渐变、玻璃面板），但遵守第 8 节的 token 和动效规则       | 默认 `AdminSurface` 平铺，不做氛围背景                 |
| 密度 | 低到中，优先叙事和引导                                                       | 中到高，优先扫描和批量操作                             |
| 卡片 | 允许品牌化卡片，但不卡片套卡片                                               | 只用于真正分组和重复项                                 |

两条铁律：

1. **不把管理区的 app 模板反向强加给用户面。** 公开入口、衣柜、材质库不要变成 Drive 式的“工具面板 + 数据表”堆叠；那里的视觉叙事是产品要求，不是违规。
2. **不把用户面的品牌氛围带进管理区。** `/admin/**` 不放 hero 背景、装饰渐变和大图；管理区的“皮肤站识别”只通过绿色 accent 和品牌色 token 体现。

判断一个页面属于哪个范式，看路由：`/admin/**` 一律应用模式，其余一律玩家偏好模式。`/account/**` 在玩家偏好模式里偏“工具感”（它是玩家的工作台），但 hero、氛围背景和绿色识别仍然适用。

## 1. 产品气质

### 1.1 用户面（玩家偏好型）

关键词：

- 玩家偏好站点，不是营销官网，也不是管理后台
- 有 Minecraft / Yggdrasil 的绿色识别
- 有站点感：hero、氛围背景、品牌叙事允许存在
- 流程引导优先于数据展示
- 文案面向玩家工作流（“准备角色档案和材质，然后进入游戏”），不写成内部模块清单

避免：

- 营销页堆砌：多屏 hero、大卡片墙、CTA 轰炸。公开入口只做一屏 hero + 少量入口区 + 安静辅助区
- 把 hero、流程卡、快捷操作、审计记录做成同等重要的大卡片
- 大面积绿色底色（绿色是强调色，见第 8 节）
- emoji 做图标或标题装饰

### 1.2 管理区（应用模式）

关键词：

- 操作型、清楚、稳定
- 可扫描：字段分层明确，常看信息直接展示，低频操作收进详情
- 可重复操作：表格、筛选、批量动作语义一致
- 信息密度中高，但不拥挤

避免：

- 装饰性卡片、氛围背景、大图
- 把所有数据堆进一张表；字段变多时拆 detail dialog / detail panel
- 在列表里塞一排输入框、下拉框、开关；能展示态 + 详情编辑解决的不做行内编辑
- 交互元素数量失控；可点击区域做小逼用户点窄条

### 1.3 用户工作台 `/account` 的特殊地位

`/account` 是玩家工作台，介于站点与工具之间：

- 用户侧页面优先复用 `AppLayout` / `.app-shell` 的背景体系，不要每个页面再铺一套独立大背景；只有确实需要建立页面主视觉时，才在局部 hero 内做背景
- hero 承担欢迎和方向感，不堆统计（“角色档案数量”这类指标对玩家下一步行动没帮助，不进 hero）
- hero 可以用 Minecraft 相关图片作右侧背景，但必须是柔和的侧面渐变融合，不出现硬切边界；桌面和手机端都保持“文字侧到图片侧”的横向渐变，手机端不改成上下渐变；hero 图片只作氛围和识别，文字区域必须保持足够对比度
- “下一步”是流程导航，不是三张同权重大入口卡
- 快速操作是纯入口：裸 `Icon`（可放大到 `size-5`），不在 icon 外套背景方块
- 近期活动来自用户审计记录，是安静的辅助信息，不做成过重状态面板，也不让重复登录/刷新事件抢主视觉
- 工具区块（如 `LauncherSetupCard`）内部自足，不塞重复导航按钮

## 2. 操作入口分类法

所有按钮和入口必须能归入下面五类之一，位置跟着类别走。

### 2.1 全局操作

作用范围：整个站点或当前登录会话。

位置：ShellTopbar、用户菜单、主题/语言切换。

例子：主题切换、语言切换、退出登录、返回管理区。

### 2.2 导航入口

作用范围：切换当前位置。

位置：ShellSidebar（`shellNavigation.ts` 驱动）、面包屑、页面间 Link。

例子：工作台、角色档案、衣柜、材质库、管理分区导航。

规则：

- 用户面导航要表达玩家流程顺序（档案 → 材质 → 启动器 → 会话），不是后端模块清单
- 兼容 redirect（`/dashboard/*`、`/app/*`）不作为导航目标出现

### 2.3 当前页面操作

作用范围：当前页面或当前资源集合。

位置：页面工具条（搜索、筛选、刷新、上传入口）、PageHeader 右侧。

例子：衣柜搜索、材质标签筛选、刷新、上传材质、批量管理。

规则：

- 页面主操作最多一个 primary 按钮；其余用 outline / ghost
- 筛选控件（如标签筛选 Popover）归这里，不用 Dialog 承载轻量筛选

### 2.4 对象操作

作用范围：具体对象（材质、档案、会话、用户、任务）。

位置：卡片/行内按钮、对象详情面板、对象菜单。

例子：预览、下载、设为默认、改名、撤销会话、封禁、复核举报。

规则：

- 危险操作（删除档案、封禁、撤销他人会话）走 destructive 视觉 + 确认
- 对象菜单动作顺序稳定：查看 → 编辑/管理 → 危险操作在底部

### 2.5 危险操作

规则：

- destructive 视觉状态 + 明确确认（ConfirmDialog）
- 复杂 Dialog 内使用 inline confirmation，不嵌套第二层弹窗
- 文案必须说明影响对象和数量（“删除材质并释放存储”优于“删除”）
- 管理区危险操作必须留审计痕迹（后端职责，但文案要如实）

## 3. Dialog 规范

### 3.1 Dialog 类型

#### ConfirmDialog

简单确认。内容短，只有取消和确认，不承载搜索、编辑、分页。危险确认必须写明影响对象。

#### FormDialog

创建或编辑单个对象（档案改名、封禁设置、外部认证 provider 配置）。

- 字段分组清楚，错误靠近字段显示
- footer 固定，保存按钮提交中 loading
- 提交有 ref 门闩（见 3.3）

#### PickerDialog

选择目标（选择材质应用方向、选择标签集合超轻量时例外走 Popover）。

- 可搜索、当前选择明确、确认按钮在 footer

#### ManagerDialog

管理一组资源（标签库、封禁记录、会话列表升级版）。

- 搜索/过滤区固定在列表上方，列表独立滚动
- 支持分页或加载更多
- 行内编辑/删除用 inline confirmation，不嵌套确认弹窗

### 3.2 Footer 与滚动

- footer 不随内容滚动；header 不被滚走；搜索框不被列表滚走
- 主按钮右侧，移动端可整宽
- 滚动只发生在内容列表区域，body 和内部列表不同时滚
- 移动端 Dialog 尽量近全屏，header 固定、footer 固定、内容独立滚动
- 有未保存变更时保存按钮才可用；取消必须能明确放弃草稿

### 3.3 异步提交 ref 门闩（usePendingAction 契约）

创建、保存、确认、登录、上传等异步提交动作必须有**同步防重复提交门闩**。

规则：

- 不只依赖按钮 `disabled` 或 React `useState` 的 `submitting` 防重复提交；state 更新存在异步空窗，连续点击或回车仍可能重复进入
- 表单提交、按钮点击、键盘回车共享同一个 pending guard
- 单例提交用 ref-backed hook（`usePendingAction`，落地于共享 hooks 议题）；按资源 ID 操作用 `usePendingId` 形态
- guard 在进入异步动作前**同步置位**，在 `finally` 中释放
- UI 仍要展示 pending：禁用相关按钮、按钮内 spinner、替换文案
- 批量 key、跨列表刷新等额外语义可以保留局部 ref-backed 状态，但不退回只靠 `useState`

当前代码状态：门闩契约以本节为准；`usePendingAction` hook 由共享 hooks 议题统一落地，落地前新代码用 `useRef<boolean>` 同步置位模式，不许裸 `useState` 门闩。

## 4. 加载状态规则（Skeleton vs Spinner）

**列表和区域 loading 用 Skeleton；按钮和动作 loading 用 spinner。** 这是硬规则，不是偏好。

用 Skeleton（`@/components/ui/skeleton.tsx`，或 `SkeletonTable` 等组合件）：

- 列表、卡片网格、表格区域的首次加载
- 详情面板/区块的内容加载
- 图片占位（captcha 预览、材质大图、裁剪源图）
- 整页/整区过渡（路由 Suspense fallback 之外的页面内首次加载）

用 spinner（`Icon name="Spinner"` + `animate-spin`）：

- 按钮内的提交/刷新/保存进行态
- 行内动作（撤销、重试、测试连接）的进行态
- 搜索输入框内的即时查询进行态
- 有具体语义的短动作（“正在验证登录方式”），spinner 旁边可以带文案

不允许：

- 用一个居中 spinner + “加载中”文案占满列表/区域（这是被替换掉的旧模式）
- 给 Skeleton 加循环动画之外的解释文字；Skeleton 不说话，`aria-hidden` 由组件自己保证
- 区域已经渲染出结构后（刷新、翻页）整区闪 Skeleton；翻页保留旧内容 + 顶部 spinner 指示

## 5. 视觉规范

### 5.1 色彩 token 与公开区

全站颜色从主题 token 走。**禁止在组件里写 `bg-[#hex]`、`text-[#hex]`、`dark:bg-[#hex]` 这类硬编码颜色**——它绕过主题系统，让暗色模式失去覆盖面。

常规表面/文字用主题 token：`background`、`card`、`muted`、`border`、`popover`、`foreground`、`muted-foreground`、`accent`……（语义见 `src/index.css`）。

用户面有一层**公开区专属 token**，同样在 `src/index.css` 定义、亮暗两套值、跟随主题切换：

```text
--color-public-surface      公开区页面底色（浅绿灰/深绿黑）
--color-public-foreground   公开区主文字
--color-public-card         公开区玻璃卡（次要面板：nav 卡、说明小卡）
--color-public-card-strong  公开区主内容面板（法务正文、错误页侧栏）
--color-public-footer       公开区 footer 底色
```

组件里写作 `bg-public-surface`、`text-public-foreground`、`bg-public-card` 等。

### 5.2 品牌固定色（唯一允许的固定色）

有一类颜色**有意不随主题变化**——品牌识别色，例如 shell 侧栏的深绿底 `#0b271b`。这类颜色的处理方式：

1. 收口为 `src/index.css` 里的 CSS 变量（如 `--shell-sidebar`），亮暗同值，注释标明“品牌固定色，不随主题”
2. 组件通过注册后的 utility 引用（如 `bg-shell-sidebar`），不允许直接写 hex
3. 必须在本节登记：变量名、色值、使用位置、为什么不随主题

当前登记：

| 变量              | 色值      | 位置                                   | 理由                               |
| ----------------- | --------- | -------------------------------------- | ---------------------------------- |
| `--shell-sidebar` | `#0b271b` | `ShellSidebar`（移动/桌面/折叠三形态） | 深绿品牌底，亮暗模式同色是刻意识别 |

除本表登记项外，任何新的“固定色”需求都先问：它真的不跟随主题吗？表面/文字类颜色一律跟随主题，答案几乎总是“跟随”。

### 5.3 绿色纪律

- emerald/绿色是 Minecraft/Yggdrasil 强调色：图标、状态点、当前项、主按钮、关键 accent
- 大面积背景和卡片用 token，不用绿色铺底
- 暗色模式控制绿色面积，避免绿黑同色系把层级糊掉

### 5.4 间距、密度、字体

- 横向安全边距统一；shell 层边距收口在 shell 组件，页面不各写一套 magic spacing
- 管理区表格“分割线满宽，文本内容留边距”；容器 padding 优先用 `AdminSurface` 的 `padded` 开关而不是手写 `p-0`
- 字体不随 viewport 缩放；小面板内不用 hero 级标题；长名称 truncate；letter spacing 默认 0
- 卡片不套卡片；卡片只用于真正分组、工具面板和重复项

### 5.5 图标

- 统一走 `Icon` 组件（react-icons/pi），不手写 SVG，不用 emoji
- icon-only 按钮必须有 `aria-label`
- 普通行入口 icon `size-5`；按钮内图标随按钮尺寸；大型空状态可用 `size-8`+
- 入口列表、流程项、说明步骤的图标裸露展示；只有需要明确点击目标、徽章、头像占位或视觉分组时才加圆角背景

## 6. 公开区背景与暗色切换规则

公开区的氛围背景（渐变、图片叠加）是产品识别的一部分，但它的主题切换必须是平滑的。

### 6.1 唯一允许的机制：亮/暗双层 opacity 交叉淡入

**禁止在同一元素上用 `dark:bg-[linear-gradient(...)]`、`dark:bg-[url(...)]`、`dark:bg-[radial-gradient(...)]` 切换 `background-image`。** 渐变和图片无法平滑插值，主题切换时表现为硬切。

需要亮暗不同背景图/渐变时，用**两层元素、各持一套背景、用 opacity 交叉淡入**：

```tsx
{
	/* 亮色层 */
}
<div
	data-theme-surface="public-backdrop"
	className="absolute inset-0 bg-[linear-gradient(...亮色版...)] opacity-100 dark:opacity-0"
/>;
{
	/* 暗色层 */
}
<div
	data-theme-surface="public-backdrop"
	className="absolute inset-0 bg-[linear-gradient(...暗色版...)] opacity-0 dark:opacity-100"
/>;
```

要点：

- 两层都要挂 `data-theme-surface="public-backdrop"`，主题切换时 opacity 走 `index.css` 里 180ms 的过渡
- 两层内容完全一致时（如同底渐变）不需要双层；只有亮暗**内容不同**才拆两层
- 纯颜色（`background-color`）可以插值，永远用 token 切换，不需要双层

这条规则已被 react-doctor 和测试锚定（`PublicEntryShell.test.tsx` 断言 backdrop 层无 `dark:bg-[`）；review 时见到新的 `dark:bg-[linear-gradient|radial-gradient|url]` 一律打回。

### 6.2 公开区表面

公开区的可读面板（hero 上的卡、法务正文、错误页侧栏）用第 5.1 节的 public token，让主题系统真正覆盖公开区。面板上的文字对比度按 WCAG AA 检查亮暗两态。

## 7. 动效档位

动效用于说明状态变化和层级关系，不用于装饰，不用于掩盖信息架构问题。

| 场景                                             | 时长                       | 机制                                                                     |
| ------------------------------------------------ | -------------------------- | ------------------------------------------------------------------------ |
| 主题切换（颜色/边框/阴影 token 过渡）            | 160ms                      | `index.css` `theme-switching` + `data-theme-surface`                     |
| 公开区背景层亮暗交叉淡入                         | 180ms                      | `data-theme-surface="public-backdrop"` opacity 过渡                      |
| 浮层（Dialog/Popover/Menu/Select/Tooltip）入退场 | 100-150ms                  | `data-open`/`data-closed` + `animate-in/out` + `fade`/`zoom`             |
| Dialog/Sheet/侧栏                                | 100-220ms                  | 同上，复杂面板可到 240-300ms                                             |
| 常规 hover/focus/selected/disabled               | 120-180ms                  | 只过渡颜色、边框、阴影、透明度                                           |
| 高度/折叠类                                      | 展开 180-220ms，收起 160ms | `grid-template-rows` 模式（参考 `ErrorPage` 详情折叠），不用自然高度过渡 |
| 路由级进场                                       | 220-260ms                  | `app-route-transition` / `auth-card-transition` keyframes                |

规则：

- 浮层必须保留入退场动效；扁平化 Dialog 结构时不删 `data-open`、`data-closed`、`animate-in`、`animate-out`、`fade`、`zoom` 类
- 进入 `cubic-bezier(0.22, 1, 0.36, 1)` 或 `ease-out`，退出 `ease-in`；同类组件不要各自发明 easing
- hover/动效不允许改变布局尺寸；可以用背景、边框、阴影、透明度，谨慎用 transform
- 所有非必要动效支持 `prefers-reduced-motion`：Tailwind 类用 `motion-reduce:animate-none` / `motion-reduce:transition-none`，JS 动画读 `matchMedia("(prefers-reduced-motion: reduce)")` 后 duration 置 0
- 循环动画只用于 loading / 播放中 / 任务进行中；普通页面不用循环装饰动画
- 动画不能是唯一信息来源；动画消失后状态仍要由文本、图标、颜色或布局表达

## 8. 状态规范

每个重要组件考虑这些状态：

- loading（按第 4 节归位：区域 Skeleton / 动作 spinner）
- loading more（列表底部加载更多，保留已有内容）
- empty
- filtered empty（筛选/搜索无结果，与 empty 区分文案）
- error
- saving / submitting
- disabled
- selected / hover / focus
- destructive confirming

异步按钮规则：

- 提交中显示 loading 并禁用重复点击（配合 3.3 的 ref 门闩）
- 成功反馈用 toast 或局部状态；错误反馈不只写 console
- 空状态文案说明当前状态，不泛化（“没有匹配标签” ≠ “标签库为空” ≠ “还没输入条件”）

## 9. 可访问性规范

最低要求：

- 文本对比度 WCAG AA（亮暗两态都查）
- 所有交互元素有可见 focus（`focus-visible:ring-*`）
- icon-only 按钮有 `aria-label`，且文案走 i18n
- 键盘 tab 顺序符合视觉顺序
- Dialog/Popover 打开后焦点进入，Escape 关闭规则一致（Base UI 组件自带，不要手写 portal + 全局 keydown 重造）
- 表单字段有 label
- 不用 hover 作为唯一信息来源
- 浮层 trigger 带 `aria-haspopup` / `aria-expanded` / `aria-controls`（Base UI 的 Popover/Menu/Select 会自动接线；手写组件必须补齐）

## 10. 文案和 i18n

- 所有用户可见文本走 i18n；zh-CN 与 en-US 同步补，英文不机器直译，保持同等信息密度和语气
- 按钮文案表达动作结果（“移动到”“管理标签”），不写含糊动词
- 用户面文案面向玩家工作流，不写成内部模块清单
- 首页/工作台短描述要短；完整步骤说明放对应工具卡
- 危险操作确认文案必须包含影响对象或数量
- 不同上下文不复用含义不清的 key；新 key 放对应 locale 文件（`shell.json` 站点框架/通用、`account.json` 工作台、`admin*.json` 管理区、`public.json` 公开区）

## 11. 前端工程规范

代码约束：

- React 19 + Vite + TypeScript 当前栈；禁止 TS enum，用 `as const` 对象或 union type；类型导入 `import type`
- API 类型从 `@/types/api` 导入，不手写 `api.generated.ts` 的替代品
- 优先复用 `components/ui`、`components/common`、`lib` 下已有模块；新浮层用 Base UI 封装（`ui/dialog.tsx`、`ui/dropdown-menu.tsx`、`ui/select.tsx`），不手写 portal/定位/Escape
- 服务层策略（credentials、缓存、重试）不进组件；按 `AGENTS.md` 服务层边界

测试和验证：

- `cd frontend-panel && bun run check`；组件行为变更补/同步对应测试
- react-doctor 门禁：新增豁免必须带理由；违规修复不留豁免尾巴
- 关键 UI 变更至少检查 375px、768px、1024px、1440px
- 3D/canvas 组件（`StaticSkinPreview`、`MinecraftPreview`）必须做非空渲染与失败回退检查；普通 UI 不引入无关 3D

## 12. 新功能设计自查清单

提交或评审新前端功能前，先回答：

- 这个页面属于玩家偏好模式还是应用模式？规则用对了吗？
- 这个入口属于全局、导航、页面、对象还是危险操作？
- Dialog 是哪种类型？header/footer 固定了吗？滚动区域唯一吗？
- 异步提交有 ref 门闩吗？
- loading 归位了吗（区域 Skeleton / 动作 spinner）？empty 和 filtered empty 区分了吗？
- 新颜色是 token 吗？如果是有意固定色，登记进 5.2 了吗？
- 有没有 `dark:bg-[...]` 切背景图？有就拆双层 opacity
- 浮层用的是 Base UI 封装吗？trigger 的 aria 属性在吗？
- 动效在档位表内吗？支持 `prefers-reduced-motion` 吗？
- icon-only 按钮有 `aria-label` 吗？文案走 i18n 了吗？
- 这个组件复用了已有模式，还是新造了一次性布局？

答不上来就先不要写 UI。先把交互层级想清楚，代码自然会少很多。

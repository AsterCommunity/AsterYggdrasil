export const USER_PAGE_SIZE_OPTIONS = [10, 20, 50] as const;
export const DEFAULT_USER_PAGE_SIZE = 20 as const;

export type UserFilterValue<T extends string> = "__all__" | T;

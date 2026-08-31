import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

/**
 * 函数 `cn`
 *
 * 作者: gaohongshun
 *
 * 时间: 2026-04-02
 *
 * # 参数
 * - inputs: 参数 inputs
 *
 * # 返回
 * 返回函数执行结果
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

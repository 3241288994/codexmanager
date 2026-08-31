import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // 暂时禁用 Beta 版编译器以确保稳定性
  reactCompiler: false,
  experimental: {
    staleTimes: {
      dynamic: 30,
      static: 300,
    },
  },
  // 桌面开发态不展示右下角 Next 渲染指示器，避免用户误判为页面卡顿。
  devIndicators: false,
  // Tauri 开发态通过 127.0.0.1 加载 Next 资源，显式放行避免 dev 跨源告警。
  allowedDevOrigins: ["127.0.0.1", "[::1]"],
  output: 'export',
  // 中文注释：导出静态站点时强制 trailing slash，生成 /xxx/index.html，避免 Tauri 打包后导航丢失。
  trailingSlash: true,
  images: {
    unoptimized: true,
  },
};

export default nextConfig;
